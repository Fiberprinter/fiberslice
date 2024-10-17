use core::{f32, panic};
use std::{
    collections::{HashMap, LinkedList, VecDeque},
    f32::consts::PI,
    path::Path,
    sync::Arc,
};

use glam::{Quat, Vec3, Vec3Swizzles};
use ordered_float::OrderedFloat;

use parking_lot::RwLock;
use shared::{
    loader::{LoadError, Loader},
    object::ObjectMesh,
};

use tokio::{sync::oneshot::error::TryRecvError, task::JoinHandle};

use uni_path::PathBuf;
use wgpu::{BufferAddress, Color};

use crate::{
    geometry::{
        mesh::{vec3s_into_vertices, IntoArrayColor},
        BoundingBox,
    },
    picking::{
        self,
        hitbox::{Hitbox, HitboxNode, HitboxRoot},
    },
    prelude::{LockModel, WgpuContext},
    render::{
        model::{
            Model, Rotate, RotateMut, Scale, ScaleMut, Transform, TransformMut, Translate,
            TranslateMut,
        },
        Renderable, Vertex,
    },
    ui::{api::trim_text, custom_toasts::MODEL_LOAD_PROGRESS},
    viewer::tracker::Process,
    GlobalState, RootEvent, GLOBAL_STATE,
};

// const MAIN_LOADED_TOOLPATH: &str = "main"; // HACK: This is a solution to ease the dev when only one toolpath is loaded which is the only supported(for now)

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    LoadError(LoadError),
    #[error("NoGeometryObject")]
    NoGeometryObject,
}

#[derive(Debug)]
pub struct LoadResult {
    model: CADModel,
    mesh: ObjectMesh,

    process: Arc<Process>,
    origin_path: String,
}

#[derive(Debug)]
pub struct CADModelHandle {
    model: Arc<CADModel>,
    mesh: ObjectMesh,
}

type CADModelResult = Result<LoadResult, CADModelError>;

// TODO also use vertex indices
#[derive(Debug)]
pub struct CADModelServer {
    queue: Vec<(
        tokio::sync::oneshot::Receiver<CADModelResult>,
        JoinHandle<()>,
    )>,

    root_hitbox: HitboxRoot<CADModel>,
    models: HashMap<String, CADModelHandle>,
}

impl CADModelServer {
    pub fn instance(_context: &WgpuContext) -> Self {
        Self {
            queue: Vec::new(),
            root_hitbox: HitboxRoot::root(),
            models: HashMap::new(),
        }
    }

    pub fn load<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_str().unwrap_or("").to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            let mesh = match (shared::loader::STLLoader {}).load(&path) {
                Ok(model) => model,
                Err(e) => {
                    tx.send(Err(CADModelError::LoadError(e))).unwrap();

                    return;
                }
            };

            let global_state = GLOBAL_STATE.read();
            let global_state = global_state.as_ref().unwrap();

            let process_tracking = global_state
                .progress_tracker
                .write()
                .add(MODEL_LOAD_PROGRESS, trim_text::<20, 4>(&path));

            let vertices: Vec<Vec3> = mesh.vertices().iter().map(|v| v.xyz()).collect();

            let mut triangles: Vec<(shared::IndexedTriangle, Vec3)> = mesh
                .triangles()
                .iter()
                .map(|triangle| {
                    // calculate the normal of the triangle
                    let normal = (vertices[triangle[1]] - vertices[triangle[0]])
                        .cross(vertices[triangle[2]] - vertices[triangle[0]])
                        .normalize();

                    (*triangle, normal)
                })
                .collect();

            process_tracking.set_task(
                "
Clustering models"
                    .to_string(),
            );
            process_tracking.set_progress(0.0);
            let models = clusterize_models(&triangles);

            process_tracking.set_task("Creating vertices".to_string());
            process_tracking.set_progress(0.2);
            let vertices = triangles
                .iter_mut()
                .fold(Vec::new(), |mut vec, (triangle, _)| {
                    vec.push(vertices[triangle[0]]);
                    triangle[0] = vec.len() - 1;
                    vec.push(vertices[triangle[2]]);
                    triangle[2] = vec.len() - 1;
                    vec.push(vertices[triangle[1]]);
                    triangle[1] = vec.len() - 1;
                    vec
                });

            process_tracking.set_task("Clustering faces".to_string());
            process_tracking.set_progress(0.4);
            let plane_entries = clusterize_faces(&triangles, &vertices);

            process_tracking.set_task("Creating polygons".to_string());
            process_tracking.set_progress(0.6);
            let polygons: Vec<PolygonFace> = plane_entries
                .iter()
                .map(|entry| PolygonFace::from_entry(entry.clone(), &triangles, &vertices))
                .collect();

            let mut triangle_vertices = vec3s_into_vertices(vertices.clone(), Color::BLACK);

            process_tracking.set_task("Filtering polygons".to_string());
            process_tracking.set_progress(0.8);
            let polygon_faces: Vec<PolygonFace> = polygons
                .into_iter()
                .filter(|face| {
                    let x = face.max.x - face.min.x;
                    let y = face.max.y - face.min.y;
                    let z = face.max.z - face.min.z;

                    if x < y && x < z {
                        z > 2.0 && y > 2.0
                    } else if y < x && y < z {
                        x > 2.0 && z > 2.0
                    } else {
                        x > 2.0 && y > 2.0
                    }
                })
                .collect();

