use crate::{
    assets::{self, ImageAsset, Material, Mesh, Model, Vertex},
    geo,
};
use glam::{Vec2, Vec3};
use rayon::prelude::*;

#[allow(unused)]
pub struct Earth {
    pub resolution: usize,
    pub subdivisions: usize,
    pub model: Model,
    pub elevation: ImageAsset,
}

impl Earth {
    pub fn load(
        resolution: usize,
        subdivisions: usize,
        elevation_filename: &str,
        diffuse_filename: &str,
    ) -> anyhow::Result<Earth> {
        assert!(resolution >= 2, "resolution must be at least 2"); // TODO return an error instead
        assert!(subdivisions >= 1, "subdivisions must be at least 1");
        assert!(subdivisions % 2 == 0, "subdivisions must be even");

        let (elevation, (meshes, materials)) = rayon::join(
            || create_elevation(elevation_filename),
            || {
                rayon::join(
                    || create_meshes(resolution + 1, subdivisions),
                    || create_material(diffuse_filename),
                )
            },
        );

        let elevation = elevation?;
        let materials = materials?;

        let result = Earth {
            resolution: resolution + 1,
            subdivisions,
            elevation,
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
    CUBE_FACE_NORMALS
        .par_iter()
        .flat_map(|&normal| {
            (0..subdivisions).into_par_iter().flat_map(move |part_y| {
                (0..subdivisions).into_par_iter().map(move |part_x| {
                    create_face_part(normal, part_x, part_y, subdivisions, resolution, 0)
                })
            })
        })
        .collect()
}

fn create_face_part(
    normal: Vec3,
    part_x: usize,
    part_y: usize,
    subdivisions: usize,
    resolution: usize,
    material_index: usize,
) -> Mesh {
    let axis_u = Vec3::new(normal.y, normal.z, normal.x);
    let axis_v = normal.cross(axis_u);
    let to_cube = |t: Vec2| normal + axis_u * (2.0 * t.x - 1.0) + axis_v * (2.0 * t.y - 1.0);

    // Longitude (u) of the patch center, used as a reference to unwrap the vertex UVs.
    let center_t = (Vec2::new(part_x as f32, part_y as f32) + 0.5) / subdivisions as f32;
    let center_u = geo::sphere_to_uv(to_sphere_point(to_cube(center_t))).x;

    let mut vertices: Vec<Vertex> = {
        let vertices_count = resolution * resolution;
        Vec::with_capacity(vertices_count)
    };

    for y in 0..resolution {
        for x in 0..resolution {
            let local_t = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;
            let t = (Vec2::new(part_x as f32, part_y as f32) + local_t) / subdivisions as f32;

            let sphere_position = to_sphere_point(to_cube(t));

            let vertex = Vertex {
                position: sphere_position.into(),
                tex_coords: patch_uv(sphere_position, center_u).into(),
            };
            vertices.push(vertex);
        }
    }

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

    Mesh::with_lods(
        format!("cube_face_{normal:?}_{part_x}_{part_y}"),
        vertices,
        all_indices,
        lod_ranges,
        material_index,
    )
}

fn generate_lod_indices(resolution: usize, step: usize) -> Vec<u32> {
    let mut indices = Vec::new();
    let mut y = 0;
    while y < resolution - 1 {
        let next_y = (y + step).min(resolution - 1);
        let mut x = 0;
        while x < resolution - 1 {
            let next_x = (x + step).min(resolution - 1);

            let top_left = (y * resolution + x) as u32;
            let top_right = (y * resolution + next_x) as u32;
            let bottom_left = (next_y * resolution + x) as u32;
            let bottom_right = (next_y * resolution + next_x) as u32;

            // First triangle: top_left -> bottom_right -> bottom_left
            indices.push(top_left);
            indices.push(bottom_right);
            indices.push(bottom_left);

            // Second triangle: top_left -> top_right -> bottom_right
            indices.push(top_left);
            indices.push(top_right);
            indices.push(bottom_right);

            x = next_x;
        }
        y = next_y;
    }
    indices
}

fn patch_uv(sphere_position: Vec3, center_u: f32) -> Vec2 {
    let uv = geo::sphere_to_uv(sphere_position);
    let at_pole = sphere_position.x.abs() + sphere_position.z.abs() < 1e-6;
    let u = if at_pole {
        center_u
    } else {
        let du = uv.x - center_u;
        center_u + (du - du.round())
    };
    Vec2::new(u, uv.y)
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
