use crate::assets::{ImageAsset, MaterialDiffuse, Model, Vertex};
use crate::geo::BoundingSphere;
use crate::gpu::Texture;
use std::mem;
use std::ops::Range;
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
    pub lod_ranges: Vec<Range<u32>>,
    pub material_index: usize,
    pub bounding_sphere: BoundingSphere,
}

impl RenderMesh {
    pub fn lod_range(&self, lod: usize) -> Range<u32> {
        let max_idx = self.lod_ranges.len().saturating_sub(1);
        self.lod_ranges[lod.min(max_idx)].clone()
    }

    #[allow(dead_code)]
    pub fn num_elements(&self) -> u32 {
        self.lod_ranges
            .first()
            .map(|r| r.end - r.start)
            .unwrap_or(0)
    }
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
                    &image.mip_levels,
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

                let lod_ranges = if mesh.lod_ranges.is_empty() {
                    vec![0..mesh.indices.len() as u32]
                } else {
                    mesh.lod_ranges.clone()
                };

                RenderMesh {
                    vertex_buffer,
                    index_buffer,
                    lod_ranges,
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
