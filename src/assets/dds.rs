use crate::assets::ImageAsset;
use rayon::prelude::*;

pub const DDS_MAGIC: u32 = 0x2053_4444; // "DDS " in little-endian

#[allow(dead_code)]
pub const DDSD_CAPS: u32 = 0x1;
#[allow(dead_code)]
pub const DDSD_HEIGHT: u32 = 0x2;
#[allow(dead_code)]
pub const DDSD_WIDTH: u32 = 0x4;
#[allow(dead_code)]
pub const DDSD_PITCH: u32 = 0x8;
#[allow(dead_code)]
pub const DDSD_PIXELFORMAT: u32 = 0x1000;
#[allow(dead_code)]
pub const DDSD_MIPMAPCOUNT: u32 = 0x20000;

#[allow(dead_code)]
pub const DDPF_ALPHAPIXELS: u32 = 0x1;
#[allow(dead_code)]
pub const DDPF_FOURCC: u32 = 0x4;
#[allow(dead_code)]
pub const DDPF_RGB: u32 = 0x40;

#[allow(dead_code)]
pub const DDSCAPS_COMPLEX: u32 = 0x8;
#[allow(dead_code)]
pub const DDSCAPS_TEXTURE: u32 = 0x1000;
#[allow(dead_code)]
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
    let mut mip_sizes = Vec::with_capacity(mip_count as usize);
    let mut total_expected_bytes = 0usize;

    for _ in 0..mip_count {
        let level_len = (cur_w as usize) * (cur_h as usize) * 4;
        mip_sizes.push(level_len);
        total_expected_bytes += level_len;
        cur_w = (cur_w / 2).max(1);
        cur_h = (cur_h / 2).max(1);
    }

    let actual_payload = data.len().saturating_sub(header_size);
    if actual_payload < total_expected_bytes {
        anyhow::bail!(
            "DDS payload truncated: expected at least {} bytes for {} mips, got {}",
            total_expected_bytes,
            mip_count,
            actual_payload
        );
    }

    // Drop the header bytes so only pixel bytes remain
    data.drain(0..header_size);
    data.truncate(total_expected_bytes);

    // If pixel order is BGRA, swizzle B and R to RGBA
    if is_bgra {
        data.par_chunks_exact_mut(4).for_each(|pixel| {
            pixel.swap(0, 2);
        });
    }

    let mut mip_levels = Vec::with_capacity(mip_count as usize);
    let mut remaining = data;
    for &level_len in &mip_sizes[..mip_sizes.len().saturating_sub(1)] {
        let next = remaining.split_off(level_len);
        mip_levels.push(remaining);
        remaining = next;
    }
    mip_levels.push(remaining);

    Ok(ImageAsset {
        width,
        height,
        mip_levels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dds_header_sizes() {
        assert_eq!(std::mem::size_of::<DdsPixelFormat>(), 32);
        assert_eq!(std::mem::size_of::<DdsHeader>(), 124);
        assert_eq!(std::mem::size_of::<DdsHeaderDxt10>(), 20);
    }

    #[test]
    fn test_dds_invalid_magic() {
        let bad_buffer = vec![0u8; 128];
        assert!(parse_dds(bad_buffer).is_err());
    }

    #[test]
    fn test_parse_dds_multi_mip() {
        // Create a 4x2 texture with 3 mip levels (4x2, 2x1, 1x1)
        let width = 4u32;
        let height = 2u32;
        let mip_count = 3u32;

        let l0_len = 4 * 2 * 4; // 32
        let l1_len = 2 * 1 * 4; // 8
        let l2_len = 1 * 1 * 4; // 4
        let _total_payload = l0_len + l1_len + l2_len;

        let header = DdsHeader {
            size: 124,
            flags: DDSD_CAPS
                | DDSD_HEIGHT
                | DDSD_WIDTH
                | DDSD_PITCH
                | DDSD_PIXELFORMAT
                | DDSD_MIPMAPCOUNT,
            height,
            width,
            pitch_or_linear_size: width * 4,
            depth: 1,
            mipmap_count: mip_count,
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
            caps: DDSCAPS_TEXTURE | DDSCAPS_COMPLEX | DDSCAPS_MIPMAP,
            caps2: 0,
            caps3: 0,
            caps4: 0,
            reserved2: 0,
        };

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&DDS_MAGIC.to_le_bytes());
        buffer.extend_from_slice(bytemuck::bytes_of(&header));

        // Fill mip payloads
        buffer.extend(vec![1u8; l0_len]);
        buffer.extend(vec![2u8; l1_len]);
        buffer.extend(vec![3u8; l2_len]);

        let asset = parse_dds(buffer).expect("Should successfully parse mipmapped DDS");
        assert_eq!(asset.width, 4);
        assert_eq!(asset.height, 2);
        assert_eq!(asset.mip_levels[0].len(), l0_len);
        assert_eq!(asset.mip_levels[0][0], 1);
        assert_eq!(asset.mip_levels[1].len(), l1_len);
        assert_eq!(asset.mip_levels[1][0], 2);
        assert_eq!(asset.mip_levels[2].len(), l2_len);
        assert_eq!(asset.mip_levels[2][0], 3);
    }
}
