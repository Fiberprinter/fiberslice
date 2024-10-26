use glam::{Vec2, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::render::model::Transform;

#[derive(Debug, Clone)]
pub enum Action {
    Mouse(MouseButton),
    Keyboard(KeyCode),
}

#[derive(Debug, Clone)]
pub struct DragEvent {
    pub delta: Vec2,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct ScrollEvent {
    pub delta: f32,
    pub action: Action,
}

pub trait InteractiveModel {
    fn aabb(&self) -> (Vec3, Vec3);
    fn transformation(&self) -> glam::Mat4;

    fn as_transformable(&self) -> Option<&dyn Transform>;

    fn destroy(&self) {}
}
