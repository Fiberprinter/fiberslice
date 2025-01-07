mod settings;

use command_pass::{CommandPass, OptimizePass, SlowDownLayerPass};
use glam::Vec3;
use mask::ObjectMask;
use plotter::{convert_objects_into_moves, polygon_operations::PolygonOperations};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
pub use settings::*;
use shared::{process::Process, SliceInput};
use slice_pass::*;
use strum_macros::{EnumIter, EnumString};
use tower::create_towers;

mod calculation;
mod command_pass;
mod error;
pub mod gcode;
mod mask;
mod r#move;
mod optimizer;
mod plotter;
mod slice_pass;
mod slicing;
mod tower;
mod utils;
mod warning;

pub use gcode::SlicedGCode;
pub use mask::Mask;

pub use r#move::*;

use error::SlicerErrors;
use geo::{
    Contains, Coord, LineString, MultiLineString, MultiPolygon, Polygon, SimplifyVw,
    SimplifyVwPreserve,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct SliceResult {
    pub moves: Vec<Command>,
    pub calculated_values: CalculatedValues,
    pub settings: Settings,
}

pub fn slice(
    input: SliceInput<Mask>,
    settings: &Settings,
    process: &Process,
) -> Result<SliceResult, SlicerErrors> {
    let max = input
        .objects
        .iter()
        .fold(Vec3::NEG_INFINITY, |max, obj| max.max(obj.min_max().1));

    process.set_task("Creating Towers".to_string());
    process.set_progress(0.1);

    let mut masks: Vec<mask::ObjectMask> = input
        .masks
        .into_iter()
        .map(|mask| mask.into_object(max, settings))
        .try_collect()?;

    let towers = create_towers(&input.objects)?;

    process.set_task("Slicing".to_string());
    process.set_progress(0.2);
    // println!("Max: {:?}", max);

    let mut objects = slicing::slice(&towers, max.z, settings)?;

    process.set_task("Cropping Masks".to_string());
    process.set_progress(0.5);
    masks.iter_mut().for_each(|mask| {
        mask.crop(&objects, max);

        if mask.mask_settings().epsilon.abs() > f32::EPSILON {
            mask.randomize_mask_underlaps(mask.mask_settings().epsilon);
        }
    });

    generate_mask_moves(&mut masks, settings, process)?;

    masks.iter_mut().for_each(|mask| {
        // let settings = mask
        //     .mask_settings()
        //     .clone()
        //     .combine_settings(settings.clone());

        mask.layers.iter_mut().for_each(|_layer| {
            // dispatch_fiber_moves(&mut layer.chains, &settings);
            // dispatch_fiber_moves(&mut layer.fixed_chains, &settings);
        });
    });

    combine_mask_moves(&mut objects, masks);

    let mut moves = generate_moves(objects, settings, process)?;

    process.set_task("Optimizing".to_string());
    process.set_progress(0.6);
    OptimizePass::pass(&mut moves, settings);

    process.set_task("Slowing Down Layers".to_string());
    process.set_progress(0.7);
    SlowDownLayerPass::pass(&mut moves, settings);

    MergeFiberPass::pass(&mut moves, settings);

    EvalIdPass::pass(&mut moves, settings);

    process.set_task("Calculating Values".to_string());
    process.set_progress(0.75);

    let calculated_values = calculation::calculate_values(&moves, settings);

    Ok(SliceResult {
        moves,
        calculated_values,
        settings: settings.clone(),
    })
}

fn combine_mask_moves(objects: &mut Vec<Object>, mut masks: Vec<ObjectMask>) {
    for object in objects.iter_mut() {
        object
            .layers
            .iter_mut()
            .enumerate()
            .for_each(|(index, layer)| {
                for mask in masks.iter_mut() {
                    if let Some(mask_layer) = mask.layers.get_mut(index) {
                        layer.remaining_area = layer
                            .remaining_area
                            .difference_with(&mask_layer.main_polygon);
                        layer.chains.append(&mut mask_layer.chains);
                    }
                }
            });
    }
}

fn generate_moves(
    mut objects: Vec<Object>,
    settings: &Settings,
    process: &Process,
) -> Result<Vec<Command>, SlicerErrors> {
    //Creates Support Towers
    process.set_task("Creating Support Towers".to_string());
    process.set_progress(0.3);
    SupportTowerPass::pass(&mut objects, settings);

    //Adds a skirt
    process.set_task("Creating Skirt".to_string());
    SkirtPass::pass(&mut objects, settings);

    //Adds a brim
    process.set_task("Creating Brim".to_string());
    BrimPass::pass(&mut objects, settings);

    process.set_task("Generate Moves".to_string());
    let v: Result<Vec<()>, SlicerErrors> = objects
        .par_iter_mut()
        .map(|object| {
            let slices = &mut object.layers;

            //Shrink layer
            ShrinkPass::pass(slices, settings)?;

            //Handle Perimeters
            WallPass::pass(slices, settings)?;

            //Handle Bridging
            BridgingPass::pass(slices, settings)?;

            //Handle Top Layer
            TopLayerPass::pass(slices, settings)?;

            //Handle Top And Bottom Layers
            TopAndBottomLayersPass::pass(slices, settings)?;

            //Handle Support
            SupportPass::pass(slices, settings)?;

            FiberInfillPass::pass(slices, settings)?;

            //Lightning Infill
            LightningFillPass::pass(slices, settings)?;

            //Fill Remaining areas
            FillAreaPass::pass(slices, settings)?;

            //Order the move chains
            OrderPass::pass(slices, settings)
        })
        .collect();

    process.set_progress(0.5);

    v?;

    Ok(convert_objects_into_moves(objects, settings))
}

fn generate_mask_moves(
    masks: &mut Vec<ObjectMask>,
    settings: &Settings,
    process: &Process,
) -> Result<(), SlicerErrors> {
    let v: Result<Vec<()>, SlicerErrors> = masks
        .par_iter_mut()
        .map(|object| {
            let settings = &object
                .mask_settings()
                .clone()
                .combine_settings(settings.clone());

            let slices = &mut object.layers;

            //Shrink layer
            ShrinkPass::pass(slices, settings)?;

            //Handle Perimeters
            // PerimeterPass::pass(slices, settings)?;

            //Handle Bridging
            BridgingPass::pass(slices, settings)?;

            //Handle Top Layer
            TopLayerPass::pass(slices, settings)?;

            //Handle Top And Bottom Layers
            TopAndBottomLayersPass::pass(slices, settings)?;

            //Lightning Infill
            LightningFillPass::pass(slices, settings)?;

            //Fill Remaining areas
            FillAreaPass::pass(slices, settings)?;

            //Order the move chains
            OrderPass::pass(slices, settings)
        })
        .collect();

    process.set_progress(0.5);

    v?;

    Ok(())
}

#[derive(Debug)]
///A single slice of an object containing it's current plotting status.
pub struct Slice {
    ///The slice's entire polygon. Should not be modified after creation by the slicing process.
    pub main_polygon: MultiPolygon<f32>,

    ///The slice's remaining area that needs to be processes. Passes will slowly subtract from this until finally infill will fill the space.
    pub remaining_area: MultiPolygon<f32>,

    /// The area that will be filled by support interface material.
    pub support_interface: Option<MultiPolygon<f32>>,

    ///The area that will be filled by support towers
    pub support_tower: Option<MultiPolygon<f32>>,

    ///Theses moves ares applied in order and the start of the commands for the slice.
    pub fixed_chains: Vec<MoveChain>,

    ///The move chains generaated by various passses. These chains can be reordered by the optomization process to create faster commands.
    pub chains: Vec<MoveChain>,

    ///The lower height of this slice.
    pub bottom_height: f32,

    ///The upper height of tis slice.
    pub top_height: f32,

    ///A copy of this layers settings
    pub layer_settings: LayerSettings,

    pub layer: usize,
}
impl Slice {
    ///Creates a slice from a spefic iterator of points
    pub fn from_single_point_loop<I>(
        line: I,
        bottom_height: f32,
        top_height: f32,
        layer: usize,
        settings: &Settings,
    ) -> Self
    where
        I: Iterator<Item = (f32, f32)>,
    {
        let polygon = Polygon::new(LineString::from_iter(line), vec![]);

        let layer_settings = settings.get_layer_settings(layer, (bottom_height + top_height) / 2.0);

        Slice {
            main_polygon: MultiPolygon(vec![polygon.simplify_vw_preserve(&0.01)]),
            remaining_area: MultiPolygon(vec![polygon]),
            support_interface: None,
            support_tower: None,
            fixed_chains: vec![],
            chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
            layer,
        }
    }

    ///creates a slice from  a multi line string
    pub fn from_multiple_point_loop(
        lines: MultiLineString<f32>,
        bottom_height: f32,
        top_height: f32,
        layer: usize,
        settings: &Settings,
    ) -> Result<Self, SlicerErrors> {
        let mut lines_and_area: Vec<(LineString<f32>, f32)> = lines
            .into_iter()
            .map(|line| {
                let area: f32 = line
                    .clone()
                    .into_points()
                    .iter()
                    .circular_tuple_windows::<(_, _)>()
                    .map(|(p1, p2)| (p1.x() + p2.x()) * (p2.y() - p1.y()))
                    .sum();
                (line, area)
            })
            .filter(|(_, area)| area.abs() > 0.0001)
            .collect();

        lines_and_area
            .sort_by(|(_l1, a1), (_l2, a2)| a2.partial_cmp(a1).expect("Areas should not be NAN"));
        let mut polygons = vec![];

        for (line, area) in lines_and_area {
            if area > 0.0 {
                polygons.push(Polygon::new(line.clone(), vec![]));
            } else {
                //counter clockwise interior polygon
                let smallest_polygon = polygons
                    .iter_mut()
                    .rev()
                    .find(|poly| poly.contains(&line.0[0]))
                    .ok_or(SlicerErrors::SliceGeneration)?;
                smallest_polygon.interiors_push(line);
            }
        }

        let multi_polygon: MultiPolygon<f32> = MultiPolygon(polygons);

        let layer_settings = settings.get_layer_settings(layer, (bottom_height + top_height) / 2.0);

        Ok(Slice {
            main_polygon: multi_polygon.simplify_vw(&0.001),
            remaining_area: multi_polygon.simplify_vw(&0.001),
            support_interface: None,
            support_tower: None,
            chains: vec![],
            fixed_chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
            layer,
        })
    }

    ///return the reference height of the slice
    pub fn get_height(&self) -> f32 {
        (self.bottom_height + self.top_height) / 2.0
    }
}

///Types of solid infill
#[derive(Clone, Copy, Debug, PartialEq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum SolidInfillTypes {
    ///Back and forth lines to fill polygons, Rotating 120 degree each layer
    Rectilinear,

    ///Back and forth lines to fill polygons, rotating custom degrees each layer
    RectilinearCustom(f32),
}

