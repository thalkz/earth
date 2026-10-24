use earth::{DDS_MAGIC, DdsHeader};
use glam::Vec3;
use rayon::prelude::*;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn write_dds_cubemap<P: AsRef<Path>>(
    path: P,
    face_size: u32,
    mip_levels_all_faces: &[Vec<u8>], // Total count: 6 * mip_count
) -> std::io::Result<()> {
    let mip_count_per_face = (mip_levels_all_faces.len() / 6) as u32;
    let header = DdsHeader::new_rgba8_cubemap(face_size, mip_count_per_face);
    let mut file = File::create(path)?;
    file.write_all(&DDS_MAGIC.to_le_bytes())?;
    file.write_all(bytemuck::bytes_of(&header))?;
    for level in mip_levels_all_faces {
        file.write_all(level)?;
    }
    Ok(())
}

fn cube_uv_to_direction(face: usize, u: f32, v: f32) -> Vec3 {
    // Face convention matching standard DirectX/wgpu Cubemap:
    // 0: +X, 1: -X, 2: +Y, 3: -Y, 4: +Z, 5: -Z
    match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        5 => Vec3::new(-u, -v, -1.0),
        _ => unreachable!(),
    }
    .normalize()
}

fn direction_to_equirect_uv(dir: Vec3) -> (f32, f32) {
    let lat = dir.y.clamp(-1.0, 1.0).asin();
    let lng = dir.x.atan2(-dir.z);
    let u = (0.5 - lng / (2.0 * PI)).rem_euclid(1.0);
    let v = (0.5 - lat / PI).clamp(0.0, 1.0);
    (u, v)
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

    const EQUIRECT_WIDTH: usize = 8192;
    const EQUIRECT_HEIGHT: usize = 4096;
    let width = EQUIRECT_WIDTH;
    let height = EQUIRECT_HEIGHT;
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
        "Normals computed in {:?}! Projecting into 6 cubemap faces...",
        start.elapsed()
    );
    let cube_start = Instant::now();
    let face_size = CUBE_FACE_SIZE;
    let mut all_cubemap_mips = Vec::new();

    for face in 0..6 {
        println!("Resampling face {}/6...", face + 1);
        let mut face_bytes = vec![0u8; (face_size * face_size * 4) as usize];
        face_bytes
            .par_chunks_exact_mut((face_size * 4) as usize)
            .enumerate()
            .for_each(|(fy, row)| {
                let v = ((fy as f32 + 0.5) / face_size as f32) * 2.0 - 1.0;
                for fx in 0..face_size {
                    let u = ((fx as f32 + 0.5) / face_size as f32) * 2.0 - 1.0;
                    let dir = cube_uv_to_direction(face, u, v);
                    let (eq_u, eq_v) = direction_to_equirect_uv(dir);

                    let sx_f = eq_u * width as f32 - 0.5;
                    let sy_f = eq_v * height as f32 - 0.5;

                    let x0 = (sx_f.floor() as usize).clamp(0, width - 1);
                    let x1 = (x0 + 1) % width;
                    let y0 = (sy_f.floor() as usize).clamp(0, height - 1);
                    let y1 = (y0 + 1).min(height - 1);

                    let fx_w = (sx_f - x0 as f32).clamp(0.0, 1.0);
                    let fy_w = (sy_f - y0 as f32).clamp(0.0, 1.0);

                    let off00 = (y0 * width + x0) * 4;
                    let off10 = (y0 * width + x1) * 4;
                    let off01 = (y1 * width + x0) * 4;
                    let off11 = (y1 * width + x1) * 4;

                    let off = (fx * 4) as usize;
                    for c in 0..4 {
                        let c00 = rgba_bytes[off00 + c] as f32;
                        let c10 = rgba_bytes[off10 + c] as f32;
                        let c01 = rgba_bytes[off01 + c] as f32;
                        let c11 = rgba_bytes[off11 + c] as f32;

                        let top = c00 * (1.0 - fx_w) + c10 * fx_w;
                        let bot = c01 * (1.0 - fx_w) + c11 * fx_w;
                        let val = top * (1.0 - fy_w) + bot * fy_w;
                        row[off + c] = val.round() as u8;
                    }
                }
            });

        let face_mips = generate_mipmaps_normal_elevation(face_bytes, face_size, face_size);
        all_cubemap_mips.extend(face_mips);
    }

    println!(
        "Cubemap faces generated in {:?}! Writing DDS to {:?}...",
        cube_start.elapsed(),
        output_dds
    );
    write_dds_cubemap(output_dds, face_size, &all_cubemap_mips)?;

    let total_bytes: usize = all_cubemap_mips.iter().map(|m| m.len()).sum();
    println!(
        "Done in {:?}! Saved {:?} (6 faces of {}x{}, {} bytes total)",
        start.elapsed(),
        output_dds,
        face_size,
        face_size,
        total_bytes + 128
    );

    Ok(())
}

