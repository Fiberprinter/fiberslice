use std::sync::Arc;

use glam::Vec3;
use parking_lot::RwLock;
use wgpu::BufferAddress;

use crate::{
    geometry::BoundingBox,
    input::{
        hitbox::{Hitbox, HitboxNode},
        interact::InteractiveModel,
    },
    prelude::LockModel,
    render::{model::Model, Renderable, Vertex},
};

use super::{mesh::TraceHitbox, vertex::TraceVertex};

#[derive(Debug)]
pub enum TraceTree {
    Root {
        model: LockModel<TraceVertex>,
        fiber_model: LockModel<TraceVertex>,
        travel_model: LockModel<Vertex>,
        bounding_box: RwLock<BoundingBox>,
        children: Vec<Arc<Self>>,
        size: BufferAddress,
        travel_size: BufferAddress,
        fiber_size: BufferAddress,
    },
    Travel {
        offset: BufferAddress,
        size: BufferAddress,
        start: RwLock<Vec3>,
        end: RwLock<Vec3>,
    },
    Fiber {
        offset: BufferAddress,
        size: BufferAddress,
        start: RwLock<Vec3>,
        end: RwLock<Vec3>,
    },
    Trace {
        offset: BufferAddress,
        size: BufferAddress,
        r#box: RwLock<Box<TraceHitbox>>,
    },
}

impl TraceTree {
    pub fn create_root() -> Self {
        Self::Root {
            model: LockModel::new(Model::create()),
            travel_model: LockModel::new(Model::create()),
            fiber_model: LockModel::new(Model::create()),

            children: Vec::new(),
            bounding_box: RwLock::new(BoundingBox::default()),
            size: 0,
            travel_size: 0,
            fiber_size: 0,
        }
    }

    pub fn create_travel(offset: BufferAddress, start: Vec3, end: Vec3) -> Self {
        Self::Travel {
            offset,
            size: 2,
            start: RwLock::new(start),
            end: RwLock::new(end),
        }
    }

    pub fn create_fiber(offset: BufferAddress, start: Vec3, end: Vec3) -> Self {
        Self::Fiber {
            offset,
            size: 2,
            start: RwLock::new(start),
            end: RwLock::new(end),
        }
    }

    pub fn create_move(path_box: TraceHitbox, offset: BufferAddress, size: BufferAddress) -> Self {
        Self::Trace {
            offset,
            size,
            r#box: RwLock::new(Box::new(path_box)),
        }
    }

    pub fn push(&mut self, node: Self) {
        match self {
            Self::Root {
                children,
                bounding_box,
                size: model_size,
                travel_size,
                fiber_size,
                ..
            } => {
                match &node {
                    Self::Travel { size, .. } => {
                        *travel_size += size;
                    }
                    Self::Fiber { size, .. } => {
                        *fiber_size += size;
                    }
                    Self::Trace { size, .. } => {
                        *model_size += size;
                    }
                    Self::Root { .. } => panic!("Cannot push root to root"),
                }

                bounding_box.get_mut().expand_point(node.get_min());
                bounding_box.get_mut().expand_point(node.get_max());
                children.push(Arc::new(node));
            }
            Self::Travel { .. } => panic!("Cannot push node to travel"),
            Self::Fiber { .. } => panic!("Cannot push node to fiber"),
            Self::Trace { .. } => panic!("Cannot push node to move"),
        }
    }

    pub fn update_offset(&mut self, offset: BufferAddress) {
        match self {
            Self::Root { .. } => {
                /*
                let mut current_offset = offset;
                for child in children {
                    // child.update_offset(current_offset);
                    current_offset += child.size();
                }
                */

                // TODO update the offset of the children
            }
            Self::Trace { offset: o, .. } => *o = offset,
            Self::Travel { offset: o, .. } => *o = offset,
            Self::Fiber { offset: o, .. } => *o = offset,
        }
    }

    #[allow(dead_code)]
    pub fn size(&self) -> BufferAddress {
        match self {
            Self::Root { size, .. } => *size,
            Self::Travel { size, .. } => *size,
            Self::Fiber { size, .. } => *size,
            Self::Trace { size, .. } => *size,
        }
    }

