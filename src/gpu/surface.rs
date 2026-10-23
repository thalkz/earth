use crate::gpu::Texture;
use wgpu::{
    Adapter, CurrentSurfaceTexture, Device, PresentMode, Surface, SurfaceColorSpace,
    SurfaceConfiguration, SurfaceTexture, TextureUsages,
};

pub struct SurfaceHandler {
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub depth_texture: Texture,
    pub is_configured: bool,
}

impl SurfaceHandler {
    pub fn new(
        surface: Surface<'static>,
        adapter: &Adapter,
        device: &Device,
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

        let present_mode = if surface_caps.present_modes.contains(&PresentMode::Mailbox) {
            PresentMode::Mailbox // Triple-buffering: drops stale frames rather than blocking the CPU
        } else if surface_caps.present_modes.contains(&PresentMode::AutoVsync) {
            PresentMode::AutoVsync // Gracefully handles late frames without stutter
        } else {
            PresentMode::Fifo
        };

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: SurfaceColorSpace::Auto,
        };

        let depth_texture = Texture::create_depth_texture(device, &config, "depth_texture");

        Self {
            surface,
            config,
            depth_texture,
            is_configured: false,
        }
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
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
        device: &Device,
    ) -> anyhow::Result<Option<SurfaceTexture>> {
        if !self.is_configured {
            return Ok(None);
        }

        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => Ok(Some(surface_texture)),
            CurrentSurfaceTexture::Suboptimal(surface_texture) => Ok(Some(surface_texture)),
            CurrentSurfaceTexture::Timeout
            | CurrentSurfaceTexture::Occluded
            | CurrentSurfaceTexture::Validation => Ok(None),
            CurrentSurfaceTexture::Outdated => {
                self.surface.configure(device, &self.config);
                Ok(None)
            }
            CurrentSurfaceTexture::Lost => anyhow::bail!("Lost surface/device"),
        }
    }
}
