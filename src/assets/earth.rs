use crate::{
    assets::{self, ImageAsset, Material, MaterialDiffuse, Mesh, Model, Vertex},
    geo,
};
use glam::{Vec2, Vec3};

#[allow(unused)]
pub struct Earth {
    resolution: usize,
    pub model: Model,
    pub elevation: ImageAsset,
}

impl Earth {
    pub async fn load(
        resolution: usize,
        elevation_filename: &str,
        diffuse_filename: &str,
    ) -> anyhow::Result<Earth> {
        assert!(resolution >= 2, "resolution must be at least 2"); // TODO return an error instead
        let elevation = create_elevation(elevation_filename).await?;

        let result = Earth {
            resolution: resolution,
            model: Model {
                meshes: create_meshes(resolution),
                materials: create_material(diffuse_filename).await?,
            },
            elevation: elevation,
        };
        Ok(result)
    }
}

async fn create_elevation(filename: &str) -> anyhow::Result<ImageAsset> {
    let elevation_image: ImageAsset = assets::load_image_16(filename).await?;
    Ok(elevation_image)
}

async fn create_material(filename: &str) -> anyhow::Result<Vec<Material>> {
    let elevation: ImageAsset = assets::load_image(filename).await?;
    Ok(vec![Material {
        name: "earth_elevation".to_string(),
        diffuse: MaterialDiffuse::Texture(elevation),
    }])
}

// fn create_materials() -> Vec<Material> {
//     CUBE_FACE_MATERIALS
//         .map(|(name, color)| Material {
//             name: name.to_string(),
//             diffuse: MaterialDiffuse::Color(color),
//         })
//         .into()
// }

fn create_meshes(resolution: usize) -> Vec<Mesh> {
    CUBE_FACE_NORMALS
        .iter()
        .map(|&normal| create_face(normal, resolution, 0))
        .collect()
}

fn create_face(normal: Vec3, resolution: usize, material_index: usize) -> Mesh {
    let axis_u = Vec3::new(normal.y, normal.z, normal.x);
    let axis_v = normal.cross(axis_u);

    let mut vertices: Vec<Vertex> = {
        let vertices_count = resolution * resolution;
        Vec::with_capacity(vertices_count)
    };
    let mut triangles: Vec<u32> = {
        let triangles_count = (resolution - 1) * (resolution - 1) * 6;
        Vec::with_capacity(triangles_count)
    };

    for y in 0..resolution {
        for x in 0..resolution {
            let t = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;

            let cube_position = normal + axis_u * (2.0 * t.x - 1.0) + axis_v * (2.0 * t.y - 1.0);
            let sphere_position = to_sphere_point(cube_position);

            let vertex = Vertex {
                position: sphere_position.into(),
                normal: sphere_position.into(),
                tex_coords: geo::sphere_to_uv(sphere_position).into(),
            };
            vertices.push(vertex);

            let vertex_index = x + y * resolution;
            if x != resolution - 1 && y != resolution - 1 {
                triangles.push(vertex_index as u32);
                triangles.push((vertex_index + resolution + 1) as u32);
                triangles.push((vertex_index + resolution) as u32);
                triangles.push(vertex_index as u32);
                triangles.push((vertex_index + 1) as u32);
                triangles.push((vertex_index + resolution + 1) as u32);
            }
        }
    }

    Mesh {
        name: "cube_face".to_string(),
        vertices,
        indices: triangles,
        material_index,
    }
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
