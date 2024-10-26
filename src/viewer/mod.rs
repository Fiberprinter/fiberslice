use std::collections::BTreeSet;

use egui_code_editor::Syntax;
use parking_lot::RwLock;
use wgpu::RenderPass;

use crate::{
    prelude::{Mode, Viewport, WgpuContext},
    render::{self, Pipelines, Vertex},
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
    fn mode_changed(&mut self, mode: Mode) {}
}

#[derive(Debug)]
pub struct Viewer {
    pub env_server: RwLock<server::EnvironmentServer>,
    pub toolpath_server: RwLock<server::ToolpathServer>,
    pub model_server: RwLock<server::CADModelServer>,

    pub selector: RwLock<select::Selector>,
}

impl Viewer {
    pub fn instance(context: &WgpuContext) -> Self {
        Self {
            env_server: RwLock::new(server::EnvironmentServer::instance(context)),
            toolpath_server: RwLock::new(server::ToolpathServer::instance(context)),
            model_server: RwLock::new(server::CADModelServer::instance(context)),
            selector: RwLock::new(select::Selector::instance()),
        }
    }

    pub fn mode_changed(&self, mode: Mode) {
        self.env_server.write().mode_changed(mode);
        self.toolpath_server.write().mode_changed(mode);
        self.model_server.write().mode_changed(mode);
    }

    pub fn update(&self, global_state: &GlobalState<RootEvent>) {
        // self.env_server.write().update(global_state);
        self.toolpath_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update toolpath server");
        self.model_server
            .write()
            .update(global_state.clone())
            .expect("Failed to update model server");
    }
}

// render Viewer functions
impl Viewer {
    pub fn render(&self, mut render_descriptor: render::RenderDescriptor, mode: Mode) {
        let env_server_read = self.env_server.read();
        let toolpath_server_read = self.toolpath_server.read();
        let model_server_read = self.model_server.read();

        if let Some((pipelines, mut render_pass)) = render_descriptor.pass() {
            match mode {
                Mode::Preview => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    toolpath_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_lines(&mut render_pass);
                }
                Mode::Prepare => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    model_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_lines(&mut render_pass);
                }
                Mode::ForceAnalytics => {
                    render_pass.set_pipeline(&pipelines.no_cull);
                    env_server_read.render(&mut render_pass);

                    render_pass.set_pipeline(&pipelines.line);
                    env_server_read.render_lines(&mut render_pass);
                }
            };
        }
    }

    pub fn render_widgets(&self, mut render_descriptor: render::RenderDescriptor, mode: Mode) {
        let model_server_read = self.model_server.read();

        if let Some((pipelines, mut render_pass)) = render_descriptor.pass() {
            match mode {
                Mode::Preview => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    model_server_read.render(&mut render_pass);
                }
                Mode::Prepare => {}
                Mode::ForceAnalytics => {
                    render_pass.set_pipeline(&pipelines.back_cull);
                    model_server_read.render(&mut render_pass);
                }
            }
        }
    }
}

unsafe impl Send for Viewer {}
unsafe impl Sync for Viewer {}
