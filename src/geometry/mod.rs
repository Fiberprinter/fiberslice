use glam::{vec3, vec4, Vec3, Vec4};
use mesh::{construct_triangle_vertices, construct_wire_vertices, WireMesh};

pub mod r#box;
pub mod mesh;

pub use r#box::BoundingBox;

use crate::{input::hitbox::Hitbox, render::Vertex, viewer::trace::mesh::TraceCrossSection};

#[derive(Debug, Clone, Copy)]
pub struct QuadFace {
    pub normal: Vec3,
    pub point: Vec3,
    pub min: Vec3,
    pub max: Vec3,
}

impl QuadFace {
    pub fn with_transform(mut self, transform: glam::Mat4) -> Self {
        self.point = transform.transform_point3(self.point);
        self.normal = transform.transform_vector3(self.normal);
        self.min = transform.transform_point3(self.min);
        self.max = transform.transform_point3(self.max);

        self
    }
}

impl Hitbox for QuadFace {
    fn check_hit(&self, ray: &crate::input::Ray) -> Option<f32> {
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

    fn get_min(&self) -> Vec3 {
        self.min
    }

    fn get_max(&self) -> Vec3 {
        self.max
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileExtrusion {
    profile_start: TraceCrossSection,
    profile_end: TraceCrossSection,
}

#[allow(dead_code)]
impl ProfileExtrusion {
    pub fn new(profile_start: TraceCrossSection, profile_end: TraceCrossSection) -> Self {
        Self {
            profile_start,
            profile_end,
        }
    }

    pub fn with_transform(mut self, transform: glam::Mat4) -> Self {
        self.profile_start.a = transform.transform_point3(self.profile_start.a);
        self.profile_start.b = transform.transform_point3(self.profile_start.b);
        self.profile_start.c = transform.transform_point3(self.profile_start.c);
        self.profile_start.d = transform.transform_point3(self.profile_start.d);

        self.profile_end.a = transform.transform_point3(self.profile_end.a);
        self.profile_end.b = transform.transform_point3(self.profile_end.b);
        self.profile_end.c = transform.transform_point3(self.profile_end.c);
        self.profile_end.d = transform.transform_point3(self.profile_end.d);

        self
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
            profile_start: TraceCrossSection {
                a: box_.min,
                c: vec3(box_.max.x, box_.max.y, box_.min.z),
                b: vec3(box_.min.x, box_.max.y, box_.min.z),
                d: vec3(box_.max.x, box_.min.y, box_.min.z),
            },
            profile_end: TraceCrossSection {
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

#[allow(dead_code)]
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

    pub const fn triangle_vertex_count() -> usize {
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
