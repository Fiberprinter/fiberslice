use std::{collections::BTreeSet, path::Path, sync::Arc};

use egui::ahash::HashMap;
use egui_code_editor::Syntax;
use glam::Mat4;
use log::warn;
use parking_lot::RwLock;
use shared::{object::ObjectMesh, process::Process};
use slicer::{MovePrintType, Settings, SliceResult};
use winit::{
    event::{KeyEvent, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    input::{interact::InteractiveModel, MouseInputEvent},
    prelude::{Mode, WgpuContext},
    render::{RenderDescriptor, Vertex},
    GlobalState, RootEvent,
};

mod camera;
pub use camera::*;

pub mod select;
pub mod server;
pub mod toolpath;
pub mod tracker;
pub mod volume;

pub struct Visual<const T: usize, const W: usize> {
    pub vertices: [Vertex; T],
    pub wires: [Vertex; W],
}

pub trait GCodeSyntax {
    fn gcode() -> Syntax;
}

impl GCodeSyntax for Syntax {
    fn gcode() -> Syntax {
        Syntax {
            language: "GCode",
            case_sensitive: true,
            comment: ";",
            comment_multiline: [r#";;"#, r#";;"#],
            hyperlinks: BTreeSet::from([]),
            keywords: BTreeSet::from([
                "G0", "G1", "G2", "G3", "G4", "G10", "G17", "G18", "G19", "G20", "G21", "G28",
            ]),
            types: BTreeSet::from(["X", "Y", "Z", "E", "F"]),
            special: BTreeSet::from(["False", "None", "True"]),
        }
    }
}

#[allow(unused_variables)]
pub trait RenderServer {
    fn instance(context: &WgpuContext) -> Self;
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>);
}

#[derive(Debug)]
pub struct Viewer {
    env_server: RwLock<server::EnvironmentServer>,
    sliced_object_server: RwLock<server::SlicedObjectServer>,
    object_server: RwLock<server::ObjectServer>,
    mask_server: RwLock<server::MaskServer>,

    selector: RwLock<select::Selector>,
}

impl Viewer {
    pub fn instance(context: &WgpuContext) -> Self {
        Self {
            env_server: RwLock::new(server::EnvironmentServer::instance(context)),
            sliced_object_server: RwLock::new(server::SlicedObjectServer::instance(context)),
            object_server: RwLock::new(server::ObjectServer::instance(context)),
            mask_server: RwLock::new(server::MaskServer::instance(context)),
            selector: RwLock::new(select::Selector::instance()),
        }
    }

    pub fn mode_changed(&self, mode: Mode) {
        match mode {
            Mode::Prepare => {
                self.object_server.write().set_transparency(1.0);
                self.mask_server.write().set_transparency(0.5);
            }
            Mode::Masks => {
                self.object_server.write().set_transparency(0.5);
                self.mask_server.write().set_transparency(1.0);
            }
            Mode::Preview => {
                self.object_server.write().set_transparency(0.15);
                self.mask_server.write().set_transparency(0.15);
            }
        }
    }

    pub fn update(&self, global_state: &GlobalState<RootEvent>) {
        // self.env_server.write().update(global_state);
        self.sliced_object_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update toolpath server");
        self.object_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update model server");
    }
}

impl Viewer {
    pub fn transform_selected(&self, r#fn: impl FnMut(&mut Mat4) -> bool) {
        self.selector.write().transform(r#fn);
    }

    pub fn sliced_count_map(&self) -> Option<HashMap<MovePrintType, usize>> {
        self.sliced_object_server
            .read()
            .get_sliced()
            .map(|toolpath| toolpath.count_map.clone())
    }

    pub fn sliced_gcode(&self) -> Option<&str> {
        Some("Not implemented")
    }

    pub fn sliced_max_layer(&self) -> Option<u32> {
        self.sliced_object_server
            .read()
            .get_sliced()
            .map(|toolpath| toolpath.max_layer as u32)
    }

    pub fn update_gpu_min_layer(&self, layer: u32) {
        self.sliced_object_server.write().update_min_layer(layer);
    }

    pub fn update_gpu_max_layer(&self, layer: u32) {
        self.sliced_object_server.write().update_max_layer(layer);
    }

    pub fn update_gpu_visibility(&self, visibility: u32) {
        self.sliced_object_server
            .write()
            .update_visibility(visibility);
    }

    pub fn already_sliced(&self) -> bool {
        self.sliced_object_server.read().get_sliced().is_some()
    }

    pub fn export_gcode(&self) {
        self.sliced_object_server.write().export();
    }
}

// Slicing
impl Viewer {
    pub fn prepare_objects(&self, settings: &Settings) -> Vec<ObjectMesh> {
        self.object_server.read().prepare_objects(settings)
    }

    pub fn prepare_masks(&self, settings: &Settings) -> Vec<ObjectMesh> {
        self.mask_server.read().prepare_objects(settings)
    }

    pub fn load_sliced(&self, result: SliceResult, process: Arc<Process>) {
        self.sliced_object_server
            .write()
            .load_from_slice_result(result, process);
    }

    pub fn load_object_from_file<P: AsRef<Path>>(&self, path: P) {
        self.object_server.write().load(path);
    }

    pub fn load_mask_from_file<P: AsRef<Path>>(&self, path: P) {
        self.mask_server.write().load(path);
    }
}

// input
impl Viewer {
    pub fn mouse_delta(&self, _delta: (f64, f64)) {}

    pub fn mouse_input(&self, event: MouseInputEvent) {
        if event.state.is_pressed() {
            if let MouseButton::Right = event.button {
                if let Some(model) = self.object_server.read().check_hit(&event.ray, 0, true) {
                    let interact_model = model as Arc<dyn InteractiveModel>;

                    self.selector.write().select(interact_model);
                }
            }
        }
    }

    pub fn keyboard_input(&self, event: KeyEvent) {
        if event.state.is_pressed() {
            if let PhysicalKey::Code(key) = event.physical_key {
                #[allow(clippy::single_match)]
                match key {
                    KeyCode::Delete => {
                        self.selector.write().delete_selected();
                    }
                    _ => (),
                }
            } else {
                warn!("Unknown key: {:?}", event);
            }
        }
    }
}

// rendering
impl Viewer {
    pub fn render(&self, mut render_descriptor: RenderDescriptor, mode: Mode) {
        let env_server_read = self.env_server.read();
        let sliced_object_server_read = self.sliced_object_server.read();
        let model_server_read = self.object_server.read();
        let mask_server_read = self.mask_server.read();
        let selector_read = self.selector.read();

        if let Some((pipelines, mut render_pass)) = render_descriptor.pass() {
            match mode {
                Mode::Preview => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    sliced_object_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_lines(&mut render_pass);
                }
                Mode::Prepare => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    model_server_read.render(&mut render_pass);
                    mask_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);
                    selector_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_lines(&mut render_pass);
                    selector_read.render_lines(&mut render_pass);
                }
                Mode::Masks => {
                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);
                    selector_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_lines(&mut render_pass);
                    selector_read.render_lines(&mut render_pass);
                }
            };
        }
    }

    pub fn render_widgets(&self, mut render_descriptor: RenderDescriptor, mode: Mode) {
        let model_server_read = self.object_server.read();
        let mask_server_read = self.mask_server.read();

        if let Some((pipelines, mut render_pass)) = render_descriptor.pass() {
            match mode {
                Mode::Preview => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    mask_server_read.render(&mut render_pass);
                    model_server_read.render(&mut render_pass);
                }
                Mode::Prepare => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    mask_server_read.render(&mut render_pass);
                }
                Mode::Masks => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    model_server_read.render(&mut render_pass);
                }
            }
        }
    }
}

unsafe impl Send for Viewer {}
unsafe impl Sync for Viewer {}
