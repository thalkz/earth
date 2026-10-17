use crate::scene::{self, GizmosVertex};
use wgpu::util::DeviceExt;

pub struct RenderGizmos {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

impl RenderGizmos {
    pub fn from_gizmos(device: &wgpu::Device, gizmos: &scene::Gizmos) -> RenderGizmos {
        let gizmos_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmos Vertex Buffer"),
            contents: bytemuck::cast_slice(&gizmos.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        RenderGizmos {
            vertex_buffer: gizmos_vertex_buffer,
            vertex_count: gizmos.vertices.len() as u32,
        }
    }
}

impl GizmosVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<crate::scene::GizmosVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
