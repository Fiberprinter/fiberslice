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
    pub fn render_lines<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render_lines(render_pass);
    }
}
