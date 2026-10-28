use crate::assets::ImageAsset;
use rayon::prelude::*;

pub const DDS_MAGIC: u32 = 0x2053_4444; // "DDS " in little-endian

pub const DDSD_CAPS: u32 = 0x1;
pub const DDSD_HEIGHT: u32 = 0x2;
pub const DDSD_WIDTH: u32 = 0x4;
pub const DDSD_PITCH: u32 = 0x8;
pub const DDSD_PIXELFORMAT: u32 = 0x1000;
pub const DDSD_MIPMAPCOUNT: u32 = 0x20000;
pub const DDSD_LINEARSIZE: u32 = 0x80000;

pub const DDPF_ALPHAPIXELS: u32 = 0x1;
pub const DDPF_FOURCC: u32 = 0x4;
pub const DDPF_RGB: u32 = 0x40;

pub const DDSCAPS_COMPLEX: u32 = 0x8;
pub const DDSCAPS_TEXTURE: u32 = 0x1000;
pub const DDSCAPS_MIPMAP: u32 = 0x400000;

pub const DDSCAPS2_CUBEMAP: u32 = 0x200;
pub const DDSCAPS2_CUBEMAP_POSITIVEX: u32 = 0x400;
pub const DDSCAPS2_CUBEMAP_NEGATIVEX: u32 = 0x800;
pub const DDSCAPS2_CUBEMAP_POSITIVEY: u32 = 0x1000;
pub const DDSCAPS2_CUBEMAP_NEGATIVEY: u32 = 0x2000;
pub const DDSCAPS2_CUBEMAP_POSITIVEZ: u32 = 0x4000;
pub const DDSCAPS2_CUBEMAP_NEGATIVEZ: u32 = 0x8000;
pub const DDSCAPS2_CUBEMAP_ALLFACES: u32 = DDSCAPS2_CUBEMAP
    | DDSCAPS2_CUBEMAP_POSITIVEX
    | DDSCAPS2_CUBEMAP_NEGATIVEX
    | DDSCAPS2_CUBEMAP_POSITIVEY
    | DDSCAPS2_CUBEMAP_NEGATIVEY
    | DDSCAPS2_CUBEMAP_POSITIVEZ
    | DDSCAPS2_CUBEMAP_NEGATIVEZ;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DdsPixelFormat {
    pub size: u32,
    pub flags: u32,
    pub four_cc: u32,
    pub rgb_bit_count: u32,
    pub r_bit_mask: u32,
    pub g_bit_mask: u32,
    pub b_bit_mask: u32,
    pub a_bit_mask: u32,
}

impl DdsPixelFormat {
    const RGBA8: Self = Self {
        size: 32,
        flags: DDPF_RGB | DDPF_ALPHAPIXELS,
        four_cc: 0,
        rgb_bit_count: 32,
        r_bit_mask: 0x0000_00FF,
        g_bit_mask: 0x0000_FF00,
        b_bit_mask: 0x00FF_0000,
        a_bit_mask: 0xFF00_0000,
    };

