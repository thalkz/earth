use super::texture::Texture;

pub struct SurfaceHandler {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: Texture,
    pub is_configured: bool,
}

impl SurfaceHandler {
    pub fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Self {
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        let depth_texture = Texture::create_depth_texture(device, &config, "depth_texture");

        Self {
            surface,
            config,
            depth_texture,
            is_configured: false,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(device, &self.config);
            self.depth_texture =
                Texture::create_depth_texture(device, &self.config, "depth_texture");
            self.is_configured = true;
        }
    }

    pub fn get_current_texture(
        &mut self,
        device: &wgpu::Device,
    ) -> anyhow::Result<Option<wgpu::SurfaceTexture>> {
        if !self.is_configured {
            return Ok(None);
        }

        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => Ok(Some(surface_texture)),
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => Ok(Some(surface_texture)),
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => Ok(None),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(device, &self.config);
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Lost => anyhow::bail!("Lost surface/device"),
        }
    }
}
