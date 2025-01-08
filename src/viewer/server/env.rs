use std::fmt::Debug;

use wgpu::BindGroup;

use crate::{
    geometry::BoundingBox,
    prelude::WgpuContext,
    render::{model::Model, PipelineBuilder, Renderable, Texture, TextureVertex},
    viewer::{volume::Volume, RenderServer},
};

#[derive(Debug)]
pub struct EnvironmentServer {
    volume: Volume,

    texture_pipeline: wgpu::RenderPipeline,
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
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            })
            .build(
                "Texture Pipeline",
                include_str!("../../render/texture_shader.wgsl"),
                &[
                    &context.camera_bind_group_layout,
                    &context.light_bind_group_layout,
                    &context.transform_bind_group_layout,
                    &context.texture_bind_group_layout,
                ],
                &[TextureVertex::desc()],
                context.surface_format,
            );

        let logo_bytes = include_bytes!("../../../assets/icons/main_icon.png");
        let logo_texture = Texture::from_bytes(&context.device, &context.queue, logo_bytes, "Logo")
            .expect("Error when creating Texture");

        let logo_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &context.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(logo_texture.view()), // CHANGED!
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(logo_texture.sampler()), // CHANGED!
                    },
                ],
                label: Some("diffuse_bind_group"),
            });

        Self {
            volume: Volume::instance(),

            texture_pipeline,
            logo: Model::create(),
            logo_texture,
            logo_bind_group,
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render(render_pass);

        render_pass.set_pipeline(&self.texture_pipeline);

        render_pass.set_bind_group(3, &self.logo_bind_group, &[]);

        println!("Render Logooooo");
        self.logo.render_without_color(render_pass);
    }
}

impl EnvironmentServer {
    pub fn volume_box(&self) -> &BoundingBox {
        self.volume.bounding_box()
    }

    pub fn update_printer_dimension(&mut self, x: f32, y: f32, z: f32) {
        self.volume.awaken(x, y, z);

        let height = x * 0.1;
        let width = x * 0.1;

        let vertices = &[
            TextureVertex {
                position: [x, 10.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
            TextureVertex {
                position: [x, 10.0, height],
                tex_coords: [1.0, 0.0],
            },
            TextureVertex {
                position: [x - width, 10.0, height],
                tex_coords: [0.0, 0.0],
            },
            TextureVertex {
                position: [x - width, 10.0, height],
                tex_coords: [0.0, 0.0],
            },
            TextureVertex {
                position: [x - width, 10.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
            TextureVertex {
                position: [x, 10.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
        ];

        println!("Awaken Logo");

        self.logo.awaken(vertices);
    }

    pub fn render_line<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render_lines(render_pass);
    }
}
