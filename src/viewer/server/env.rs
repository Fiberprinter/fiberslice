use std::fmt::Debug;

use crate::{
    prelude::WgpuContext,
    viewer::{volume::Volume, RenderServer},
};

#[derive(Debug)]
pub struct EnvironmentServer {
    volume: Volume,
}

impl RenderServer for EnvironmentServer {
    fn instance(_context: &WgpuContext) -> Self {
        Self {
            volume: Volume::instance(),
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render(render_pass);
    }
}

impl EnvironmentServer {
    pub fn update_printer_dimension(&mut self, x: f32, y: f32, z: f32) {
        self.volume.awaken(x, y, z);
    }

    pub fn render_wire<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render_lines(render_pass);
    }
}
