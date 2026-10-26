use crate::assets::{ImageAsset, Model, Vertex};
use crate::geo::BoundingSphere;
use crate::gpu::Texture;
use std::mem::{offset_of, size_of};
use std::ops::Range;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferAddress, BufferUsages, Device, Queue, Sampler, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexStepMode,
};

pub struct RenderModel {
    pub meshes: Vec<RenderMesh>,
    pub materials: Vec<RenderMaterial>,
    pub index_buffer: Buffer,
}

pub struct RenderMesh {
    pub vertex_buffer: Buffer,
    pub lod_ranges: Vec<Range<u32>>,
    pub material_index: usize,
    pub bounding_sphere: BoundingSphere,
}

impl RenderMesh {
    pub fn lod_range(&self, lod: usize) -> Range<u32> {
        let max_idx = self.lod_ranges.len().saturating_sub(1);
        self.lod_ranges[lod.min(max_idx)].clone()
    }
}

pub struct RenderMaterial {
    pub bind_group: BindGroup,
}

impl RenderModel {
    #[allow(clippy::single_range_in_vec_init)]
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
            .map(|material| {
                let image = &material.diffuse;
                let mip_levels: Vec<&[u8]> = image.mip_levels().collect();
                let texture = match image {
                    ImageAsset::Cubemap { width, format, .. } => Texture::create_cube(
                        device,
                        queue,
                        *width,
                        *format,
                        &mip_levels,
                        Some(&material.name),
                    ),
                    ImageAsset::Texture2D {
                        width,
                        height,
                        format,
                        ..
                    } => Texture::create_2d(
                        device,
                        queue,
                        *width,
                        *height,
                        *format,
                        &mip_levels,
                        Some(&material.name),
                    ),
                };

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
                    label: Some(&format!("Material Bind Group {}", material.name)),
                });

                RenderMaterial {
                    bind_group: texture_bind_group,
                }
            })
            .collect();

        let index_buffer = {
            let sample_indices = model
                .meshes
                .first()
                .map(|m| m.indices.as_slice())
                .unwrap_or(&[]);
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Shared Earth Mesh Index Buffer"),
                contents: bytemuck::cast_slice(sample_indices),
                usage: BufferUsages::INDEX,
            })
        };

        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: BufferUsages::VERTEX,
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
                    lod_ranges,
                    material_index: mesh.material_index,
                    bounding_sphere,
                }
            })
            .collect();

        Self {
            meshes,
            materials,
            index_buffer,
        }
    }
}

impl Vertex {
    pub const ATTRIBUTES: [VertexAttribute; 1] = [
        VertexAttribute {
            offset: offset_of!(Vertex, position) as BufferAddress,
            shader_location: 0,
            format: VertexFormat::Float32x3,
        },
    ];

    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
