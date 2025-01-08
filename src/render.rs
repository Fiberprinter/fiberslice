use std::time::Instant;

use glam::{Mat4, Vec3};
use wgpu::{util::DeviceExt, CommandEncoder};

use crate::{
    prelude::*,
    ui::UiUpdateOutput,
    viewer::{CameraResult, CameraUniform},
    GlobalState, RootEvent,
};

mod descriptor;
mod group;
mod light;
mod pipeline;
mod texture;
mod vertex;

pub use group::ColorBinding;

pub use pipeline::DefaultPipelines;
pub use pipeline::PipelineBuilder;

pub mod model;

pub use descriptor::RenderDescriptor;

pub use light::*;
pub use texture::*;
pub use vertex::*;

pub trait Renderable {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>);
    fn render_without_color<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>);
}

const MSAA_SAMPLE_COUNT: u32 = 1;

#[derive(Debug)]
pub enum RenderEvent {}

struct RenderState {
    depth_texture: Texture,

    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
}

impl RenderState {
    fn update(&mut self, wgpu_context: &WgpuContext, view_proj: Mat4, eye: Vec3) {
        self.camera_uniform.update_view_proj(view_proj, eye);

        wgpu_context.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Update the light so that it is transformed with the camera
        self.light_uniform.position = [
            self.camera_uniform.view_position[0],
            self.camera_uniform.view_position[1],
            self.camera_uniform.view_position[2],
            1.0,
        ];
        wgpu_context.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }
}

pub struct RenderAdapter {
    multisampled_framebuffer: wgpu::TextureView,

    egui_rpass: egui_wgpu_backend::RenderPass,

    pipelines: DefaultPipelines,

    render_state: RenderState,

    event_reader: EventReader<RenderEvent>,
}

impl RenderAdapter {
    fn render(
        &self,
        encoder: &mut CommandEncoder,
        texture_view: &wgpu::TextureView,
        viewport: &Viewport,
        global_state: &GlobalState<RootEvent>,
    ) {
        let clear_color = wgpu::Color {
            r: 0.7,
            g: 0.7,
            b: 0.7,
            a: 1.0,
        };

        let rpass_color_attachment = wgpu::RenderPassColorAttachment {
            view: texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: wgpu::StoreOp::Store,
            },
        };

        let pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(rpass_color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.render_state.depth_texture.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let descriptor = RenderDescriptor {
            pipelines: &self.pipelines,
            bind_groups: &[
                &self.render_state.camera_bind_group,
                &self.render_state.light_bind_group,
            ],
            encoder,
            viewport,
            pass_descriptor,
        };

        global_state
            .viewer
            .render(descriptor, *global_state.ui_state.mode.read());
    }

    fn render_secondary(
        &self,
        encoder: &mut CommandEncoder,
        texture_view: &wgpu::TextureView,
        viewport: &Viewport,
        global_state: &GlobalState<RootEvent>,
    ) {
        let rpass_color_attachment = wgpu::RenderPassColorAttachment {
            view: texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        };

        let pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(rpass_color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.render_state.depth_texture.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let descriptor = RenderDescriptor {
            pipelines: &self.pipelines,
            bind_groups: &[
                &self.render_state.camera_bind_group,
                &self.render_state.light_bind_group,
            ],
            encoder,
            viewport,
            pass_descriptor,
        };

        global_state
            .viewer
            .render_secondary(descriptor, *global_state.ui_state.mode.read());
    }
}

impl<'a> FrameHandle<'a, RootEvent, (), (Option<UiUpdateOutput>, &CameraResult)> for RenderAdapter {
    fn handle_frame(
        &'a mut self,
        wgpu_context: &WgpuContext,
        state: GlobalState<RootEvent>,
        (ui_output, camera_result): (Option<UiUpdateOutput>, &CameraResult),
    ) -> Result<(), Error> {
        puffin::profile_function!("Render handle_frame");

        let CameraResult {
            view,
            proj,
            eye,
            viewport,
        } = *camera_result;

        self.render_state.update(wgpu_context, proj * view, eye);

        let now = Instant::now();

        let output = wgpu_context
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            wgpu_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let UiUpdateOutput {
            paint_jobs,
            tdelta,
            screen_descriptor,
        } = ui_output.unwrap();

        // self.render_transparent(&mut encoder, &view, &viewport, &state);
        self.render(&mut encoder, &view, &viewport, &state);
        self.render_secondary(&mut encoder, &view, &viewport, &state);

        self.egui_rpass
            .add_textures(&wgpu_context.device, &wgpu_context.queue, &tdelta)
            .expect("add texture ok");

        self.egui_rpass.update_buffers(
            &wgpu_context.device,
            &wgpu_context.queue,
            &paint_jobs,
            &screen_descriptor,
        );

        self.egui_rpass
            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
            .expect("execute render pass ok");

        wgpu_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.egui_rpass
            .remove_textures(tdelta)
            .expect("remove texture ok");

        println!("Render time: {:?}", now.elapsed());

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event: &winit::event::WindowEvent,
        _id: winit::window::WindowId,
        wgpu_context: &WgpuContext,
        _global_state: GlobalState<RootEvent>,
    ) {
        match event {
            winit::event::WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.render_state.depth_texture = Texture::create_depth_texture(
                        &wgpu_context.device,
                        &wgpu_context.surface_config,
                        MSAA_SAMPLE_COUNT,
                        "depth_texture",
                    );
                    self.multisampled_framebuffer = Texture::create_multisampled_framebuffer(
                        &wgpu_context.device,
                        &wgpu_context.surface_config,
                        MSAA_SAMPLE_COUNT,
                        "multisampled_framebuffer",
                    );
                }
            }
            winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                let size = wgpu_context.window.inner_size();

