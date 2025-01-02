use std::{
    collections::{HashMap, LinkedList, VecDeque},
    fmt::Display,
    sync::Arc,
};

use glam::Vec3;
use ordered_float::OrderedFloat;
use parking_lot::RwLock;
use shared::{loader::LoadError, object::ObjectMesh, process::Process};
use wgpu::BufferAddress;

use crate::{
    geometry::BoundingBox,
    input::{
        self,
        hitbox::{Hitbox, HitboxNode},
        interact::InteractiveModel,
    },
    prelude::LockModel,
    render::{
        model::{Model, Transform, TransformMut},
        Renderable, Vertex,
    },
};

pub mod mask;
pub mod model;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    LoadError(LoadError),
}

#[derive(Debug)]
pub struct LoadResult {
    model: CADObject,
    mesh: ObjectMesh,

    process: Arc<Process>,
    origin_path: String,
}

type CADObjectResult = Result<LoadResult, Error>;

#[derive(Debug)]
pub enum CADObject {
    Root {
        simple_name: String,
        model: LockModel<Vertex>,
        bounding_box: RwLock<BoundingBox>,
        children: Vec<Arc<Self>>,
        size: BufferAddress,
    },
    Face {
        face: RwLock<PolygonFace>,
    },
}

impl Display for CADObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root { simple_name, .. } => write!(f, "CADObject({})", simple_name),
            Self::Face { .. } => write!(f, "CADObject(Face)"),
        }
    }
}

impl CADObject {
    pub fn create_root(min: Vec3, max: Vec3, simple_name: String) -> Self {
        Self::Root {
            simple_name,
            model: LockModel::new(Model::create()),
            bounding_box: RwLock::new(BoundingBox::new(min, max)),
            children: Vec::new(),
            size: 0,
        }
    }

    pub fn push_face(&mut self, face: PolygonFace) {
        match self {
            Self::Root {
                children,
                bounding_box,
                size,
                ..
            } => {
                *size += face.size();
                bounding_box.get_mut().expand_point(face.get_min());
                bounding_box.get_mut().expand_point(face.get_max());

                children.push(Arc::new(Self::Face {
                    face: RwLock::new(face),
                }));
            }
            _ => panic!("Not root"),
        }
    }

    fn is_destroyed(&self) -> bool {
        match self {
            Self::Root { model, .. } => model.read().is_destroyed(),
            Self::Face { .. } => false,
        }
    }

    fn awaken(&mut self, data: &[Vertex]) {
        match self {
            Self::Root { model, .. } => model.get_mut().awaken(data),
            Self::Face { .. } => panic!("Cannot awaken face"),
        }
    }
}

impl InteractiveModel for CADObject {
    fn aabb(&self) -> (Vec3, Vec3) {
        match self {
            Self::Root { bounding_box, .. } => (
                bounding_box.read().init_min(),
                bounding_box.read().init_max(),
            ),
            Self::Face { face, .. } => (face.read().min, face.read().max),
        }
    }

    fn transformation(&self) -> glam::Mat4 {
        match self {
            Self::Root { model, .. } => model.read().transformation(),
            Self::Face { .. } => panic!("Cannot get transform"),
        }
    }

    fn destroy(&self) {
        match self {
            Self::Root { model, .. } => model.write().destroy(),
            Self::Face { .. } => panic!("Cannot destroy face"),
        }
    }

    fn as_transformable(&self) -> Option<&dyn Transform> {
        Some(self)
    }
}

impl Renderable for CADObject {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { model, .. } => model.render(render_pass),
            Self::Face { .. } => panic!("Cannot render face"),
        }
    }

    fn render_without_color<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { model, .. } => model.render_without_color(render_pass),
            Self::Face { .. } => panic!("Cannot render face"),
        }
    }
}

impl HitboxNode for CADObject {
    fn check_hit(&self, ray: &crate::input::Ray) -> Option<f32> {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().check_hit(ray),
            Self::Face { face, .. } => face.read().check_hit(ray),
        }
    }

    fn inner_nodes(&self) -> &[Arc<Self>] {
        match self {
            Self::Root { children, .. } => children,
            Self::Face { .. } => &[],
        }
    }

    fn get_min(&self) -> Vec3 {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().min,
            Self::Face { face, .. } => face.read().min,
        }
    }

    fn get_max(&self) -> Vec3 {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().max,
            Self::Face { face, .. } => face.read().max,
        }
    }
}

