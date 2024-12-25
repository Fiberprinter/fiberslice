use bytemuck::Zeroable;
use slicer::MovePrintType;

use crate::render::Vertex;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TraceVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub print_type: u32,
    pub layer: u32,
}

impl Default for TraceVertex {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl TraceVertex {
    pub fn from_vertex(vertex: Vertex, print_type: MovePrintType, layer: u32) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
            color: vertex.color,
            print_type: (1 << print_type as u32),
            layer,
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TraceVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress
                        + mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress
                        + mem::size_of::<[f32; 4]>() as wgpu::BufferAddress
                        + mem::size_of::<u32>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GPUTraceContext {
    pub visibility: u32,
    pub min_layer: u32,
    pub max_layer: u32,
}

impl Default for GPUTraceContext {
    fn default() -> Self {
        GPUTraceContext {
            visibility: u32::MAX,
            min_layer: 0,
            max_layer: u32::MAX,
        }
    }
}
