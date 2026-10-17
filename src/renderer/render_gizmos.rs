use crate::world::{Gizmos, GizmosVertex};
use std::mem;
use wgpu::{
    Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Queue, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexStepMode,
};

pub struct RenderGizmos {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
}

impl RenderGizmos {
    /// Minimum buffer capacity (e.g., 64 vertices) to avoid reallocating on tiny counts.
    const INITIAL_CAPACITY: usize = 64;

    pub fn new(device: &Device) -> Self {
        let initial_bytes = (Self::INITIAL_CAPACITY * mem::size_of::<GizmosVertex>()) as u64;
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Gizmos Vertex Buffer"),
            size: initial_bytes,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            vertex_count: 0,
        }
    }

    pub fn update(&mut self, device: &Device, queue: &Queue, gizmos: &Gizmos) {
        let count = gizmos.vertices.len();
        self.vertex_count = count as u32;

        if count == 0 {
            return;
        }

        let needed_bytes = (count * mem::size_of::<GizmosVertex>()) as u64;

        // Reallocate only if the current GPU buffer cannot fit the new vertices
        if needed_bytes > self.vertex_buffer.size() {
            // Use next_power_of_two to amortize reallocations (like a Vec)
            let new_size = needed_bytes.next_power_of_two();
            self.vertex_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Gizmos Vertex Buffer (Resized)"),
                size: new_size,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&gizmos.vertices),
        );
    }
}

impl GizmosVertex {
    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: mem::size_of::<GizmosVertex>() as BufferAddress,
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
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}
