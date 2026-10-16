use crate::assets::{Model, Vertex};
use crate::gpu::Texture;
use wgpu::util::DeviceExt;

pub struct GpuModel {
    pub meshes: Vec<GpuMesh>,
    pub materials: Vec<GpuMaterial>,
}

pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material_index: usize,
}

pub struct GpuMaterial {
    pub bind_group: wgpu::BindGroup,
}

impl GpuModel {
    pub fn from_cpu_model(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_layout: &wgpu::BindGroupLayout,
        model: &Model,
    ) -> Self {
        let materials = model
            .materials
            .iter()
            .map(|mat| {
                let texture = Texture::create_2d(
                    device,
                    queue,
                    mat.diffuse_texture.width,
                    mat.diffuse_texture.height,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    &mat.diffuse_texture.data,
                    Some(&mat.name),
                );

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&texture.sampler),
                        },
                    ],
                    label: Some(&format!("Material Bind Group {}", mat.name)),
                });

                GpuMaterial { bind_group }
            })
            .collect();

        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", mesh.name)),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                GpuMesh {
                    vertex_buffer,
                    index_buffer,
                    num_elements: mesh.indices.len() as u32,
                    material_index: mesh.material_index,
                }
            })
            .collect();

        Self { meshes, materials }
    }
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<crate::assets::Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