fn convert_image_to_rgba8(input_path: &Path, output_dds: &Path) -> anyhow::Result<()> {
    println!("Converting {:?} to RGBA8 Cubemap DDS...", input_path);
    let start = Instant::now();

    let mut reader = image::io::Reader::open(input_path)?;
    reader.no_limits();
    let img = reader.decode()?;
    let rgb = img.to_rgb8();
    let src_w = rgb.width() as usize;
    let src_h = rgb.height() as usize;
    let src_raw = rgb.as_raw();

    println!("Projecting diffuse into 6 cubemap faces...");
    let cube_start = Instant::now();
    let face_size = CUBE_FACE_SIZE;
    let mut all_cubemap_mips = Vec::new();

    for face in 0..6 {
        println!("Resampling diffuse face {}/6...", face + 1);
        let mut face_bytes = vec![0u8; (face_size * face_size * 4) as usize];
        face_bytes
            .par_chunks_exact_mut((face_size * 4) as usize)
            .enumerate()
            .for_each(|(fy, row)| {
                let v = ((fy as f32 + 0.5) / face_size as f32) * 2.0 - 1.0;
                for fx in 0..face_size {
                    let u = ((fx as f32 + 0.5) / face_size as f32) * 2.0 - 1.0;
                    let dir = cube_uv_to_direction(face, u, v);
                    let (eq_u, eq_v) = direction_to_equirect_uv(dir);

                    let sx_f = eq_u * src_w as f32 - 0.5;
                    let sy_f = eq_v * src_h as f32 - 0.5;

                    let x0 = (sx_f.floor() as usize).clamp(0, src_w - 1);
                    let x1 = (x0 + 1) % src_w;
                    let y0 = (sy_f.floor() as usize).clamp(0, src_h - 1);
                    let y1 = (y0 + 1).min(src_h - 1);

                    let fx_w = (sx_f - x0 as f32).clamp(0.0, 1.0);
                    let fy_w = (sy_f - y0 as f32).clamp(0.0, 1.0);

                    let off00 = (y0 * src_w + x0) * 3;
                    let off10 = (y0 * src_w + x1) * 3;
                    let off01 = (y1 * src_w + x0) * 3;
                    let off11 = (y1 * src_w + x1) * 3;

                    let off = (fx * 4) as usize;
                    for c in 0..3 {
                        let c00 = src_raw[off00 + c] as f32;
                        let c10 = src_raw[off10 + c] as f32;
                        let c01 = src_raw[off01 + c] as f32;
                        let c11 = src_raw[off11 + c] as f32;

                        let top = c00 * (1.0 - fx_w) + c10 * fx_w;
                        let bot = c01 * (1.0 - fx_w) + c11 * fx_w;
                        let val = top * (1.0 - fy_w) + bot * fy_w;
                        row[off + c] = val.round() as u8;
                    }
                    row[off + 3] = 255;
                }
            });

        let face_mips = generate_mipmaps_diffuse(face_bytes, face_size, face_size);
        all_cubemap_mips.extend(face_mips);
    }

    println!(
        "Diffuse cubemap generated in {:?}! Writing DDS to {:?}...",
        cube_start.elapsed(),
        output_dds
    );
    write_dds_cubemap(output_dds, face_size, &all_cubemap_mips)?;

    let total_bytes: usize = all_cubemap_mips.iter().map(|m| m.len()).sum();
    println!(
        "Done in {:?}! Saved {:?} (6 faces of {}x{}, {} bytes total)",
        start.elapsed(),
        output_dds,
        face_size,
        face_size,
        total_bytes + 128
    );

    Ok(())
}

const CUBE_FACE_SIZE: u32 = 2048;

fn main() -> anyhow::Result<()> {
    let elevation_input = Path::new("res/in/gebco_2026_geotiff");
    let elevation_output = &format!(
        "res/earth/elevation_cube_{}x{}.dds",
        CUBE_FACE_SIZE, CUBE_FACE_SIZE
    );
    let elevation_output = Path::new(elevation_output);

    if elevation_input.exists() {
        convert_gebco_to_normal_elevation_rgba8(elevation_input, elevation_output)?;
    } else {
        println!("{} is missing", elevation_input.to_str().unwrap());
    }

    let diffuse_input = Path::new("res/in/diffuse.tif");
    let diffuse_output = &format!(
        "res/earth/diffuse_cube_{}x{}.dds",
        CUBE_FACE_SIZE, CUBE_FACE_SIZE
    );
    let diffuse_output = Path::new(&diffuse_output);

    if diffuse_input.exists() {
        convert_image_to_rgba8(diffuse_input, diffuse_output)?;
    } else {
        println!("{} is missing", diffuse_input.to_str().unwrap());
    }

    Ok(())
}
