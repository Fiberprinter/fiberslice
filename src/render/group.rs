use wgpu::util::DeviceExt;

use crate::QUEUE;

use super::model::ModelColorUniform;

#[derive(Debug)]
pub struct ColorBinding {
    color: [f32; 4],
    buffer: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl ColorBinding {
    pub fn new_with_default(color: [f32; 4]) -> Self {
        let device_read = crate::DEVICE.read();
        let device = device_read.as_ref().unwrap();

        let color_uniform = ModelColorUniform { color };

        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Color Buffer"),
            contents: bytemuck::cast_slice(&[color_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let color_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &color_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: color_buffer.as_entire_binding(),
            }],
            label: None,
        });

        Self {
            color,
            buffer: color_buffer,
            layout: color_bind_group_layout,
            bind_group: color_bind_group,
        }
    }

    pub fn set_transparency(&mut self, transparency: f32) {
        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        self.color[3] = transparency;
        let color_uniform = ModelColorUniform { color: self.color };

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[color_uniform]));
    }

    #[allow(dead_code)]
    pub fn set_color(&mut self, color: [f32; 4]) {
        let queue_read = QUEUE.read();
        let queue = queue_read.as_ref().unwrap();

        self.color = color;
        let color_uniform = ModelColorUniform { color };

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[color_uniform]));
    }

    #[allow(dead_code)]
    pub fn color(&self) -> [f32; 4] {
        self.color
    }

    pub fn binding(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
}
