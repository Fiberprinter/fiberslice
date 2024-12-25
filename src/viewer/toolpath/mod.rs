use std::{fmt::Debug, sync::Arc};

use egui::ahash::{HashMap, HashMapExt};
use glam::{Vec3, Vec4};
use log::info;
use mesh::{
    TraceConnectionMesh, TraceCrossSection, TraceCrossSectionMesh, TraceHitbox, TraceMesh,
    TraceMesher, TRACE_MESH_VERTICES,
};
use shared::process::Process;
use slicer::{Command, FiberSettings, MovePrintType, StateChange};
use tree::TraceTree;
use vertex::TraceVertex;
use wgpu::BufferAddress;

use crate::{geometry::mesh::Mesh, render::Vertex};

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

#[derive(Debug, Clone, Copy)]
pub enum TraceContext {
    Setup,
    Travel,
    Fiber,
    Move,
}

#[derive(Debug)]
pub struct SlicedObject {
    pub model: Arc<TraceTree>,
    pub count_map: HashMap<MovePrintType, usize>,
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
        let mut current_state = StateChange::default();
        let mut current_type = None;
        let mut current_layer = 0;
        let mut current_height_z = 0.0;

        let mut last_position = Vec3::ZERO;

        let mut count_map = HashMap::new();

        let fiber_diameter = settings
            .fiber
            .as_ref()
            .unwrap_or(&FiberSettings::default())
            .diameter;

        let mut root = TraceTree::create_root();

        let mut tracer = TraceMesher::new();
        tracer.set_context(TraceContext::Setup);

        let mut fiber_tracer = TraceMesher::new();
        fiber_tracer.set_context(TraceContext::Fiber);
        fiber_tracer.set_color(Vec4::new(0.0, 0.0, 0.0, 1.0));

        let mut travel_vertices = Vec::new();

        for command in commands {
            tracer.set_print_type(current_type.unwrap_or(MovePrintType::Infill));
            tracer.set_current_layer(current_layer);
            tracer.set_color(
                current_type
                    .unwrap_or(MovePrintType::Infill)
                    .into_color_vec4(),
            );

            fiber_tracer.set_print_type(current_type.unwrap_or(MovePrintType::Infill));
            fiber_tracer.set_current_layer(current_layer);

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
                        color: [0.0, 0.0, 0.0, 1.0],
                    });

                    travel_vertices.push(Vertex {
                        position: end.to_array(),
                        normal: [0.0; 3],
                        color: [0.0, 0.0, 0.0, 1.0],
                    });

                    last_position = end;
                }
                slicer::Command::MoveAndExtrude {
                    start,
                    end,
                    thickness,
                    width,
                } => {
                    tracer.set_context(TraceContext::Move);

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

                    let (offset, hitbox) = tracer.next(start, end, *thickness, *width);

                    let tree_move = TraceTree::create_move(
                        hitbox,
                        offset as u64,
                        TRACE_MESH_VERTICES as BufferAddress,
                    );

                    root.push(tree_move);

                    count_map
                        .entry(current_type.unwrap_or(MovePrintType::Infill))
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
                    tracer.set_context(TraceContext::Move);

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

                    let (offset, _) = fiber_tracer.next(start, end, *thickness, *width);

                    let fiber = TraceTree::create_fiber(
                        offset as u64,
                        TRACE_MESH_VERTICES as BufferAddress,
                        start,
                        end,
                    );

                    root.push(fiber);

                    last_position = end;
                }
                slicer::Command::LayerChange { z, index } => {
                    current_layer = *index;
                    current_height_z = *z;
                }
                slicer::Command::SetState { new_state } => {
                    current_state = new_state.clone();
                }
                slicer::Command::ChangeType { print_type } => current_type = Some(*print_type),
                _ => {}
            }

            if !command.needs_filament() {
                tracer.finish_chain();
            }
        }

        let trace_vertices = tracer.finish();
        let fiber_vertices = fiber_tracer.finish();

        info!("Fiber vertices: {:?}", fiber_vertices);

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

    pub fn from_file(path: &str, settings: &slicer::Settings) -> Result<Self, ()> {
        todo!()
    }
}
