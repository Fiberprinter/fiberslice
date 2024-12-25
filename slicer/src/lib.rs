mod settings;

use command_pass::{CommandPass, OptimizePass, SlowDownLayerPass};
use dispatcher::dispatch_fiber_moves;
use glam::{Vec3, Vec4};
use mask::ObjectMask;
use plotter::{convert_objects_into_moves, polygon_operations::PolygonOperations};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
pub use settings::*;
use shared::{process::Process, SliceInput};
use slice_pass::*;
use strum_macros::{EnumCount, EnumIter, EnumString};
use tower::create_towers;

mod calculation;
mod command_pass;
mod converter;
mod dispatcher;
mod error;
mod mask;
mod optimizer;
mod plotter;
mod slice_pass;
mod slicing;
mod tower;
mod utils;
mod warning;

pub use converter::convert;
pub use mask::Mask;

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
        mask.randomize_mask_underlaps(15.0);
    });

    generate_mask_moves(&mut masks, settings, process)?;

    masks.iter_mut().for_each(|mask| {
        let settings = mask
            .mask_settings()
            .clone()
            .combine_settings(settings.clone());

        mask.layers.iter_mut().for_each(|layer| {
            dispatch_fiber_moves(&mut layer.chains, &settings);
            dispatch_fiber_moves(&mut layer.fixed_chains, &settings);
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
            PerimeterPass::pass(slices, settings)?;

            //Handle Bridging
            BridgingPass::pass(slices, settings)?;

            //Handle Top Layer
            TopLayerPass::pass(slices, settings)?;

            //Handle Top And Bottom Layers
            TopAndBottomLayersPass::pass(slices, settings)?;

            //Handle Support
            SupportPass::pass(slices, settings)?;

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
            main_polygon: multi_polygon.clone(),
            remaining_area: multi_polygon.simplify_vw(&0.0001),
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

///A move of the plotter
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    ///The end Coordinate of the Move. The start of the move is the previous moves end point.
    pub end: Coord<f32>,
    ///The width of plastic to extrude for this move
    pub width: f32,
    ///The type of move
    pub move_type: MoveType,
}

#[derive(Debug)]
/// A chain of moves that should happen in order
pub struct MoveChain {
    ///start point for the chain of moves. Needed as Moves don't contain there own start point.
    pub start_point: Coord<f32>,

    ///List of all moves in order that they must be moved
    pub moves: Vec<Move>,

    ///Indicates that chain is a loop where the start can be changed to any point
    pub is_loop: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, EnumCount)]
pub enum MovePrintType {
    Unkown,
    ///The top later of infill
    TopSolidInfill,

    ///Solid Infill
    SolidInfill,

    ///Standard Partial infill
    Infill,

    ///The exterior surface Layer of perimeters
    WallOuter,

    ///The interior surface Layer of perimeters
    WallInner,

    ///The exterior inner Layer of perimeters
    InteriorWallOuter,

    ///The interior inner Layer of perimeters
    InteriorWallInner,

    ///A bridge over open air
    Bridging,

    ///Support towers and interface
    Support,
}

impl std::fmt::Display for MovePrintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MovePrintType::Unkown => write!(f, "Unkown"),
            MovePrintType::TopSolidInfill => write!(f, "Top Solid Infill"),
            MovePrintType::SolidInfill => write!(f, "Solid Infill"),
            MovePrintType::Infill => write!(f, "Infill"),
            MovePrintType::WallOuter => write!(f, "Wall Outer"),
            MovePrintType::WallInner => write!(f, "Wall Inner"),
            MovePrintType::InteriorWallOuter => write!(f, "Wall Inner"),
            MovePrintType::InteriorWallInner => write!(f, "Interior Inner Perimeter"),
            MovePrintType::Bridging => write!(f, "Bridging"),
            MovePrintType::Support => write!(f, "Support"),
        }
    }
}

