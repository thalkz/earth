use crate::assets::{self, ImageAsset, Material, Mesh, Model, Vertex};
use glam::{Vec2, Vec3};
use rayon::prelude::*;
use std::ops::Range;

#[allow(unused)]
pub struct Earth {
    pub resolution: usize,
    pub subdivisions: usize,
    pub model: Model,
    pub elevation: ImageAsset,
    pub ocean: ImageAsset,
    pub water_normal_1: ImageAsset,
    pub water_normal_2: ImageAsset,
}

impl Earth {
    pub fn load(
        resolution: usize,
        subdivisions: usize,
        elevation_filename: &str,
        diffuse_filename: &str,
    ) -> anyhow::Result<Earth> {
        anyhow::ensure!(resolution >= 2, "resolution must be at least 2");
        anyhow::ensure!(subdivisions >= 1, "subdivisions must be at least 1");

        let (((elevation, ocean), (water_normal_1, water_normal_2)), (meshes, materials)) =
            rayon::join(
                || {
                    rayon::join(
                        || {
                            rayon::join(
                                || create_elevation(elevation_filename),
                                || assets::load_dds_image("earth/ocean_cube_1024x1024.dds"),
                            )
                        },
                        || {
                            rayon::join(
                                || assets::load_dds_image("earth/water_normal_1.dds"),
                                || assets::load_dds_image("earth/water_normal_2.dds"),
                            )
                        },
                    )
                },
                || {
                    rayon::join(
                        || create_meshes(resolution + 1, subdivisions),
                        || create_material(diffuse_filename),
                    )
                },
            );

        let elevation = elevation?;
        let ocean = ocean?;
        let water_normal_1 = water_normal_1?;
        let water_normal_2 = water_normal_2?;
        let materials = materials?;

        let result = Earth {
            resolution: resolution + 1,
            subdivisions,
            elevation,
            ocean,
            water_normal_1,
            water_normal_2,
            model: Model { meshes, materials },
        };
        Ok(result)
    }
}

fn create_elevation(filename: &str) -> anyhow::Result<ImageAsset> {
    let elevation_image: ImageAsset = assets::load_dds_image(filename)?;
    Ok(elevation_image)
}

fn create_material(filename: &str) -> anyhow::Result<Vec<Material>> {
    let diffuse: ImageAsset = assets::load_dds_image(filename)?;
    Ok(vec![Material {
        name: "earth_elevation".to_string(),
        diffuse,
    }])
}

fn create_meshes(resolution: usize, subdivisions: usize) -> Vec<Mesh> {
    let (shared_indices, lod_ranges) = generate_patch_indices(resolution);

    CUBE_FACE_NORMALS
        .par_iter()
        .flat_map(|&normal| {
            let shared_indices = &shared_indices;
            let lod_ranges = &lod_ranges;
            (0..subdivisions).into_par_iter().flat_map(move |part_y| {
                (0..subdivisions).into_par_iter().map(move |part_x| {
                    create_face_part(
                        normal,
                        part_x,
                        part_y,
                        subdivisions,
                        resolution,
                        0,
                        shared_indices.clone(),
                        lod_ranges.clone(),
                    )
                })
            })
        })
        .collect()
}

fn generate_patch_indices(resolution: usize) -> (Vec<u32>, Vec<Range<u32>>) {
    let mut all_indices = Vec::new();
    let mut lod_ranges = Vec::with_capacity(4);

    // LOD 0 (step 1), LOD 1 (step 2), LOD 2 (step 4), LOD 3 (step 8)
    for step in [1, 2, 4, 8] {
        let start = all_indices.len() as u32;
        let lod_indices = generate_lod_indices(resolution, step);
        all_indices.extend(lod_indices);
        let end = all_indices.len() as u32;
        lod_ranges.push(start..end);
    }

    (all_indices, lod_ranges)
}

