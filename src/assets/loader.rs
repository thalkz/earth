use crate::assets::{ImageAsset, Material, MaterialDiffuse, Mesh, Model, Vertex, dds};
use image::GenericImageView;
use std::path::Path;
use std::{fs, io};
use tobj::LoadOptions;

const OUT_DIR: &str = env!("OUT_DIR");

pub fn load_string(file_name: &str) -> anyhow::Result<String> {
    let txt = {
        let path = Path::new(OUT_DIR).join("res").join(file_name);
        fs::read_to_string(path)?
    };

    Ok(txt)
}

pub fn load_dds_image(filename: &str) -> anyhow::Result<ImageAsset> {
    let data = load_binary(filename)?;
    dds::parse_dds(data)
}

pub fn load_binary(filename: &str) -> anyhow::Result<Vec<u8>> {
    let base_path = Path::new(OUT_DIR).join("res");
    let path = base_path.join(filename);

    // If caller requests a .png file, check if an optimized .dds counterpart exists
    if filename.ends_with(".png") {
        let dds_filename = filename.replace(".png", ".dds");
        let dds_path = base_path.join(&dds_filename);
        if dds_path.exists() {
            return Ok(fs::read(dds_path)?);
        }
    }

    let data = fs::read(path)?;
    Ok(data)
}

pub fn load_image(filename: &str) -> anyhow::Result<ImageAsset> {
    let data = load_binary(filename)?;
    let img = image::load_from_memory(&data)?;
    let (width, height) = img.dimensions();
    let rgba = img.into_rgba8();
    Ok(ImageAsset {
        width,
        height,
        data: rgba.into_raw(),
    })
}

#[allow(dead_code)]
pub async fn load_model(filename: &str) -> anyhow::Result<Model> {
    let obj_text = load_string(filename)?;
    let obj_cursor = futures::io::Cursor::new(obj_text);
    let mut obj_reader = futures::io::BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::futures::load_obj_buf(
        &mut obj_reader,
        &LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(p.to_str().unwrap()).unwrap();
            tobj::load_mtl_buf(&mut io::BufReader::new(io::Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse = if let Some(filename) = m.diffuse_texture {
            MaterialDiffuse::Texture(load_image(&filename)?)
        } else {
            let color = m.diffuse.unwrap_or([1.0, 1.0, 1.0]);
            MaterialDiffuse::Color([color[0], color[1], color[2], 1.0])
        };

        materials.push(Material {
            name: m.name,
            diffuse,
        });
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
