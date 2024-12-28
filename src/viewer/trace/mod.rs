use std::{fmt::Debug, sync::Arc};

use egui::ahash::{HashMap, HashMapExt};
use glam::{Vec3, Vec4};
use mesh::{LineMesher, TraceMesher, TRACE_MESH_VERTICES};
use shared::process::Process;
use slicer::{Command, TraceType};
use tree::TraceTree;
use wgpu::BufferAddress;

use crate::render::Vertex;

pub mod mesh;
pub mod tree;
pub mod vertex;

/// Returns the bit representation of the path type.
/// The first bit is the setup flag, the second bit is the travel flag. The rest of the bits are the print type.
/// The print type is represented by the enum variant index.
/// # Example
/// ```
/// use slicer::print_type::{PathType, PrintType};
///
/// let path_type = PathType::Work {
///
///    print_type: PrintType::InternalInfill,
///   travel: false,
/// };
///
/// assert_eq!(path_type.bit_representation(), 1);
///
pub fn bit_representation(print_type: &TraceType) -> u32 {
    0x01 << (*print_type as u32)
}

pub const fn bit_representation_setup() -> u32 {
    0x01
}

pub const TRAVEL_COLOR: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0);

pub const FIBER_COLOR: Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);

#[derive(Debug)]
pub struct SlicedObject {
    pub model: Arc<TraceTree>,
    pub count_map: HashMap<TraceType, usize>,
    pub max_layer: usize,
    pub moves: Vec<Command>,
    pub settings: slicer::Settings,
}

unsafe impl Sync for SlicedObject {}
unsafe impl Send for SlicedObject {}

impl SlicedObject {
    pub fn from_commands(
        commands: &[slicer::Command],
        settings: &slicer::Settings,
        _process: &Process,
    ) -> Result<Self, ()> {
        // let mut current_state = StateChange::default();
        let mut current_type = None;
        let mut current_layer = 0;
        let mut current_height_z = 0.0;

        let mut last_position = Vec3::ZERO;

        let mut count_map = HashMap::new();

        let mut root = TraceTree::create_root();

        let mut mesher = TraceMesher::new();

        let mut fiber_mesher = LineMesher::new();
        fiber_mesher.set_color(FIBER_COLOR);

        let mut travel_vertices = Vec::new();

        for command in commands {
            if let Some(ty) = current_type {
                mesher.set_type(ty);
            }
            mesher.set_current_layer(current_layer);
            mesher.set_color(current_type.unwrap_or(TraceType::Infill).into_color_vec4());

            if let Some(ty) = current_type {
                fiber_mesher.set_type(ty);
            }
            fiber_mesher.set_current_layer(current_layer);

            match command {
                slicer::Command::MoveTo { end } => {
                    let start = last_position;
                    let end = Vec3::new(
                        end.x - settings.print_x / 2.0,
                        current_height_z,
                        end.y - settings.print_y / 2.0,
                    );

                    travel_vertices.push(Vertex {
                        position: start.to_array(),
                        normal: [0.0; 3],
                        color: TRAVEL_COLOR.to_array(),
                    });

                    travel_vertices.push(Vertex {
                        position: end.to_array(),
                        normal: [0.0; 3],
                        color: TRAVEL_COLOR.to_array(),
                    });

                    let travel = TraceTree::create_travel(2, start, end);

                    root.push(travel);

                    last_position = end;
                }
                slicer::Command::MoveAndExtrude {
                    start,
                    end,
                    thickness,
                    width,
                } => {
                    let start = Vec3::new(
                        start.x - settings.print_x / 2.0,
                        current_height_z - thickness / 2.0,
                        start.y - settings.print_y / 2.0,
                    );
                    let end = Vec3::new(
                        end.x - settings.print_x / 2.0,
                        current_height_z - thickness / 2.0,
                        end.y - settings.print_y / 2.0,
                    );

                    if let Some(ty) = current_type {
                        count_map.entry(ty).and_modify(|e| *e += 1).or_insert(1);
                    }

                    let (offset, hitbox) = mesher.next(start, end, *thickness, *width);

                    let tree_move = TraceTree::create_move(
                        hitbox,
                        offset as u64,
                        TRACE_MESH_VERTICES as BufferAddress,
                    );

                    root.push(tree_move);

                    count_map
                        .entry(current_type.unwrap_or(TraceType::Infill))
                        .and_modify(|e| *e += 1)
                        .or_insert(1);

                    last_position = end;
                }
                slicer::Command::MoveAndExtrudeFiber {
                    start,
                    end,
                    thickness,
                    width,
                } => {
                    let start = Vec3::new(
                        start.x - settings.print_x / 2.0,
                        current_height_z - thickness / 2.0,
                        start.y - settings.print_y / 2.0,
                    );
                    let end = Vec3::new(
                        end.x - settings.print_x / 2.0,
                        current_height_z - thickness / 2.0,
                        end.y - settings.print_y / 2.0,
                    );

                    if let Some(ty) = current_type {
                        count_map.entry(ty).and_modify(|e| *e += 1).or_insert(1);
                    }

                    let (offset, hitbox) = mesher.next(start, end, *thickness, *width);

                    let tree_move = TraceTree::create_move(
                        hitbox,
                        offset as u64,
                        TRACE_MESH_VERTICES as BufferAddress,
                    );

                    root.push(tree_move);

                    let offset = fiber_mesher.next(start, end);

                    let fiber = TraceTree::create_fiber(offset as u64, start, end);

                    root.push(fiber);

                    last_position = end;
                }
                slicer::Command::LayerChange { z, index } => {
                    current_layer = *index;
                    current_height_z = *z;
                }
                slicer::Command::SetState { .. } => {}
                slicer::Command::ChangeType { print_type } => current_type = Some(*print_type),
                _ => {}
            }

            if !command.needs_filament() {
                mesher.finish_chain();
            }
        }

        let trace_vertices = mesher.finish();
        let fiber_vertices = fiber_mesher.finish();

        log::info!("Trace Vertices: {}", trace_vertices.len());

        root.awaken(&trace_vertices, &travel_vertices, &fiber_vertices);
        root.update_offset(0);

        Ok(Self {
            model: Arc::new(root),
            count_map,
            max_layer: current_layer,
            moves: commands.to_vec(),
            settings: settings.clone(),
        })
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    pub fn from_file(path: &str, settings: &slicer::Settings) -> Result<Self, ()> {
        todo!()
    }
}
