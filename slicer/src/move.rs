use geo::{line_string, Coord, EuclideanDistance, MultiPolygon, Polygon};
use glam::{vec2, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::EnumCount;

use crate::{command_pass::CommandPass, LayerSettings};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct MoveId(pub(crate) usize);

struct MoveIdGenerator {
    current: usize,
}

impl MoveIdGenerator {
    pub fn new() -> Self {
        Self { current: 0 }
    }

    pub fn next_id(&mut self) -> MoveId {
        self.current += 1;

        MoveId(self.current - 1)
    }
}

pub struct EvalIdPass {}

impl CommandPass for EvalIdPass {
    fn pass(cmds: &mut Vec<Command>, _settings: &crate::Settings) {
        let mut gen = MoveIdGenerator::new();

        for command in cmds.iter_mut() {
            match command {
                Command::MoveAndExtrude { id, .. }
                | Command::MoveAndExtrudeFiber { id, .. }
                | Command::MoveAndExtrudeFiberAndCut { id, .. } => *id = Some(gen.next_id()),
                _ => {}
            }
        }
    }
}

pub struct MergeFiberPass {}

impl CommandPass for MergeFiberPass {
    fn pass(cmds: &mut Vec<Command>, settings: &crate::Settings) {
        let mut current_index = 0;
        while current_index < cmds.len() {
            if let Some(chain) = FiberChain::find_next(cmds, current_index, settings) {
                if chain.start_index == chain.end_index {
                    // assume that the chain is a single move
                    let (start, end, thickness, width) = match cmds[chain.start_index] {
                        Command::MoveAndExtrudeFiber {
                            start,
                            end,
                            thickness,
                            width,
                            ..
                        } => (start, end, thickness, width),
                        _ => unreachable!(),
                    };

                    if chain.length >= settings.fiber.min_length {
                        cmds[chain.start_index] = Command::MoveAndExtrudeFiberAndCut {
                            start,
                            end,
                            thickness,
                            width,
                            id: None,
                            cut_pos: settings.fiber.cut_before,

                            #[cfg(debug_assertions)]
                            debug: format!("Cut at {}", settings.fiber.cut_before),
                        };
                    } else {
                        cmds[chain.start_index] = Command::MoveAndExtrude {
                            start,
                            end,
                            thickness,
                            width,
                            id: None,

                            #[cfg(debug_assertions)]
                            debug: format!("Fiber Chain too short"),
                        };
                    }
                } else if chain.length >= settings.fiber.min_length {
                    // backtrace where to cut
                    chain.find_cut_and_set(cmds, settings.fiber.cut_before);
                } else {
                    // change fiber chain to normal moves
                    for i in chain.start_index..=chain.end_index {
                        let (start, end, thickness, width) = match cmds[i] {
                            Command::MoveAndExtrudeFiber {
                                start,
                                end,
                                thickness,
                                width,
                                ..
                            } => (start, end, thickness, width),
                            _ => unreachable!(),
                        };

                        cmds[i] = Command::MoveAndExtrude {
                            start,
                            end,
                            thickness,
                            width,
                            id: None,

                            #[cfg(debug_assertions)]
                            debug: format!("Fiber Chain too short"),
                        };
                    }
                }

                current_index = chain.end_index + 1;
            } else {
                current_index += 1;
            }
        }
    }
}

#[derive(Debug, Clone)]
struct FiberChain {
    start_index: usize,
    end_index: usize,
    length: f32,
}

impl FiberChain {
    fn find_next(
        cmds: &mut [Command],
        mut current_index: usize,
        settings: &crate::Settings,
    ) -> Option<FiberChain> {
        let start_index = current_index;
        let mut last_direction = None;
        let mut length = 0.0;

        while current_index < cmds.len() {
            match cmds[current_index] {
                Command::MoveAndExtrudeFiber { start, end, .. } => {
                    let direction = vec2(end.x - start.x, end.y - start.y).normalize();

                    if let Some(last_dir) = last_direction {
                        let angle = direction.angle_to(last_dir);

                        #[cfg(debug_assertions)]
                        cmds[current_index].set_debug(format!("Angle: {}", angle.to_degrees()));

                        if angle.to_degrees().abs() <= settings.fiber.max_angle {
                            length += start.euclidean_distance(&end);
                            last_direction = Some(direction);

                            current_index += 1;
                        } else {
                            return Some(FiberChain {
                                start_index,
                                end_index: current_index - 1,
                                length,
                            });
                        }
                    } else {
                        length += start.euclidean_distance(&end);

                        last_direction = Some(direction);

                        current_index += 1;
                    }
                }
                _ => {
                    if start_index == current_index {
                        return None;
                    } else {
                        return Some(FiberChain {
                            start_index,
                            end_index: current_index - 1,
                            length,
                        });
                    }
                }
            }
        }

        if start_index == current_index {
            None
        } else {
            Some(FiberChain {
                start_index,
                end_index: current_index - 1,
                length,
            })
        }
    }

    fn find_cut_and_set(&self, cmds: &mut [Command], cut_before: f32) {
        let mut distance_backtraced = 0.0;

        for i in (self.start_index..=self.end_index).rev() {
            match cmds[i] {
                Command::MoveAndExtrudeFiber {
                    start,
                    end,
                    thickness,
                    width,
                    ..
                } => {
                    distance_backtraced += start.euclidean_distance(&end);

                    if distance_backtraced >= cut_before {
                        let cut_pos = cut_before;

                        cmds[i] = Command::MoveAndExtrudeFiberAndCut {
                            start,
                            end,
                            thickness,
                            width,
                            id: None,
                            cut_pos,

                            #[cfg(debug_assertions)]
                            debug: format!("Cut at {}", cut_pos),
                        };

                        return;
                    }
                }
                _ => {}
            }
        }

        unreachable!()
    }
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

impl MoveChain {
    pub fn trace_area(&self) -> MultiPolygon<f32> {
        let mut polygons = vec![];
        let mut current_loc = self.start_point;

        for m in self.moves.iter() {
            let end = m.end;
            let end = vec2(end.x, end.y);

            let start = vec2(current_loc.x, current_loc.y);

            let direction = (end - start).normalize();

            let p1 = start + vec2(direction.x, -direction.y) * (m.width / 2.0);
            let p2 = start + vec2(-direction.x, direction.y) * (m.width / 2.0);

            let p3 = end + vec2(-direction.x, direction.y) * (m.width / 2.0);
            let p4 = end + vec2(direction.x, -direction.y) * (m.width / 2.0);

            let line = line_string![
                (x: p1.x, y: p1.y),
                (x: p2.x, y: p2.y),
                (x: p3.x, y: p3.y),
                (x: p4.x, y: p4.y),
                (x: p1.x, y: p1.y),
            ];

            polygons.push(Polygon::new(line, vec![]));

            current_loc = m.end;
        }

        MultiPolygon(polygons)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, EnumCount)]
pub enum TraceType {
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

impl std::fmt::Display for TraceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceType::TopSolidInfill => write!(f, "Top Solid Infill"),
            TraceType::SolidInfill => write!(f, "Solid Infill"),
            TraceType::Infill => write!(f, "Infill"),
            TraceType::WallOuter => write!(f, "Wall Outer"),
            TraceType::WallInner => write!(f, "Wall Inner"),
            TraceType::InteriorWallOuter => write!(f, "Wall Inner"),
            TraceType::InteriorWallInner => write!(f, "Interior Inner Perimeter"),
            TraceType::Bridging => write!(f, "Bridging"),
            TraceType::Support => write!(f, "Support"),
        }
    }
}

