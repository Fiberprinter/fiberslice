use crate::plotter::support::Supporter;

use crate::error::SlicerErrors;
use crate::plotter::lightning_infill::lightning_infill;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::plotter::Plotter;
use crate::settings::Settings;
use crate::{MoveType, Object, PartialInfillTypes, Slice, TraceType};
use geo::prelude::*;
use geo::*;
use log::info;
use rayon::prelude::*;

#[derive(Debug)]
pub struct PassContext {
    fiber: bool,
    subtract: bool,
}

impl PassContext {
    pub fn new() -> Self {
        Self {
            fiber: false,
            subtract: true,
        }
    }

    pub fn with_fiber(self) -> Self {
        Self {
            fiber: true,
            ..self
        }
    }

    pub fn without_fiber(self) -> Self {
        Self {
            fiber: false,
            ..self
        }
    }

    pub fn subtract(self) -> Self {
        Self {
            subtract: true,
            ..self
        }
    }

    pub fn no_subtract(self) -> Self {
        Self {
            subtract: false,
            ..self
        }
    }

    pub fn is_subtract_pass(&self) -> bool {
        self.subtract
    }

    pub fn move_from_trace_type(&self, trace_type: TraceType) -> MoveType {
        if self.fiber {
            MoveType::WithFiber(trace_type)
        } else {
            MoveType::WithoutFiber(trace_type)
        }
    }

    pub fn is_fiber_pass(&self) -> bool {
        self.fiber
    }
}

pub trait ObjectPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings);
}

pub struct BrimPass {}

impl ObjectPass for BrimPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings) {
        if settings.brim_width.is_enabled() {
            let width = *settings.brim_width;

            // display_state_update("Generating Moves: Brim", send_messages);
            //Add to first object

            let first_layer_multipolygon: MultiPolygon<f32> = MultiPolygon(
                objects
                    .iter()
                    .flat_map(|poly| {
                        let first_slice = poly.layers.first().expect("Object needs a Slice");

                        first_slice
                            .main_polygon
                            .0
                            .clone()
                            .into_iter()
                            .chain(first_slice.main_polygon.clone())
                    })
                    .collect(),
            );

            objects
                .get_mut(0)
                .expect("Needs an object")
                .layers
                .get_mut(0)
                .expect("Object needs a Slice")
                .generate_brim(first_layer_multipolygon, width);
        }
    }
}

pub struct SupportTowerPass {}

impl ObjectPass for SupportTowerPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings) {
        if settings.support.is_enabled() {
            let support = &settings.support;

            // display_state_update("Generating Support Towers", send_messages);
            //Add to first object

            objects.par_iter_mut().for_each(|obj| {
                (1..obj.layers.len()).rev().for_each(|q| {
                    //todo Fix this, it feels hacky
                    if let [ref mut layer, ref mut above, ..] = &mut obj.layers[q - 1..=q] {
                        layer.add_support_polygons(above, support);
                    } else {
                        unreachable!()
                    }
                });
            });
        }
    }
}

pub struct SkirtPass {}

impl ObjectPass for SkirtPass {
    fn pass(objects: &mut Vec<Object>, settings: &Settings) {
        //Handle Walls

        if settings.skirt.is_enabled() {
            let skirt = &settings.skirt;

            // display_state_update("Generating Moves: Skirt", send_messages);
            let convex_hull = objects
                .iter()
                .flat_map(|object| {
                    object
                        .layers
                        .iter()
                        .take(skirt.layers)
                        .map(|m| m.main_polygon.union_with(&m.get_support_polygon()))
                })
                .fold(MultiPolygon(vec![]), |a, b| a.union_with(&b))
                .convex_hull();

            //Add to first object
            objects
                .get_mut(0)
                .expect("Needs an object")
                .layers
                .iter_mut()
                .take(skirt.layers)
                .for_each(|slice| slice.generate_skirt(&convex_hull, skirt, settings))
        }
    }
}

pub trait SlicePass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors>;
}

pub struct ShrinkPass {}

impl SlicePass for ShrinkPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) -> Result<(), SlicerErrors> {
        // display_state_update("Generating Moves: Shrink Layers", send_messages);
        slices.par_iter_mut().for_each(|slice| {
            slice.shrink_layer();
        });

        Ok(())
    }
}

pub struct WallPass {}

impl SlicePass for WallPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors> {
        // display_state_update("Generating Moves: Walls", send_messages);
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, slice)| {
                slice.slice_walls_into_chains(settings.number_of_perimeters, layer_num);
            });
        Ok(())
    }
}

pub struct BridgingPass {}

impl SlicePass for BridgingPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) -> Result<(), SlicerErrors> {
        // display_state_update("Generating Moves: Bridging", send_messages);
        (1..slices.len()).for_each(|q| {
            let below = slices[q - 1].main_polygon.clone();

            slices[q].fill_solid_bridge_area(&below, &PassContext::new().without_fiber());
        });
        Ok(())
    }
}
pub struct TopLayerPass {}

impl SlicePass for TopLayerPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) -> Result<(), SlicerErrors> {
        // display_state_update("Generating Moves: Top Layer", send_messages);
        (0..slices.len() - 1).for_each(|q| {
            let above = slices[q + 1].main_polygon.clone();

            slices[q].fill_solid_top_layer(&above, q, &PassContext::new().without_fiber());
        });
        Ok(())
    }
}

