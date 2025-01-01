/// A texture typically contains one or more images that share the same format.
pub struct Texture;

impl Texture {
    /// Creates a new depth texture.
    ///
    /// Arguments:
    ///
    /// * `device`: The wgpu device for which the texture will be generated.
    /// * `config`: The wgpu surface configuration for which the texture will be generated.
    /// * `sample_count`: This has to be the same as the number of samples used for _MSAA_.
    /// * `label`: The label of the texture.
    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
        label: &str,
    ) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, /*  | wgpu::TextureUsages::TEXTURE_BINDING */
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Creates a framebuffer that can be used for multisample anti-aliasing.
    ///
    /// Arguments:
    ///
    /// * `device`: The wgpu device for which the texture will be generated.
    /// * `config`: The wgpu surface configuration for which the texture will be generated.
    /// * `sample_count`: The sample count used for _MSAA_. Valid values are `1` (no MSAA) or `4`.
    /// * `label`: The label of the texture.
    pub fn create_multisampled_framebuffer(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
        label: &str,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some(label),
            view_formats: &[],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}
