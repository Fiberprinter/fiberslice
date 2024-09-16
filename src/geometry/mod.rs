use glam::{vec3, vec4, Vec3, Vec4};
use mesh::{construct_triangle_vertices, construct_wire_vertices, WireMesh};

pub mod r#box;
pub mod mesh;

pub use r#box::BoundingBox;
use rether::{
    picking::Hitbox,
    vertex::Vertex,
    {Rotate, Scale, Translate},
};

use crate::viewer::gcode::mesh::ProfileCross;

#[derive(Debug, Clone, Copy)]
pub struct QuadFace {
    pub normal: Vec3,
    pub point: Vec3,
    pub min: Vec3,
    pub max: Vec3,
}

impl Translate for QuadFace {
    fn translate(&mut self, translation: Vec3) {
        self.min += translation;
        self.max += translation;
        self.point += translation;
    }
}

impl Rotate for QuadFace {
    fn rotate(&mut self, rotation: glam::Quat) {
        todo!("Implement rotate for QuadFace")
    }
}

impl Scale for QuadFace {
    fn scale(&mut self, scale: Vec3) {
        todo!("Implement scale for QuadFace")
    }
}

impl Hitbox for QuadFace {
    fn check_hit(&self, ray: &rether::picking::Ray) -> Option<f32> {
        let intersection = ray.intersection_plane(self.normal, self.point);

        const EPSILON: f32 = 0.0001;

        // check if the intersection point is inside the face with epsilon
        if (self.max.x + EPSILON) >= intersection.x
            && intersection.x >= (self.min.x - EPSILON)
            && (self.max.y + EPSILON) >= intersection.y
            && intersection.y >= (self.min.y - EPSILON)
            && (self.max.z + EPSILON) >= intersection.z
            && intersection.z >= (self.min.z - EPSILON)
        {
            let distance = (intersection - ray.origin).length();

            Some(distance)
        } else {
            None
        }
    }

    fn expand_hitbox(&mut self, _box: &dyn Hitbox) {
        panic!("QuadFace does not have an expand method");
    }

    fn set_enabled(&mut self, _enabled: bool) {
        panic!("QuadFace does not have an enabled method");
    }

    fn enabled(&self) -> bool {
        panic!("QuadFace does not have an enabled method");
    }

    fn get_min(&self) -> Vec3 {
        self.min
    }

    fn get_max(&self) -> Vec3 {
        self.max
    }
}

#[derive(Debug, Clone)]
pub struct ProfileExtrusion {
    profile_start: ProfileCross,
    profile_end: ProfileCross,
}

impl ProfileExtrusion {
    pub fn new(profile_start: ProfileCross, profile_end: ProfileCross) -> Self {
        Self {
            profile_start,
            profile_end,
        }
    }

    pub fn scaled(self, scale: f32) -> Self {
        Self {
            profile_start: self.profile_start.scaled(scale),
            profile_end: self.profile_end.scaled(scale),
        }
    }
}

impl Translate for ProfileExtrusion {
    fn translate(&mut self, translation: Vec3) {
        self.profile_start.translate(translation);
        self.profile_end.translate(translation);
    }
}

impl Rotate for ProfileExtrusion {
    fn rotate(&mut self, rotation: glam::Quat) {
        self.profile_start.rotate(rotation);
        self.profile_end.rotate(rotation);
    }
}

impl Scale for ProfileExtrusion {
    fn scale(&mut self, scale: Vec3) {
        self.profile_start.scale(scale);
        self.profile_end.scale(scale);
    }
}

pub struct SelectBox {
    box_: ProfileExtrusion,
    triangle_color: Option<Vec4>,
    wire_color: Option<Vec4>,
    corner_expansion: f32,
}

impl From<BoundingBox> for SelectBox {
    fn from(box_: BoundingBox) -> Self {
        // box_.expand_point(box_.max + Vec3::new(2.0, 2.0, 2.0));
        // box_.expand_point(box_.min + Vec3::new(-2.0, -2.0, -2.0));

        let box_ = ProfileExtrusion {
            profile_start: ProfileCross {
                a: box_.min,
                c: vec3(box_.max.x, box_.max.y, box_.min.z),
                b: vec3(box_.min.x, box_.max.y, box_.min.z),
                d: vec3(box_.max.x, box_.min.y, box_.min.z),
            },
            profile_end: ProfileCross {
                a: box_.max,
                c: vec3(box_.min.x, box_.min.y, box_.max.z),
                b: vec3(box_.min.x, box_.max.y, box_.max.z),
                d: vec3(box_.max.x, box_.min.y, box_.max.z),
            },
        };

        Self {
            box_,
            triangle_color: None,
            wire_color: None,
            corner_expansion: 0.2,
        }
    }
}

impl From<ProfileExtrusion> for SelectBox {
    fn from(box_: ProfileExtrusion) -> Self {
        Self {
            box_,
            triangle_color: None,
            wire_color: None,
            corner_expansion: 0.2,
        }
    }
}

impl SelectBox {
    pub fn with_color(mut self, triangle: Vec4, wire: Vec4) -> Self {
        self.triangle_color = Some(triangle);
        self.wire_color = Some(wire);
        self
    }

    pub fn with_corner_expansion(mut self, corner_expansion: f32) -> Self {
        self.corner_expansion = corner_expansion;
        self
    }

    pub const fn traingle_vertex_count() -> usize {
        72
    }

    pub const fn wire_vertex_count() -> usize {
        28
    }
}

