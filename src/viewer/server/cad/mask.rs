use core::f32;
use std::{collections::HashMap, path::Path, sync::Arc};

use glam::{vec3, Mat4, Quat, Vec3, Vec3Swizzles};
use shared::{
    loader::{BytesLoader, FileLoader},
    object::ObjectMesh,
};

use slicer::{Mask, MaskSettings, Settings};
use tokio::{sync::oneshot::error::TryRecvError, task::JoinHandle};

use uni_path::PathBuf;
use wgpu::{util::DeviceExt, Color};

use crate::{
    geometry::mesh::vec3s_into_vertices,
    input::{hitbox::HitboxRoot, interact::InteractiveModel},
    prelude::{Destroyable, WgpuContext},
    render::{
        model::{ModelColorUniform, Transform},
        Renderable,
    },
    ui::{api::trim_text, custom_toasts::OBJECT_LOAD_PROGRESS},
    viewer::RenderServer,
    GlobalState, RootEvent, GLOBAL_STATE, QUEUE,
};

use super::{clusterize_faces, CADObject, CADObjectResult, Error, LoadResult, PolygonFace};

#[derive(Debug)]
pub struct MaskHandle {
    model: Arc<CADObject>,
    mask: Mask,
}

#[derive(Debug)]
pub struct MaskServer {
    queue: Vec<(
        tokio::sync::oneshot::Receiver<CADObjectResult>,
        JoinHandle<()>,
    )>,

    root_hitbox: HitboxRoot<CADObject>,
    models: HashMap<String, MaskHandle>,

    color: [f32; 4],
    color_buffer: wgpu::Buffer,
    color_bind_group: wgpu::BindGroup,
}

impl RenderServer for MaskServer {
    fn instance(context: &WgpuContext) -> Self {
        let color = [1.0, 1.0, 1.0, 1.0];

        let color_uniform = ModelColorUniform { color };

        let color_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Color Buffer"),
                contents: bytemuck::cast_slice(&[color_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let color_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: None,
                });

        let color_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &color_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: color_buffer.as_entire_binding(),
                }],
                label: None,
            });

        Self {
            queue: Vec::new(),
            root_hitbox: HitboxRoot::root(),
            models: HashMap::new(),

            color,
            color_buffer,
            color_bind_group,
        }
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_bind_group(3, &self.color_bind_group, &[]);

        self.models
            .values()
            .for_each(|model| model.model.render_without_color(render_pass));
    }
}