impl MovePrintType {
    pub fn into_color_vec4(&self) -> Vec4 {
        match self {
            MovePrintType::Unkown => Vec4::new(0.0, 0.0, 0.0, 1.0),
            MovePrintType::TopSolidInfill => Vec4::new(1.0, 0.0, 0.0, 1.0),
            MovePrintType::SolidInfill => Vec4::new(1.0, 0.0, 0.0, 1.0),
            MovePrintType::Infill => Vec4::new(0.0, 0.0, 1.0, 1.0),
            MovePrintType::WallOuter => Vec4::new(1.0, 1.0, 0.0, 1.0),
            MovePrintType::WallInner => Vec4::new(1.0, 1.0, 0.0, 1.0),
            MovePrintType::InteriorWallOuter => Vec4::new(1.0, 1.0, 0.0, 1.0),
            MovePrintType::InteriorWallInner => Vec4::new(1.0, 1.0, 0.0, 1.0),
            MovePrintType::Bridging => Vec4::new(0.0, 1.0, 1.0, 1.0),
            MovePrintType::Support => Vec4::new(1.0, 1.0, 0.0, 1.0),
        }
    }
}

///Types of Moves
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum MoveType {
    WithFiber(MovePrintType),
    WithoutFiber(MovePrintType),
    ///Standard travel moves without extrusion
    Travel,
}

impl MoveType {
    pub fn from_type(print_type: MovePrintType, fiber: bool) -> Self {
        if fiber {
            MoveType::WithFiber(print_type)
        } else {
            MoveType::WithoutFiber(print_type)
        }
    }

    pub fn print_type(&self) -> Option<MovePrintType> {
        match self {
            MoveType::WithFiber(print_type) => Some(*print_type),
            MoveType::WithoutFiber(print_type) => Some(*print_type),
            _ => None,
        }
    }
}

///The intermediate representation of the commands to send to the printer. The commands will be optimized organized and converted into the output expected ( for example GCode)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Command {
    ///Move to a specific location without extrusion
    MoveTo {
        ///The end point of the move
        end: Coord<f32>,
    },
    ///Move to a location while extruding plastic
    MoveAndExtrude {
        ///Start point of the move
        start: Coord<f32>,

        ///End point of the move
        end: Coord<f32>,

        ///The height thickness of the move
        thickness: f32,

        /// The extrusion width
        width: f32,
    },
    MoveAndExtrudeFiber {
        ///Start point of the move
        start: Coord<f32>,

        ///End point of the move
        end: Coord<f32>,

        ///The height thickness of the move
        thickness: f32,

        /// The extrusion width
        width: f32,
    },
    ///Change the layer height
    LayerChange {
        ///The height the print head should move to
        z: f32,

        ///The layer index of this move
        index: usize,
    },

    ///Sets the System state to the new values
    SetState {
        ///The new state to change into
        new_state: StateChange,
    },

    ///A fixed duration delay
    Delay {
        ///Number of milliseconds to delay
        msec: u64,
    },

    ///An arc move of the extruder
    Arc {
        ///start point of the arc
        start: Coord<f32>,

        ///end point of the arc
        end: Coord<f32>,

        ///The center point that the arc keeps equidistant from
        center: Coord<f32>,

        ///Whether the arc is clockwise or anticlockwise
        clockwise: bool,

        ///Thickness of the arc, the height
        thickness: f32,

        ///The width of the extrusion
        width: f32,
    },

    ///Change the object that is being printed
    ChangeObject {
        ///The index of the new object being changed to
        object: usize,
    },
    ChangeType {
        ///The new print type to change to
        print_type: MovePrintType,
    },
    ///Used in optimization , should be optimized out
    NoAction,
}

impl Command {
    pub fn needs_filament(&self) -> bool {
        match self {
            Command::MoveAndExtrude { .. } => true,
            Command::MoveAndExtrudeFiber { .. } => true,
            _ => false,
        }
    }
}

