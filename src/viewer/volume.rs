use glam::vec3;

use crate::{
    geometry::BoundingBox,
    render::model::Model,
    render::{Renderable, Vertex},
};

#[derive(Debug)]
pub struct Volume {
    bounding_box: BoundingBox,
    r#box: Model<Vertex>,
}

impl Volume {
    pub fn instance() -> Self {
        Self {
            bounding_box: BoundingBox::new(vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0)),
            r#box: Model::create(),
        }
    }

    pub fn awaken(&mut self, x: f32, y: f32, z: f32) {
        let bounding_box = BoundingBox::new(vec3(0.0, 0.0, 0.0), vec3(x.abs(), z.abs(), y.abs()));

        let visual = bounding_box.to_select_visual(0.005);

        self.bounding_box = bounding_box;
        self.r#box.awaken(&visual.wires);
    }

    pub fn bounding_box(&self) -> &BoundingBox {
        &self.bounding_box
    }

    pub fn render_lines<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.r#box.render(render_pass);
    }
}