impl Transform for CADObject {
    fn transform(&self, transform: glam::Mat4) {
        match self {
            Self::Root {
                model,
                bounding_box,
                children,
                ..
            } => {
                model.write().transform(transform);
                bounding_box.write().transform(transform);

                for child in children {
                    child.transform(transform);
                }
            }
            Self::Face { face } => face.write().transform(transform),
        }
    }
}

impl TransformMut for CADObject {
    fn transform(&mut self, transform: glam::Mat4) {
        match self {
            Self::Root {
                model,
                bounding_box,
                children,
                ..
            } => {
                model.get_mut().transform(transform);
                bounding_box.get_mut().transform(transform);

                for child in children {
                    child.transform(transform);
                }
            }
            Self::Face { face } => face.get_mut().transform(transform),
        }
    }
}

#[derive(Debug, Clone)]
struct Plane {
    normal: glam::Vec3,
    point: glam::Vec3,
}

impl PartialEq for Plane {
    fn eq(&self, other: &Self) -> bool {
        // check if the planes are mathematically equal

        let cross_product = self.normal.cross(other.normal);
        if cross_product.length() > f32::EPSILON {
            return false; // Normals are not parallel
        }

        // Step 2: Check if p2 lies on the first plane and p1 lies on the second plane
        (other.point - self.point).dot(self.normal).abs() < f32::EPSILON
    }
}

impl Eq for Plane {}

#[derive(Debug, Clone)]
struct PlaneEntry {
    plane: Plane,
    triangles: Vec<usize>,
}

impl PartialEq for PlaneEntry {
    fn eq(&self, other: &Self) -> bool {
        self.plane == other.plane
    }
}

impl Eq for PlaneEntry {}

fn clusterize_models(triangles: &[(shared::IndexedTriangle, Vec3)]) -> Vec<Vec<usize>> {
    let mut neighbor_map: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

    for (index, (triangle, _)) in triangles.iter().enumerate() {
        let t1 = triangle[0];
        let t2 = triangle[1];
        let t3 = triangle[2];

        let mut handle = |t1, t2| {
            if let Some(neighbors) = neighbor_map.get_mut(&(t1, t2)) {
                neighbors.push(index);
            } else if let Some(neighbors) = neighbor_map.get_mut(&(t2, t1)) {
                neighbors.push(index);
            } else {
                neighbor_map.insert((t1, t2), vec![index]);
            }
        };

        handle(t1, t2);
        handle(t2, t3);
        handle(t3, t1);
    }

    let mut visited = vec![false; triangles.len()];

    let mut model_faces: LinkedList<Vec<usize>> = LinkedList::new();
    let mut queue: VecDeque<usize> = VecDeque::new();

    visited[0] = true;
    model_faces.push_back(vec![0]);
    queue.push_back(0);

    while let Some(index) = queue.pop_front() {
        let (triangle, _) = &triangles[index];

        let mut handle_edge = |t1, t2| {
            if let Some(neighbors) = neighbor_map.get(&(t1, t2)) {
                for neighbor in neighbors {
                    if !visited[*neighbor] {
                        visited[*neighbor] = true;

                        model_faces.back_mut().unwrap().push(*neighbor);
                        queue.push_back(*neighbor);
                    }
                }
            } else if let Some(neighbors) = neighbor_map.get(&(t2, t1)) {
                for neighbor in neighbors {
                    if !visited[*neighbor] {
                        visited[*neighbor] = true;

                        model_faces.back_mut().unwrap().push(*neighbor);
                        queue.push_back(*neighbor);
                    }
                }
            }
        };

        handle_edge(triangle[0], triangle[1]);
        handle_edge(triangle[1], triangle[2]);
        handle_edge(triangle[2], triangle[0]);

        if queue.is_empty() {
            if let Some(index) = (0..triangles.len()).find(|index| !visited[*index]) {
                visited[index] = true;
                model_faces.push_back(vec![index]);
                queue.push_back(index);
            }
        }
    }

    model_faces.into_iter().collect()
}

