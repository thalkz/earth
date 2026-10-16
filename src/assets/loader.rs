use super::model::{ImageAsset, Material, Mesh, Model, Vertex};
use image::GenericImageView;
use std::{fs, io, path};

const OUT_DIR: &str = env!("OUT_DIR");

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let txt = {
        let path = path::Path::new(OUT_DIR).join("res").join(file_name);
        fs::read_to_string(path)?
    };

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let data = {
        let path = path::Path::new(OUT_DIR).join("res").join(file_name);
        fs::read(path)?
    };

    Ok(data)
}

pub async fn load_image(file_name: &str) -> anyhow::Result<ImageAsset> {
    let data = load_binary(file_name).await?;
    let img = image::load_from_memory(&data)?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    Ok(ImageAsset {
        width,
        height,
        data: rgba.into_raw(),
    })
}

pub async fn load_model(file_name: &str) -> anyhow::Result<Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = futures::io::Cursor::new(obj_text);
    let mut obj_reader = futures::io::BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::futures::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(p.to_str().unwrap()).await.unwrap();
            tobj::load_mtl_buf(&mut io::BufReader::new(io::Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        if let Some(texture_filename) = m.diffuse_texture {
            let diffuse_texture = load_image(&texture_filename).await?;
            materials.push(Material {
                name: m.name,
                diffuse_texture,
            });
        }
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty() {
                        Vertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [
                                m.mesh.texcoords[i * 2],
                                1.0 - m.mesh.texcoords[i * 2 + 1],
                            ],
                            normal: [0.0, 0.0, 0.0],
                        }
                    } else {
                        Vertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [
                                m.mesh.texcoords[i * 2],
                                1.0 - m.mesh.texcoords[i * 2 + 1],
                            ],
                            normal: [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ],
                        }
                    }
                })
                .collect::<Vec<_>>();

            Mesh {
                name: m.name,
                vertices,
                indices: m.mesh.indices,
                material_index: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(Model { meshes, materials })
}
