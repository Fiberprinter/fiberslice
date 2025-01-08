use glam::{vec3, Vec4};

use crate::{
    geometry::{mesh::construct_triangle_vertices, BoundingBox},
    render::model::Model,
    render::{Renderable, Vertex},
};

#[derive(Debug)]
pub struct Volume {
    bounding_box: BoundingBox,
    bed: Model<Vertex>,
    // grid_model: Model<Vertex>,
    r#box: Model<Vertex>,
}

impl Volume {
    pub fn instance() -> Self {
        Self {
            bounding_box: BoundingBox::new(vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0)),
            bed: Model::create(),
            r#box: Model::create(),
            // grid_model: Model::create(),
        }
    }

    pub fn awaken(&mut self, x: f32, y: f32, z: f32) {
        let bounding_box = BoundingBox::new(vec3(0.0, 0.0, 0.0), vec3(x.abs(), z.abs(), y.abs()));

        let visual = bounding_box.to_select_visual(0.005);

        let vertices = construct_triangle_vertices(
            [
                bounding_box.min,
                vec3(bounding_box.max.x, bounding_box.min.y, bounding_box.max.z),
                vec3(bounding_box.max.x, bounding_box.min.y, bounding_box.min.z),
                vec3(bounding_box.min.x, bounding_box.min.y, bounding_box.max.z),
                vec3(bounding_box.max.x, bounding_box.min.y, bounding_box.max.z),
                bounding_box.min,
            ],
            Vec4::new(0.4, 0.4, 0.4, 0.5),
        );

        self.bounding_box = bounding_box;
        self.bed.awaken(&vertices);
        self.r#box.awaken(&visual.wires);
    }

    pub fn bounding_box(&self) -> &BoundingBox {
        &self.bounding_box
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.bed.render(render_pass);
    }

    pub fn render_lines<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.r#box.render(render_pass);
    }
}