    pub fn awaken(&mut self, data: &[TraceVertex], travel: &[Vertex], fiber: &[TraceVertex]) {
        match self {
            Self::Root {
                model,
                travel_model,
                fiber_model,
                ..
            } => {
                model.write().awaken(data);
                travel_model.write().awaken(travel);
                fiber_model.write().awaken(fiber);
            }
            Self::Travel { .. } => panic!("Cannot awaken travel"),
            Self::Fiber { .. } => panic!("Cannot awaken fiber"),
            Self::Trace { .. } => panic!("Cannot awaken path"),
        }
    }

    pub fn render_travel<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { travel_model, .. } => {
                travel_model.render(render_pass);
            }
            Self::Travel { .. } => panic!("Cannot render travel"),
            Self::Fiber { .. } => panic!("Cannot render fiber"),
            Self::Trace { .. } => panic!("Cannot render path"),
        }
    }

    pub fn render_fiber<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { fiber_model, .. } => {
                fiber_model.render(render_pass);
            }
            Self::Travel { .. } => panic!("Cannot render travel"),
            Self::Fiber { .. } => panic!("Cannot render fiber"),
            Self::Trace { .. } => panic!("Cannot render path"),
        }
    }
}

impl Renderable for TraceTree {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { model, .. } => model.render(render_pass),
            Self::Travel { .. } => panic!("Cannot render travel"),
            Self::Fiber { .. } => panic!("Cannot render fiber"),
            Self::Trace { .. } => panic!("Cannot render path"),
        }
    }

    fn render_without_color<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { model, .. } => model.render_without_color(render_pass),
            Self::Travel { .. } => panic!("Cannot render travel"),
            Self::Fiber { .. } => panic!("Cannot render fiber"),
            Self::Trace { .. } => panic!("Cannot render path"),
        }
    }
}

impl HitboxNode for TraceTree {
    fn check_hit(&self, ray: &crate::input::Ray) -> Option<f32> {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().check_hit(ray),
            Self::Trace {
                r#box: path_box, ..
            } => path_box.read().check_hit(ray),
            Self::Travel { .. } => None,
            Self::Fiber { .. } => None,
        }
    }

    fn inner_nodes(&self) -> &[Arc<Self>] {
        match self {
            Self::Root { children, .. } => children,
            Self::Travel { .. } => &[],
            Self::Fiber { .. } => &[],
            Self::Trace { .. } => &[],
        }
    }

    fn get_min(&self) -> glam::Vec3 {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().get_min(),
            Self::Trace {
                r#box: path_box, ..
            } => path_box.read().get_min(),
            Self::Travel { start, end, .. } => start.read().min(*end.read()),
            Self::Fiber { start, end, .. } => start.read().min(*end.read()),
        }
    }

    fn get_max(&self) -> glam::Vec3 {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().get_max(),
            Self::Trace {
                r#box: path_box, ..
            } => path_box.read().get_max(),
            Self::Travel { start, end, .. } => start.read().max(*end.read()),
            Self::Fiber { start, end, .. } => start.read().max(*end.read()),
        }
    }
}

impl InteractiveModel for TraceTree {
    fn aabb(&self) -> (Vec3, Vec3) {
        match self {
            Self::Root { bounding_box, .. } => {
                let bb = bounding_box.read();
                (bb.get_min(), bb.get_max())
            }
            Self::Trace {
                r#box: path_box, ..
            } => {
                let bb = path_box.read();
                (bb.get_min(), bb.get_max())
            }
            Self::Travel { start, end, .. } => (*start.read(), *end.read()),
            Self::Fiber { start, end, .. } => (*start.read(), *end.read()),
        }
    }

    fn transformation(&self) -> glam::Mat4 {
        match self {
            Self::Root { model, .. } => model.read().transformation(),
            _ => glam::Mat4::IDENTITY,
        }
    }

    fn as_transformable(&self) -> Option<&dyn crate::render::model::Transform> {
        None
    }
}
