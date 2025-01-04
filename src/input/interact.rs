use std::fmt::Debug;

use glam::Vec3;

use crate::{prelude::Destroyable, render::model::Transform};

pub trait InteractiveModel: Debug + Destroyable {
    fn aabb(&self) -> (Vec3, Vec3);
    fn transformation(&self) -> glam::Mat4;

    fn as_transformable(&self) -> Option<&dyn Transform>;

    #[allow(dead_code)]
    fn mouse_left_click(&self) {}

    fn mouse_right_click(&self) {}
}
