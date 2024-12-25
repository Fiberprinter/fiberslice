use crate::render;

use super::DefaultPipelines;

pub struct RenderDescriptor<'a> {
    pub(super) pipelines: &'a render::DefaultPipelines,
    pub(super) bind_groups: &'a [&'a wgpu::BindGroup],

    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) viewport: &'a render::Viewport,
    pub(super) pass_descriptor: wgpu::RenderPassDescriptor<'a>,
}

impl<'a> RenderDescriptor<'a> {
    pub fn pass(&mut self) -> Option<(&DefaultPipelines, wgpu::RenderPass)> {
        let (x, y, width, height) = *self.viewport;

        if width > 0.0 && height > 0.0 {
            let mut render_pass = self.encoder.begin_render_pass(&self.pass_descriptor);

            render_pass.set_viewport(x, y, width, height, 0.0, 1.0);

            for (index, bind_group) in self.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(index as u32, bind_group, &[]);
            }

            Some((self.pipelines, render_pass))
        } else {
            None
        }
    }
}