///A change in the state of the printer. all fields are optional and should only be set when the state is changing.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RetractionType {
    ///No retract
    NoRetract,

    ///Unretract
    Unretract,

    ///Standard Retract without Move
    Retract,

    ///MoveWhileRetracting
    ///Vector of (retraction amount, points to travel to)
    MoveRetract(Vec<(f32, Coord<f32>)>),
}

impl RetractionType {
    ///returns the retraction type of self or if it's no retraction the other retraction type
    /// See Options or function
    #[must_use]
    pub fn or(self, rtb: RetractionType) -> RetractionType {
        match self {
            RetractionType::NoRetract => rtb,
            RetractionType::Unretract => RetractionType::Unretract,
            RetractionType::Retract => RetractionType::Retract,
            RetractionType::MoveRetract(m) => RetractionType::MoveRetract(m),
        }
    }
}

impl Default for RetractionType {
    fn default() -> Self {
        RetractionType::NoRetract
    }
}

///A change in the state of the printer. all fields are optional and should only be set when the state is changing.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct StateChange {
    ///The temperature of the current extruder
    pub extruder_temp: Option<f32>,

    ///The temperature of the printing bed
    pub bed_temp: Option<f32>,

    ///The speed of the fan
    pub fan_speed: Option<f32>,

    ///The spped movement commands are performed at
    pub movement_speed: Option<f32>,

    ///The acceleration that movement commands are performed at
    pub acceleration: Option<f32>,

    ///Whether the filament is retracted
    pub retract: RetractionType,
}

impl StateChange {
    ///Change the current state to the new state and return the differences between the 2 states
    #[must_use]
    pub fn state_diff(&mut self, new_state: &StateChange) -> StateChange {
        StateChange {
            extruder_temp: {
                if self.extruder_temp == new_state.extruder_temp {
                    None
                } else {
                    self.extruder_temp = new_state.extruder_temp.or(self.extruder_temp);
                    new_state.extruder_temp
                }
            },
            bed_temp: {
                if self.bed_temp == new_state.bed_temp {
                    None
                } else {
                    self.bed_temp = new_state.bed_temp.or(self.bed_temp);
                    new_state.bed_temp
                }
            },
            fan_speed: {
                if self.fan_speed == new_state.fan_speed {
                    None
                } else {
                    self.fan_speed = new_state.fan_speed.or(self.fan_speed);
                    new_state.fan_speed
                }
            },
            movement_speed: {
                if self.movement_speed == new_state.movement_speed {
                    None
                } else {
                    self.movement_speed = new_state.movement_speed.or(self.movement_speed);
                    new_state.movement_speed
                }
            },
            acceleration: {
                if self.acceleration == new_state.acceleration {
                    None
                } else {
                    self.acceleration = new_state.acceleration.or(self.acceleration);
                    new_state.acceleration
                }
            },
            retract: {
                if self.retract == new_state.retract {
                    RetractionType::NoRetract
                } else if let RetractionType::MoveRetract(_m) = &self.retract {
                    if new_state.retract == RetractionType::Retract {
                        RetractionType::NoRetract
                    } else {
                        self.retract = new_state.retract.clone().or(self.retract.clone());
                        new_state.retract.clone()
                    }
                } else {
                    self.retract = new_state.retract.clone().or(self.retract.clone());
                    new_state.retract.clone()
                }
            },
        }
    }

    ///combine the 2 state changes into one, prioritizing the new state if both contain a file
    #[must_use]
    pub fn combine(&self, new_state: &StateChange) -> StateChange {
        StateChange {
            extruder_temp: { new_state.extruder_temp.or(self.extruder_temp) },
            bed_temp: { new_state.bed_temp.or(self.bed_temp) },
            fan_speed: { new_state.fan_speed.or(self.fan_speed) },
            movement_speed: { new_state.movement_speed.or(self.movement_speed) },
            acceleration: { new_state.acceleration.or(self.acceleration) },
            retract: { new_state.retract.clone().or(self.retract.clone()) },
        }
    }
}

