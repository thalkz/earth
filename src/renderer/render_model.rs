use crate::assets::{Model, Vertex};
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
    pub index_buffer: Buffer,
    pub lod_ranges: [Range<u32>; 4],
    pub diffuse_bind_group: BindGroup,
}

pub struct RenderMesh {
    pub vertex_buffer: Buffer,
    pub bounding_sphere: BoundingSphere,
}

impl RenderModel {
    pub fn lod_range(&self, lod: usize) -> Range<u32> {
        self.lod_ranges[lod.min(self.lod_ranges.len() - 1)].clone()
    }

    pub fn from_model(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &BindGroupLayout,
        sampler: &Sampler,
        model: &Model,
    ) -> Self {
        let texture = Texture::from_asset(device, queue, &model.diffuse, Some("earth_diffuse"));
        let diffuse_bind_group = device.create_bind_group(&BindGroupDescriptor {
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
            label: Some("Earth Diffuse Bind Group"),
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Shared Earth Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&model.indices),
            usage: BufferUsages::INDEX,
        });

        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Earth Patch Vertex Buffer"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: BufferUsages::VERTEX,
                });

                let mut bounding_sphere =
                    BoundingSphere::from_points(mesh.vertices.iter().map(|v| v.position));
                // Add safety margin for vertex shader displacements (e.g. elevation height scale)
                bounding_sphere.radius *= 1.05;

                RenderMesh {
                    vertex_buffer,
                    bounding_sphere,
                }
            })
            .collect();

        Self {
            meshes,
            index_buffer,
            lod_ranges: model.lod_ranges.clone(),
            diffuse_bind_group,
        }
    }
}

impl Vertex {
    pub const ATTRIBUTES: [VertexAttribute; 1] = [VertexAttribute {
        offset: offset_of!(Vertex, position) as BufferAddress,
        shader_location: 0,
        format: VertexFormat::Float32x3,
    }];

    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