    const DXT1: Self = Self {
        size: 32,
        flags: DDPF_FOURCC,
        four_cc: u32::from_le_bytes(*b"DXT1"),
        rgb_bit_count: 0,
        r_bit_mask: 0,
        g_bit_mask: 0,
        b_bit_mask: 0,
        a_bit_mask: 0,
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DdsHeader {
    pub size: u32,
    pub flags: u32,
    pub height: u32,
    pub width: u32,
    pub pitch_or_linear_size: u32,
    pub depth: u32,
    pub mipmap_count: u32,
    pub reserved1: [u32; 11],
    pub pixel_format: DdsPixelFormat,
    pub caps: u32,
    pub caps2: u32,
    pub caps3: u32,
    pub caps4: u32,
    pub reserved2: u32,
}

impl DdsHeader {
    fn build(
        width: u32,
        height: u32,
        mipmap_count: u32,
        pitch_or_linear_size: u32,
        size_flag: u32,
        pixel_format: DdsPixelFormat,
        is_cubemap: bool,
    ) -> Self {
        let mut flags = DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PIXELFORMAT | size_flag;
        let mut caps = DDSCAPS_TEXTURE | if is_cubemap { DDSCAPS_COMPLEX } else { 0 };
        if mipmap_count > 1 {
            flags |= DDSD_MIPMAPCOUNT;
            caps |= DDSCAPS_COMPLEX | DDSCAPS_MIPMAP;
        }

        Self {
            size: 124,
            flags,
            height,
            width,
            pitch_or_linear_size,
            depth: 1,
            mipmap_count,
            reserved1: [0; 11],
            pixel_format,
            caps,
            caps2: if is_cubemap { DDSCAPS2_CUBEMAP_ALLFACES } else { 0 },
            caps3: 0,
            caps4: 0,
            reserved2: 0,
        }
    }

    pub fn new_rgba8_2d(width: u32, height: u32, mipmap_count: u32) -> Self {
        Self::build(
            width,
            height,
            mipmap_count,
            width * 4,
            DDSD_PITCH,
            DdsPixelFormat::RGBA8,
            false,
        )
    }

    pub fn new_rgba8_cubemap(face_size: u32, mipmap_count: u32) -> Self {
        Self::build(
            face_size,
            face_size,
            mipmap_count,
            face_size * 4,
            DDSD_PITCH,
            DdsPixelFormat::RGBA8,
            true,
        )
    }

    pub fn new_bc1_cubemap(face_size: u32, mipmap_count: u32) -> Self {
        let blocks = face_size.div_ceil(4).max(1);
        Self::build(
            face_size,
            face_size,
            mipmap_count,
            blocks * blocks * 8,
            DDSD_LINEARSIZE,
            DdsPixelFormat::DXT1,
            true,
        )
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DdsHeaderDxt10 {
    pub dxgi_format: u32,
    pub resource_dimension: u32,
    pub misc_flag: u32,
    pub array_size: u32,
    pub misc_flags2: u32,
}

pub fn parse_dds(mut data: Vec<u8>) -> anyhow::Result<ImageAsset> {
    if data.len() < 128 {
        anyhow::bail!("DDS file too small (less than 128 bytes)");
    }
    let magic = u32::from_le_bytes(data[0..4].try_into()?);
    if magic != DDS_MAGIC {
        anyhow::bail!("Invalid DDS magic number: {:#x}", magic);
    }

    let header: DdsHeader = *bytemuck::from_bytes(&data[4..128]);
    if header.size != 124 || header.pixel_format.size != 32 {
        anyhow::bail!("Invalid DDS header dimensions");
    }

    let width = header.width;
    let height = header.height;
    let pf = header.pixel_format;

    let mut header_size = 128;
    let mut is_bgra = false;
    let format = if pf.flags & DDPF_FOURCC != 0 {
        if &pf.four_cc.to_le_bytes() == b"DXT1" {
            wgpu::TextureFormat::Bc1RgbaUnormSrgb
        } else if &pf.four_cc.to_le_bytes() == b"DX10" {
            if data.len() < 148 {
                anyhow::bail!("DDS DX10 header truncated");
            }
            let dxt10: DdsHeaderDxt10 = *bytemuck::from_bytes(&data[128..148]);
            header_size = 148;
            match dxt10.dxgi_format {
                87 => {
                    is_bgra = true;
                    wgpu::TextureFormat::Rgba8Unorm
                }
                98 | 99 => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
                other => anyhow::bail!("Unsupported DX10 format: {}", other),
            }
        } else {
            anyhow::bail!(
                "Unsupported FourCC: {:?}",
                std::str::from_utf8(&pf.four_cc.to_le_bytes())
            );
        }
    } else if pf.flags & DDPF_RGB != 0 {
        if pf.rgb_bit_count != 32 {
            anyhow::bail!("Unsupported DDS bit count: {}", pf.rgb_bit_count);
        }
        is_bgra = pf.r_bit_mask == 0x00FF_0000 && pf.b_bit_mask == 0x0000_00FF;
        wgpu::TextureFormat::Rgba8Unorm
    } else {
        anyhow::bail!("Unsupported DDS pixel format flags: {:#x}", pf.flags);
    };

    let mip_count = if header.flags & DDSD_MIPMAPCOUNT != 0 && header.mipmap_count > 0 {
        header.mipmap_count
    } else {
        1
    };

    let is_cubemap = header.caps2 & DDSCAPS2_CUBEMAP != 0;
    if is_cubemap && width != height {
        anyhow::bail!("Cubemap faces must be square, got {}x{}", width, height);
    }
    let face_count: usize = if is_cubemap { 6 } else { 1 };

    let mut mip_offsets = Vec::with_capacity(face_count * mip_count as usize);
    let mut current_offset = header_size;

    for _face in 0..face_count {
        let mut cur_w = width;
        let mut cur_h = height;
        for _ in 0..mip_count {
            let blocks_x = cur_w.div_ceil(4).max(1) as usize;
            let blocks_y = cur_h.div_ceil(4).max(1) as usize;
            let level_len = match format {
                wgpu::TextureFormat::Bc1RgbaUnorm | wgpu::TextureFormat::Bc1RgbaUnormSrgb => {
                    blocks_x * blocks_y * 8
                }
                wgpu::TextureFormat::Bc7RgbaUnorm | wgpu::TextureFormat::Bc7RgbaUnormSrgb => {
                    blocks_x * blocks_y * 16
                }
                _ => (cur_w as usize) * (cur_h as usize) * 4,
            };
            let next_offset = current_offset + level_len;
            mip_offsets.push(current_offset..next_offset);
            current_offset = next_offset;
            cur_w = (cur_w / 2).max(1);
            cur_h = (cur_h / 2).max(1);
        }
    }

    if data.len() < current_offset {
        anyhow::bail!(
            "DDS payload truncated: expected at least {} bytes for {} faces x {} mips, got {}",
            current_offset,
            face_count,
            mip_count,
            data.len()
        );
    }

    data.truncate(current_offset);

    if is_bgra {
        data[header_size..current_offset]
            .par_chunks_exact_mut(4)
            .for_each(|pixel| {
                pixel.swap(0, 2);
            });
    }

    Ok(ImageAsset {
        width,
        height,
        format,
        is_cubemap,
        data,
        mip_offsets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_bc1_cubemap() {
        let path = "res/earth/diffuse_cube_4096x4096.dds";
        if let Ok(bytes) = fs::read(path) {
            let asset = parse_dds(bytes).expect("Should parse BC1 DDS");
            assert!(asset.is_cubemap);
            assert_eq!(asset.width, 4096);
            assert_eq!(asset.height, 4096);
            assert_eq!(asset.format, wgpu::TextureFormat::Bc1RgbaUnormSrgb);
            assert_eq!(asset.mip_offsets.len(), 6 * 8);
        }
    }

    #[test]
    fn test_parse_2d_rgba8_dds() {
        let header = DdsHeader::new_rgba8_2d(4, 4, 2);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&DDS_MAGIC.to_le_bytes());
        bytes.extend_from_slice(bytemuck::bytes_of(&header));
        bytes.extend_from_slice(&[128u8; 64]);
        bytes.extend_from_slice(&[128u8; 16]);

        let asset = parse_dds(bytes).expect("Should parse 2D RGBA8 DDS");
        assert!(!asset.is_cubemap);
        assert_eq!(asset.width, 4);
        assert_eq!(asset.height, 4);
        assert_eq!(asset.format, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(asset.mip_offsets.len(), 2);
    }
}
