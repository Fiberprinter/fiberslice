use std::{
    collections::BTreeSet,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use egui::ahash::HashMap;
use egui_code_editor::Syntax;
use glam::Mat4;
use log::{info, warn};
use parking_lot::RwLock;
use server::CADObject;
use shared::{object::ObjectMesh, process::Process};
use slicer::{Mask, MoveType, Settings, SliceResult, SlicedGCode, TraceType};
use trace::bit_representation;
use winit::{
    event::{KeyEvent, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    geometry::BoundingBox,
    input::{interact::InteractiveModel, MouseClickEvent, MouseMotionEvent},
    prelude::{Mode, PrepareMode, WgpuContext},
    render::{RenderDescriptor, Vertex},
    ui::screen::ViewerTooltip,
    GlobalState, RootEvent,
};

mod camera;
pub use camera::*;

pub mod select;
pub mod server;
pub mod trace;

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

    object_selector: RwLock<select::Selector>,
    mask_selector: RwLock<select::Selector>,
    trace_selector: RwLock<select::Selector>,

    tooltip: RwLock<Option<ViewerTooltip>>,
    mode: RwLock<Option<Mode>>,

    transparent_vision: AtomicBool,
}

impl Viewer {
    pub fn instance(context: &WgpuContext) -> Self {
        Self {
            env_server: RwLock::new(server::EnvironmentServer::instance(context)),
            sliced_object_server: RwLock::new(server::SlicedObjectServer::instance(context)),
            object_server: RwLock::new(server::ObjectServer::instance(context)),
            mask_server: RwLock::new(server::MaskServer::instance(context)),

            object_selector: RwLock::new(select::Selector::instance()),
            mask_selector: RwLock::new(select::Selector::instance()),
            trace_selector: RwLock::new(select::Selector::instance()),

            tooltip: RwLock::new(None),
            mode: RwLock::new(None),
            transparent_vision: AtomicBool::new(false),
        }
    }

    pub fn set_mode(&self, mode: Mode) {
        *self.mode.write() = Some(mode);

        self.update_tooltip(None);

        match mode {
            Mode::Prepare(PrepareMode::Objects) => {
                self.object_server.write().set_transparency(1.0);
                self.mask_server.write().set_transparency(0.5);
            }
            Mode::Prepare(PrepareMode::Masks) => {
                self.object_server.write().set_transparency(0.5);
                self.mask_server.write().set_transparency(1.0);
            }
            Mode::Preview => {
                self.object_server.write().set_transparency(0.15);
                self.mask_server.write().set_transparency(0.15);
            }
        }
    }

    pub fn update_printer_dimension(&self, x: f32, y: f32, z: f32) {
        self.env_server.write().update_printer_dimension(x, y, z);
    }

    pub fn update(&self, global_state: &GlobalState<RootEvent>) {
        // self.env_server.write().update(global_state);
        self.object_selector.write().update();
        self.mask_selector.write().update();

        self.sliced_object_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update toolpath server");
        self.object_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update model server");
        self.mask_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update mask server");
    }

    pub fn volume_box(&self) -> BoundingBox {
        *self.env_server.read().volume_box()
    }
}

impl Viewer {
    pub fn gizmo_enabled(&self) -> bool {
        match *self.mode.read() {
            Some(Mode::Preview) => false,
            Some(Mode::Prepare(PrepareMode::Objects)) => {
                !self.object_selector.read().selected().is_empty()
            }
            Some(Mode::Prepare(PrepareMode::Masks)) => {
                !self.mask_selector.read().selected().is_empty()
            }
            _ => false,
        }
    }

    pub fn update_tooltip(&self, tooltip: Option<ViewerTooltip>) {
        *self.tooltip.write() = tooltip;
    }

    pub fn read_tooltip_with_fn(&self, r#fn: impl FnOnce(&ViewerTooltip)) {
        if let Some(tooltip) = &*self.tooltip.read() {
            r#fn(tooltip);
        }
    }

    pub fn transform_selected(&self, r#fn: impl FnMut(&mut Mat4) -> bool) {
        match *self.mode.read() {
            Some(Mode::Prepare(PrepareMode::Objects)) => {
                self.object_selector.write().transform(r#fn);
            }
            Some(Mode::Prepare(PrepareMode::Masks)) => {
                self.mask_selector.write().transform(r#fn);
            }
            _ => (),
        }
    }

    pub fn objects(&self) -> Vec<(String, Arc<CADObject>)> {
        self.object_server.read().models()
    }

    pub fn masks(&self) -> Vec<(String, Arc<CADObject>)> {
        self.mask_server.read().models()
    }

    pub fn sliced_count_map(&self) -> Option<HashMap<TraceType, usize>> {
        self.sliced_object_server
            .read()
            .get_sliced()
            .map(|toolpath| toolpath.count_map.clone())
    }

    pub fn sliced_gcode<R>(&self, read_fn: impl FnOnce(&SlicedGCode) -> R) -> Option<R> {
        let read = self.sliced_object_server.read();
        let gcode = read.get_gcode();

        gcode.map(read_fn)
    }

    pub fn sliced_max_layer(&self) -> Option<u32> {
        self.sliced_object_server
            .read()
            .get_sliced()
            .map(|toolpath| toolpath.max_layer as u32)
    }

    pub fn enable_travel(&self, opaque: bool) {
        self.sliced_object_server.write().enable_travel(opaque);
    }

    pub fn enable_fiber(&self, opaque: bool) {
        self.sliced_object_server.write().enable_fiber(opaque);
    }

    pub fn update_gpu_min_layer(&self, layer: u32) {
        self.sliced_object_server.write().update_min_layer(layer);
    }

    pub fn update_gpu_max_layer(&self, layer: u32) {
        self.sliced_object_server.write().update_max_layer(layer);
    }

    pub fn is_move_active(&self, move_type: &MoveType, layer: u32) -> bool {
        let server_read = self.sliced_object_server.read();

        let (min, max) = (*server_read.min_layer(), *server_read.max_layer());

        let visibility = *server_read.visibility();

        match move_type {
            MoveType::Travel => server_read.is_travel_visible(),
            MoveType::WithoutFiber(trace_type) => {
                if (bit_representation(trace_type) & visibility) > 0
                    && !self.is_transparent_vision()
                {
                    layer >= min && layer <= max
                } else {
                    false
                }
            }
            MoveType::WithFiber(trace_type) => {
                if (bit_representation(trace_type) & visibility) > 0
                    && server_read.is_fiber_visible()
                {
                    layer >= min && layer <= max
                } else {
                    false
                }
            }
        }
    }

    pub fn enable_transparent_vision(&self, b: bool) {
        self.transparent_vision
            .store(b, std::sync::atomic::Ordering::Relaxed);

        if b {
            self.sliced_object_server.write().set_transparency(0.1);
        } else {
            self.sliced_object_server.write().set_transparency(1.0);
        }
    }

    pub fn is_transparent_vision(&self) -> bool {
        self.transparent_vision
            .load(std::sync::atomic::Ordering::Relaxed)
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

    pub fn prepare_masks(&self, settings: &Settings) -> Vec<Mask> {
        self.mask_server.read().prepare_objects(settings)
    }

    pub fn load_sliced(&self, result: SliceResult, process: Arc<Process>) {
        self.sliced_object_server
            .write()
            .load_from_slice_result(result, process);
    }

    pub fn load_object_from_file<P: AsRef<Path>>(&self, path: P) {
        self.object_server.write().load_from_file(path);
    }

    pub fn load_mask_from_file<P: AsRef<Path>>(&self, path: P) {
        self.mask_server.write().load_from_file(path);
    }

    pub fn load_object_from_bytes(&self, name: &str, bytes: &[u8]) {
        self.object_server
            .write()
            .load_from_bytes(name.to_string(), bytes);
    }

    pub fn load_mask_from_bytes(&self, name: &str, bytes: &[u8]) {
        self.mask_server
            .write()
            .load_from_bytes(name.to_string(), bytes);
    }

    pub fn delete_object(&self, obj: &Arc<CADObject>) {
        self.object_selector.write().select(obj.clone());
        self.object_selector.write().delete_selected();
    }

    pub fn select_object(&self, obj: &Arc<CADObject>) {
        self.object_selector.write().select(obj.clone());
    }

    pub fn delete_mask(&self, obj: &Arc<CADObject>) {
        self.mask_selector.write().select(obj.clone());
        self.mask_selector.write().delete_selected();
    }

    pub fn select_mask(&self, obj: &Arc<CADObject>) {
        self.mask_selector.write().select(obj.clone());
    }
}

// input
impl Viewer {
    pub fn mouse_delta(&self, event: MouseMotionEvent) {
        match *self.mode.read() {
            Some(Mode::Prepare(PrepareMode::Objects)) => {
                if let Some(model) = self.object_server.read().check_hit(&event.ray, 0, false) {
                    self.update_tooltip(Some(ViewerTooltip::new(
                        "Obj".to_string(),
                        format!("{}", model),
                    )));
                } else if let Some(model) = self.mask_server.read().check_hit(&event.ray, 0, false)
                {
                    self.update_tooltip(Some(ViewerTooltip::new(
                        "Mask".to_string(),
                        format!("{}", model),
                    )));
                } else {
                    self.update_tooltip(None);
                }
            }
            Some(Mode::Prepare(PrepareMode::Masks)) => {
                if let Some(model) = self.mask_server.read().check_hit(&event.ray, 0, false) {
                    self.update_tooltip(Some(ViewerTooltip::new(
                        "Mask".to_string(),
                        format!("{}", model),
                    )));
                } else if let Some(model) =
                    self.object_server.read().check_hit(&event.ray, 0, false)
                {
                    self.update_tooltip(Some(ViewerTooltip::new(
                        "Obj".to_string(),
                        format!("{}", model),
                    )));
                } else {
                    self.update_tooltip(None);
                }
            }
            _ => (),
        }
    }

    pub fn mouse_input(&self, event: MouseClickEvent) {
        if event.state.is_pressed() {
            if let MouseButton::Right = event.button {
                match *self.mode.read() {
                    Some(Mode::Prepare(PrepareMode::Objects)) => {
                        if let Some(model) =
                            self.object_server.read().check_hit(&event.ray, 0, false)
                        {
                            let interact_model = model as Arc<dyn InteractiveModel>;

                            self.object_selector.write().select(interact_model);
                        }
                    }
                    Some(Mode::Prepare(PrepareMode::Masks)) => {
                        if let Some(model) = self.mask_server.read().check_hit(&event.ray, 0, false)
                        {
                            let interact_model = model as Arc<dyn InteractiveModel>;

                            self.mask_selector.write().select(interact_model);
                        }
                    }
                    Some(Mode::Preview) => {
                        if let Some(model) =
                            self.sliced_object_server.read().check_hit(&event.ray, 2)
                        {
                            info!("Selected: {:?}", model);

                            let interact_model = model as Arc<dyn InteractiveModel>;
                            interact_model.mouse_right_click();

                            self.trace_selector.write().select(interact_model);
                        } else {
                            info!("Nothing selected");
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn keyboard_input(&self, event: KeyEvent) {
        if event.state.is_pressed() {
            if let PhysicalKey::Code(key) = event.physical_key {
                #[allow(clippy::single_match)]
                match key {
                    KeyCode::Delete => match *self.mode.read() {
                        Some(Mode::Prepare(PrepareMode::Objects)) => {
                            self.object_selector.write().delete_selected()
                        }
                        Some(Mode::Prepare(PrepareMode::Masks)) => {
                            self.mask_selector.write().delete_selected()
                        }
                        _ => (),
                    },
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
        let object_server_read = self.object_server.read();
        let mask_server_read = self.mask_server.read();

        if let Some((pipelines, mut render_pass)) = render_descriptor.pass() {
            match mode {
                Mode::Preview => {
                    sliced_object_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_line(&mut render_pass);
                    sliced_object_server_read.render_travel(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.back_cull);
                    mask_server_read.render(&mut render_pass);
                    object_server_read.render(&mut render_pass);
                }
                Mode::Prepare(PrepareMode::Objects) => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    object_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_line(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.back_cull);
                    mask_server_read.render(&mut render_pass);
                }
                Mode::Prepare(PrepareMode::Masks) => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    mask_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_line(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.back_cull);
                    object_server_read.render(&mut render_pass);
                }
            };
        }
    }

    pub fn render_secondary(&self, mut render_descriptor: RenderDescriptor, mode: Mode) {
        let trace_selector_read = self.trace_selector.read();
        let object_selector_read = self.object_selector.read();
        let mask_selector_read = self.mask_selector.read();

        if let Some((pipelines, mut render_pass)) = render_descriptor.pass() {
            match mode {
                Mode::Preview => {
                    // Needs to be rendered first
                    {
                        render_pass.set_pipeline(&pipelines.line);
                        trace_selector_read.render_wire(&mut render_pass);

                        render_pass.set_pipeline(&pipelines.no_cull);
                        trace_selector_read.render(&mut render_pass);
                    }
                }
                Mode::Prepare(PrepareMode::Objects) => {
                    render_pass.set_pipeline(&pipelines.line);
                    object_selector_read.render_wire(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    object_selector_read.render(&mut render_pass);
                }
                Mode::Prepare(PrepareMode::Masks) => {
                    render_pass.set_pipeline(&pipelines.line);
                    mask_selector_read.render_wire(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    mask_selector_read.render(&mut render_pass);
                }
            }
        }
    }
}

unsafe impl Send for Viewer {}
unsafe impl Sync for Viewer {}
