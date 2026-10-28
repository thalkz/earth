use wgpu::{
    AddressMode, Device, Extent3d, Features, FilterMode, MipmapFilterMode, Origin3d, Queue,
    Sampler, SamplerDescriptor, SurfaceConfiguration, TexelCopyBufferLayout, TexelCopyTextureInfo,
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

    pub fn create_repeat_sampler(device: &Device) -> Sampler {
        device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
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
        let mip_level_count = mip_levels.len() as u32;
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

        let (block_w, block_h) = format.block_dimensions();
        let block_size = format.block_copy_size(None).unwrap_or(4);

        let mut mip_w = width;
        let mut mip_h = height;

        for (mip, data) in mip_levels.iter().enumerate() {
            let mip_size = Extent3d {
                width: mip_w,
                height: mip_h,
                depth_or_array_layers: 1,
            };
            let blocks_x = mip_w.div_ceil(block_w);
            let blocks_y = mip_h.div_ceil(block_h);
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
                    bytes_per_row: Some(blocks_x * block_size),
                    rows_per_image: Some(blocks_y),
                },
                mip_size,
            );
            mip_w = (mip_w / 2).max(1);
            mip_h = (mip_h / 2).max(1);
        }

        let view = texture.create_view(&TextureViewDescriptor {
            label,
            format: Some(format),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: Some(1),
            usage: None,
        });

        Self { texture, view }
    }

    pub fn create_cube(
        device: &Device,
        queue: &Queue,
        face_size: u32,
        format: TextureFormat,
        mip_levels_per_face: &[impl AsRef<[u8]>], // Total count: 6 * mip_level_count
        label: Option<&str>,
    ) -> Self {
        let is_bc1 = matches!(
            format,
            TextureFormat::Bc1RgbaUnorm | TextureFormat::Bc1RgbaUnormSrgb
        );
        let decompress_bc1 = is_bc1 && !device.features().contains(Features::TEXTURE_COMPRESSION_BC);
        let actual_format = if decompress_bc1 {
            if format == TextureFormat::Bc1RgbaUnormSrgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            }
        } else {
            format
        };

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
            format: actual_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let (block_w, block_h) = actual_format.block_dimensions();
        let block_size = actual_format.block_copy_size(None).unwrap_or(4);

        for face in 0..6 {
            let mut mip_w = face_size;
            let mut mip_h = face_size;

            for mip in 0..mip_level_count {
                let raw_data =
                    mip_levels_per_face[(face * mip_level_count as usize) + mip as usize].as_ref();
                let decoded_storage;
                let upload_bytes: &[u8] = if decompress_bc1 {
                    decoded_storage = decode_bc1_to_rgba8(raw_data, mip_w, mip_h);
                    &decoded_storage
                } else {
                    raw_data
                };

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
                    upload_bytes,
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
            format: Some(actual_format),
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

fn decode_rgb565(c: u16) -> [u8; 4] {
    let r5 = ((c >> 11) & 31) as u32;
    let g6 = ((c >> 5) & 63) as u32;
    let b5 = (c & 31) as u32;
    [
        ((r5 * 255 + 15) / 31) as u8,
        ((g6 * 255 + 31) / 63) as u8,
        ((b5 * 255 + 15) / 31) as u8,
        255,
    ]
}

fn decode_bc1_to_rgba8(bc1: &[u8], width: u32, height: u32) -> Vec<u8> {
    let blocks_x = ((width + 3) / 4).max(1) as usize;
    let blocks_y = ((height + 3) / 4).max(1) as usize;
    let w = width as usize;
    let h = height as usize;
    let mut rgba = vec![0u8; w * h * 4];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_idx = (by * blocks_x + bx) * 8;
            if block_idx + 8 > bc1.len() {
                break;
            }
            let c0_565 = u16::from_le_bytes([bc1[block_idx], bc1[block_idx + 1]]);
            let c1_565 = u16::from_le_bytes([bc1[block_idx + 2], bc1[block_idx + 3]]);
            let indices = u32::from_le_bytes([
                bc1[block_idx + 4],
                bc1[block_idx + 5],
                bc1[block_idx + 6],
                bc1[block_idx + 7],
            ]);

            let c0 = decode_rgb565(c0_565);
            let c1 = decode_rgb565(c1_565);
            let palette = if c0_565 > c1_565 {
                let c2 = [
                    ((2 * c0[0] as u16 + c1[0] as u16 + 1) / 3) as u8,
                    ((2 * c0[1] as u16 + c1[1] as u16 + 1) / 3) as u8,
                    ((2 * c0[2] as u16 + c1[2] as u16 + 1) / 3) as u8,
                    255,
                ];
                let c3 = [
                    ((c0[0] as u16 + 2 * c1[0] as u16 + 1) / 3) as u8,
                    ((c0[1] as u16 + 2 * c1[1] as u16 + 1) / 3) as u8,
                    ((c0[2] as u16 + 2 * c1[2] as u16 + 1) / 3) as u8,
                    255,
                ];
                [c0, c1, c2, c3]
            } else {
                let c2 = [
                    ((c0[0] as u16 + c1[0] as u16) / 2) as u8,
                    ((c0[1] as u16 + c1[1] as u16) / 2) as u8,
                    ((c0[2] as u16 + c1[2] as u16) / 2) as u8,
                    255,
                ];
                [c0, c1, c2, [0, 0, 0, 0]]
            };

            for py in 0..4 {
                let y = by * 4 + py;
                if y >= h {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= w {
                        continue;
                    }
                    let idx = ((indices >> ((py * 4 + px) * 2)) & 3) as usize;
                    let dst = (y * w + x) * 4;
                    rgba[dst..dst + 4].copy_from_slice(&palette[idx]);
                }
            }
        }
    }

    rgba
}