impl crate::geometry::mesh::Mesh<72> for SelectBox {
    fn to_triangle_vertices(&self) -> [Vertex; 72] {
        let max = self
            .box_
            .profile_end
            .max()
            .max(self.box_.profile_start.max());
        let min = self
            .box_
            .profile_end
            .min()
            .min(self.box_.profile_start.min());

        let corner_expansion = self.corner_expansion
            * (min.x - max.x)
                .abs()
                .min((min.y - max.y).abs())
                .min((min.z - max.z).abs());

        construct_triangle_vertices(
            [
                vec3(min.x, min.y, min.z),
                vec3(min.x, min.y + corner_expansion, min.z),
                vec3(min.x, min.y, min.z + corner_expansion),
                vec3(min.x, min.y, min.z),
                vec3(min.x, min.y + corner_expansion, min.z),
                vec3(min.x + corner_expansion, min.y, min.z),
                vec3(min.x, min.y, min.z),
                vec3(min.x, min.y, min.z + corner_expansion),
                vec3(min.x + corner_expansion, min.y, min.z),
                vec3(max.x, max.y, max.z),
                vec3(max.x, max.y - corner_expansion, max.z),
                vec3(max.x, max.y, max.z - corner_expansion),
                vec3(max.x, max.y, max.z),
                vec3(max.x, max.y - corner_expansion, max.z),
                vec3(max.x - corner_expansion, max.y, max.z),
                vec3(max.x, max.y, max.z),
                vec3(max.x, max.y, max.z - corner_expansion),
                vec3(max.x - corner_expansion, max.y, max.z),
                vec3(min.x, max.y, min.z),
                vec3(min.x, max.y - corner_expansion, min.z),
                vec3(min.x, max.y, min.z + corner_expansion),
                vec3(min.x, max.y, min.z),
                vec3(min.x, max.y - corner_expansion, min.z),
                vec3(min.x + corner_expansion, max.y, min.z),
                vec3(min.x, max.y, min.z),
                vec3(min.x, max.y, min.z + corner_expansion),
                vec3(min.x + corner_expansion, max.y, min.z),
                vec3(max.x, min.y, max.z),
                vec3(max.x, min.y + corner_expansion, max.z),
                vec3(max.x, min.y, max.z - corner_expansion),
                vec3(max.x, min.y, max.z),
                vec3(max.x, min.y + corner_expansion, max.z),
                vec3(max.x - corner_expansion, min.y, max.z),
                vec3(max.x, min.y, max.z),
                vec3(max.x, min.y, max.z - corner_expansion),
                vec3(max.x - corner_expansion, min.y, max.z),
                vec3(min.x, min.y, max.z),
                vec3(min.x, min.y + corner_expansion, max.z),
                vec3(min.x, min.y, max.z - corner_expansion),
                vec3(min.x, min.y, max.z),
                vec3(min.x, min.y + corner_expansion, max.z),
                vec3(min.x + corner_expansion, min.y, max.z),
                vec3(min.x, min.y, max.z),
                vec3(min.x, min.y, max.z - corner_expansion),
                vec3(min.x + corner_expansion, min.y, max.z),
                vec3(max.x, max.y, min.z),
                vec3(max.x, max.y - corner_expansion, min.z),
                vec3(max.x, max.y, min.z + corner_expansion),
                vec3(max.x, max.y, min.z),
                vec3(max.x, max.y - corner_expansion, min.z),
                vec3(max.x - corner_expansion, max.y, min.z),
                vec3(max.x, max.y, min.z),
                vec3(max.x, max.y, min.z + corner_expansion),
                vec3(max.x - corner_expansion, max.y, min.z),
                vec3(min.x, max.y, max.z),
                vec3(min.x, max.y - corner_expansion, max.z),
                vec3(min.x, max.y, max.z - corner_expansion),
                vec3(min.x, max.y, max.z),
                vec3(min.x, max.y - corner_expansion, max.z),
                vec3(min.x + corner_expansion, max.y, max.z),
                vec3(min.x, max.y, max.z),
                vec3(min.x, max.y, max.z - corner_expansion),
                vec3(min.x + corner_expansion, max.y, max.z),
                vec3(max.x, min.y, min.z),
                vec3(max.x, min.y + corner_expansion, min.z),
                vec3(max.x, min.y, min.z + corner_expansion),
                vec3(max.x, min.y, min.z),
                vec3(max.x, min.y + corner_expansion, min.z),
                vec3(max.x - corner_expansion, min.y, min.z),
                vec3(max.x, min.y, min.z),
                vec3(max.x, min.y, min.z + corner_expansion),
                vec3(max.x - corner_expansion, min.y, min.z),
            ],
            self.triangle_color.unwrap_or(vec4(0.0, 0.0, 0.0, 1.0)),
        )
    }
}

impl WireMesh<24> for SelectBox {
    fn to_wire_vertices(&self) -> [Vertex; 24] {
        construct_wire_vertices(
            [
                self.box_.profile_start.a,
                self.box_.profile_start.d,
                self.box_.profile_start.d,
                self.box_.profile_start.c,
                self.box_.profile_start.c,
                self.box_.profile_start.b,
                self.box_.profile_start.b,
                self.box_.profile_start.a,
                //end
                self.box_.profile_end.a,
                self.box_.profile_end.d,
                self.box_.profile_end.d,
                self.box_.profile_end.c,
                self.box_.profile_end.c,
                self.box_.profile_end.b,
                self.box_.profile_end.b,
                self.box_.profile_end.a,
                // connection
                self.box_.profile_start.a,
                self.box_.profile_end.c,
                self.box_.profile_start.d,
                self.box_.profile_end.d,
                self.box_.profile_start.c,
                self.box_.profile_end.a,
                self.box_.profile_start.b,
                self.box_.profile_end.b,
            ],
            self.wire_color.unwrap_or(vec4(0.0, 0.0, 0.0, 1.0)),
        )
    }
}
