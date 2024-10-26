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
        let bounding_box = BoundingBox::new(vec3(-110.0, 0.0, -110.0), vec3(110.0, 250.0, 110.0));

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

        let mut bed = Model::create();
        bed.awaken(&vertices);

        let mut r#box = Model::create();
        r#box.awaken(&visual.wires);

        let mut grid_model = Model::create();
        grid_model.awaken(&grid.to_visual(10.0));

        Self {
            bed,
            r#box,
            grid_model,
        }
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
        let color = vec4(0.0, 1.0, 1.0, 1.0).to_array();

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
