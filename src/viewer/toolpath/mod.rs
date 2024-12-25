use std::{fmt::Debug, sync::Arc};

use egui::ahash::{HashMap, HashMapExt};
use glam::{Vec3, Vec4};
use mesh::{
    TraceConnectionMesh, TraceCrossSection, TraceCrossSectionMesh, TraceHitbox, TraceMesh,
    MOVE_MESH_VERTICES,
};
use shared::process::Process;
use slicer::{Command, MovePrintType, StateChange};
use tree::ToolpathTree;
use vertex::ToolpathVertex;
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
///
pub fn bit_representation(print_type: &MovePrintType) -> u32 {
    0x01 << ((*print_type as u32) + 0x02)
}

#[allow(dead_code)]
pub const fn bit_representation_travel() -> u32 {
    0x02
}

pub const fn bit_representation_setup() -> u32 {
    0x01
}

#[derive(Debug)]
pub struct SlicedObject {
    pub model: Arc<ToolpathTree>,
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

        let mut root = ToolpathTree::create_root();

        let mut last_extrusion_profile = None;

        let mut move_vertices = Vec::new();
        let mut travel_vertices = Vec::new();
        let mut fiber_vertices = Vec::new();
        // let mut travel_vertices = Vec::new();
        // let mut fiber_vertices = Vec::new();

        for command in commands {
            let print_type_bit = match current_type {
                Some(ty) => bit_representation(&ty),
                None => bit_representation_setup(),
            };

            let color = current_type
                .unwrap_or(MovePrintType::Infill)
                .into_color_vec4();

            match command {
                slicer::Command::MoveTo { end } => {
                    let start = last_position;
                    let end = Vec3::new(
                        end.x - settings.print_x / 2.0,
                        current_height_z,
                        end.y - settings.print_y / 2.0,
                    );

                    travel_vertices.push(ToolpathVertex::from_vertex(
                        Vertex {
                            position: start.to_array(),
                            normal: [0.0; 3],
                            color: [0.0, 0.0, 0.0, 1.0],
                        },
                        print_type_bit | bit_representation_travel(),
                        current_layer as u32,
                    ));

                    travel_vertices.push(ToolpathVertex::from_vertex(
                        Vertex {
                            position: end.to_array(),
                            normal: [0.0; 3],
                            color: [0.0, 0.0, 0.0, 1.0],
                        },
                        print_type_bit | bit_representation_travel(),
                        current_layer as u32,
                    ));

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

                    let start_profile =
                        TraceCrossSection::from_direction(end - start, *thickness, *width)
                            .with_offset(start);

                    let end_profile =
                        TraceCrossSection::from_direction(end - start, *thickness, *width)
                            .with_offset(end);

                    let mesh =
                        TraceMesh::from_profiles(start_profile, end_profile).with_color(color);

                    extend_connection_vertices(
                        last_extrusion_profile,
                        start_profile,
                        print_type_bit,
                        current_layer,
                        color,
                        &mut move_vertices,
                    );

                    last_extrusion_profile = Some(end_profile);

                    let offset = move_vertices.len() as BufferAddress;
                    let toolpath_vertices = mesh.to_triangle_vertices().into_iter().map(|v| {
                        ToolpathVertex::from_vertex(v, print_type_bit, current_layer as u32)
                    });

                    move_vertices.extend(toolpath_vertices);

                    let tree_move = ToolpathTree::create_move(
                        TraceHitbox::from(mesh),
                        offset,
                        MOVE_MESH_VERTICES as BufferAddress,
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

                    fiber_vertices.push(ToolpathVertex::from_vertex(
                        Vertex {
                            position: start.to_array(),
                            normal: [0.0; 3],
                            color: [0.0, 0.0, 0.0, 1.0],
                        },
                        print_type_bit,
                        current_layer as u32,
                    ));

                    fiber_vertices.push(ToolpathVertex::from_vertex(
                        Vertex {
                            position: end.to_array(),
                            normal: [0.0; 3],
                            color: [0.0, 0.0, 0.0, 1.0],
                        },
                        print_type_bit,
                        current_layer as u32,
                    ));

                    let start_profile =
                        TraceCrossSection::from_direction(end - start, *thickness, *width)
                            .with_offset(start);

                    let end_profile =
                        TraceCrossSection::from_direction(end - start, *thickness, *width)
                            .with_offset(end);

                    let mesh = TraceMesh::from_profiles(
                        TraceCrossSection::from_direction(end - start, *thickness, *width)
                            .with_offset(start),
                        TraceCrossSection::from_direction(end - start, *thickness, *width)
                            .with_offset(end),
                    )
                    .with_color(color);

                    if let Some(ty) = current_type {
                        count_map.entry(ty).and_modify(|e| *e += 1).or_insert(1);
                    }

                    extend_connection_vertices(
                        last_extrusion_profile,
                        start_profile,
                        print_type_bit,
                        current_layer,
                        color,
                        &mut move_vertices,
                    );

                    last_extrusion_profile = Some(end_profile);

                    let offset = move_vertices.len() as BufferAddress;
                    let single_move_vertices = mesh.to_triangle_vertices().into_iter().map(|v| {
                        ToolpathVertex::from_vertex(v, print_type_bit, current_layer as u32)
                    });

                    move_vertices.extend(single_move_vertices);

                    let tree_move = ToolpathTree::create_move(
                        TraceHitbox::from(mesh),
                        offset,
                        MOVE_MESH_VERTICES as BufferAddress,
                    );

                    root.push(tree_move);

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
                if let Some(last_extrusion_profile) = last_extrusion_profile {
                    let mesh = TraceCrossSectionMesh::from_profile(last_extrusion_profile)
                        .with_color(color);

                    let vertices = mesh.to_triangle_vertices().into_iter().map(|v| {
                        ToolpathVertex::from_vertex(v, print_type_bit, current_layer as u32)
                    });

                    move_vertices.extend(vertices);
                }

                last_extrusion_profile = None;
            }
        }

        root.awaken(&move_vertices, &travel_vertices, &fiber_vertices);

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

fn extend_connection_vertices(
    last_extrusion_profile: Option<TraceCrossSection>,
    start_profile: TraceCrossSection,
    print_type_bit: u32,
    current_layer: usize,
    color: Vec4,
    move_vertices: &mut Vec<ToolpathVertex>,
) {
    if let Some(last_extrusion_profile) = last_extrusion_profile {
        let connection = TraceConnectionMesh::from_profiles(last_extrusion_profile, start_profile)
            .with_color(color);

        let connection_vertices = connection
            .to_triangle_vertices()
            .into_iter()
            .map(|v| ToolpathVertex::from_vertex(v, print_type_bit, current_layer as u32));

        move_vertices.extend(connection_vertices);
    } else {
        let mesh = TraceCrossSectionMesh::from_profile(start_profile).with_color(color);

        let vertices = mesh
            .to_triangle_vertices_flipped()
            .into_iter()
            .map(|v| ToolpathVertex::from_vertex(v, print_type_bit, current_layer as u32));

        move_vertices.extend(vertices);
    }
}
