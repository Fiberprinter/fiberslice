use glam::{vec4, Vec3};

use crate::{
    input::hitbox::Hitbox,
    render::{model::TransformMut, Vertex},
    viewer::Visual,
};

use super::{
    mesh::{Mesh, WireMesh},
    QuadFace, SelectBox,
};

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub max: Vec3,
    pub min: Vec3,

    init_max: Vec3,
    init_min: Vec3,
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            max: Vec3::new(f32::MIN, f32::MIN, f32::MIN),
            min: Vec3::new(f32::MAX, f32::MAX, f32::MAX),

            init_max: Vec3::new(f32::MIN, f32::MIN, f32::MIN),
            init_min: Vec3::new(f32::MAX, f32::MAX, f32::MAX),
        }
    }
}

impl BoundingBox {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            max,
            min,
            init_max: max,
            init_min: min,
        }
    }

    pub fn init_min(&self) -> Vec3 {
        self.init_min
    }

    pub fn init_max(&self) -> Vec3 {
        self.init_max
    }

    pub fn center(&self) -> Vec3 {
        (self.max + self.min) / 2.0
    }

    pub fn diagonal(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn expand(&mut self, other: &Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.init_min = self.init_min.max(other.min);
        self.init_max = self.init_max.max(other.max);
    }

    pub fn expand_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
        self.init_min = self.init_min.min(point);
        self.init_max = self.init_max.max(point);
    }

    pub fn contains(&self, point: Vec3) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
            && self.min.z <= point.z
            && point.z <= self.max.z
    }

    pub fn faces(&self) -> [QuadFace; 6] {
        [
            QuadFace {
                normal: Vec3::new(1.0, 0.0, 0.0),
                point: Vec3::new(self.max.x, self.max.y, self.max.z),
                max: Vec3::new(self.max.x, self.max.y, self.max.z),
                min: Vec3::new(self.max.x, self.min.y, self.min.z),
            },
            QuadFace {
                normal: Vec3::new(-1.0, 0.0, 0.0),
                point: Vec3::new(self.min.x, self.max.y, self.max.z),
                max: Vec3::new(self.min.x, self.max.y, self.max.z),
                min: Vec3::new(self.min.x, self.min.y, self.min.z),
            },
            QuadFace {
                normal: Vec3::new(0.0, 1.0, 0.0),
                point: Vec3::new(self.max.x, self.max.y, self.max.z),
                max: Vec3::new(self.max.x, self.max.y, self.max.z),
                min: Vec3::new(self.min.x, self.max.y, self.min.z),
            },
            QuadFace {
                normal: Vec3::new(0.0, -1.0, 0.0),
                point: Vec3::new(self.max.x, self.min.y, self.max.z),
                max: Vec3::new(self.max.x, self.min.y, self.max.z),
                min: Vec3::new(self.min.x, self.min.y, self.min.z),
            },
            QuadFace {
                normal: Vec3::new(0.0, 0.0, 1.0),
                point: Vec3::new(self.max.x, self.max.y, self.max.z),
                max: Vec3::new(self.max.x, self.max.y, self.max.z),
                min: Vec3::new(self.min.x, self.min.y, self.max.z),
            },
            QuadFace {
                normal: Vec3::new(0.0, 0.0, -1.0),
                point: Vec3::new(self.max.x, self.max.y, self.min.z),
                max: Vec3::new(self.max.x, self.max.y, self.min.z),
                min: Vec3::new(self.min.x, self.min.y, self.min.z),
            },
        ]
    }
}

impl TransformMut for BoundingBox {
    fn transform(&mut self, transform: glam::Mat4) {
        self.min = transform.transform_point3(self.init_min);
        self.max = transform.transform_point3(self.init_max);

        self.min = self.min.min(self.max);
        self.max = self.max.max(self.min);
    }
}

impl Hitbox for BoundingBox {
    fn check_hit(&self, ray: &crate::input::Ray) -> Option<f32> {
        // bounding box min max

        if self.contains(ray.origin) {
            return Some(0.0);
        }

        let mut min = None;

        for quad_face in self.faces() {
            let distance = quad_face.check_hit(ray);

            if let Some(distance) = distance {
                if min.unwrap_or(f32::MAX) > distance || min.is_none() {
                    min = Some(distance);
                }
            }
        }

        min
    }

    fn get_min(&self) -> Vec3 {
        self.min
    }

    fn get_max(&self) -> Vec3 {
        self.max
    }
}

impl BoundingBox {
    pub fn to_select_visual(self, border_f: f32) -> Visual<72, 48> {
        let diagonal = self.max - self.min;
        let distance = diagonal.x.min(diagonal.y).min(diagonal.z);

        let select_smaller_box: SelectBox = SelectBox::from(BoundingBox::new(
            self.min - distance * border_f,
            self.max + distance * border_f,
        ))
        .with_color(vec4(1.0, 0.0, 0.0, 1.0), vec4(0.0, 1.0, 1.0, 1.0));

        let mut wires = [Vertex::default(); 48];

        wires[..24].copy_from_slice(&select_smaller_box.to_wire_vertices());

        Visual {
            vertices: select_smaller_box.to_triangle_vertices(),
            wires,
        }
    }
}