impl TraceType {
    pub fn into_color_vec4(&self) -> Vec4 {
        match self {
            TraceType::TopSolidInfill => Vec4::new(1.0, 0.0, 0.0, 1.0),
            TraceType::SolidInfill => Vec4::new(1.0, 0.0, 0.0, 1.0),
            TraceType::Infill => Vec4::new(0.0, 0.0, 1.0, 1.0),
            TraceType::WallOuter => Vec4::new(1.0, 1.0, 0.0, 1.0),
            TraceType::WallInner => Vec4::new(1.0, 1.0, 0.0, 1.0),
            TraceType::InteriorWallOuter => Vec4::new(1.0, 1.0, 0.0, 1.0),
            TraceType::InteriorWallInner => Vec4::new(1.0, 1.0, 0.0, 1.0),
            TraceType::Bridging => Vec4::new(0.0, 1.0, 1.0, 1.0),
            TraceType::Support => Vec4::new(1.0, 1.0, 0.0, 1.0),
        }
    }
}

///Types of Moves
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum MoveType {
    WithFiber(TraceType),
    WithoutFiber(TraceType),
    ///Standard travel moves without extrusion
    Travel,
}

impl MoveType {
    pub fn from_type(print_type: TraceType, fiber: bool) -> Self {
        if fiber {
            MoveType::WithFiber(print_type)
        } else {
            MoveType::WithoutFiber(print_type)
        }
    }