pub struct TopAndBottomLayersPass {}

impl SlicePass for TopAndBottomLayersPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors> {
        let top_layers = settings.top_layers;
        let bottom_layers = settings.bottom_layers;

        //Make sure at least 1 layer will not be solid
        if slices.len() > bottom_layers + top_layers {
            // display_state_update("Generating Moves: Above and below support", send_messages);

            (bottom_layers..slices.len() - top_layers).for_each(|q| {
                let below = if bottom_layers != 0 {
                    Some(
                        slices[(q - bottom_layers + 1)..q]
                            .iter()
                            .map(|m| m.main_polygon.clone())
                            .fold(
                                slices
                                    .get(q - bottom_layers)
                                    .expect("Bounds Checked above")
                                    .main_polygon
                                    .clone(),
                                |a, b| a.intersection_with(&b),
                            ),
                    )
                } else {
                    None
                };
                let above = if top_layers != 0 {
                    Some(
                        slices[q + 1..q + top_layers + 1]
                            .iter()
                            .map(|m| m.main_polygon.clone())
                            .fold(
                                slices
                                    .get(q + 1)
                                    .expect("Bounds Checked above")
                                    .main_polygon
                                    .clone(),
                                |a, b| a.intersection_with(&b),
                            ),
                    )
                } else {
                    None
                };
                if let Some(intersection) = match (above, below) {
                    (None, None) => None,
                    (None, Some(poly)) | (Some(poly), None) => Some(poly),
                    (Some(polya), Some(polyb)) => Some(polya.intersection_with(&polyb)),
                } {
                    slices
                        .get_mut(q)
                        .expect("Bounds Checked above")
                        .fill_solid_subtracted_area(
                            &intersection,
                            q,
                            &PassContext::new().without_fiber(),
                        );
                }
            });
        }

        let slice_count = slices.len();

        slices
            .par_iter_mut()
            .enumerate()
            .filter(|(layer_num, _)| {
                *layer_num < settings.bottom_layers
                    || settings.top_layers + *layer_num + 1 > slice_count
            })
            .for_each(|(layer_num, slice)| {
                slice.fill_remaining_area(true, layer_num, &PassContext::new().without_fiber());
            });
        Ok(())
    }
}

pub struct SupportPass {}

impl SlicePass for SupportPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors> {
        if settings.support.is_enabled() {
            let support = &settings.support;

            for slice in slices.iter_mut() {
                slice.fill_support_polygons(support);
            }
        }
        Ok(())
    }
}

pub struct FiberPass {}

impl SlicePass for FiberPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors> {
        if settings.fiber.infill.is_enabled() {
            //Fill all remaining areas
            let width = settings.fiber.infill.width;
            let spacing = settings.fiber.infill.spacing;
            let cycle_length = width + spacing;

            slices
                .par_iter_mut()
                .enumerate()
                .for_each(|(layer_num, slice)| {
                    if ((layer_num + 1) % cycle_length) < spacing {
                        if !settings.fiber.infill.air_spacing {
                            slice.fill_remaining_area(
                                false,
                                layer_num,
                                &PassContext::new().without_fiber().no_subtract(),
                            );
                        }
                    } else {
                        slice.fill_remaining_area(
                            false,
                            layer_num,
                            &PassContext::new().with_fiber().no_subtract(),
                        );
                    }

                    info!("Fiber Infill: {:?}", layer_num);
                });
        }

        Ok(())
    }
}

pub struct FillAreaPass {}

impl SlicePass for FillAreaPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors> {
        // display_state_update("Generating Moves: Fill Areas", send_messages);

        let width = settings.fiber.infill.width;
        let spacing = settings.fiber.infill.spacing;
        let cycle_length = width + spacing;

        //Fill all remaining areas
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, slice)| {
                if settings.fiber.infill.is_enabled() && settings.fiber.infill.solid_infill {
                    let fiber = if ((layer_num + 1) % cycle_length) < spacing {
                        false
                    } else {
                        true
                    };

                    slice.fill_remaining_area(
                        fiber,
                        layer_num,
                        &PassContext::new().without_fiber(),
                    );
                } else {
                    slice.fill_remaining_area(
                        false,
                        layer_num,
                        &PassContext::new().without_fiber(),
                    );
                }
            });
        Ok(())
    }
}
pub struct LightningFillPass {}

impl SlicePass for LightningFillPass {
    fn pass(slices: &mut Vec<Slice>, settings: &Settings) -> Result<(), SlicerErrors> {
        if settings.partial_infill_type == PartialInfillTypes::Lightning {
            // display_state_update("Generating Moves: Lightning Infill", send_messages);

            lightning_infill(slices);
        }
        Ok(())
    }
}

pub struct OrderPass {}

impl SlicePass for OrderPass {
    fn pass(slices: &mut Vec<Slice>, _settings: &Settings) -> Result<(), SlicerErrors> {
        // display_state_update("Generating Moves: Order Chains", send_messages);

        //Fill all remaining areas
        slices.par_iter_mut().for_each(|slice| {
            slice.order_chains();
        });
        Ok(())
    }
}
