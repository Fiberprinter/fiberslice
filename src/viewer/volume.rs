use glam::{vec3, vec4, Vec2, Vec4};

use crate::{
    geometry::{mesh::construct_triangle_vertices, BoundingBox},
    render::model::Model,
    render::{Renderable, Vertex},
};

#[derive(Debug)]
pub struct Volume {
    pub bed: Model<Vertex>,
    pub grid_model: Model<Vertex>,
    pub r#box: Model<Vertex>,
}

impl Volume {
    pub fn instance() -> Self {
        Self {
            bed: Model::create(),
            r#box: Model::create(),
            grid_model: Model::create(),
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

        let grid = Grid::from(bounding_box);

        self.bed.awaken(&vertices);
        self.r#box.awaken(&visual.wires);
        self.grid_model.awaken(&grid.to_visual(10.0));
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.bed.render(render_pass);
    }

    pub fn render_lines<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.r#box.render(render_pass);
        self.grid_model.render(render_pass);
    }
}

#[derive(Debug)]
pub struct Grid {
    min: Vec2,
    max: Vec2,
    z: f32,
}

impl From<BoundingBox> for Grid {
    fn from(bounding_box: BoundingBox) -> Self {
        Self {
            min: Vec2::new(bounding_box.min.x, bounding_box.min.z),
            max: Vec2::new(bounding_box.max.x, bounding_box.max.z),
            z: bounding_box.min.y,
        }
    }
}

impl Grid {
    pub fn to_visual(&self, step: f32) -> Vec<Vertex> {
        let color = vec4(0.0, 0.0, 0.0, 1.0).to_array();

        let mut vertices = Vec::new();

        for x in (self.min.x as i32..self.max.x as i32).step_by(step as usize) {
            vertices.push(Vertex {
                position: vec3(x as f32, self.z, self.min.y).to_array(),
                normal: [0.0, 1.0, 0.0],
                color,
            });

            vertices.push(Vertex {
                position: vec3(x as f32, self.z, self.max.y).to_array(),
                normal: [0.0, 1.0, 0.0],
                color,
            });
        }

        for z in (self.min.y as i32..self.max.y as i32).step_by(step as usize) {
            vertices.push(Vertex {
                position: vec3(self.min.x, self.z, z as f32).to_array(),
                normal: [0.0, 1.0, 0.0],
                color,
            });

            vertices.push(Vertex {
                position: vec3(self.max.x, self.z, z as f32).to_array(),
                normal: [0.0, 1.0, 0.0],
                color,
            });
        }

        vertices
    }
}
