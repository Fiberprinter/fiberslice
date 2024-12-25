use std::sync::Arc;

use wgpu::{BindGroupLayout, Device};

pub struct PipelineBuilder {
    device: Arc<Device>,
    primitive: Option<wgpu::PrimitiveState>,
    depth_stencil: Option<wgpu::DepthStencilState>,
}

impl PipelineBuilder {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            primitive: None,
            depth_stencil: None,
        }
    }

    pub fn with_primitive(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = Some(primitive);
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    pub fn build(
        self,
        label: &str,
        shader: &str,
        bind_groups: &[&BindGroupLayout],
        vertex_desc: &[wgpu::VertexBufferLayout<'_>],
        surface_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(format!("{} Shader", label).as_str()),
                source: wgpu::ShaderSource::Wgsl(shader.into()),
            });

        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("{} Layout", label).as_str()),
                bind_group_layouts: bind_groups,
                push_constant_ranges: &[],
            });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: vertex_desc,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }),

                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: self.primitive.unwrap_or(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                }),
                depth_stencil: Some(self.depth_stencil.unwrap_or(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                })),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    ..Default::default()
                },
                multiview: None,
                cache: None,
            })
    }
}

pub struct DefaultPipelines {
    pub back_cull: wgpu::RenderPipeline,
    pub no_cull: wgpu::RenderPipeline,
    pub line: wgpu::RenderPipeline,
}

impl DefaultPipelines {
    pub fn instance(
        device: Arc<Device>,
        shader: &str,
        bind_groups: &[&wgpu::BindGroupLayout],
        vertex_desc: &[wgpu::VertexBufferLayout<'_>],
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            back_cull: PipelineBuilder::new(device.clone())
                .with_primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                })
                .build(
                    "Back Cull Pipeline",
                    shader,
                    bind_groups,
                    vertex_desc,
                    surface_format,
                ),
            no_cull: PipelineBuilder::new(device.clone())
                .with_primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    // Requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                })
                .build(
                    "No Cull Pipeline",
                    shader,
                    bind_groups,
                    vertex_desc,
                    surface_format,
                ),

            line: PipelineBuilder::new(device.clone())
                .with_primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    // Requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                })
                .build(
                    "Line Pipeline",
                    shader,
                    bind_groups,
                    vertex_desc,
                    surface_format,
                ),
        }
    }
}
