use std::sync::Arc;

use glam::Mat4;

use crate::{
    geometry::{
        mesh::{Mesh, WireMesh},
        BoundingBox, SelectBox,
    },
    input::interact::InteractiveModel,
    render::{
        model::{Model, TransformMut},
        Renderable, Vertex,
    },
};

pub struct Selector {
    selected: Vec<Arc<dyn InteractiveModel>>,
    grouped_transform: Option<Mat4>,

    select_box: Model<Vertex>,
    select_box_lines: Model<Vertex>,
}

impl std::fmt::Debug for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Selector")
            .field("selected", &self.selected.len())
            .finish()
    }
}

#[allow(dead_code)]
impl Selector {
    pub fn instance() -> Self {
        Self {
            selected: Vec::new(),
            grouped_transform: None,

            select_box: Model::create(),
            select_box_lines: Model::create(),
        }
    }

    pub fn select_multiple(&mut self, model: Arc<dyn InteractiveModel>) {
        if self.selected.iter().any(|m| Arc::ptr_eq(m, &model)) {
            self.deselect(&model);

            self.grouped_transform = None;
            return;
        }

        self.selected.push(model.clone());

        self.grouped_transform = None;

        self.update_hitbox();
    }

    pub fn select(&mut self, model: Arc<dyn InteractiveModel>) {
        if self.selected.iter().any(|m| Arc::ptr_eq(m, &model)) {
            self.deselect(&model);

            self.grouped_transform = None;
            return;
        }

        self.selected.clear();
        self.selected.push(model.clone());

        println!("Select {:?}", self.selected.len());

        self.grouped_transform = None;

        self.update_hitbox();
    }

    fn deselect(&mut self, model: &Arc<dyn InteractiveModel>) {
        let size = self.selected.len();
        self.selected.retain(|m| !Arc::ptr_eq(m, model));

        if size != self.selected.len() {
            self.grouped_transform = None;
        }

        println!("Deselect {:?}", self.selected.len());
        self.update_hitbox();
    }

    fn update_hitbox(&mut self) {
        self.selected.retain(|model| !model.is_destroyed());

        let (min, max) = if self.selected.len() == 1 {
            self.select_box.transform(self.selected[0].transformation());
            self.select_box_lines
                .transform(self.selected[0].transformation());

            self.selected[0].aabb()
        } else if !self.selected.is_empty() {
            self.selected.iter().fold(
                (
                    glam::Vec3::splat(f32::INFINITY),
                    glam::Vec3::splat(f32::NEG_INFINITY),
                ),
                |(min, max), model| {
                    let (model_min, model_max) = model.aabb();

                    (min.min(model_min), max.max(model_max))
                },
            )
        } else {
            self.select_box.set_enabled(false);
            self.select_box_lines.set_enabled(false);
            return;
        };

        let select_box = SelectBox::from(BoundingBox::new(
            min - glam::Vec3::splat(0.4),
            max + glam::Vec3::splat(0.4),
        ));

        self.select_box.awaken(&select_box.to_triangle_vertices());
        self.select_box_lines.awaken(&select_box.to_wire_vertices());

        self.select_box.set_enabled(true);
        self.select_box_lines.set_enabled(true);
    }

    pub fn transform(&mut self, mut r#fn: impl FnMut(&mut Mat4) -> bool) {
        if self.selected.len() == 1 {
            let mut transform = self.selected[0].transformation();

            if let Some(transformable_model) = self.selected[0].as_transformable() {
                let response = r#fn(&mut transform);

                if response {
                    transformable_model.transform(transform);
                    self.select_box.transform(transform);
                    self.select_box_lines.transform(transform);
                }
            }
        } else {
            let mut transform = self
                .grouped_transform
                .unwrap_or(Mat4::from_translation(glam::Vec3::ZERO));

            let response = r#fn(&mut transform);

            if response {
                for model in &self.selected {
                    let (scale, rotate, translate) =
                        model.transformation().to_scale_rotation_translation();

                    if let Some(transformable_model) = model.as_transformable() {
                        let (grouped_scale, grouped_rotate, grouped_translate) = (transform
                            .inverse()
                            * self
                                .grouped_transform
                                .unwrap_or(Mat4::from_translation(glam::Vec3::ZERO)))
                        .to_scale_rotation_translation();

                        let transform = Mat4::from_scale_rotation_translation(
                            scale * grouped_scale,
                            rotate * grouped_rotate,
                            translate + grouped_translate,
                        );

                        transformable_model.transform(transform);
                        self.select_box.transform(transform);
                        self.select_box_lines.transform(transform);
                    }
                }
            }

            self.grouped_transform = Some(transform);
        }
    }

    pub fn update(&mut self) {
        let len = self.selected.len();
        self.selected.retain(|model| !model.is_destroyed());

        if len != self.selected.len() {
            self.update_hitbox();
        }
    }

    pub fn selected(&self) -> &[Arc<dyn InteractiveModel>] {
        &self.selected
    }

    pub fn clear(&mut self) {
        self.selected.clear();
        self.update_hitbox();
    }

    pub fn delete_selected(&mut self) {
        self.selected.iter_mut().for_each(|model| {
            model.destroy();
        });

        self.selected.clear();
        self.update_hitbox();
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        // self.volume.render(render_pass);
        self.select_box.render(render_pass);
    }

    pub fn render_wire<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        // self.volume.render_lines(render_pass);
        self.select_box_lines.render(render_pass);
    }
}
