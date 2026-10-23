use wgpu::{
    AddressMode, Device, Extent3d, FilterMode, MipmapFilterMode, Origin3d, Queue, Sampler,
    SamplerDescriptor, SurfaceConfiguration, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture as WgpuTexture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

pub struct Texture {
    #[allow(unused)]
    pub texture: WgpuTexture,
    pub view: TextureView,
}

impl Texture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn create_sampler(device: &Device) -> Sampler {
        device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        })
    }

    pub fn create_2d(
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        format: TextureFormat,
        mip_levels: &[impl AsRef<[u8]>],
        label: Option<&str>,
    ) -> Self {
        let mip_level_count = (mip_levels.len() as u32).max(1);
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_pixel = format.block_copy_size(None).unwrap_or(4);
        let mut mip_w = width;
        let mut mip_h = height;

        for (mip, data) in mip_levels.iter().enumerate() {
            let mip_size = Extent3d {
                width: mip_w,
                height: mip_h,
                depth_or_array_layers: 1,
            };
            queue.write_texture(
                TexelCopyTextureInfo {
                    aspect: TextureAspect::All,
                    texture: &texture,
                    mip_level: mip as u32,
                    origin: Origin3d::ZERO,
                },
                data.as_ref(),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_pixel * mip_w),
                    rows_per_image: Some(mip_h),
                },
                mip_size,
            );
            mip_w = (mip_w / 2).max(1);
            mip_h = (mip_h / 2).max(1);
        }

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self { texture, view }
    }

    pub fn create_depth_texture(
        device: &Device,
        config: &SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&TextureViewDescriptor::default());
        Self { texture, view }
    }
}
