use crate::assets::ImageAsset;
use rayon::prelude::*;

pub const DDS_MAGIC: u32 = 0x2053_4444; // "DDS " in little-endian

pub const DDPF_FOURCC: u32 = 0x4;
pub const DDPF_RGB: u32 = 0x40;

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
    let _span = tracing::trace_span!("parse_dds", bytes = data.len()).entered();
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

    let expected_payload = (width as usize) * (height as usize) * 4;
    let actual_payload = data.len() - header_size;
    if actual_payload < expected_payload {
        anyhow::bail!(
            "DDS payload truncated: expected at least {} bytes, got {}",
            expected_payload,
            actual_payload
        );
    }

    // Drop the header bytes so only pixel bytes remain
    data.drain(0..header_size);
    // Truncate any trailing data (such as lower mipmaps if present) to just level 0
    data.truncate(expected_payload);

    // If pixel order is BGRA, swizzle B and R to RGBA
    if is_bgra {
        data.par_chunks_exact_mut(4).for_each(|pixel| {
            pixel.swap(0, 2);
        });
    }

    Ok(ImageAsset {
        width,
        height,
        data,
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
}
