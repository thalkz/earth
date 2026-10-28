use earth::{DDS_MAGIC, DdsHeader, parse_dds};
use glam::Vec3;
use rayon::prelude::*;
use std::f32::consts::PI;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn write_dds(path: &Path, header: &DdsHeader, mips: &[Vec<u8>]) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&DDS_MAGIC.to_le_bytes())?;
    file.write_all(bytemuck::bytes_of(header))?;
    for level in mips {
        file.write_all(level)?;
    }
    Ok(())
}

fn to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r as u16 * 31 + 127) / 255;
    let g6 = (g as u16 * 63 + 127) / 255;
    let b5 = (b as u16 * 31 + 127) / 255;
    (r5 << 11) | (g6 << 5) | b5
}

fn from_rgb565(c: u16) -> [i32; 3] {
    let r5 = ((c >> 11) & 31) as i32;
    let g6 = ((c >> 5) & 63) as i32;
    let b5 = (c & 31) as i32;
    [
        (r5 * 255 + 15) / 31,
        (g6 * 255 + 31) / 63,
        (b5 * 255 + 15) / 31,
    ]
}

fn encode_bc1_block(block_pixels: &[[u8; 3]; 16]) -> [u8; 8] {
    let mut max_dist = -1i32;
    let mut p_min = block_pixels[0];
    let mut p_max = block_pixels[0];

    for i in 0..16 {
        for j in (i + 1)..16 {
            let dr = block_pixels[i][0] as i32 - block_pixels[j][0] as i32;
            let dg = block_pixels[i][1] as i32 - block_pixels[j][1] as i32;
            let db = block_pixels[i][2] as i32 - block_pixels[j][2] as i32;
            let dist = 3 * dr * dr + 4 * dg * dg + 2 * db * db;
            if dist > max_dist {
                max_dist = dist;
                p_min = block_pixels[i];
                p_max = block_pixels[j];
            }
        }
    }

    let mut c0_565 = to_rgb565(p_max[0], p_max[1], p_max[2]);
    let mut c1_565 = to_rgb565(p_min[0], p_min[1], p_min[2]);

    if c0_565 < c1_565 {
        std::mem::swap(&mut c0_565, &mut c1_565);
    }

    let (pal, num_colors) = if c0_565 > c1_565 {
        let c0 = from_rgb565(c0_565);
        let c1 = from_rgb565(c1_565);
        let c2 = [
            (2 * c0[0] + c1[0] + 1) / 3,
            (2 * c0[1] + c1[1] + 1) / 3,
            (2 * c0[2] + c1[2] + 1) / 3,
        ];
        let c3 = [
            (c0[0] + 2 * c1[0] + 1) / 3,
            (c0[1] + 2 * c1[1] + 1) / 3,
            (c0[2] + 2 * c1[2] + 1) / 3,
        ];
        ([c0, c1, c2, c3], 4)
    } else {
        let c0 = from_rgb565(c0_565);
        ([c0, c0, c0, [0, 0, 0]], 1)
    };

    let mut indices: u32 = 0;
    for (i, p) in block_pixels.iter().enumerate() {
        let mut best_dist = i32::MAX;
        let mut best_idx = 0u32;
        let p_i = [p[0] as i32, p[1] as i32, p[2] as i32];

        for (k, pal_k) in pal.iter().enumerate().take(num_colors) {
            let dr = p_i[0] - pal_k[0];
            let dg = p_i[1] - pal_k[1];
            let db = p_i[2] - pal_k[2];
            let dist = 3 * dr * dr + 4 * dg * dg + 2 * db * db;
            if dist < best_dist {
                best_dist = dist;
                best_idx = k as u32;
            }
        }
        indices |= best_idx << (i * 2);
    }

    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&c0_565.to_le_bytes());
    out[2..4].copy_from_slice(&c1_565.to_le_bytes());
    out[4..8].copy_from_slice(&indices.to_le_bytes());
    out
}

