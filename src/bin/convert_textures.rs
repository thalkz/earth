use glam::Vec3;
use rayon::prelude::*;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

const DDS_MAGIC: u32 = 0x2053_4444; // "DDS " in little-endian

const DDSD_CAPS: u32 = 0x1;
const DDSD_HEIGHT: u32 = 0x2;
const DDSD_WIDTH: u32 = 0x4;
const DDSD_PITCH: u32 = 0x8;
const DDSD_PIXELFORMAT: u32 = 0x1000;

const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_RGB: u32 = 0x40;

const DDSCAPS_TEXTURE: u32 = 0x1000;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
struct DdsPixelFormat {
    size: u32,
    flags: u32,
    four_cc: u32,
    rgb_bit_count: u32,
    r_bit_mask: u32,
    g_bit_mask: u32,
    b_bit_mask: u32,
    a_bit_mask: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
struct DdsHeader {
    size: u32,
    flags: u32,
    height: u32,
    width: u32,
    pitch_or_linear_size: u32,
    depth: u32,
    mipmap_count: u32,
    reserved1: [u32; 11],
    pixel_format: DdsPixelFormat,
    caps: u32,
    caps2: u32,
    caps3: u32,
    caps4: u32,
    reserved2: u32,
}

impl DdsHeader {
    fn new_rgba8(width: u32, height: u32) -> Self {
        Self {
            size: 124,
            flags: DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PITCH | DDSD_PIXELFORMAT,
            height,
            width,
            pitch_or_linear_size: width * 4,
            depth: 1,
            mipmap_count: 1,
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
            caps: DDSCAPS_TEXTURE,
            caps2: 0,
            caps3: 0,
            caps4: 0,
            reserved2: 0,
        }
    }
}

fn write_dds_file<P: AsRef<Path>>(
    path: P,
    width: u32,
    height: u32,
    rgba_data: &[u8],
) -> std::io::Result<()> {
    let header = DdsHeader::new_rgba8(width, height);
    let mut file = File::create(path)?;
    file.write_all(&DDS_MAGIC.to_le_bytes())?;
    file.write_all(bytemuck::bytes_of(&header))?;
    file.write_all(rgba_data)?;
    Ok(())
}

fn uv_to_sphere_point(u: f32, v: f32) -> Vec3 {
    let lat = (0.5 - v) * PI;
    let lng = (0.5 - u) * (2.0 * PI);
    let y = lat.sin();
    let r = lat.cos();
    let x = lng.sin() * r;
    let z = -lng.cos() * r;
    Vec3::new(x, y, z)
}

fn convert_elevation_to_normal_elevation_rgba8(
    input_path: &Path,
    output_dds: &Path,
) -> anyhow::Result<()> {
    println!(
        "Converting {:?} to RGBA8 normal+elevation DDS...",
        input_path
    );
    let start = Instant::now();
    let img = image::open(input_path)?;
    let luma = img.into_luma16();
    let (width, height) = luma.dimensions();
    let raw_u16 = luma.into_raw();
    let w = width as usize;
    let h = height as usize;

    println!(
        "Image loaded in {:?} ({}x{}). Computing normals in parallel with Rayon...",
        start.elapsed(),
        width,
        height
    );

    let height_scale = 0.03f32;

    // Helper closure to calculate displaced position on the sphere for pixel (px, py)
    let get_displaced = |px: usize, py: usize| -> Vec3 {
        let u = (px as f32 + 0.5) / width as f32;
        let v = (py as f32 + 0.5) / height as f32;
        let p = uv_to_sphere_point(u, v);
        let elev = raw_u16[py * w + px] as f32 / 65535.0;
        p * (1.0 + elev * height_scale)
    };

    // Allocate output RGBA8 buffer (4 bytes per pixel)
    let mut rgba_bytes = vec![0u8; w * h * 4];

    // Process each scanline in parallel
    rgba_bytes
        .par_chunks_exact_mut(w * 4)
        .enumerate()
        .for_each(|(py, row)| {
            let py_north = if py > 0 { py - 1 } else { 0 };
            let py_south = if py < h - 1 { py + 1 } else { h - 1 };

            for px in 0..w {
                let px_west = (px + 1) % w;
                let px_east = (px + w - 1) % w;

                let u = (px as f32 + 0.5) / width as f32;
                let v = (py as f32 + 0.5) / height as f32;
                let sphere_p = uv_to_sphere_point(u, v);

                let raw_val = raw_u16[py * w + px];
                let alpha_u8 = ((raw_val as f64 / 65535.0) * 255.0).round() as u8;

                let normal = if alpha_u8 == 0 {
                    // Water: unperturbed smooth sphere normal (aligns with shader is_water condition)
                    sphere_p
                } else {
                    let s_nw = get_displaced(px_east, py_north);
                    let s_n = get_displaced(px, py_north);
                    let s_ne = get_displaced(px_west, py_north);

                    let s_w = get_displaced(px_east, py);
                    let s_e = get_displaced(px_west, py);

                    let s_sw = get_displaced(px_east, py_south);
                    let s_s = get_displaced(px, py_south);
                    let s_se = get_displaced(px_west, py_south);

                    // Sobel horizontal derivative (East to West along increasing u)
                    let v_u = (s_ne - s_nw) + (s_e - s_w) * 2.0 + (s_se - s_sw);
                    // Sobel vertical derivative (North to South along increasing v)
                    let v_v = (s_sw - s_nw) + (s_s - s_n) * 2.0 + (s_se - s_ne);

                    let n = v_v.cross(v_u);
                    if n.length_squared() > 1e-20 {
                        n.normalize()
                    } else {
                        sphere_p
                    }
                };

                let nx = (normal.x * 0.5 + 0.5).clamp(0.0, 1.0);
                let ny = (normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
                let nz = (normal.z * 0.5 + 0.5).clamp(0.0, 1.0);

                let offset = px * 4;
                row[offset] = (nx * 255.0).round() as u8;
                row[offset + 1] = (ny * 255.0).round() as u8;
                row[offset + 2] = (nz * 255.0).round() as u8;
                row[offset + 3] = alpha_u8;
            }
        });

    println!(
        "Normals computed in {:?}! Writing DDS to {:?}...",
        start.elapsed(),
        output_dds
    );

    write_dds_file(output_dds, width, height, &rgba_bytes)?;

    println!(
        "Done in {:?}! Saved {:?} ({}x{}, {} bytes)",
        start.elapsed(),
        output_dds,
        width,
        height,
        rgba_bytes.len() + 128
    );

    Ok(())
}

fn convert_png_to_rgba8(
    input_path: &Path,
    output_dds: &Path,
) -> anyhow::Result<()> {
    println!("Converting {:?} to RGBA8 DDS...", input_path);
    let start = Instant::now();
    let img = image::open(input_path)?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    let raw_bytes = rgba.into_raw();

    write_dds_file(output_dds, width, height, &raw_bytes)?;

    println!(
        "Done in {:?}! Saved {:?} ({}x{}, {} bytes)",
        start.elapsed(),
        output_dds,
        width,
        height,
        raw_bytes.len() + 128
    );

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let elev_png = Path::new("res/earth/elevation.png");
    let elev_dds = Path::new("res/earth/elevation.dds");
    if elev_png.exists() {
        convert_elevation_to_normal_elevation_rgba8(elev_png, elev_dds)?;
    }

    let diff_png = Path::new("res/earth/diffuse.png");
    let diff_dds = Path::new("res/earth/diffuse.dds");
    if diff_png.exists() {
        convert_png_to_rgba8(diff_png, diff_dds)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_to_sphere_point() {
        let p_equator = uv_to_sphere_point(0.5, 0.5);
        assert!((p_equator.x).abs() < 1e-6);
        assert!((p_equator.y).abs() < 1e-6);
        assert!((p_equator.z - (-1.0)).abs() < 1e-6);

        let p_north = uv_to_sphere_point(0.5, 0.0);
        assert!((p_north.y - 1.0).abs() < 1e-6);

        let p_south = uv_to_sphere_point(0.5, 1.0);
        assert!((p_south.y - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_normal_packing_roundtrip() {
        let original = Vec3::new(0.3, 0.8, -0.5).normalize();
        let nx = (original.x * 0.5 + 0.5).clamp(0.0, 1.0);
        let ny = (original.y * 0.5 + 0.5).clamp(0.0, 1.0);
        let nz = (original.z * 0.5 + 0.5).clamp(0.0, 1.0);

        let r = (nx * 255.0).round() as u8;
        let g = (ny * 255.0).round() as u8;
        let b = (nz * 255.0).round() as u8;

        let unpacked = Vec3::new(
            (r as f32 / 255.0) * 2.0 - 1.0,
            (g as f32 / 255.0) * 2.0 - 1.0,
            (b as f32 / 255.0) * 2.0 - 1.0,
        )
        .normalize();

        assert!(original.dot(unpacked) > 0.999);
    }

    #[test]
    fn test_surface_normal_outward_direction() {
        let u = 0.5f32;
        let v = 0.5f32;
        let sphere_p = uv_to_sphere_point(u, v);

        let eps = 0.001f32;
        let s_nw = uv_to_sphere_point(u - eps, v - eps);
        let s_n = uv_to_sphere_point(u, v - eps);
        let s_ne = uv_to_sphere_point(u + eps, v - eps);

        let s_w = uv_to_sphere_point(u - eps, v);
        let s_e = uv_to_sphere_point(u + eps, v);

        let s_sw = uv_to_sphere_point(u - eps, v + eps);
        let s_s = uv_to_sphere_point(u, v + eps);
        let s_se = uv_to_sphere_point(u + eps, v + eps);

        let v_u = (s_ne - s_nw) + (s_e - s_w) * 2.0 + (s_se - s_sw);
        let v_v = (s_sw - s_nw) + (s_s - s_n) * 2.0 + (s_se - s_ne);
        let n = v_v.cross(v_u).normalize();

        assert!(
            n.dot(sphere_p) > 0.99,
            "Cross product must point outwards in the direction of sphere normal"
        );
    }

    #[test]
    fn test_mountain_slope_directional_tilt() {
        let u = 0.25f32;
        let v = 0.5f32;
        let eps = 0.001f32;

        let p_west = uv_to_sphere_point(u + eps, v);
        let p_east = uv_to_sphere_point(u - eps, v);
        let p_south = uv_to_sphere_point(u, v + eps);
        let p_north = uv_to_sphere_point(u, v - eps);

        let h = 0.03f32;
        // Peak is to the East (p_east displaced), slope faces West
        let s_west = p_west * 1.0;
        let s_east = p_east * (1.0 + h);
        let s_south = p_south * (1.0 + h * 0.5);
        let s_north = p_north * (1.0 + h * 0.5);

        let v_u = s_west - s_east;
        let v_v = s_south - s_north;
        let n = v_v.cross(v_u).normalize();

        let light_from_east = Vec3::new(2.0, 0.0, 2.0).normalize();
        let light_from_west = Vec3::new(2.0, 0.0, -2.0).normalize();

        assert!(n.dot(light_from_west) > n.dot(light_from_east), "West slope must face West!");
    }
}