            process_tracking.set_task("Coloring polygons".to_string());
            process_tracking.set_progress(0.85);
            models.iter().for_each(|indices| {
                let r = rand::random::<f64>();
                let g = rand::random::<f64>();
                let b = rand::random::<f64>();

                for triangle in indices.iter() {
                    triangle_vertices[triangles[*triangle].0[0]].color =
                        Color { r, g, b, a: 1.0 }.to_array();

                    triangle_vertices[triangles[*triangle].0[1]].color =
                        Color { r, g, b, a: 1.0 }.to_array();

                    triangle_vertices[triangles[*triangle].0[2]].color =
                        Color { r, g, b, a: 1.0 }.to_array();
                }
            });

            process_tracking.set_task("Creating models".to_string());
            process_tracking.set_progress(0.9);
            let mut root = polygon_faces.clone().into_iter().fold(
                CADModel::create_root(),
                |mut root, face| {
                    root.push_face(face);

                    root
                },
            );
            root.awaken(&triangle_vertices);

            process_tracking.set_task("Rotating model to largest face".to_string());
            process_tracking.set_progress(0.95);
            let bounding_box = match &root {
                CADModel::Root { bounding_box, .. } => bounding_box,
                _ => panic!("Not root"),
            };

            // let it look like z and y are swapped
            root.rotate(Quat::from_euler(glam::EulerRot::XYZ, -PI / 2.0, 0.0, 0.0));

            tx.send(Ok(LoadResult {
                process: process_tracking,
                model: root,
                mesh,
                origin_path: path,
            }))
            .unwrap();
        });

        self.queue.push((rx, handle));
    }
    // i love you
    pub fn insert(&mut self, model_handle: LoadResult) -> Result<Arc<CADModel>, Error> {
        let path: PathBuf = model_handle.origin_path.into();
        let file_name = if let Some(path) = path.file_name() {
            path.to_string()
        } else {
            path.to_string()
        };

        // model_handle.process.set_task("Finding Name".to_string());
        let mut name = file_name.clone();

        let mut counter: u8 = 1;

        while self.models.contains_key(&name) {
            name = format!("{} ({counter})", file_name);

            counter += 1;
        }

        model_handle.process.set_task("Write to GPU".to_string());
        model_handle.process.set_progress(1.0);

        model_handle.process.finish();

        let handle = Arc::new(model_handle.model);

        let ctx = CADModelHandle {
            model: handle.clone(),
            mesh: model_handle.mesh,
        };

        self.models.insert(name.clone(), ctx);

        Ok(handle)
    }

    pub fn remove(&mut self, name: String) {
        self.models.remove(&name);
    }

    pub fn update(&mut self, global_state: GlobalState<RootEvent>) -> Result<(), Error> {
        if !self.queue.is_empty() {
            let mut results = Vec::new();

            self.queue.retain_mut(|(rx, ..)| match rx.try_recv() {
                Ok(result) => {
                    results.push(result);

                    false
                }
                Err(TryRecvError::Closed) => false,
                _ => true,
            });

            for model_result in results {
                let model = match model_result {
                    Ok(model) => model,
                    Err(e) => {
                        global_state
                            .ui_event_writer
                            .send(crate::ui::UiEvent::ShowError(format!("{}", e)));

                        continue;
                    }
                };

                let handle = self.insert(model)?;

                global_state
                    .ui_event_writer
                    .send(crate::ui::UiEvent::ShowSuccess("Object loaded".to_string()));

                global_state.camera_event_writer.send(
                    crate::viewer::camera::CameraEvent::UpdatePreferredDistance(BoundingBox::new(
                        handle.get_min(),
                        handle.get_max(),
                    )),
                );

                global_state.viewer.selector().write().select(&handle);

                // self.root_hitbox.add_node(handle);
                // global_state.window.request_redraw();
            }
        }

        // self.models.values_mut().for_each(|model| model.update());

        Ok(())
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = &String> {
        self.models.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn kill(&mut self) {
        for (_, handle) in self.queue.drain(..) {
            handle.abort();
        }
    }

    pub fn root_hitbox(&self) -> &HitboxRoot<CADModel> {
        &self.root_hitbox
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = (&String, ObjectMesh)> {
        self.models.iter().map(|(key, model)| {
            let geometry = model.mesh.clone();

            (key, geometry)
        })
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.models
            .values()
            .for_each(|model| model.model.render(render_pass));
    }
}

#[derive(Debug)]
pub enum CADModel {
    Root {
        model: LockModel<Vertex>,
        bounding_box: RwLock<BoundingBox>,
        children: Vec<Self>,
        size: BufferAddress,
    },
    Face {
        face: RwLock<PolygonFace>,
    },
}

impl CADModel {
    pub fn create_root() -> Self {
        Self::Root {
            model: LockModel::new(Model::create()),
            bounding_box: RwLock::new(BoundingBox::default()),
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

                children.push(Self::Face {
                    face: RwLock::new(face),
                });
            }
            _ => panic!("Not root"),
        }
    }

    pub fn awaken(&mut self, data: &[Vertex]) {
        match self {
            Self::Root { model, .. } => model.get_mut().awaken(data),
            Self::Face { .. } => panic!("Cannot awaken face"),
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { model, .. } => model.render(render_pass),
            Self::Face { .. } => panic!("Cannot render face"),
        }
    }

    pub fn get_transform(&self) -> glam::Mat4 {
        match self {
            Self::Root { model, .. } => model.read().get_transform(),
            Self::Face { .. } => panic!("Cannot get transform"),
        }
    }

    pub fn get_color(&self) -> [f32; 4] {
        match self {
            Self::Root { model, .. } => model.read().get_color(),
            Self::Face { .. } => panic!("Cannot get color"),
        }
    }

    pub fn set_transparency(&mut self, transparency: f32) {
        match self {
            Self::Root { model, .. } => model.write().set_transparency(transparency),
            Self::Face { .. } => panic!("Cannot set transparency"),
        }
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        match self {
            Self::Root { model, .. } => model.write().set_color(color),
            Self::Face { .. } => panic!("Cannot set color"),
        }
    }
}

impl Renderable for CADModel {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        match self {
            Self::Root { model, .. } => model.render(render_pass),
            Self::Face { .. } => panic!("Cannot render face"),
        }
    }
}

