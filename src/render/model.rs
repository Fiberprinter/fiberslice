use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{render::Renderable, DEVICE, QUEUE};

use super::ColorBinding;
pub trait TransformMut {
    fn transform(&mut self, transform: glam::Mat4);
}

pub trait Transform {
    fn transform(&self, transform: glam::Mat4);
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    pub transform: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelColorUniform {
    pub color: [f32; 4],
}

#[derive(Debug)]
pub enum ModelState {
    Dormant,
    Awake(wgpu::Buffer, u32),
}

#[derive(Debug)]
pub struct Model<T> {
    state: ModelState,

    transform: Mat4,
    transform_buffer: wgpu::Buffer,
    transform_bind_group: wgpu::BindGroup,

    color_group: ColorBinding,

    enabled: bool,
    destroyed: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + bytemuck::Pod + bytemuck::Zeroable> Model<T> {
    pub fn create() -> Self {
        let device_read = DEVICE.read();
        let device = device_read.as_ref().unwrap();

        let transform = Mat4::from_translation(Vec3::ZERO);

        let transform_uniform = TransformUniform {
            transform: transform.to_cols_array_2d(),
        };

        let transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Buffer"),
            contents: bytemuck::cast_slice(&[transform_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let color_group = ColorBinding::new_with_default([1.0, 1.0, 1.0, 1.0]);

        Self {
            state: ModelState::Dormant,
            transform,
            transform_buffer,
            transform_bind_group,

            color_group,

            enabled: true,
            destroyed: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn transformation(&self) -> Mat4 {
        self.transform
    }

    #[allow(dead_code)]
    pub fn color(&self) -> [f32; 4] {
        self.color_group.color()
    }

    #[allow(dead_code)]
    pub fn set_transparency(&mut self, transparency: f32) {
        self.color_group.set_transparency(transparency);
    }

    #[allow(dead_code)]
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color_group.set_color(color);
    }

    pub fn awaken(&mut self, data: &[T]) {
        let device_read = DEVICE.read();
        let device = device_read.as_ref().unwrap();

        if data.is_empty() {
            return;
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.state = ModelState::Awake(buffer, data.len() as u32);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn destroy(&mut self) {
        self.destroyed = true;

        match &self.state {
            ModelState::Dormant => {}
            ModelState::Awake(buffer, ..) => {
                buffer.destroy();
            }
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }
}

impl<T> Renderable for Model<T> {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.destroyed || !self.enabled {
            return;
        }

        let (buffer, size) = match &self.state {
            ModelState::Dormant => return,
            ModelState::Awake(buffer, size) => (buffer, size),
        };

        render_pass.set_bind_group(2, &self.transform_bind_group, &[]);
        render_pass.set_bind_group(3, self.color_group.binding(), &[]);

        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..*size, 0..1);
    }

    fn render_without_color<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.destroyed || !self.enabled {
            return;
        }

        let (buffer, size) = match &self.state {
            ModelState::Dormant => return,
            ModelState::Awake(buffer, size) => (buffer, size),
        };

        render_pass.set_bind_group(2, &self.transform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..*size, 0..1);
    }
}

impl<T> Drop for Model<T> {
    fn drop(&mut self) {
        match &self.state {
            ModelState::Dormant => {}
            ModelState::Awake(buffer, ..) => {
                buffer.destroy();
            }
        }

        self.destroyed = true;
    }
}

impl<T> TransformMut for Model<T> {
    fn transform(&mut self, transform: Mat4) {
        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        self.transform = transform;
        let transform_uniform = TransformUniform {
            transform: self.transform.to_cols_array_2d(),
        };

        queue.write_buffer(
            &self.transform_buffer,
            0,
            bytemuck::cast_slice(&[transform_uniform]),
        );
    }
}
