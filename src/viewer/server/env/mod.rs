use std::fmt::Debug;

use glam::{vec3, vec4, Vec3};
use volume::Volume;
use wgpu::BindGroup;

mod volume;

use crate::{
    geometry::{mesh::construct_triangle_vertices, BoundingBox},
    prelude::WgpuContext,
    render::{
        model::{Model, TransformMut},
        PipelineBuilder, Renderable, Texture, TextureVertex, Vertex,
    },
    viewer::RenderServer,
};

#[derive(Debug)]
pub struct EnvironmentServer {
    volume: Volume,

    texture_pipeline: wgpu::RenderPipeline,

    reflect: Model<Vertex>,
    // flap: Model<Vertex>,
    logo: Model<TextureVertex>,

    #[allow(unused)]
    logo_texture: Texture,
    logo_bind_group: BindGroup,
}

impl RenderServer for EnvironmentServer {
    fn instance(context: &WgpuContext) -> Self {
        let texture_pipeline = PipelineBuilder::new(context.device.clone())
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            })
            .build(
                "Texture Pipeline",
                include_str!("../../../render/texture_shader.wgsl"),
                &[
                    &context.camera_bind_group_layout,
                    &context.light_bind_group_layout,
                    &context.transform_bind_group_layout,
                    &context.texture_bind_group_layout,
                ],
                &[TextureVertex::desc()],
                context.surface_format,
            );

        let logo_bytes = include_bytes!("../../../../assets/icons/build_plate_img.png");
        let logo_texture = Texture::from_bytes(&context.device, &context.queue, logo_bytes, "Logo")
            .expect("Error when creating Texture");

        let logo_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &context.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(logo_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(logo_texture.sampler()),
                    },
                ],
                label: Some("logo_bind_group"),
            });

        Self {
            volume: Volume::instance(),

            texture_pipeline,

            reflect: Model::create(),
            logo: Model::create(),
            logo_texture,
            logo_bind_group,
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.reflect.render(render_pass);

        render_pass.set_pipeline(&self.texture_pipeline);

        render_pass.set_bind_group(3, &self.logo_bind_group, &[]);

        self.logo.render_without_color(render_pass);
    }
}

impl EnvironmentServer {
    pub fn volume_box(&self) -> &BoundingBox {
        self.volume.bounding_box()
    }

    pub fn update_printer_dimension(&mut self, x: f32, y: f32, z: f32) {
        self.volume.awaken(x, y, z);

        self.reflect
            .awaken(&build_plate_reflection(x * 1.1, y * 1.1, z));
        self.logo.awaken(&build_plate_vertices(x, y, z));

        self.reflect.transform(glam::Mat4::from_translation(vec3(
            -(x * 0.05),
            -(z * 0.05),
            -(y * 0.05),
        )));
    }

    pub fn render_line<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render_lines(render_pass);
    }
}

fn build_plate_reflection(x: f32, y: f32, _z: f32) -> [Vertex; 30] {
    let color = vec4(0.4, 0.4, 0.4, 1.0);

    let plate = construct_triangle_vertices(
        [
            vec3(x, -0.25, y),
            vec3(x, -0.25, 0.0),
            vec3(0.0, -0.25, y),
            vec3(0.0, -0.25, y),
            vec3(x, -0.25, 0.0),
            vec3(0.0, -0.25, 0.0),
        ],
        color,
    );

    let left_flap = construct_triangle_vertices(flap(0.0, 0.0, x * 0.15, y * 0.05), color);
    let right_flap = construct_triangle_vertices(flap(x * 0.85, 0.0, x * 0.15, y * 0.05), color);

    let mut reflection = [Vertex::default(); 30];

    reflection[..6].copy_from_slice(&plate);
    reflection[6..18].copy_from_slice(&left_flap);
    reflection[18..].copy_from_slice(&right_flap);

    reflection
}

const fn build_plate_vertices(x: f32, y: f32, _z: f32) -> [TextureVertex; 6] {
    [
        TextureVertex {
            position: [x, -0.25, y],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
        TextureVertex {
            position: [0.0, -0.25, y],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        TextureVertex {
            position: [x, -0.25, 0.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
        },
        TextureVertex {
            position: [0.0, -0.25, y],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        TextureVertex {
            position: [0.0, -0.25, 0.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        TextureVertex {
            position: [x, -0.25, 0.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
        },
    ]
}

fn flap(x: f32, y: f32, width: f32, height: f32) -> [Vec3; 12] {
    [
        // left triangle
        vec3(x, -0.25, 0.0),
        vec3(x + width * 0.3, -0.25, y),
        vec3(x + width * 0.3, -0.25, y - height),
        // quad
        vec3(x + width * 0.3, -0.25, y),
        vec3(x + width * 0.8, -0.25, y - height),
        vec3(x + width * 0.3, -0.25, y - height),
        vec3(x + width * 0.3, -0.25, y),
        vec3(x + width * 0.8, -0.25, y),
        vec3(x + width * 0.8, -0.25, y - height),
        // right triangle
        vec3(x + width * 0.8, -0.25, y),
        vec3(x + width, -0.25, y),
        vec3(x + width * 0.8, -0.25, y - height),
    ]
}
