use std::fmt::Debug;

use crate::{
    prelude::WgpuContext,
    render::Vertex,
    viewer::{select::Selector, volume::Volume, Server},
};

#[derive(Debug)]
pub struct EnvironmentServer {
    volume: Volume,
    selector: Selector,
}

impl Server for EnvironmentServer {
    fn instance(_context: &WgpuContext) -> Self {
        Self {
            volume: Volume::instance(),
            selector: Selector::instance(),
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render(render_pass);
        self.selector.render(render_pass);
    }
}

impl EnvironmentServer {
    pub fn render_lines<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.volume.render_lines(render_pass);
        self.selector.render_lines(render_pass);
    }

    pub fn volume(&self) -> &Volume {
        &self.volume
    }

    pub fn selector_mut(&mut self) -> &mut Selector {
        &mut self.selector
    }
}