fn clusterize_faces(
    triangles: &[(shared::IndexedTriangle, Vec3)],
    vertices: &[Vec3],
) -> Vec<PlaneEntry> {
    let mut plane_map: HashMap<[OrderedFloat<f32>; 6], Vec<usize>> = HashMap::new();

    for (index, (triangle, normal)) in triangles.iter().enumerate() {
        let normal = normal.normalize();

        let point = vertices[triangle[0]];

        let ray = input::Ray {
            origin: Vec3::new(0.0, 0.0, 0.0),
            direction: normal,
        };

        let intersection = ray.intersection_plane(normal, point);

        fn round(value: f32) -> f32 {
            let factor = 10f32.powi(4); // 10^4 = 10000
            (value * factor).round() / factor
        }

        let key = [
            OrderedFloat(round(normal.x)),
            OrderedFloat(round(normal.y)),
            OrderedFloat(round(normal.z)),
            OrderedFloat(round(intersection.x)),
            OrderedFloat(round(intersection.y)),
            OrderedFloat(round(intersection.z)),
        ];

        plane_map.entry(key).or_default().push(index);
    }

    plane_map
        .into_iter()
        .map(|(key, indices)| {
            let normal = Vec3::new(key[0].0, key[1].0, key[2].0);
            let point = Vec3::new(key[3].0, key[4].0, key[5].0);

            PlaneEntry {
                plane: Plane { normal, point },
                triangles: indices,
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct PolygonFace {
    plane: Plane,
    indices: Vec<usize>,
    min: Vec3,
    max: Vec3,
}

impl PolygonFace {
    fn from_entry(
        entry: PlaneEntry,
        triangles: &[(shared::IndexedTriangle, Vec3)],
        vertices: &[Vec3],
    ) -> PolygonFace {
        let plane = Plane {
            normal: triangles[entry.triangles[0]].1.normalize(),
            point: vertices[triangles[entry.triangles[0]].0[0]],
        };

        let mut min = Vec3::INFINITY;
        let mut max = Vec3::NEG_INFINITY;

        for triangle in entry.triangles.iter() {
            min = min
                .min(vertices[triangles[*triangle].0[0]])
                .min(vertices[triangles[*triangle].0[1]])
                .min(vertices[triangles[*triangle].0[2]]);
            max = max
                .max(vertices[triangles[*triangle].0[0]])
                .max(vertices[triangles[*triangle].0[1]])
                .max(vertices[triangles[*triangle].0[2]]);
        }

        let indices = entry
            .triangles
            .iter()
            .flat_map(|index| {
                let (triangle, _) = &triangles[*index];

                vec![triangle[0], triangle[1], triangle[2]]
            })
            .collect();

        Self {
            plane,
            indices,
            min,
            max,
        }
    }

    pub fn size(&self) -> BufferAddress {
        self.indices.len() as BufferAddress
    }
}

impl Hitbox for PolygonFace {
    fn check_hit(&self, ray: &input::Ray) -> Option<f32> {
        let denominator = self.plane.normal.dot(ray.direction);

        if denominator.abs() < f32::EPSILON {
            return None;
        }

        let t = (self.plane.point - ray.origin).dot(self.plane.normal) / denominator;

        if t < 0.0 {
            return None;
        }

        let intersection = ray.origin + ray.direction * t;

        if intersection.x > self.min.x
            && intersection.x < self.max.x
            && intersection.y > self.min.y
            && intersection.y < self.max.y
            && intersection.z > self.min.z
            && intersection.z < self.max.z
        {
            Some(t)
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

impl TransformMut for PolygonFace {
    fn transform(&mut self, transform: glam::Mat4) {
        self.plane.normal = (transform * self.plane.normal.extend(0.0)).truncate();
        self.plane.point = (transform * self.plane.point.extend(0.0)).truncate();

        self.min = (transform * self.min.extend(0.0)).truncate();
        self.max = (transform * self.max.extend(0.0)).truncate();
    }
}
