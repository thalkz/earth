use glam;

use crate::assets::model;

const CUBE_FACE_NORMALS: [glam::Vec3; 6] = [
    glam::Vec3::X,
    glam::Vec3::Y,
    glam::Vec3::Z,
    glam::Vec3::NEG_X,
    glam::Vec3::NEG_Y,
    glam::Vec3::NEG_Z,
];

const CUBE_FACE_MATERIALS: [(&str, [f32; 4]); 6] = [
    ("red", [1.0, 0.0, 0.0, 1.0]),
    ("green", [0.0, 1.0, 0.0, 1.0]),
    ("blue", [0.0, 0.0, 1.0, 1.0]),
    ("yellow", [0.5, 0.5, 0.0, 1.0]),
    ("cyan", [0.0, 0.5, 0.5, 1.0]),
    ("purple", [0.5, 0.0, 0.5, 1.0]),
];

pub fn load_cube(resolution: usize) -> model::Model {
    assert!(resolution >= 2, "resolution must be at least 2");

    let meshes: Vec<model::Mesh> = CUBE_FACE_NORMALS
        .iter()
        .enumerate()
        .map(|(i, &normal)| create_face(normal, resolution, i))
        .collect();
    let materials = CUBE_FACE_MATERIALS
        .map(|(name, color)| model::Material {
            name: name.to_string(),
            diffuse: model::MaterialDiffuse::Color(color),
        })
        .into();

    model::Model {
        meshes: meshes,
        materials: materials,
    }
}

fn create_face(normal: glam::Vec3, resolution: usize, material_index: usize) -> model::Mesh {
    let axis_u = glam::Vec3::new(normal.y, normal.z, normal.x);
    let axis_v = normal.cross(axis_u);

    let mut vertices: Vec<model::Vertex> = {
        let vertices_count = resolution * resolution;
        Vec::with_capacity(vertices_count)
    };
    let mut triangles: Vec<u32> = {
        let triangles_count = (resolution - 1) * (resolution - 1) * 6;
        Vec::with_capacity(triangles_count)
    };

    for y in 0..resolution {
        for x in 0..resolution {
            let t = glam::Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;

            let position = normal + axis_u * (2.0 * t.x - 1.0) + axis_v * (2.0 * t.y - 1.0);

            let vertex = model::Vertex {
                position: position.into(),
                tex_coords: [t.x, 1.0 - t.y],
                normal: normal.into(),
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

    model::Mesh {
        name: "cube_face".to_string(),
        vertices: vertices,
        indices: triangles,
        material_index: material_index,
    }
}