impl MoveChain {
    ///Convert a move chain into a list of commands
    pub fn create_commands(self, settings: &LayerSettings, thickness: f32) -> Vec<Command> {
        let mut cmds = vec![];
        let mut current_print_type = None;

        let mut current_type = None;
        let mut current_loc = self.start_point;

        for m in self.moves {
            if Some(m.move_type) != current_type {
                match m.move_type {
                    MoveType::WithFiber(move_print_type) => {
                        update_state(&move_print_type, settings, &mut cmds)
                    }
                    MoveType::WithoutFiber(move_print_type) => {
                        update_state(&move_print_type, settings, &mut cmds)
                    }
                    MoveType::Travel => {
                        cmds.push(Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(settings.speed.travel),
                                acceleration: Some(settings.acceleration.travel),
                                retract: RetractionType::Retract,
                            },
                        });
                    }
                    _ => {}
                }

                current_type = Some(m.move_type);
            }

            match m.move_type {
                MoveType::WithFiber(print_type) => {
                    if Some(print_type) != current_print_type {
                        cmds.push(Command::ChangeType { print_type });
                        current_print_type = Some(print_type);
                    }

                    cmds.push(Command::MoveAndExtrudeFiber {
                        start: current_loc,
                        end: m.end,
                        thickness,
                        width: m.width,
                    });
                    current_loc = m.end;
                }
                MoveType::WithoutFiber(print_type) => {
                    if Some(print_type) != current_print_type {
                        cmds.push(Command::ChangeType { print_type });
                        current_print_type = Some(print_type);
                    }

                    cmds.push(Command::MoveAndExtrude {
                        start: current_loc,
                        end: m.end,
                        thickness,
                        width: m.width,
                    });
                    current_loc = m.end;
                }
                MoveType::Travel => {
                    cmds.push(Command::MoveTo { end: m.end });
                    current_loc = m.end;
                }
                _ => {}
            }
        }

        cmds
    }

    ///Rotate all moves in the movechain by a specific angle in radians.
    pub fn rotate(&mut self, angle: f32) {
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        for m in self.moves.iter_mut() {
            let nx = m.end.x * cos_a - m.end.y * sin_a;
            let ny = m.end.x * sin_a + m.end.y * cos_a;
            m.end.x = nx;
            m.end.y = ny;
        }
        let nx = self.start_point.x * cos_a - self.start_point.y * sin_a;
        let ny = self.start_point.x * sin_a + self.start_point.y * cos_a;

        self.start_point.x = nx;
        self.start_point.y = ny;
    }
}

fn update_state(move_type: &MovePrintType, settings: &LayerSettings, cmds: &mut Vec<Command>) {
    match move_type {
        MovePrintType::Unkown => {}
        MovePrintType::TopSolidInfill => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.solid_top_infill),
                    acceleration: Some(settings.acceleration.solid_top_infill),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::SolidInfill => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.solid_infill),
                    acceleration: Some(settings.acceleration.solid_infill),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::Infill => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.infill),
                    acceleration: Some(settings.acceleration.infill),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::Bridging => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.bridge),
                    acceleration: Some(settings.acceleration.bridge),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::WallOuter => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.exterior_surface_perimeter),
                    acceleration: Some(settings.acceleration.exterior_surface_perimeter),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::InteriorWallOuter => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.exterior_inner_perimeter),
                    acceleration: Some(settings.acceleration.exterior_inner_perimeter),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::WallInner => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.interior_surface_perimeter),
                    acceleration: Some(settings.acceleration.interior_surface_perimeter),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::InteriorWallInner => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.interior_inner_perimeter),
                    acceleration: Some(settings.acceleration.interior_inner_perimeter),
                    retract: RetractionType::Unretract,
                },
            });
        }
        MovePrintType::Support => {
            cmds.push(Command::SetState {
                new_state: StateChange {
                    bed_temp: None,
                    extruder_temp: None,
                    fan_speed: None,
                    movement_speed: Some(settings.speed.support),
                    acceleration: Some(settings.acceleration.support),
                    retract: RetractionType::Unretract,
                },
            });
        }
    }
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
