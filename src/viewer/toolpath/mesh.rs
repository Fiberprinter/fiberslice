use glam::{Vec3, Vec4};

use crate::{
    geometry::{
        mesh::{construct_triangle_vertices, Mesh},
        QuadFace,
    },
    picking::{hitbox::Hitbox, Ray},
    render::{
        model::{RotateMut, ScaleMut, TranslateMut},
        Vertex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct ProfileCross {
    pub a: Vec3,
    pub c: Vec3,
    pub b: Vec3,
    pub d: Vec3,
}

impl ProfileCross {
    pub fn from_direction(direction: Vec3, horizontal: f32, vertical: f32) -> Self {
        let horizontal_radius = horizontal / 2.0;
        let vertical_radius = vertical / 2.0;

        let horizontal = if direction.z.abs() > 0.0 {
            direction.cross(Vec3::X)
        } else {
            direction.cross(Vec3::Z)
        };

        let vertical = direction.cross(horizontal);

        Self {
            a: vertical.normalize() * vertical_radius,
            c: vertical.normalize() * -vertical_radius,
            b: horizontal.normalize() * horizontal_radius,
            d: horizontal.normalize() * -horizontal_radius,
        }
    }

    pub fn axis_aligned(self) -> Self {
        let horizontal = self.b - self.d;
        let vertical = self.a - self.c;

        let corner = self.a - (horizontal / 2.0);

        ProfileCross {
            a: corner,
            c: corner - vertical + horizontal,
            b: corner - vertical,
            d: corner + horizontal,
        }
    }

    pub fn scaled(self, scale: f32) -> Self {
        let diagonal_1 = (self.a - self.c) * scale;
        let diagonal_2 = (self.b - self.d) * scale;

        let center = (self.a + self.c + self.b + self.d) / 4.0;

        Self {
            a: center + diagonal_1 / 2.0,
            c: center - diagonal_1 / 2.0,
            b: center + diagonal_2 / 2.0,
            d: center - diagonal_2 / 2.0,
        }
    }

    pub fn with_offset(self, offset: Vec3) -> Self {
        Self {
            a: self.a + offset,
            c: self.c + offset,
            b: self.b + offset,
            d: self.d + offset,
        }
    }

    pub fn min(&self) -> Vec3 {
        self.a.min(self.c).min(self.b).min(self.d)
    }

    pub fn max(&self) -> Vec3 {
        self.a.max(self.c).max(self.b).max(self.d)
    }
}

impl TranslateMut for ProfileCross {
    fn translate(&mut self, translation: Vec3) {
        self.a += translation;
        self.c += translation;
        self.b += translation;
        self.d += translation;
    }
}

impl RotateMut for ProfileCross {
    fn rotate(&mut self, rotation: glam::Quat) {
        self.a = rotation * self.a;
        self.c = rotation * self.c;
        self.b = rotation * self.b;
        self.d = rotation * self.d;
    }
}

impl ScaleMut for ProfileCross {
    fn scale(&mut self, scale: Vec3) {
        let diagonal_1 = (self.a - self.c) * scale;
        let diagonal_2 = (self.b - self.d) * scale;

        let center = (self.a + self.c + self.b + self.d) / 4.0;

        self.a = center + diagonal_1 / 2.0;
        self.c = center - diagonal_1 / 2.0;
        self.b = center + diagonal_2 / 2.0;
        self.d = center - diagonal_2 / 2.0;
    }
}

pub struct ProfileCrossMesh {
    profile: ProfileCross,
    color: Option<Vec4>,
}

impl ProfileCrossMesh {
    pub fn from_profile(profile: ProfileCross) -> Self {
        Self {
            profile,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = Some(color);
        self
    }
}

impl Mesh<6> for ProfileCrossMesh {
    fn to_triangle_vertices(&self) -> [Vertex; 6] {
        construct_triangle_vertices(
            [
                self.profile.a,
                self.profile.d,
                self.profile.c,
                self.profile.a,
                self.profile.c,
                self.profile.b,
            ],
            self.color.unwrap_or(Vec4::new(0.0, 0.0, 0.0, 1.0)),
        )
    }

    fn to_triangle_vertices_flipped(&self) -> [Vertex; 6] {
        construct_triangle_vertices(
            [
                self.profile.a,
                self.profile.c,
                self.profile.d,
                self.profile.a,
                self.profile.b,
                self.profile.c,
            ],
            self.color.unwrap_or(Vec4::new(0.0, 0.0, 0.0, 1.0)),
        )
    }
}

pub struct MoveMesh {
    profile_start: ProfileCross,
    profile_end: ProfileCross,
    color: Option<Vec4>,
}

impl MoveMesh {
    pub fn from_profiles(profile_start: ProfileCross, profile_end: ProfileCross) -> Self {
        Self {
            profile_start,
            profile_end,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = Some(color);
        self
    }
}

pub const MOVE_MESH_VERTICES: usize = 24;

impl Mesh<MOVE_MESH_VERTICES> for MoveMesh {
    fn to_triangle_vertices(&self) -> [Vertex; MOVE_MESH_VERTICES] {
        construct_triangle_vertices(
            [
                // asdasd
                self.profile_end.d,
                self.profile_end.a,
                self.profile_start.a,
                self.profile_end.d,
                self.profile_start.a,
                self.profile_start.d,
                // asdasd
                self.profile_end.c,
                self.profile_end.d,
                self.profile_start.c,
                self.profile_end.d,
                self.profile_start.d,
                self.profile_start.c,
                // asdasd
                self.profile_end.b,
                self.profile_end.c,
                self.profile_start.c,
                self.profile_end.b,
                self.profile_start.c,
                self.profile_start.b,
                // asdasd
                self.profile_end.a,
                self.profile_end.b,
                self.profile_start.a,
                self.profile_end.b,
                self.profile_start.b,
                self.profile_start.a,
            ],
            self.color.unwrap_or(Vec4::new(0.0, 0.0, 0.0, 1.0)),
        )
    }
}

pub struct MoveConnectionMesh {
    profile_start: ProfileCross,
    profile_end: ProfileCross,
    color: Option<Vec4>,
}

impl MoveConnectionMesh {
    pub fn from_profiles(profile_start: ProfileCross, profile_end: ProfileCross) -> Self {
        Self {
            profile_start,
            profile_end,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = Some(color);
        self
    }
}

impl Mesh<12> for MoveConnectionMesh {
    fn to_triangle_vertices(&self) -> [Vertex; 12] {
        construct_triangle_vertices(
            [
                self.profile_start.d,
                self.profile_end.d,
                self.profile_start.a,
                // asdasd
                self.profile_start.c,
                self.profile_start.d,
                self.profile_end.d,
                // asdasd
                self.profile_start.b,
                self.profile_end.b,
                self.profile_start.c,
                // asdasd
                self.profile_end.b,
                self.profile_start.b,
                self.profile_start.a,
            ],
            self.color.unwrap_or(Vec4::new(0.0, 0.0, 0.0, 1.0)),
        )
    }
}

impl From<MoveMesh> for MoveHitbox {
    fn from(val: MoveMesh) -> Self {
        let north_west = QuadFace {
            normal: (val.profile_end.a - val.profile_start.a)
                .cross(val.profile_start.d - val.profile_start.a),
            point: val.profile_start.a,
            max: val
                .profile_end
                .a
                .max(val.profile_start.a)
                .max(val.profile_start.d)
                .max(val.profile_end.d),
            min: val
                .profile_end
                .a
                .min(val.profile_start.a)
                .min(val.profile_start.d)
                .min(val.profile_end.d),
        };

        let north_east = QuadFace {
            normal: (val.profile_end.d - val.profile_start.d)
                .cross(val.profile_start.c - val.profile_start.d),
            point: val.profile_start.d,
            max: val
                .profile_end
                .d
                .max(val.profile_start.d)
                .max(val.profile_start.c)
                .max(val.profile_end.c),
            min: val
                .profile_end
                .d
                .min(val.profile_start.d)
                .min(val.profile_start.c)
                .min(val.profile_end.c),
        };

        let south_west = QuadFace {
            normal: (val.profile_end.c - val.profile_start.c)
                .cross(val.profile_start.b - val.profile_start.c),
            point: val.profile_start.c,
            max: val
                .profile_end
                .c
                .max(val.profile_start.c)
                .max(val.profile_start.b)
                .max(val.profile_end.b),
            min: val
                .profile_end
                .c
                .min(val.profile_start.c)
                .min(val.profile_start.b)
                .min(val.profile_end.b),
        };

        let south_east = QuadFace {
            normal: (val.profile_end.b - val.profile_start.b)
                .cross(val.profile_start.a - val.profile_start.b),
            point: val.profile_start.b,
            max: val
                .profile_end
                .b
                .max(val.profile_start.b)
                .max(val.profile_start.a)
                .max(val.profile_end.a),
            min: val
                .profile_end
                .b
                .min(val.profile_start.b)
                .min(val.profile_start.a)
                .min(val.profile_end.a),
        };

        MoveHitbox {
            north_west,
            north_east,
            south_west,
            south_east,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineHitbox {
    start: Vec3,
    end: Vec3,
}

impl Hitbox for LineHitbox {
    fn check_hit(&self, _ray: &Ray) -> Option<f32> {
        None
    }

    fn expand_hitbox(&mut self, _box: &dyn Hitbox) {
        // Not expandable
    }

    fn set_enabled(&mut self, _enabled: bool) {}

    fn enabled(&self) -> bool {
        true
    }

    fn get_min(&self) -> Vec3 {
        self.start.min(self.end)
    }

    fn get_max(&self) -> Vec3 {
        self.start.max(self.end)
    }
}

#[derive(Debug, Clone)]
pub struct MoveHitbox {
    north_west: QuadFace,
    north_east: QuadFace,
    south_west: QuadFace,
    south_east: QuadFace,
}

impl Hitbox for MoveHitbox {
    fn check_hit(&self, ray: &Ray) -> Option<f32> {
        let faces = [
            &self.north_west,
            &self.north_east,
            &self.south_west,
            &self.south_east,
        ];

        let mut min = None;

        for quad_face in faces {
            let distance = quad_face.check_hit(ray);

            if let Some(distance) = distance {
                if min.unwrap_or(f32::MAX) > distance || min.is_none() {
                    min = Some(distance);
                }
            }
        }

        min
    }

    fn expand_hitbox(&mut self, _box: &dyn Hitbox) {
        // Not expandable
        // TODO either figure out how to expand this or remove this method for this type or make it clear that this is not expandable
    }

    fn set_enabled(&mut self, _enabled: bool) {}

    fn enabled(&self) -> bool {
        true
    }

    fn get_min(&self) -> Vec3 {
        self.north_west
            .min
            .min(self.north_east.min)
            .min(self.south_west.min)
            .min(self.south_east.min)
    }

    fn get_max(&self) -> Vec3 {
        self.north_west
            .max
            .max(self.north_east.max)
            .max(self.south_west.max)
            .max(self.south_east.max)
    }
}