impl HitboxNode<CADModel> for CADModel {
    fn check_hit(&self, ray: &crate::picking::Ray) -> Option<f32> {
        match self {
            Self::Root { bounding_box, .. } => bounding_box.read().check_hit(ray),
            Self::Face { face, .. } => face.read().check_hit(ray),
        }
    }

    fn inner_nodes(&self) -> &[CADModel] {
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

impl Translate for CADModel {
    fn translate(&self, translation: Vec3) {
        match self {
            Self::Root { model, .. } => model.write().translate(translation),
            Self::Face { face } => face.write().translate(translation),
        }
    }
}

impl Rotate for CADModel {
    fn rotate(&self, rotation: glam::Quat) {
        match self {
            Self::Root { model, .. } => model.write().rotate(rotation),
            Self::Face { face } => face.write().rotate(rotation),
        }
    }
}

impl Scale for CADModel {
    fn scale(&self, scale: Vec3) {
        match self {
            Self::Root { model, .. } => model.write().scale(scale),
            Self::Face { face } => face.write().scale(scale),
        }
    }
}

impl Transform for CADModel {
    fn transform(&self, transform: glam::Mat4) {
        match self {
            Self::Root { model, .. } => model.write().transform(transform),
            Self::Face { face } => face.write().transform(transform),
        }
    }
}

impl TranslateMut for CADModel {
    fn translate(&mut self, translation: Vec3) {
        match self {
            Self::Root { model, .. } => model.get_mut().translate(translation),
            Self::Face { face } => face.get_mut().translate(translation),
        }
    }
}

impl RotateMut for CADModel {
    fn rotate(&mut self, rotation: glam::Quat) {
        match self {
            Self::Root { model, .. } => model.get_mut().rotate(rotation),
            Self::Face { face } => face.get_mut().rotate(rotation),
        }
    }
}

impl ScaleMut for CADModel {
    fn scale(&mut self, scale: Vec3) {
        match self {
            Self::Root { model, .. } => model.get_mut().scale(scale),
            Self::Face { face } => face.get_mut().scale(scale),
        }
    }
}

impl TransformMut for CADModel {
    fn transform(&mut self, transform: glam::Mat4) {
        match self {
            Self::Root { model, .. } => model.get_mut().transform(transform),
            Self::Face { face } => face.get_mut().transform(transform),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CADModelError {
    #[error("File not found {0}")]
    FileNotFound(String),
    #[error("Load Error {0}")]
    LoadError(LoadError),
    #[error("NoGeometryObject")]
    NoGeometryObject,
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

        let ray = picking::Ray {
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
    fn check_hit(&self, ray: &picking::Ray) -> Option<f32> {
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

    fn expand_hitbox(&mut self, _box: &dyn Hitbox) {
        panic!("Not implemented")
    }

    fn set_enabled(&mut self, _enabled: bool) {
        panic!("Not implemented")
    }

    fn enabled(&self) -> bool {
        panic!("Not implemented")
    }

    fn get_min(&self) -> Vec3 {
        self.min
    }

    fn get_max(&self) -> Vec3 {
        self.max
    }
}

impl RotateMut for PolygonFace {
    fn rotate(&mut self, rotation: glam::Quat) {
        self.plane.normal = rotation * self.plane.normal;
        self.plane.point = rotation * self.plane.point;

        self.min = rotation * self.min;
        self.max = rotation * self.max;
    }
}

impl TranslateMut for PolygonFace {
    fn translate(&mut self, translation: Vec3) {
        self.plane.point += translation;
        self.min += translation;
        self.max += translation;
    }
}

impl ScaleMut for PolygonFace {
    fn scale(&mut self, _scale: Vec3) {
        panic!("Not implemented")
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
