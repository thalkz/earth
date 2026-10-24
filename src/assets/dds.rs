use crate::assets::ImageAsset;
use rayon::prelude::*;

pub const DDS_MAGIC: u32 = 0x2053_4444; // "DDS " in little-endian

pub const DDSD_CAPS: u32 = 0x1;
pub const DDSD_HEIGHT: u32 = 0x2;
pub const DDSD_WIDTH: u32 = 0x4;
pub const DDSD_PITCH: u32 = 0x8;
pub const DDSD_PIXELFORMAT: u32 = 0x1000;
pub const DDSD_MIPMAPCOUNT: u32 = 0x20000;

pub const DDPF_ALPHAPIXELS: u32 = 0x1;
pub const DDPF_FOURCC: u32 = 0x4;
pub const DDPF_RGB: u32 = 0x40;

pub const DDSCAPS_COMPLEX: u32 = 0x8;
pub const DDSCAPS_TEXTURE: u32 = 0x1000;
pub const DDSCAPS_MIPMAP: u32 = 0x400000;

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
    pub fn new_rgba8(width: u32, height: u32, mipmap_count: u32) -> Self {
        let mut flags = DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PITCH | DDSD_PIXELFORMAT;
        let mut caps = DDSCAPS_TEXTURE;
        if mipmap_count > 1 {
            flags |= DDSD_MIPMAPCOUNT;
            caps |= DDSCAPS_COMPLEX | DDSCAPS_MIPMAP;
        }

        Self {
            size: 124,
            flags,
            height,
            width,
            pitch_or_linear_size: width * 4,
            depth: 1,
            mipmap_count,
            reserved1: [0; 11],
            pixel_format: DdsPixelFormat {
                size: 32,
                flags: DDPF_RGB | DDPF_ALPHAPIXELS,
                four_cc: 0,
                rgb_bit_count: 32,
                r_bit_mask: 0x0000_00FF,
                g_bit_mask: 0x0000_FF00,
                b_bit_mask: 0x00FF_0000,
                a_bit_mask: 0xFF00_0000,
            },
            caps,
            caps2: 0,
            caps3: 0,
            caps4: 0,
            reserved2: 0,
        }
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
    let is_bgra = if pf.flags & DDPF_FOURCC != 0 && &pf.four_cc.to_le_bytes() == b"DX10" {
        if data.len() < 148 {
            anyhow::bail!("DDS DX10 header truncated");
        }
        let dxt10: DdsHeaderDxt10 = *bytemuck::from_bytes(&data[128..148]);
        header_size = 148;
        // DXGI_FORMAT_B8G8R8A8_UNORM = 87
        dxt10.dxgi_format == 87
    } else if pf.flags & DDPF_RGB != 0 {
        if pf.rgb_bit_count != 32 {
            anyhow::bail!("Unsupported DDS bit count: {}", pf.rgb_bit_count);
        }
        pf.r_bit_mask == 0x00FF_0000 && pf.b_bit_mask == 0x0000_00FF
    } else {
        anyhow::bail!("Unsupported DDS pixel format flags: {:#x}", pf.flags);
    };

    let mip_count = if header.flags & DDSD_MIPMAPCOUNT != 0 && header.mipmap_count > 0 {
        header.mipmap_count
    } else {
        1
    };

    let mut cur_w = width;
    let mut cur_h = height;
    let mut mip_offsets = Vec::with_capacity(mip_count as usize);
    let mut current_offset = header_size;

    for _ in 0..mip_count {
        let level_len = (cur_w as usize) * (cur_h as usize) * 4;
        let next_offset = current_offset + level_len;
        mip_offsets.push(current_offset..next_offset);
        current_offset = next_offset;
        cur_w = (cur_w / 2).max(1);
        cur_h = (cur_h / 2).max(1);
    }

    if data.len() < current_offset {
        anyhow::bail!(
            "DDS payload truncated: expected at least {} bytes for {} mips, got {}",
            current_offset,
            mip_count,
            data.len()
        );
    }

    // Truncate any trailing padding without reallocating
    data.truncate(current_offset);

    // If pixel order is BGRA, swizzle B and R to RGBA in-place
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
        data,
        mip_offsets,
    })
}
