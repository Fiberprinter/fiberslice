use std::ops::{Deref, DerefMut};

use geo::{Area, HasDimensions, Simplify};
use glam::{Mat4, Vec3};
use log::info;
use shared::object::ObjectMesh;

use crate::{
    error::SlicerErrors, plotter::polygon_operations::PolygonOperations, slicing,
    tower::TriangleTower, MaskSettings, Object, Settings,
};

#[derive(Debug, Clone)]
pub struct Mask {
    mesh: ObjectMesh,
    settings: MaskSettings,
}

impl Deref for Mask {
    type Target = ObjectMesh;

    fn deref(&self) -> &Self::Target {
        &self.mesh
    }
}

impl DerefMut for Mask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mesh
    }
}

impl Mask {
    pub fn new(mesh: ObjectMesh, settings: MaskSettings) -> Self {
        Self { mesh, settings }
    }

    pub fn into_mesh(self) -> ObjectMesh {
        self.mesh
    }

    pub fn settings(&self) -> &MaskSettings {
        &self.settings
    }

    pub fn transform(&mut self, transform: Mat4) {
        self.mesh.transform(transform);
    }

    pub fn into_object(self, max: Vec3, settings: &Settings) -> Result<ObjectMask, SlicerErrors> {
        let tower = TriangleTower::from_triangles_and_vertices(
            self.mesh.triangles(),
            self.mesh.vertices().to_vec(),
        )?;

        let settings = self.settings.clone().combine_settings(settings.clone());

        let obj = slicing::slice_single(&tower, max.z, &settings)?;

        Ok(ObjectMask {
            obj,
            settings: self.settings,
        })
    }
}

pub struct ObjectMask {
    obj: Object,
    settings: MaskSettings,
}

impl Deref for ObjectMask {
    type Target = Object;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl DerefMut for ObjectMask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.obj
    }
}

impl ObjectMask {
    pub fn mask_settings(&self) -> &MaskSettings {
        &self.settings
    }

    pub fn crop(&mut self, objects: &[Object], max: Vec3) {
        self.layers
            .iter_mut()
            .enumerate()
            .for_each(|(index, layer)| {
                let mut remaining_polygon = layer.main_polygon.clone();
                for object in objects.iter() {
                    if let Some(layer) = object.layers.get(index) {
                        if layer.main_polygon.is_empty() {
                            info!("Layer is empty, skipping {}", index);
                            info!("Area: {}", layer.main_polygon.unsigned_area());
                            continue;
                        }

                        remaining_polygon =
                            remaining_polygon.difference_with(&layer.main_polygon.simplify(&0.2));
                    }
                }

                layer.main_polygon = layer.main_polygon.difference_with(&remaining_polygon);
                layer.remaining_area = layer.main_polygon.clone();
            });

        self.layers.retain(|layer| {
            layer.main_polygon.unsigned_area() > f32::EPSILON || layer.top_height <= max.z
        });
    }

    pub fn randomize_mask_underlaps(&mut self, epsilon: f32) {
        self.layers.iter_mut().for_each(|layer| {
            let inset: f32 = rand::random::<f32>() * epsilon;

            layer.main_polygon = layer.main_polygon.offset_from(-inset);
            layer.remaining_area = layer.main_polygon.clone();
        });
    }
}
