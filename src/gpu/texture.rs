use wgpu::{
    AddressMode, Device, Extent3d, FilterMode, MipmapFilterMode, Origin3d, Queue, Sampler,
    SamplerDescriptor, SurfaceConfiguration, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture as WgpuTexture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
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
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        })
    }

    pub fn create_cube(
        device: &Device,
        queue: &Queue,
        face_size: u32,
        format: TextureFormat,
        mip_levels_per_face: &[impl AsRef<[u8]>], // Total count: 6 * mip_level_count
        label: Option<&str>,
    ) -> Self {
        let mip_level_count = (mip_levels_per_face.len() / 6) as u32;
        let size = Extent3d {
            width: face_size,
            height: face_size,
            depth_or_array_layers: 6,
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

        let (block_w, block_h) = format.block_dimensions();
        let block_size = format.block_copy_size(None).unwrap_or(4);

        for face in 0..6 {
            let mut mip_w = face_size;
            let mut mip_h = face_size;

            for mip in 0..mip_level_count {
                let data = &mip_levels_per_face[(face * mip_level_count as usize) + mip as usize];
                let mip_size = Extent3d {
                    width: mip_w,
                    height: mip_h,
                    depth_or_array_layers: 1,
                };
                let blocks_x = (mip_w + block_w - 1) / block_w;
                let blocks_y = (mip_h + block_h - 1) / block_h;
                queue.write_texture(
                    TexelCopyTextureInfo {
                        aspect: TextureAspect::All,
                        texture: &texture,
                        mip_level: mip,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: face as u32,
                        },
                    },
                    data.as_ref(),
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(blocks_x * block_size),
                        rows_per_image: Some(blocks_y),
                    },
                    mip_size,
                );
                mip_w = (mip_w / 2).max(1);
                mip_h = (mip_h / 2).max(1);
            }
        }

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(format),
            dimension: Some(TextureViewDimension::Cube),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: Some(6),
            usage: None,
        });

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
