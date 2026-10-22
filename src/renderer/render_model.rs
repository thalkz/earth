use crate::assets::{ImageAsset, MaterialDiffuse, Model, Vertex};
use crate::geo::BoundingSphere;
use crate::gpu::Texture;
use std::mem;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferAddress, BufferUsages, Device, Queue, Sampler, TextureFormat, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexStepMode,
};

pub struct RenderModel {
    pub meshes: Vec<RenderMesh>,
    pub materials: Vec<RenderMaterial>,
}

pub struct RenderMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_elements: u32,
    pub material_index: usize,
    pub bounding_sphere: BoundingSphere,
}

pub struct RenderMaterial {
    pub bind_group: BindGroup,
}

impl RenderModel {
    pub fn from_model(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &BindGroupLayout,
        sampler: &Sampler,
        model: &Model,
    ) -> Self {
        let materials = model
            .materials
            .iter()
            .map(|mat| {
                let image = match &mat.diffuse {
                    MaterialDiffuse::Texture(img) => img,
                    MaterialDiffuse::Color(rgba) => &ImageAsset::from_color(*rgba),
                };

                let texture = Texture::create_2d(
                    device,
                    queue,
                    image.width,
                    image.height,
                    TextureFormat::Rgba8UnormSrgb,
                    &image.data,
                    Some(&mat.name),
                );

                let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
                    layout: texture_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&texture.view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(sampler),
                        },
                    ],
                    label: Some(&format!("Material Bind Group {}", mat.name)),
                });

                RenderMaterial {
                    bind_group: texture_bind_group,
                }
            })
            .collect();

        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: BufferUsages::INDEX,
                });

                let mut bounding_sphere = BoundingSphere::from_points(
                    mesh.vertices.iter().map(|v| glam::Vec3::from(v.position)),
                );
                // Add safety margin for vertex shader displacements (e.g. elevation height scale)
                bounding_sphere.radius *= 1.05;

                RenderMesh {
                    vertex_buffer,
                    index_buffer,
                    num_elements: mesh.indices.len() as u32,
                    material_index: mesh.material_index,
                    bounding_sphere,
                }
            })
            .collect();

        Self { meshes, materials }
    }
}

impl Vertex {
    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}
