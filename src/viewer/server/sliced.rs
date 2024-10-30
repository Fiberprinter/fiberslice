use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;

use native_dialog::FileDialog;
use shared::process::Process;
use slicer::{convert, MovePrintType, SliceResult};
use tokio::sync::oneshot::Receiver;
use tokio::task::JoinHandle;
use wgpu::util::DeviceExt;

use crate::input::hitbox::HitboxRoot;
use crate::render::Renderable;
use crate::viewer::toolpath::vertex::{ToolpathContext, ToolpathVertex};
use crate::viewer::toolpath::SlicedObject;
use crate::viewer::RenderServer;
use crate::QUEUE;
use crate::{prelude::WgpuContext, GlobalState, RootEvent};

use crate::viewer::toolpath::tree::ToolpathTree;

// const MAIN_LOADED_TOOLPATH: &str = "main"; // HACK: This is a solution to ease the dev when only one toolpath is loaded which is the only supported(for now)

#[derive(thiserror::Error, Debug)]
pub enum SliceError {
    #[error("Load Error {0}")]
    LoadError(String),
    #[error("NoGeometryObject")]
    NoGeometryObject,
}

pub type QueuedSlicedObject = (Receiver<(SlicedObject, Arc<Process>)>, JoinHandle<()>);

#[derive(Debug)]
pub struct SlicedObjectServer {
    queued: Option<QueuedSlicedObject>,

    pipeline: wgpu::RenderPipeline,
    sliced_object: Option<SlicedObject>,
    hitbox: HitboxRoot<ToolpathTree>,

    toolpath_context_buffer: wgpu::Buffer,
    toolpath_context: ToolpathContext,
    toolpath_context_bind_group: wgpu::BindGroup,
}

impl RenderServer for SlicedObjectServer {
    fn instance(context: &WgpuContext) -> Self {
        let toolpath_context = ToolpathContext::default();

        let toolpath_context_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("SlicedObject Context Buffer"),
                    contents: bytemuck::cast_slice(&[toolpath_context]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let toolpath_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: None,
                });

        let toolpath_context_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &toolpath_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: toolpath_context_buffer.as_entire_binding(),
                    }],
                    label: None,
                });

        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Toolpath Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("sliced_shader.wgsl").into()),
            });

        let render_pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &context.camera_bind_group_layout,
                        &context.light_bind_group_layout,
                        &context.transform_bind_group_layout,
                        &context.color_bind_group_layout,
                        &toolpath_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[ToolpathVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.surface_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }),

                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    ..Default::default()
                },
                multiview: None,
                cache: None,
            });

        Self {
            queued: None,
            sliced_object: None,
            hitbox: HitboxRoot::root(),
            pipeline,
            toolpath_context,
            toolpath_context_bind_group,
            toolpath_context_buffer,
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(toolpath) = self.sliced_object.as_ref() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(4, &self.toolpath_context_bind_group, &[]);
            toolpath.model.render(render_pass);
        }
    }
}

impl SlicedObjectServer {
    pub fn load_from_slice_result(&mut self, slice_result: SliceResult, process: Arc<Process>) {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            process.set_task("Loading toolpath".to_string());
            process.set_progress(0.8);

            let toolpath =
                SlicedObject::from_commands(&slice_result.moves, &slice_result.settings, &process)
                    .expect("Failed to load toolpath");

            tx.send((toolpath, process)).unwrap();
        });

        self.queued = Some((rx, handle));
    }

    pub fn export(&self) {
        if let Some(toolpath) = self.sliced_object.as_ref() {
            let path = FileDialog::new()
                .set_location("~")
                .set_filename("model.gcode")
                .set_title("Export GCode")
                .add_filter("GCode", &["gcode"])
                .show_save_single_file()
                .unwrap();

            if let Some(path) = path {
                let file = match File::create_new(path) {
                    Ok(file) => file,
                    Err(e) => {
                        println!("Failed to create file: {:?}", e);
                        return;
                    }
                };

                let mut writer = BufWriter::new(file);
                match convert(&toolpath.moves, &toolpath.settings, &mut writer) {
                    Ok(_) => {
                        println!("Gcode saved");
                    }
                    Err(e) => {
                        println!("Failed to save gcode: {:?}", e);
                    }
                }
            }
        }
    }

    pub fn update(&mut self, global_state: GlobalState<RootEvent>) -> Result<(), SliceError> {
        if let Some((rx, _)) = &mut self.queued {
            if let Ok((toolpath, process)) = rx.try_recv() {
                process.finish();

                global_state
                    .ui_event_writer
                    .send(crate::ui::UiEvent::ShowSuccess("Gcode loaded".to_string()));

                self.hitbox.add_node(toolpath.model.clone());

                self.sliced_object = Some(toolpath);
            }
        }

        Ok(())
    }

    pub fn update_visibility(&mut self, value: u32) {
        self.toolpath_context.visibility = value;

        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        queue.write_buffer(
            &self.toolpath_context_buffer,
            0,
            bytemuck::cast_slice(&[self.toolpath_context]),
        );
    }

    pub fn set_visibility_type(&mut self, ty: MovePrintType, visible: bool) {
        let index = ty as usize;

        if visible {
            self.toolpath_context.visibility |= 1 << index;
        } else {
            self.toolpath_context.visibility &= !(1 << index);
        }

        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        queue.write_buffer(
            &self.toolpath_context_buffer,
            0,
            bytemuck::cast_slice(&[self.toolpath_context]),
        );
    }

    pub fn update_min_layer(&mut self, min: u32) {
        self.toolpath_context.min_layer = min;

        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        queue.write_buffer(
            &self.toolpath_context_buffer,
            0,
            bytemuck::cast_slice(&[self.toolpath_context]),
        );
    }

    pub fn update_max_layer(&mut self, max: u32) {
        self.toolpath_context.max_layer = max;

        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        queue.write_buffer(
            &self.toolpath_context_buffer,
            0,
            bytemuck::cast_slice(&[self.toolpath_context]),
        );
    }

    pub fn get_sliced(&self) -> Option<&SlicedObject> {
        self.sliced_object.as_ref()
    }

    pub fn check_hit(&self, ray: &crate::input::Ray, level: usize) -> Option<Arc<ToolpathTree>> {
        self.hitbox.check_hit(ray, level, false)
    }
}
