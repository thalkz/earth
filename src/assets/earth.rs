use glam;

use crate::assets::model;

pub fn load_cube(resolution: usize) -> model::Model {
    let face_normals = vec![
        glam::Vec3::X,
        glam::Vec3::Y,
        glam::Vec3::Z,
        glam::Vec3::NEG_X,
        glam::Vec3::NEG_Y,
        glam::Vec3::NEG_Z,
    ];
    let materials = vec![
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.5, 0.5, 0.0, 1.0],
        [0.0, 0.5, 0.5, 1.0],
        [0.5, 0.0, 0.5, 1.0],
    ]
    .iter()
    .map(|&color| model::MaterialDiffuse::Color(color))
    .map(|diffuse| model::Material {
        name: "material color".to_string(),
        diffuse,
    })
    .collect();

    let meshes: Vec<model::Mesh> = face_normals
        .iter()
        .enumerate()
        .map(|(i, normal)| create_face(normal, resolution, i))
        .collect();

    model::Model {
        meshes: meshes,
        materials: materials,
    }
}

fn create_face(normal: &glam::Vec3, resolution: usize, material_index: usize) -> model::Mesh {
    let axis_u = glam::Vec3::new(normal.y, normal.z, normal.x);
    let axis_v = normal.cross(axis_u);

    let mut vertices: Vec<model::Vertex> = {
        let vertices_count = resolution * resolution;
        let mut vec = Vec::with_capacity(vertices_count);
        (0..vertices_count).for_each(|_| vec.push(model::Vertex::ZERO));
        vec
    };
    let mut triangles: Vec<u32> = {
        let triangles_count = (resolution - 1) * (resolution - 1) * 6;
        let mut vec = Vec::with_capacity(triangles_count);
        (0..triangles_count).for_each(|_| vec.push(0));
        vec
    };

    let mut triangle_index: usize = 0;
    for y in 0..resolution {
        for x in 0..resolution {
            let vertex_index = x + y * resolution;
            let t = glam::Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;

            let u = 2.0 * t.x - 1.0;
            let v = 2.0 * t.y - 1.0;
            let position = normal + axis_u * u + axis_v * v;
            let vertex = model::Vertex {
                position: position.into(),
                tex_coords: [u, v],
                normal: normal.clone().into(),
            };
            vertices[vertex_index as usize] = vertex;

            if x != resolution - 1 && y != resolution - 1 {
                triangles[triangle_index + 0] = vertex_index as u32;
                triangles[triangle_index + 1] = (vertex_index + resolution + 1) as u32;
                triangles[triangle_index + 2] = (vertex_index + resolution) as u32;
                triangles[triangle_index + 3] = vertex_index as u32;
                triangles[triangle_index + 4] = (vertex_index + 1) as u32;
                triangles[triangle_index + 5] = (vertex_index + resolution + 1) as u32;
                triangle_index += 6
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