fn create_face_part(
    normal: Vec3,
    part_x: usize,
    part_y: usize,
    subdivisions: usize,
    resolution: usize,
    material_index: usize,
    indices: Vec<u32>,
    lod_ranges: Vec<Range<u32>>,
) -> Mesh {
    let axis_u = Vec3::new(normal.y, normal.z, normal.x);
    let axis_v = normal.cross(axis_u);
    let to_cube = |t: Vec2| normal + axis_u * (2.0 * t.x - 1.0) + axis_v * (2.0 * t.y - 1.0);

    let mut vertices = Vec::with_capacity(resolution * resolution + 4 * resolution);

    // 1. Surface grid vertices (radius = 1.0)
    for y in 0..resolution {
        for x in 0..resolution {
            let local_t = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;
            let t = (Vec2::new(part_x as f32, part_y as f32) + local_t) / subdivisions as f32;
            let sphere_pos = to_sphere_point(to_cube(t));
            vertices.push(Vertex {
                position: sphere_pos.into(),
            });
        }
    }

    // 2. Skirt vertices along the 4 edges (radius = 0.98, ~127 km deep into Earth)
    const SKIRT_RADIUS_SCALE: f32 = 0.98;
    let y_max = (resolution - 1) * resolution;
    let x_max = resolution - 1;
    let skirt_indices = (0..resolution)
        .chain((0..resolution).map(|i| y_max + i))
        .chain((0..resolution).map(|i| i * resolution))
        .chain((0..resolution).map(|i| i * resolution + x_max));

    let skirt_vertices: Vec<Vertex> = skirt_indices
        .map(|idx| Vertex {
            position: (Vec3::from(vertices[idx].position) * SKIRT_RADIUS_SCALE).into(),
        })
        .collect();
    vertices.extend(skirt_vertices);

    Mesh::with_lods(
        format!("cube_face_{normal:?}_{part_x}_{part_y}"),
        vertices,
        indices,
        lod_ranges,
        material_index,
    )
}

fn generate_lod_indices(resolution: usize, step: usize) -> Vec<u32> {
    let mut indices = Vec::new();

    // 1. Surface grid triangles
    for y in (0..resolution - 1).step_by(step) {
        let next_y = (y + step).min(resolution - 1);
        for x in (0..resolution - 1).step_by(step) {
            let next_x = (x + step).min(resolution - 1);
            let tl = (y * resolution + x) as u32;
            let tr = (y * resolution + next_x) as u32;
            let bl = (next_y * resolution + x) as u32;
            let br = (next_y * resolution + next_x) as u32;
            indices.extend_from_slice(&[tl, br, bl, tl, tr, br]);
        }
    }

    // 2. Skirt vertical walls (parameterized: top_start, top_stride, skirt_start, flip)
    let r = resolution as u32;
    let edges = [
        (0, 1, r * r, false),              // Edge 0: y = 0
        ((r - 1) * r, 1, r * r + r, true), // Edge 1: y = resolution - 1
        (0, r, r * r + 2 * r, true),       // Edge 2: x = 0
        (r - 1, r, r * r + 3 * r, false),  // Edge 3: x = resolution - 1
    ];

    for (top_start, top_stride, skirt_start, flip) in edges {
        for i in (0..resolution - 1).step_by(step) {
            let next_i = (i + step).min(resolution - 1);
            let top_a = top_start + i as u32 * top_stride;
            let top_b = top_start + next_i as u32 * top_stride;
            let skirt_a = skirt_start + i as u32;
            let skirt_b = skirt_start + next_i as u32;

            let quad = if !flip {
                [top_a, skirt_a, skirt_b, top_a, skirt_b, top_b]
            } else {
                [top_b, skirt_b, skirt_a, top_b, skirt_a, top_a]
            };
            indices.extend_from_slice(&quad);
        }
    }

    indices
}

fn to_sphere_point(from: Vec3) -> Vec3 {
    let x2 = from.x * from.x;
    let y2 = from.y * from.y;
    let z2 = from.z * from.z;
    let x = from.x * (1.0 - (y2 + z2) / 2.0 + (y2 * z2) / 3.0).sqrt();
    let y = from.y * (1.0 - (x2 + z2) / 2.0 + (x2 * z2) / 3.0).sqrt();
    let z = from.z * (1.0 - (x2 + y2) / 2.0 + (x2 * y2) / 3.0).sqrt();
    Vec3 { x, y, z }
}

const CUBE_FACE_NORMALS: [Vec3; 6] = [
    Vec3::X,
    Vec3::Y,
    Vec3::Z,
    Vec3::NEG_X,
    Vec3::NEG_Y,
    Vec3::NEG_Z,
];