    pub fn print_type(&self) -> Option<TraceType> {
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
        id: Option<MoveId>,
        ///Start point of the move
        start: Coord<f32>,

        ///End point of the move
        end: Coord<f32>,

        ///The height thickness of the move
        thickness: f32,

        /// The extrusion width
        width: f32,

        #[cfg(debug_assertions)]
        debug: String,
    },
    MoveAndExtrudeFiber {
        id: Option<MoveId>,
        ///Start point of the move
        start: Coord<f32>,

        ///End point of the move
        end: Coord<f32>,

        ///The height thickness of the move
        thickness: f32,

        /// The extrusion width
        width: f32,

        #[cfg(debug_assertions)]
        debug: String,
    },
    MoveAndExtrudeFiberAndCut {
        id: Option<MoveId>,
        ///Start point of the move
        start: Coord<f32>,

        ///End point of the move
        end: Coord<f32>,

        ///The height thickness of the move
        thickness: f32,

        /// The extrusion width
        width: f32,

        cut_pos: f32,

        #[cfg(debug_assertions)]
        debug: String,
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
        print_type: TraceType,
    },
    ///Used in optimization , should be optimized out
    NoAction,
}

impl Command {
    #[cfg(debug_assertions)]
    pub fn set_debug(&mut self, debug: String) {
        match self {
            Command::MoveAndExtrude { debug: d, .. }
            | Command::MoveAndExtrudeFiber { debug: d, .. }
            | Command::MoveAndExtrudeFiberAndCut { debug: d, .. } => d.push_str(&debug),
            _ => {}
        }
    }
}

impl Command {
    pub fn needs_filament(&self) -> bool {
        match self {
            Command::MoveAndExtrude { .. } => true,
            Command::MoveAndExtrudeFiberAndCut { .. } => true,
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
                        id: None,

                        #[cfg(debug_assertions)]
                        debug: format!("{:?}", print_type),
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
                        id: None,

                        #[cfg(debug_assertions)]
                        debug: format!("{:?}", print_type),
                    });
                    current_loc = m.end;
                }
                MoveType::Travel => {
                    cmds.push(Command::MoveTo { end: m.end });
                    current_loc = m.end;
                }
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

fn update_state(move_type: &TraceType, settings: &LayerSettings, cmds: &mut Vec<Command>) {
    match move_type {
        TraceType::TopSolidInfill => {
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
        TraceType::SolidInfill => {
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
        TraceType::Infill => {
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
        TraceType::Bridging => {
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
        TraceType::WallOuter => {
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
        TraceType::InteriorWallOuter => {
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
        TraceType::WallInner => {
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
        TraceType::InteriorWallInner => {
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
        TraceType::Support => {
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
