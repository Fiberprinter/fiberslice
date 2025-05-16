use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;

use native_dialog::FileDialog;
use shared::process::Process;
use slicer::gcode::mem::GCodeMemoryWriter;
use slicer::gcode::GCodeFileWriter;
use slicer::{gcode::write_gcode, SliceResult, SlicedGCode};
use tokio::sync::oneshot::Receiver;
use tokio::task::JoinHandle;
use wgpu::util::DeviceExt;

use crate::config::gui::TOOL_TOGGLE_BUTTON;
use crate::input::hitbox::HitboxRoot;
use crate::render::{ColorBinding, PipelineBuilder, Renderable};
use crate::viewer::trace::vertex::{TraceContext, TraceVertex};
use crate::viewer::trace::{from_commands_into_layers, SlicedObject};
use crate::viewer::RenderServer;
use crate::QUEUE;
use crate::{prelude::WgpuContext, GlobalState, RootEvent};

use crate::viewer::trace::tree::TraceTree;

pub type QueuedSlicedObject = (
    Receiver<(SlicedObject, SlicedGCode, Arc<Process>)>,
    JoinHandle<()>,
);

#[derive(Debug)]
pub struct SlicedObjectServer {
    queued: Option<QueuedSlicedObject>,

    pipeline: wgpu::RenderPipeline,

    sliced_object: Option<SlicedObject>,
    sliced_gcode: Option<SlicedGCode>,

    hitbox: HitboxRoot<TraceTree>,

    travel_visible: bool,
    fiber_visible: bool,

    toolpath_context_buffer: wgpu::Buffer,
    toolpath_context: TraceContext,
    toolpath_context_bind_group: wgpu::BindGroup,

    trace_color_group: ColorBinding,
}

impl RenderServer for SlicedObjectServer {
    fn instance(context: &WgpuContext) -> Self {
        let toolpath_context = TraceContext::default();

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

        let trace_color_group = ColorBinding::new_with_default([1.0, 1.0, 1.0, 1.0]);

        let pipeline = PipelineBuilder::new(context.device.clone())
            .with_primitive(wgpu::PrimitiveState {
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
            })
            .build(
                "Toolpath Render Pipeline",
                include_str!("sliced_shader.wgsl"),
                &[
                    &context.camera_bind_group_layout,
                    &context.light_bind_group_layout,
                    &context.transform_bind_group_layout,
                    trace_color_group.layout(),
                    &toolpath_bind_group_layout,
                ],
                &[TraceVertex::desc()],
                context.surface_format,
            );

        Self {
            queued: None,
            sliced_object: None,
            sliced_gcode: None,

            hitbox: HitboxRoot::root(),
            pipeline,

            travel_visible: false,
            fiber_visible: true,

            toolpath_context,
            toolpath_context_bind_group,
            toolpath_context_buffer,

            trace_color_group,
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(toolpath) = self.sliced_object.as_ref() {
            render_pass.set_bind_group(4, &self.toolpath_context_bind_group, &[]);

            render_pass.set_pipeline(&self.pipeline);

            if self.fiber_visible {
                toolpath.model.render_fiber(render_pass);
            }

            render_pass.set_bind_group(3, self.trace_color_group.binding(), &[]);
            toolpath.model.render_without_color(render_pass);

            // render_pass.set_bind_group(3, self.fiber_trace_color_group.binding(), &[]);
        }
    }
}

impl SlicedObjectServer {
    pub fn render_travel<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.travel_visible {
            return;
        }

        if let Some(toolpath) = self.sliced_object.as_ref() {
            toolpath.model.render_travel(render_pass);
        }
    }

    pub fn load_from_slice_result(&mut self, slice_result: SliceResult, process: Arc<Process>) {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            process.set_task("Loading toolpath".to_string());
            process.set_progress(0.8);

            let obj =
                SlicedObject::from_commands(&slice_result.moves, &slice_result.settings, &process)
                    .expect("Failed to load toolpath");

            process.set_task("Build GCode".to_string());
            process.set_progress(0.9);

            let mut writer = GCodeMemoryWriter::new();
            let navigator =
                write_gcode(&slice_result.moves, &slice_result.settings, &mut writer).unwrap();

            let sliced_gcode = writer.finish(navigator);

            process.set_progress(1.0);

            tx.send((obj, sliced_gcode, process)).unwrap();
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
                let mut writer = GCodeFileWriter::new(&mut writer);

                match write_gcode(&toolpath.moves, &toolpath.settings, &mut writer) {
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

    pub fn export_stl_layers(&self) {
        if let Some(toolpath) = self.sliced_object.as_ref() {
            let path = FileDialog::new()
                .set_title("Export Stl Layers")
                .show_open_single_dir()
                .unwrap();

            if let Some(path) = path {
                let layers =
                    from_commands_into_layers(&toolpath.moves, &toolpath.settings).unwrap();

                for layer_vertices in layers {
                    let mut triangles = Vec::new();

                    (0..triangles.len())
                        .step_by(3)
                        .fold(Vec::new(), |triangles, vertex| {});

                    stl_io::write_stl(writer, mesh)
                }
            }
        }
    }

    pub fn update(&mut self, global_state: GlobalState<RootEvent>) -> Result<(), ()> {
        if let Some((rx, _)) = &mut self.queued {
            if let Ok((toolpath, gcode, process)) = rx.try_recv() {
                process.finish();

                global_state
                    .ui_event_writer
                    .send(crate::ui::UiEvent::ShowSuccess("Gcode loaded".to_string()));

                self.hitbox.clear();
                self.hitbox.add_node(toolpath.model.clone());

                self.sliced_object = Some(toolpath);
                self.sliced_gcode = Some(gcode);
            }
        }

        Ok(())
    }

    pub fn set_transparency(&mut self, transparency: f32) {
        self.trace_color_group.set_transparency(transparency);
    }

    pub fn enable_travel(&mut self, visible: bool) {
        self.travel_visible = visible;
    }

    pub fn enable_fiber(&mut self, visible: bool) {
        self.fiber_visible = visible;
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

    pub fn max_layer(&self) -> &u32 {
        &self.toolpath_context.max_layer
    }

    pub fn min_layer(&self) -> &u32 {
        &self.toolpath_context.min_layer
    }

    pub fn visibility(&self) -> &u32 {
        &self.toolpath_context.visibility
    }

    pub fn is_fiber_visible(&self) -> bool {
        self.fiber_visible
    }

    pub fn is_travel_visible(&self) -> bool {
        self.travel_visible
    }

    pub fn get_sliced(&self) -> Option<&SlicedObject> {
        self.sliced_object.as_ref()
    }

    pub fn get_gcode(&self) -> Option<&SlicedGCode> {
        self.sliced_gcode.as_ref()
    }

    pub fn check_hit(&self, ray: &crate::input::Ray, level: usize) -> Option<Arc<TraceTree>> {
        self.hitbox.check_hit(ray, level, false)
    }
}