fn compress_rgba8_to_bc1(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let blocks_x = width.div_ceil(4).max(1) as usize;
    let blocks_y = height.div_ceil(4).max(1) as usize;
    let mut out = vec![0u8; blocks_x * blocks_y * 8];

    out.par_chunks_exact_mut(blocks_x * 8)
        .enumerate()
        .for_each(|(by, row)| {
            for bx in 0..blocks_x {
                let mut block = [[0u8; 3]; 16];
                for py in 0..4 {
                    let y = (by * 4 + py).min(height as usize - 1);
                    for px in 0..4 {
                        let x = (bx * 4 + px).min(width as usize - 1);
                        let offset = (y * width as usize + x) * 4;
                        block[py * 4 + px] = [rgba[offset], rgba[offset + 1], rgba[offset + 2]];
                    }
                }
                let block_bytes = encode_bc1_block(&block);
                row[bx * 8..bx * 8 + 8].copy_from_slice(&block_bytes);
            }
        });

    out
}

fn cube_uv_to_direction(face: usize, u: f32, v: f32) -> Vec3 {
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

fn generate_mipmaps_normals(
    face: Option<usize>,
    level0: Vec<u8>,
    width: u32,
    height: u32,
) -> Vec<Vec<u8>> {
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

                    let v_sum = unpack_normal(p00_idx)
                        + unpack_normal(p10_idx)
                        + unpack_normal(p01_idx)
                        + unpack_normal(p11_idx);
                    let normal = if v_sum.length_squared() > 1e-12 {
                        v_sum.normalize()
                    } else if let Some(face_idx) = face {
                        let u = ((dx as f32 + 0.5) / dst_w as f32) * 2.0 - 1.0;
                        let v = ((dy as f32 + 0.5) / dst_h as f32) * 2.0 - 1.0;
                        cube_uv_to_direction(face_idx, u, v)
                    } else {
                        Vec3::Z
                    };

                    let nx = (normal.x * 0.5 + 0.5).clamp(0.0, 1.0);
                    let ny = (normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
                    let nz = (normal.z * 0.5 + 0.5).clamp(0.0, 1.0);

                    let avg_a = if face.is_some() {
                        let a_sum = prev_level[p00_idx + 3] as u32
                            + prev_level[p10_idx + 3] as u32
                            + prev_level[p01_idx + 3] as u32
                            + prev_level[p11_idx + 3] as u32;
                        ((a_sum + 2) / 4) as u8
                    } else {
                        255
                    };

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

const GEBCO_TILES: [[&str; 4]; 2] = [
    [
        "gebco_2026_n90.0_s0.0_w-180.0_e-90.0_geotiff.tif",
        "gebco_2026_n90.0_s0.0_w-90.0_e0.0_geotiff.tif",
        "gebco_2026_n90.0_s0.0_w0.0_e90.0_geotiff.tif",
        "gebco_2026_n90.0_s0.0_w90.0_e180.0_geotiff.tif",
    ],
    [
        "gebco_2026_n0.0_s-90.0_w-180.0_e-90.0_geotiff.tif",
        "gebco_2026_n0.0_s-90.0_w-90.0_e0.0_geotiff.tif",
        "gebco_2026_n0.0_s-90.0_w0.0_e90.0_geotiff.tif",
        "gebco_2026_n0.0_s-90.0_w90.0_e180.0_geotiff.tif",
    ],
];

const MAX_ELEVATION_METERS: f32 = 8848.86;
const OCEAN_COLOR_DEPTH_METERS: f32 = 5500.0;
const SHALLOW_WATER_COLOR: Vec3 = Vec3::new(0.10, 0.50, 0.78);
const DEEP_WATER_COLOR: Vec3 = Vec3::new(0.01, 0.05, 0.24);

fn convert_gebco_textures(
    input_dir: &Path,
    elevation_output: &Path,
    ocean_output: &Path,
) -> anyhow::Result<()> {
    println!(
        "Sampling GEBCO dataset from {:?} for elevation & ocean diffuse cubemaps...",
        input_dir
    );
    let start = Instant::now();

    let face_size = CUBE_FACE_SIZE;
    let stride = (face_size + 2) as usize; // 1-texel border around each face for seamless 3x3 Sobel normals
    let mut face_elevs = vec![vec![0.0f32; stride * stride]; 6];

    for (hemisphere_row, row_tiles) in GEBCO_TILES.iter().enumerate() {
        let hemisphere_name = if hemisphere_row == 0 {
            "Northern"
        } else {
            "Southern"
        };
        println!("Loading {hemisphere_name} hemisphere GEBCO tiles in parallel...");
        let hemi_start = Instant::now();

        let hemisphere_tiles: Vec<(usize, usize, Vec<i16>)> = row_tiles
            .par_iter()
            .map(|&filename| -> anyhow::Result<(usize, usize, Vec<i16>)> {
                let tile_path = input_dir.join(filename);
                let tile_start = Instant::now();
                let file = File::open(&tile_path)?;
                let mut decoder = tiff::decoder::Decoder::new(file)?;
                decoder = decoder.with_limits(tiff::decoder::Limits::unlimited());
                let (sw_u32, sh_u32) = decoder.dimensions()?;
                let data = match decoder.read_image()? {
                    tiff::decoder::DecodingResult::I16(d) => d,
                    _ => anyhow::bail!("Expected I16 GEBCO data in {:?}", tile_path),
                };
                println!(
                    "  Loaded {filename} ({sw_u32}x{sh_u32}) in {:?}",
                    tile_start.elapsed()
                );
                Ok((sw_u32 as usize, sh_u32 as usize, data))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let tw = hemisphere_tiles[0].0;
        let th = hemisphere_tiles[0].1;
        let global_w = tw * 4;

        let sample_hemisphere = |gx: usize, ly: usize| -> f32 {
            let col = (gx / tw).min(3);
            let tx = gx % tw;
            let (_, _, ref data) = hemisphere_tiles[col];
            data[ly * tw + tx] as f32
        };

        for (face, face_elev) in face_elevs.iter_mut().enumerate() {
            face_elev
                .par_chunks_exact_mut(stride)
                .enumerate()
                .for_each(|(gy, row)| {
                    let v = ((gy as f32 - 0.5) / face_size as f32) * 2.0 - 1.0;
                    for (gx, out_elev) in row.iter_mut().enumerate() {
                        let u = ((gx as f32 - 0.5) / face_size as f32) * 2.0 - 1.0;
                        let dir = cube_uv_to_direction(face, u, v);

                        let is_north = dir.y >= 0.0;
                        if (hemisphere_row == 0) != is_north {
                            continue;
                        }

                        let (eq_u, eq_v) = direction_to_equirect_uv(dir);
                        let local_v = if hemisphere_row == 0 {
                            (eq_v * 2.0).clamp(0.0, 1.0)
                        } else {
                            ((eq_v - 0.5) * 2.0).clamp(0.0, 1.0)
                        };

                        let sx_f = eq_u * global_w as f32 - 0.5;
                        let sy_f = local_v * th as f32 - 0.5;

                        let x0 = (sx_f.floor() as isize).rem_euclid(global_w as isize) as usize;
                        let x1 = (x0 + 1) % global_w;
                        let y0 = (sy_f.floor() as isize).clamp(0, th as isize - 1) as usize;
                        let y1 = (y0 + 1).min(th - 1);

                        let fx = (sx_f - sx_f.floor()).clamp(0.0, 1.0);
                        let fy = (sy_f - sy_f.floor()).clamp(0.0, 1.0);

                        let s00 = sample_hemisphere(x0, y0);
                        let s10 = sample_hemisphere(x1, y0);
                        let s01 = sample_hemisphere(x0, y1);
                        let s11 = sample_hemisphere(x1, y1);

                        let top = s00 * (1.0 - fx) + s10 * fx;
                        let bot = s01 * (1.0 - fx) + s11 * fx;
                        *out_elev = top * (1.0 - fy) + bot * fy;
                    }
                });
        }

        println!(
            "{hemisphere_name} hemisphere sampled directly into cubemap elevation grids in {:?}",
            hemi_start.elapsed()
        );
    }

    println!("All GEBCO tiles sampled in {:?}.", start.elapsed());

    if !elevation_output.exists() {
        let cube_start = Instant::now();
        let normal_height_scale = 0.03f32;
        let face_size_usize = face_size as usize;
        let mut all_cubemap_mips = Vec::new();

        for (face, face_elev) in face_elevs.iter().enumerate() {
            println!("Computing land normals and mipmaps for face {}/6...", face + 1);

            let get_displaced = |gx: usize, gy: usize| -> Vec3 {
                let u = ((gx as f32 - 0.5) / face_size as f32) * 2.0 - 1.0;
                let v = ((gy as f32 - 0.5) / face_size as f32) * 2.0 - 1.0;
                let p = cube_uv_to_direction(face, u, v);
                let elev_m = face_elev[gy * stride + gx];
                let elev_norm = (elev_m / MAX_ELEVATION_METERS).clamp(0.0, 1.0);
                p * (1.0 + elev_norm * normal_height_scale)
            };

            let mut face_bytes = vec![0u8; face_size_usize * face_size_usize * 4];
            face_bytes
                .par_chunks_exact_mut(face_size_usize * 4)
                .enumerate()
                .for_each(|(fy, row)| {
                    let gy = fy + 1;
                    let v = ((fy as f32 + 0.5) / face_size as f32) * 2.0 - 1.0;

                    for fx in 0..face_size_usize {
                        let gx = fx + 1;
                        let u = ((fx as f32 + 0.5) / face_size as f32) * 2.0 - 1.0;
                        let sphere_p = cube_uv_to_direction(face, u, v);

                        let elev_m = face_elev[gy * stride + gx];
                        let alpha_u8 = if elev_m <= 0.0 {
                            0u8
                        } else {
                            let elev_norm = (elev_m / MAX_ELEVATION_METERS).clamp(0.0, 1.0);
                            (elev_norm * 254.0).round() as u8 + 1
                        };

                        let normal = if alpha_u8 == 0 {
                            sphere_p
                        } else {
                            let s_nw = get_displaced(gx - 1, gy - 1);
                            let s_n = get_displaced(gx, gy - 1);
                            let s_ne = get_displaced(gx + 1, gy - 1);

                            let s_w = get_displaced(gx - 1, gy);
                            let s_e = get_displaced(gx + 1, gy);

                            let s_sw = get_displaced(gx - 1, gy + 1);
                            let s_s = get_displaced(gx, gy + 1);
                            let s_se = get_displaced(gx + 1, gy + 1);

                            let d_u = (s_ne - s_nw) + (s_e - s_w) * 2.0 + (s_se - s_sw);
                            let d_v = (s_sw - s_nw) + (s_s - s_n) * 2.0 + (s_se - s_ne);

                            let n = d_v.cross(d_u);
                            if n.length_squared() > 1e-20 {
                                let n = n.normalize();
                                if n.dot(sphere_p) < 0.0 { -n } else { n }
                            } else {
                                sphere_p
                            }
                        };

                        let nx = (normal.x * 0.5 + 0.5).clamp(0.0, 1.0);
                        let ny = (normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
                        let nz = (normal.z * 0.5 + 0.5).clamp(0.0, 1.0);

                        let off = fx * 4;
                        row[off] = (nx * 255.0).round() as u8;
                        row[off + 1] = (ny * 255.0).round() as u8;
                        row[off + 2] = (nz * 255.0).round() as u8;
                        row[off + 3] = alpha_u8;
                    }
                });

            let face_mips = generate_mipmaps_normals(Some(face), face_bytes, face_size, face_size);
            all_cubemap_mips.extend(face_mips);
        }

        println!(
            "Elevation cubemap generated in {:?}! Writing DDS to {:?}...",
            cube_start.elapsed(),
            elevation_output
        );
        let mip_count = (all_cubemap_mips.len() / 6) as u32;
        let header = DdsHeader::new_rgba8_cubemap(face_size, mip_count);
        write_dds(elevation_output, &header, &all_cubemap_mips)?;

        let total_bytes: usize = all_cubemap_mips.iter().map(|m| m.len()).sum();
        println!(
            "Saved {:?} (6 faces of {face_size}x{face_size}, {} bytes total)",
            elevation_output,
            total_bytes + 128
        );
    }

    if !ocean_output.exists() {
        let ocean_start = Instant::now();
        let ocean_size = OCEAN_CUBE_FACE_SIZE;
        let ocean_size_usize = ocean_size as usize;
        let downsample_factor = (face_size / ocean_size).max(1) as usize;
        let mut all_ocean_mips = Vec::new();

        println!(
            "Generating precomputed {ocean_size}x{ocean_size} ocean diffuse cubemap {:?}...",
            ocean_output
        );

        for face_elev in &face_elevs {
            let mut ocean_face_bytes = vec![0u8; ocean_size_usize * ocean_size_usize * 4];
            ocean_face_bytes
                .par_chunks_exact_mut(ocean_size_usize * 4)
                .enumerate()
                .for_each(|(oy, row)| {
                    for ox in 0..ocean_size_usize {
                        let mut depth_sum = 0.0f32;
                        for dy in 0..downsample_factor {
                            let gy = oy * downsample_factor + dy + 1;
                            for dx in 0..downsample_factor {
                                let gx = ox * downsample_factor + dx + 1;
                                depth_sum += face_elev[gy * stride + gx].min(0.0);
                            }
                        }
                        let avg_depth_m =
                            depth_sum / ((downsample_factor * downsample_factor) as f32);
                        let depth_t = (-avg_depth_m / OCEAN_COLOR_DEPTH_METERS).clamp(0.0, 1.0);
                        let color = SHALLOW_WATER_COLOR.lerp(DEEP_WATER_COLOR, depth_t);

                        let off = ox * 4;
                        row[off] = (color.x.clamp(0.0, 1.0) * 255.0).round() as u8;
                        row[off + 1] = (color.y.clamp(0.0, 1.0) * 255.0).round() as u8;
                        row[off + 2] = (color.z.clamp(0.0, 1.0) * 255.0).round() as u8;
                        row[off + 3] = 255;
                    }
                });

            let face_mips = generate_mipmaps_diffuse(ocean_face_bytes, ocean_size, ocean_size);
            all_ocean_mips.extend(face_mips);
        }

        let mip_count = (all_ocean_mips.len() / 6) as u32;
        let header = DdsHeader::new_rgba8_cubemap(ocean_size, mip_count);
        write_dds(ocean_output, &header, &all_ocean_mips)?;
        let total_bytes: usize = all_ocean_mips.iter().map(|m| m.len()).sum();
        println!(
            "Saved {:?} (6 faces of {ocean_size}x{ocean_size}, {} bytes total) in {:?}",
            ocean_output,
            total_bytes + 128,
            ocean_start.elapsed()
        );
    }

    Ok(())
}

fn convert_image_to_bc1(input_path: &Path, output_dds: &Path) -> anyhow::Result<()> {
    println!("Converting {:?} to BC1 Cubemap DDS...", input_path);
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
        let mut cur_w = face_size;
        let mut cur_h = face_size;
        for mip in face_mips {
            let bc1_mip = compress_rgba8_to_bc1(&mip, cur_w, cur_h);
            all_cubemap_mips.push(bc1_mip);
            cur_w = (cur_w / 2).max(1);
            cur_h = (cur_h / 2).max(1);
        }
    }

    println!(
        "Diffuse cubemap generated in {:?}! Writing BC1 DDS to {:?}...",
        cube_start.elapsed(),
        output_dds
    );
    let mip_count = (all_cubemap_mips.len() / 6) as u32;
    let header = DdsHeader::new_bc1_cubemap(face_size, mip_count);
    write_dds(output_dds, &header, &all_cubemap_mips)?;

    let total_bytes: usize = all_cubemap_mips.iter().map(|m| m.len()).sum();
    println!(
        "Done in {:?}! Saved {:?} (6 faces of {face_size}x{face_size}, {} bytes total)",
        start.elapsed(),
        output_dds,
        total_bytes + 128
    );

    Ok(())
}

fn generate_or_convert_water_normal(
    custom_inputs: &[&str],
    output_dds: &Path,
    seed: u32,
) -> anyhow::Result<()> {
    let start = Instant::now();

    for &input_str in custom_inputs {
        let input_path = Path::new(input_str);
        if input_path.exists() {
            println!("Converting custom water normal {:?} to 2D DDS...", input_path);
            let mut reader = image::io::Reader::open(input_path)?;
            reader.no_limits();
            let img = reader.decode()?.to_rgba8();
            let (w, h) = img.dimensions();
            let mips = generate_mipmaps_normals(None, img.into_raw(), w, h);
            let header = DdsHeader::new_rgba8_2d(w, h, mips.len() as u32);
            write_dds(output_dds, &header, &mips)?;
            println!("Saved {:?} ({w}x{h}) in {:?}", output_dds, start.elapsed());
            return Ok(());
        }
    }

    const WATER_NORMAL_SIZE: u32 = 512;
    let size = WATER_NORMAL_SIZE as usize;
    println!(
        "Generating seamless {size}x{size} ocean wave normal map {:?} (seed={seed})...",
        output_dds
    );

    let hash_f32 = |mut x: u32| -> f32 {
        x = ((x >> 16) ^ x).wrapping_mul(0x45d9f3b);
        x = ((x >> 16) ^ x).wrapping_mul(0x45d9f3b);
        x = (x >> 16) ^ x;
        (x as f32) / (u32::MAX as f32)
    };

    struct WaveMode {
        kx: f32,
        ky: f32,
        amp: f32,
        phase: f32,
        steepness: f32,
    }

    let mut modes = Vec::with_capacity(48);
    for i in 0..48u32 {
        let s = seed.wrapping_mul(7919).wrapping_add(i.wrapping_mul(104729));
        let band = i / 12;
        let (min_k, max_k, base_amp) = match band {
            0 => (2.0f32, 6.0f32, 0.28f32),
            1 => (6.0f32, 14.0f32, 0.14f32),
            2 => (14.0f32, 28.0f32, 0.065f32),
            _ => (28.0f32, 52.0f32, 0.028f32),
        };

        let angle = hash_f32(s) * 2.0 * PI;
        let k_mag = min_k + hash_f32(s.wrapping_add(1)) * (max_k - min_k);
        let mut kx = (k_mag * angle.cos()).round();
        let ky = (k_mag * angle.sin()).round();
        if kx == 0.0 && ky == 0.0 {
            kx = 2.0;
        }
        let actual_k = (kx * kx + ky * ky).sqrt();
        let amp =
            base_amp * (min_k / actual_k).powf(0.85) * (0.7 + 0.6 * hash_f32(s.wrapping_add(2)));
        let phase = hash_f32(s.wrapping_add(3)) * 2.0 * PI;
        let steepness = 0.25 + 0.2 * hash_f32(s.wrapping_add(4));

        modes.push(WaveMode {
            kx: kx * 2.0 * PI,
            ky: ky * 2.0 * PI,
            amp,
            phase,
            steepness,
        });
    }

    let mut rgba = vec![0u8; size * size * 4];
    rgba.par_chunks_exact_mut(size * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let v = (y as f32 + 0.5) / size as f32;
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32;

                let mut dh_du = 0.0f32;
                let mut dh_dv = 0.0f32;

                for m in &modes {
                    let theta = m.kx * u + m.ky * v + m.phase;
                    let warped = theta + m.steepness * theta.sin();
                    let d_warped = warped.cos() * (1.0 + m.steepness * theta.cos());
                    dh_du += m.amp * m.kx * d_warped;
                    dh_dv += m.amp * m.ky * d_warped;
                }

                let slope_scale = 0.12f32;
                let normal =
                    Vec3::new(-dh_du * slope_scale, -dh_dv * slope_scale, 1.0).normalize();

                let nx = (normal.x * 0.5 + 0.5).clamp(0.0, 1.0);
                let ny = (normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
                let nz = (normal.z * 0.5 + 0.5).clamp(0.0, 1.0);

                let off = x * 4;
                row[off] = (nx * 255.0).round() as u8;
                row[off + 1] = (ny * 255.0).round() as u8;
                row[off + 2] = (nz * 255.0).round() as u8;
                row[off + 3] = 255;
            }
        });

    let mips = generate_mipmaps_normals(None, rgba, WATER_NORMAL_SIZE, WATER_NORMAL_SIZE);
    let header = DdsHeader::new_rgba8_2d(WATER_NORMAL_SIZE, WATER_NORMAL_SIZE, mips.len() as u32);
    write_dds(output_dds, &header, &mips)?;
    println!(
        "Done in {:?}! Saved {:?} ({WATER_NORMAL_SIZE}x{WATER_NORMAL_SIZE}, {} mip levels)",
        start.elapsed(),
        output_dds,
        mips.len()
    );

    Ok(())
}

const CUBE_FACE_SIZE: u32 = 4096;
const OCEAN_CUBE_FACE_SIZE: u32 = 1024;
const MOBILE_FACE_SIZE: u32 = 1024;

fn extract_downsampled_dds(
    src_dds: &Path,
    dst_dds: &Path,
    target_face_size: u32,
    is_bc1: bool,
) -> anyhow::Result<()> {
    println!(
        "Extracting {target_face_size}x{target_face_size} mobile DDS from {:?} -> {:?}...",
        src_dds, dst_dds
    );
    if let Some(parent) = dst_dds.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = fs::read(src_dds)?;
    let asset = parse_dds(bytes)?;
    anyhow::ensure!(asset.is_cubemap, "Expected cubemap DDS in {:?}", src_dds);

    let skip_mips = (asset.width / target_face_size).ilog2() as usize;
    let all_mips: Vec<&[u8]> = asset.mip_levels().collect();
    let mips_per_face = all_mips.len() / 6;

    let mut extracted_mips = Vec::new();
    for face in 0..6 {
        for mip in skip_mips..mips_per_face {
            extracted_mips.push(all_mips[face * mips_per_face + mip].to_vec());
        }
    }

    let mip_count = (extracted_mips.len() / 6) as u32;
    let header = if is_bc1 {
        DdsHeader::new_bc1_cubemap(target_face_size, mip_count)
    } else {
        DdsHeader::new_rgba8_cubemap(target_face_size, mip_count)
    };
    write_dds(dst_dds, &header, &extracted_mips)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let elevation_input = Path::new("res/in/gebco_2026_geotiff");
    let elevation_output_str = format!(
        "res/earth/elevation_cube_{CUBE_FACE_SIZE}x{CUBE_FACE_SIZE}.dds"
    );
    let elevation_output = Path::new(&elevation_output_str);

    let ocean_output_str = format!(
        "res/earth/ocean_cube_{OCEAN_CUBE_FACE_SIZE}x{OCEAN_CUBE_FACE_SIZE}.dds"
    );
    let ocean_output = Path::new(&ocean_output_str);

    if elevation_input.exists() && (!elevation_output.exists() || !ocean_output.exists()) {
        convert_gebco_textures(elevation_input, elevation_output, ocean_output)?;
    } else if elevation_output.exists() && ocean_output.exists() {
        println!(
            "{} and {} already exist, skipping GEBCO conversion",
            elevation_output.display(),
            ocean_output.display()
        );
    } else {
        println!("{} is missing", elevation_input.display());
    }

    let diffuse_input = Path::new("res/in/diffuse.tif");
    let diffuse_output_str = format!(
        "res/earth/diffuse_cube_{CUBE_FACE_SIZE}x{CUBE_FACE_SIZE}.dds"
    );
    let diffuse_output = Path::new(&diffuse_output_str);

    if diffuse_input.exists() && !diffuse_output.exists() {
        convert_image_to_bc1(diffuse_input, diffuse_output)?;
    } else if diffuse_output.exists() {
        println!(
            "{} already exists, skipping diffuse conversion",
            diffuse_output.display()
        );
    } else {
        println!("{} is missing", diffuse_input.display());
    }

    let water_normal_1_output = Path::new("res/earth/water_normal_1.dds");
    if !water_normal_1_output.exists() {
        generate_or_convert_water_normal(
            &["res/in/water_normal_1.tif", "res/in/water_normal_1.jpg"],
            water_normal_1_output,
            1,
        )?;
    } else {
        println!(
            "{} already exists, skipping water_normal_1 generation",
            water_normal_1_output.display()
        );
    }

    let water_normal_2_output = Path::new("res/earth/water_normal_2.dds");
    if !water_normal_2_output.exists() {
        generate_or_convert_water_normal(
            &["res/in/water_normal_2.tif", "res/in/water_normal_2.jpg"],
            water_normal_2_output,
            2,
        )?;
    } else {
        println!(
            "{} already exists, skipping water_normal_2 generation",
            water_normal_2_output.display()
        );
    }

    let mobile_elev = Path::new("android/app/src/main/assets/earth/elevation_cube_1024x1024.dds");
    if elevation_output.exists() && !mobile_elev.exists() {
        extract_downsampled_dds(elevation_output, mobile_elev, MOBILE_FACE_SIZE, false)?;
    }

    let mobile_diff = Path::new("android/app/src/main/assets/earth/diffuse_cube_1024x1024.dds");
    if diffuse_output.exists() && !mobile_diff.exists() {
        extract_downsampled_dds(diffuse_output, mobile_diff, MOBILE_FACE_SIZE, true)?;
    }

    for (src, dst) in [
        (
            ocean_output,
            Path::new("android/app/src/main/assets/earth/ocean_cube_1024x1024.dds"),
        ),
        (
            water_normal_1_output,
            Path::new("android/app/src/main/assets/earth/water_normal_1.dds"),
        ),
        (
            water_normal_2_output,
            Path::new("android/app/src/main/assets/earth/water_normal_2.dds"),
        ),
    ] {
        if src.exists() && !dst.exists() {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src, dst)?;
        }
    }

    Ok(())
}