impl MaskServer {
    pub fn load_from_file<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let file_name = match path.as_ref().file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => path.as_ref().to_string_lossy().to_string(),
        };

        let path = path.as_ref().to_str().unwrap_or("").to_string();

        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            let mesh = match (shared::loader::STLLoader {}).load(&path) {
                Ok(model) => model,
                Err(e) => {
                    tx.send(Err(Error::LoadError(e))).unwrap();

                    return;
                }
            };

            let result = Self::load(file_name, mesh);

            tx.send(result).unwrap()
        });

        self.queue.push((rx, handle));
    }

    pub fn load_from_bytes(&mut self, name: String, bytes: &[u8]) {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let bytes = bytes.to_vec();

        let handle = tokio::spawn(async move {
            let mesh = match (shared::loader::STLLoader {}).load_from_bytes(&bytes) {
                Ok(model) => model,
                Err(e) => {
                    tx.send(Err(Error::LoadError(e))).unwrap();

                    return;
                }
            };

            let result = Self::load(name, mesh);

            tx.send(result).unwrap()
        });

        self.queue.push((rx, handle));
    }

    fn load(name: String, mesh: ObjectMesh) -> Result<LoadResult, Error> {
        let (min, max) = mesh.min_max();

        let global_state = GLOBAL_STATE.read();
        let global_state = global_state.as_ref().unwrap();

        let process_tracking = global_state
            .progress_tracker
            .write()
            .add(OBJECT_LOAD_PROGRESS, trim_text::<20, 4>(&name));

        let vertices: Vec<Vec3> = mesh.vertices().iter().map(|v| v.xzy()).collect();

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

        process_tracking.set_task("Creating vertices".to_string());
        process_tracking.set_progress(0.2);
        let vertices = triangles
            .iter_mut()
            .fold(Vec::new(), |mut vec, (triangle, _)| {
                vec.push(vertices[triangle[0]]);
                triangle[0] = vec.len() - 1;
                vec.push(vertices[triangle[1]]);
                triangle[1] = vec.len() - 1;
                vec.push(vertices[triangle[2]]);
                triangle[2] = vec.len() - 1;
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

        triangle_vertices.iter_mut().for_each(|vertex| {
            vertex.color = [0.2, 0.2, 0.2, 1.0];
        });

        process_tracking.set_task("Creating models".to_string());
        process_tracking.set_progress(0.9);
        let mut root = polygon_faces.clone().into_iter().fold(
            CADObject::create_root(min.xzy(), max.xzy(), name.clone()),
            |mut root, face| {
                root.push_face(face);

                root
            },
        );

        root.awaken(&triangle_vertices);

        let center = (min + max) / 2.0;

        {
            let global_state_read = GLOBAL_STATE.read();
            let gloabal_state = global_state_read.as_ref().unwrap();
            let x = gloabal_state.slicer.read().settings.print_x;
            let y = gloabal_state.slicer.read().settings.print_y;

            root.transform(Mat4::from_translation(vec3(
                (x / 2.0) - center.x,
                -min.xzy().y,
                (y / 2.0) - center.xzy().z,
            )));
        }

        process_tracking.set_progress(0.95);

        Ok(LoadResult {
            process: process_tracking,
            model: root,
            mesh,
            name,
        })
    }

    pub fn insert(&mut self, model_handle: LoadResult) -> Result<Arc<CADObject>, Error> {
        let path: PathBuf = model_handle.name.into();
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

        let ctx = MaskHandle {
            model: handle.clone(),
            mask: Mask::new(model_handle.mesh, MaskSettings::default()),
        };

        self.models.insert(name.clone(), ctx);

        self.root_hitbox.add_node(handle.clone());

        Ok(handle)
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

                self.insert(model)?;

                global_state
                    .ui_event_writer
                    .send(crate::ui::UiEvent::ShowSuccess("Object loaded".to_string()));
            }
        }

        self.models.retain(|_, model| !model.model.is_destroyed());
        self.root_hitbox.update();

        Ok(())
    }

    pub fn prepare_objects<'a>(&'a self, settings: &'a Settings) -> Vec<Mask> {
        self.models
            .values()
            .map(|model| {
                let transform = model.model.transformation();

                let (mut scaling, rotation, mut translation) =
                    transform.to_scale_rotation_translation();
                let (x, y, z) = rotation.to_euler(glam::EulerRot::XYZ);

                let rotation = Quat::from_euler(glam::EulerRot::XYZ, -x, -z, -y);
                std::mem::swap(&mut scaling.y, &mut scaling.z);
                std::mem::swap(&mut translation.y, &mut translation.z);

                translation.x += settings.print_x / 2.0;
                translation.y += settings.print_y / 2.0;

                let transform =
                    Mat4::from_scale_rotation_translation(scaling, rotation, translation);

                let mut mask = model.mask.clone();
                mask.transform(transform);
                mask.sort_indices();

                mask
            })
            .collect()
    }

    pub fn set_transparency(&mut self, transparency: f32) {
        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        self.color[3] = transparency;
        let color_uniform = ModelColorUniform { color: self.color };

        queue.write_buffer(
            &self.color_buffer,
            0,
            bytemuck::cast_slice(&[color_uniform]),
        );
    }

    pub fn models(&self) -> Vec<(String, Arc<CADObject>)> {
        self.models
            .iter()
            .map(|(name, model)| (name.to_string(), model.model.clone()))
            .collect()
    }

    pub fn check_hit(
        &self,
        ray: &crate::input::Ray,
        level: usize,
        reverse: bool,
    ) -> Option<Arc<CADObject>> {
        self.root_hitbox.check_hit(ray, level, reverse)
    }
}