///Types of partial infill
#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum PartialInfillTypes {
    ///Back and forth spaced lines to fill polygons
    Linear,

    ///Back and forth spaced lines to fill polygons and there perpendicular lines
    Rectilinear,

    /// Lines in 3 directions to form tessellating triangle pattern
    Triangle,

    /// Creates a 3d cube structure.
    Cubic,

    ///Creates lightning shaped infill that retracts into the print walls
    Lightning,
}

#[derive(Debug)]
///A object is the collection of slices for a particular model.
pub struct Object {
    /// The slices for this model sorted from lowest to highest.
    pub layers: Vec<Slice>,
}

///Calculated values about an entire print
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CalculatedValues {
    ///Total plastic used by the print in mm^3
    pub plastic_volume: f32,

    ///Total plastic used by the print in grams
    pub plastic_weight: f32,

    ///Total plastic used by the print in mm of filament
    pub plastic_length: f32,

    pub fiber_length: f32,

    ///Total time to print in seconds
    pub total_time: f32,
}

impl CalculatedValues {
    ///Returns total time converted to hours, minutes, seconds, and remaining fractional seconds
    pub fn get_hours_minutes_seconds_fract_time(&self) -> (usize, usize, usize, f32) {
        let total_time = self.total_time.floor() as usize;

        let fract = self.total_time - total_time as f32;
        (
            total_time / 3600,
            (total_time % 3600) / 60,
            total_time % 60,
            fract,
        )
    }
}