                if size.width > 0 && size.height > 0 {
                    self.render_state.depth_texture = Texture::create_depth_texture(
                        &wgpu_context.device,
                        &wgpu_context.surface_config,
                        MSAA_SAMPLE_COUNT,
                        "depth_texture",
                    );
                    self.multisampled_framebuffer = Texture::create_multisampled_framebuffer(
                        &wgpu_context.device,
                        &wgpu_context.surface_config,
                        MSAA_SAMPLE_COUNT,
                        "multisampled_framebuffer",
                    );
                }
            }
            _ => {}
        }
    }
}

impl<'a> Adapter<'a, RootEvent, (), (), (Option<UiUpdateOutput>, &CameraResult), RenderEvent>
    for RenderAdapter
{
    fn create(context: &WgpuContext) -> AdapterCreation<(), RenderEvent, Self> {
        let depth_texture = Texture::create_depth_texture(
            &context.device,
            &context.surface_config,
            MSAA_SAMPLE_COUNT,
            "depth_texture",
        );

        let camera_uniform = CameraUniform::default();

        let camera_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &context.camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });

        let light_uniform = LightUniform {
            position: [1000.0, 1000.0, 1000.0, 1.0],
            color: [1.0, 1.0, 1.0, 0.1],
        };

        let light_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let light_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &context.light_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                }],
                label: None,
            });

        let render_state = RenderState {
            depth_texture,

            camera_uniform,
            camera_buffer,
            camera_bind_group,

            light_uniform,
            light_buffer,
            light_bind_group,
        };

        let pipelines = DefaultPipelines::instance(
            context.device.clone(),
            include_str!("render/shader.wgsl"),
            &[
                &context.camera_bind_group_layout,
                &context.light_bind_group_layout,
                &context.transform_bind_group_layout,
                &context.color_bind_group_layout,
            ],
            &[Vertex::desc()],
            context.surface_format,
        );

        let multisampled_framebuffer = Texture::create_multisampled_framebuffer(
            &context.device,
            &context.surface_config,
            MSAA_SAMPLE_COUNT,
            "multisampled_framebuffer",
        );

        let egui_rpass = egui_wgpu_backend::RenderPass::new(
            &context.device,
            context.surface_format,
            MSAA_SAMPLE_COUNT,
        );

        let (reader, writer) = create_event_bundle::<RenderEvent>();

        (
            (),
            writer,
            RenderAdapter {
                multisampled_framebuffer,

                egui_rpass,

                pipelines,

                render_state,

                event_reader: reader,
            },
        )
    }

    fn get_adapter_description(&self) -> String {
        "RenderAdapter".to_string()
    }

    fn get_reader(&self) -> EventReader<RenderEvent> {
        self.event_reader.clone()
    }
}
