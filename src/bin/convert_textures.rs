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
const DDSD_MIPMAPCOUNT: u32 = 0x20000;

const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_RGB: u32 = 0x40;

const DDSCAPS_COMPLEX: u32 = 0x8;
const DDSCAPS_TEXTURE: u32 = 0x1000;
const DDSCAPS_MIPMAP: u32 = 0x400000;

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
    fn new_rgba8(width: u32, height: u32, mipmap_count: u32) -> Self {
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

fn write_dds_file<P: AsRef<Path>>(
    path: P,
    width: u32,
    height: u32,
    mip_levels: &[Vec<u8>],
) -> std::io::Result<()> {
    let header = DdsHeader::new_rgba8(width, height, mip_levels.len() as u32);
    let mut file = File::create(path)?;
    file.write_all(&DDS_MAGIC.to_le_bytes())?;
    file.write_all(bytemuck::bytes_of(&header))?;
    for level in mip_levels {
        file.write_all(level)?;
    }
    Ok(())
}

fn compute_mip_count(width: u32, height: u32) -> u32 {
    width.max(height).ilog2().clamp(1, 8)
}

fn generate_mipmaps_diffuse(level0: Vec<u8>, width: u32, height: u32) -> Vec<Vec<u8>> {
    let mip_count = compute_mip_count(width, height);
    let mut mips = Vec::with_capacity(mip_count as usize);
    let mut prev_w = width as usize;
    let mut prev_h = height as usize;
    mips.push(level0);

    for _ in 1..mip_count {
        let dst_w = (prev_w / 2).max(1);
        let dst_h = (prev_h / 2).max(1);
        let prev_level = mips.last().unwrap();

        let mut next_level = vec![0u8; dst_w * dst_h * 4];
        next_level
            .par_chunks_exact_mut(dst_w * 4)
            .enumerate()
            .for_each(|(dy, row)| {
                let sy0 = dy * 2;
                let sy1 = (sy0 + 1).min(prev_h - 1);

                for dx in 0..dst_w {
                    let sx0 = dx * 2;
                    let sx1 = (sx0 + 1).min(prev_w - 1);

                    let p00_idx = (sy0 * prev_w + sx0) * 4;
                    let p10_idx = (sy0 * prev_w + sx1) * 4;
                    let p01_idx = (sy1 * prev_w + sx0) * 4;
                    let p11_idx = (sy1 * prev_w + sx1) * 4;

                    let off = dx * 4;
                    for c in 0..4 {
                        let sum = prev_level[p00_idx + c] as u32
                            + prev_level[p10_idx + c] as u32
                            + prev_level[p01_idx + c] as u32
                            + prev_level[p11_idx + c] as u32;
                        row[off + c] = ((sum + 2) / 4) as u8;
                    }
                }
            });

        mips.push(next_level);
        prev_w = dst_w;
        prev_h = dst_h;
    }

    mips
}

fn generate_mipmaps_normal_elevation(level0: Vec<u8>, width: u32, height: u32) -> Vec<Vec<u8>> {
    let mip_count = compute_mip_count(width, height);
    let mut mips = Vec::with_capacity(mip_count as usize);
    let mut prev_w = width as usize;
    let mut prev_h = height as usize;
    mips.push(level0);

    for _ in 1..mip_count {
        let dst_w = (prev_w / 2).max(1);
        let dst_h = (prev_h / 2).max(1);
        let prev_level = mips.last().unwrap();

        let mut next_level = vec![0u8; dst_w * dst_h * 4];
        next_level
            .par_chunks_exact_mut(dst_w * 4)
            .enumerate()
            .for_each(|(dy, row)| {
                let sy0 = dy * 2;
                let sy1 = (sy0 + 1).min(prev_h - 1);

                for dx in 0..dst_w {
                    let sx0 = dx * 2;
                    let sx1 = (sx0 + 1).min(prev_w - 1);

                    let p00_idx = (sy0 * prev_w + sx0) * 4;
                    let p10_idx = (sy0 * prev_w + sx1) * 4;
                    let p01_idx = (sy1 * prev_w + sx0) * 4;
                    let p11_idx = (sy1 * prev_w + sx1) * 4;

                    let unpack_normal = |idx: usize| -> Vec3 {
                        Vec3::new(
                            (prev_level[idx] as f32 / 255.0) * 2.0 - 1.0,
                            (prev_level[idx + 1] as f32 / 255.0) * 2.0 - 1.0,
                            (prev_level[idx + 2] as f32 / 255.0) * 2.0 - 1.0,
                        )
                    };

                    let n00 = unpack_normal(p00_idx);
                    let n10 = unpack_normal(p10_idx);
                    let n01 = unpack_normal(p01_idx);
                    let n11 = unpack_normal(p11_idx);

                    let v_sum = n00 + n10 + n01 + n11;
                    let normal = if v_sum.length_squared() > 1e-12 {
                        v_sum.normalize()
                    } else {
                        let u = (dx as f32 + 0.5) / dst_w as f32;
                        let v = (dy as f32 + 0.5) / dst_h as f32;
                        uv_to_sphere_point(u, v)
                    };

                    let nx = (normal.x * 0.5 + 0.5).clamp(0.0, 1.0);
                    let ny = (normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
                    let nz = (normal.z * 0.5 + 0.5).clamp(0.0, 1.0);

                    let a_sum = prev_level[p00_idx + 3] as u32
                        + prev_level[p10_idx + 3] as u32
                        + prev_level[p01_idx + 3] as u32
                        + prev_level[p11_idx + 3] as u32;
                    let avg_a = ((a_sum + 2) / 4) as u8;

                    let off = dx * 4;
                    row[off] = (nx * 255.0).round() as u8;
                    row[off + 1] = (ny * 255.0).round() as u8;
                    row[off + 2] = (nz * 255.0).round() as u8;
                    row[off + 3] = avg_a;
                }
            });

        mips.push(next_level);
        prev_w = dst_w;
        prev_h = dst_h;
    }

    mips
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

struct GebcoTile {
    filename: &'static str,
    col: usize,
    row: usize,
}

const GEBCO_TILES: [GebcoTile; 8] = [
    GebcoTile {
        filename: "gebco_2026_n90.0_s0.0_w-180.0_e-90.0_geotiff.tif",
        col: 0,
        row: 0,
    },
    GebcoTile {
        filename: "gebco_2026_n90.0_s0.0_w-90.0_e0.0_geotiff.tif",
        col: 1,
        row: 0,
    },
    GebcoTile {
        filename: "gebco_2026_n90.0_s0.0_w0.0_e90.0_geotiff.tif",
        col: 2,
        row: 0,
    },
    GebcoTile {
        filename: "gebco_2026_n90.0_s0.0_w90.0_e180.0_geotiff.tif",
        col: 3,
        row: 0,
    },
    GebcoTile {
        filename: "gebco_2026_n0.0_s-90.0_w-180.0_e-90.0_geotiff.tif",
        col: 0,
        row: 1,
    },
    GebcoTile {
        filename: "gebco_2026_n0.0_s-90.0_w-90.0_e0.0_geotiff.tif",
        col: 1,
        row: 1,
    },
    GebcoTile {
        filename: "gebco_2026_n0.0_s-90.0_w0.0_e90.0_geotiff.tif",
        col: 2,
        row: 1,
    },
    GebcoTile {
        filename: "gebco_2026_n0.0_s-90.0_w90.0_e180.0_geotiff.tif",
        col: 3,
        row: 1,
    },
];

const MAX_ELEVATION_METERS: f32 = 8848.86;

fn convert_gebco_to_normal_elevation_rgba8(
    input_dir: &Path,
    output_dds: &Path,
) -> anyhow::Result<()> {
    println!(
        "Converting GEBCO dataset from {:?} to RGBA8 normal+elevation DDS...",
        input_dir
    );
    let start = Instant::now();

    let width = OUTPUT_SIZE.width as usize;
    let height = OUTPUT_SIZE.height as usize;
    let tile_w = width / 4; // 2048
    let tile_h = height / 2; // 2048

    let mut elev_grid = vec![0.0f32; width * height];

    for tile in &GEBCO_TILES {
        let tile_path = input_dir.join(tile.filename);
        println!("Loading GEBCO tile: {:?}", tile_path);
        let tile_start = Instant::now();
        let file = File::open(&tile_path)?;
        let mut decoder = tiff::decoder::Decoder::new(file)?;
        decoder = decoder.with_limits(tiff::decoder::Limits::unlimited());
        let (sw_u32, sh_u32) = decoder.dimensions()?;
        let data = match decoder.read_image()? {
            tiff::decoder::DecodingResult::I16(d) => d,
            _ => anyhow::bail!("Expected I16 GEBCO data in {:?}", tile_path),
        };
        let sw = sw_u32 as usize;
        let sh = sh_u32 as usize;

        // Downsample into a temporary tile buffer using Rayon
        let mut tile_elev = vec![0.0f32; tile_w * tile_h];
        tile_elev
            .par_chunks_exact_mut(tile_w)
            .enumerate()
            .for_each(|(ty, row)| {
                let v = (ty as f32 + 0.5) / tile_h as f32;
                let sy_f = v * sh as f32 - 0.5;
                let y0 = (sy_f.floor() as usize).clamp(0, sh - 1);
                let y1 = (y0 + 1).min(sh - 1);
                let fy = (sy_f - y0 as f32).clamp(0.0, 1.0);

                for (tx, out_val) in row.iter_mut().enumerate() {
                    let u = (tx as f32 + 0.5) / tile_w as f32;
                    let sx_f = u * sw as f32 - 0.5;
                    let x0 = (sx_f.floor() as usize).clamp(0, sw - 1);
                    let x1 = (x0 + 1).min(sw - 1);
                    let fx = (sx_f - x0 as f32).clamp(0.0, 1.0);

                    let s00 = data[y0 * sw + x0] as f32;
                    let s10 = data[y0 * sw + x1] as f32;
                    let s01 = data[y1 * sw + x0] as f32;
                    let s11 = data[y1 * sw + x1] as f32;

                    let top = s00 * (1.0 - fx) + s10 * fx;
                    let bot = s01 * (1.0 - fx) + s11 * fx;
                    let elev = top * (1.0 - fy) + bot * fy;

                    *out_val = if elev > 0.0 { elev } else { 0.0 };
                }
            });

        // Copy tile rows into the global elevation grid
        for ty in 0..tile_h {
            let gy = tile.row * tile_h + ty;
            let gx_start = tile.col * tile_w;
            let src_start = ty * tile_w;
            let dst_start = gy * width + gx_start;
            elev_grid[dst_start..dst_start + tile_w]
                .copy_from_slice(&tile_elev[src_start..src_start + tile_w]);
        }

        println!(
            "Tile {} loaded and resampled in {:?}",
            tile.filename,
            tile_start.elapsed()
        );
    }

    println!(
        "All tiles loaded in {:?}. Computing 3D normals in parallel with Rayon...",
        start.elapsed()
    );

    let height_scale = 0.03f32;

    // Helper closure to calculate displaced position on the sphere for pixel (px, py)
    let get_displaced = |px: usize, py: usize| -> Vec3 {
        let u = (px as f32 + 0.5) / width as f32;
        let v = (py as f32 + 0.5) / height as f32;
        let p = uv_to_sphere_point(u, v);
        let elev_m = elev_grid[py * width + px];
        let elev_norm = (elev_m / MAX_ELEVATION_METERS).clamp(0.0, 1.0);
        p * (1.0 + elev_norm * height_scale)
    };

    // Allocate output RGBA8 buffer (4 bytes per pixel)
    let mut rgba_bytes = vec![0u8; width * height * 4];

    // Process each scanline in parallel
    rgba_bytes
        .par_chunks_exact_mut(width * 4)
        .enumerate()
        .for_each(|(py, row)| {
            let py_north = if py > 0 { py - 1 } else { 0 };
            let py_south = if py < height - 1 { py + 1 } else { height - 1 };

            for px in 0..width {
                let px_west = (px + 1) % width;
                let px_east = (px + width - 1) % width;

                let u = (px as f32 + 0.5) / width as f32;
                let v = (py as f32 + 0.5) / height as f32;
                let sphere_p = uv_to_sphere_point(u, v);

                let elev_m = elev_grid[py * width + px];
                let alpha_u8 = if elev_m <= 0.0 {
                    0u8
                } else {
                    let elev_norm = (elev_m / MAX_ELEVATION_METERS).clamp(0.0, 1.0);
                    (elev_norm * 254.0).round() as u8 + 1
                };

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
        "Normals computed in {:?}! Generating mipmaps...",
        start.elapsed()
    );
    let mip_start = Instant::now();
    let mip_levels = generate_mipmaps_normal_elevation(rgba_bytes, width as u32, height as u32);
    let total_bytes: usize = mip_levels.iter().map(|m| m.len()).sum();
    println!(
        "Generated {} mip levels in {:?}! Writing DDS to {:?}...",
        mip_levels.len(),
        mip_start.elapsed(),
        output_dds
    );

    write_dds_file(output_dds, width as u32, height as u32, &mip_levels)?;

    println!(
        "Done in {:?}! Saved {:?} ({}x{}, {} bytes across {} mip levels)",
        start.elapsed(),
        output_dds,
        width,
        height,
        total_bytes + 128,
        mip_levels.len()
    );

    Ok(())
}

fn convert_image_to_rgba8(input_path: &Path, output_dds: &Path) -> anyhow::Result<()> {
    println!("Converting {:?} to RGBA8 DDS...", input_path);
    let start = Instant::now();

    let mut reader = image::io::Reader::open(input_path)?;
    reader.no_limits();
    let img = reader.decode()?;
    let (width, height) = (img.width(), img.height());

    let (target_w, target_h, rgba_bytes) =
        if width > OUTPUT_SIZE.width || height > OUTPUT_SIZE.height {
            println!(
                "Image dimensions ({}x{}) exceed GPU limit (8192x4096). Resampling with Rayon...",
                width, height
            );
            let resample_start = Instant::now();
            let target_w = OUTPUT_SIZE.width;
            let target_h = OUTPUT_SIZE.height;
            let rgb = img.to_rgb8();
            let src_raw = rgb.as_raw();
            let sw = width as usize;
            let sh = height as usize;
            let tw = target_w as usize;
            let th = target_h as usize;

            let mut out_bytes = vec![0u8; tw * th * 4];
            out_bytes
                .par_chunks_exact_mut(tw * 4)
                .enumerate()
                .for_each(|(dy, row)| {
                    let v = (dy as f32 + 0.5) / th as f32;
                    let sy_f = v * sh as f32 - 0.5;
                    let y0 = (sy_f.floor() as usize).clamp(0, sh - 1);
                    let y1 = (y0 + 1).min(sh - 1);
                    let fy = (sy_f - y0 as f32).clamp(0.0, 1.0);

                    for dx in 0..tw {
                        let u = (dx as f32 + 0.5) / tw as f32;
                        let sx_f = u * sw as f32 - 0.5;
                        let x0 = (sx_f.floor() as usize).clamp(0, sw - 1);
                        let x1 = (x0 + 1) % sw;
                        let fx = (sx_f - x0 as f32).clamp(0.0, 1.0);

                        let idx00 = (y0 * sw + x0) * 3;
                        let idx10 = (y0 * sw + x1) * 3;
                        let idx01 = (y1 * sw + x0) * 3;
                        let idx11 = (y1 * sw + x1) * 3;

                        let off = dx * 4;
                        for c in 0..3 {
                            let c00 = src_raw[idx00 + c] as f32;
                            let c10 = src_raw[idx10 + c] as f32;
                            let c01 = src_raw[idx01 + c] as f32;
                            let c11 = src_raw[idx11 + c] as f32;

                            let top = c00 * (1.0 - fx) + c10 * fx;
                            let bot = c01 * (1.0 - fx) + c11 * fx;
                            let val = top * (1.0 - fy) + bot * fy;
                            row[off + c] = val.round() as u8;
                        }
                        row[off + 3] = 255;
                    }
                });
            println!("Resampled in {:?}", resample_start.elapsed());
            (target_w, target_h, out_bytes)
        } else {
            let rgba = img.into_rgba8();
            (width, height, rgba.into_raw())
        };

    println!("Generating diffuse mipmaps...");
    let mip_start = Instant::now();
    let mip_levels = generate_mipmaps_diffuse(rgba_bytes, target_w, target_h);
    let total_bytes: usize = mip_levels.iter().map(|m| m.len()).sum();
    println!(
        "Generated {} mip levels in {:?}! Writing DDS to {:?}...",
        mip_levels.len(),
        mip_start.elapsed(),
        output_dds
    );

    write_dds_file(output_dds, target_w, target_h, &mip_levels)?;

    println!(
        "Done in {:?}! Saved {:?} ({}x{}, {} bytes across {} mip levels)",
        start.elapsed(),
        output_dds,
        target_w,
        target_h,
        total_bytes + 128,
        mip_levels.len()
    );

    Ok(())
}

struct Size {
    width: u32,
    height: u32,
}

const OUTPUT_SIZE: Size = Size {
    width: 8192,
    height: 4096,
};

fn main() -> anyhow::Result<()> {
    let elevation_input = Path::new("res/in/gebco_2026_geotiff");
    let elevation_output = &format!(
        "res/earth/elevation_{}x{}.dds",
        OUTPUT_SIZE.width, OUTPUT_SIZE.height
    );
    let elevation_output = Path::new(elevation_output);

    if elevation_input.exists() {
        convert_gebco_to_normal_elevation_rgba8(elevation_input, elevation_output)?;
    } else {
        println!("{} is missing", elevation_input.to_str().unwrap());
    }

    let diffuse_input = Path::new("res/in/diffuse.tif");
    let diffuse_output = &format!(
        "res/earth/diffuse_{}x{}.dds",
        OUTPUT_SIZE.width, OUTPUT_SIZE.height
    );
    let diffuse_output = Path::new(&diffuse_output);

    if diffuse_input.exists() {
        convert_image_to_rgba8(diffuse_input, diffuse_output)?;
    } else {
        println!("{} is missing", diffuse_input.to_str().unwrap());
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

        assert!(
            n.dot(light_from_west) > n.dot(light_from_east),
            "West slope must face West!"
        );
    }

    #[test]
    fn test_compute_mip_count() {
        assert_eq!(compute_mip_count(1, 1), 1);
        assert_eq!(compute_mip_count(2, 1), 2);
        assert_eq!(compute_mip_count(4, 2), 3);
        assert_eq!(compute_mip_count(4096, 2048), 13);
        assert_eq!(compute_mip_count(8192, 4096), 14);
    }

    #[test]
    fn test_generate_mipmaps_diffuse() {
        // 2x2 image with 4 distinct RGBA values
        let level0 = vec![
            10, 20, 30, 40, // (0,0)
            50, 60, 70, 80, // (1,0)
            90, 100, 110, 120, // (0,1)
            130, 140, 150, 160, // (1,1)
        ];

        let mips = generate_mipmaps_diffuse(level0.clone(), 2, 2);
        assert_eq!(mips.len(), 2);
        assert_eq!(mips[0], level0);

        // Level 1 is 1x1: each channel is average of 4 pixels
        // R: (10 + 50 + 90 + 130 + 2) / 4 = 282 / 4 = 70
        // G: (20 + 60 + 100 + 140 + 2) / 4 = 322 / 4 = 80
        // B: (30 + 70 + 110 + 150 + 2) / 4 = 362 / 4 = 90
        // A: (40 + 80 + 120 + 160 + 2) / 4 = 402 / 4 = 100
        assert_eq!(mips[1], vec![70, 80, 90, 100]);
    }

    #[test]
    fn test_generate_mipmaps_normal_elevation() {
        // 2x2 image: all water with unperturbed normal pointing in +Z direction
        // Packed normal (0, 0, 1) -> (128, 128, 255), Alpha = 0 (water)
        let normal_z = Vec3::Z;
        let nx = ((normal_z.x * 0.5 + 0.5) * 255.0).round() as u8;
        let ny = ((normal_z.y * 0.5 + 0.5) * 255.0).round() as u8;
        let nz = ((normal_z.z * 0.5 + 0.5) * 255.0).round() as u8;

        let level0 = vec![nx, ny, nz, 0, nx, ny, nz, 0, nx, ny, nz, 0, nx, ny, nz, 0];

        let mips = generate_mipmaps_normal_elevation(level0.clone(), 2, 2);
        assert_eq!(mips.len(), 2);
        assert_eq!(mips[1].len(), 4);
        assert_eq!(mips[1][3], 0); // Still water

        let unpacked_n = Vec3::new(
            (mips[1][0] as f32 / 255.0) * 2.0 - 1.0,
            (mips[1][1] as f32 / 255.0) * 2.0 - 1.0,
            (mips[1][2] as f32 / 255.0) * 2.0 - 1.0,
        )
        .normalize();

        assert!(unpacked_n.dot(Vec3::Z) > 0.999);
    }
}
