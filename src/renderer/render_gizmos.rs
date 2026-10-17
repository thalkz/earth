use crate::scene::{self, GizmosVertex};

pub struct RenderGizmos {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

impl RenderGizmos {
    /// Minimum buffer capacity (e.g., 64 vertices) to avoid reallocating on tiny counts.
    const INITIAL_CAPACITY: usize = 64;

    pub fn new(device: &wgpu::Device) -> Self {
        let initial_bytes = (Self::INITIAL_CAPACITY * std::mem::size_of::<GizmosVertex>()) as u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gizmos Vertex Buffer"),
            size: initial_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            vertex_count: 0,
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, gizmos: &scene::Gizmos) {
        let count = gizmos.vertices.len();
        self.vertex_count = count as u32;

        if count == 0 {
            return;
        }

        let needed_bytes = (count * std::mem::size_of::<GizmosVertex>()) as u64;

        // Reallocate only if the current GPU buffer cannot fit the new vertices
        if needed_bytes > self.vertex_buffer.size() {
            // Use next_power_of_two to amortize reallocations (like a Vec)
            let new_size = needed_bytes.next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Gizmos Vertex Buffer (Resized)"),
                size: new_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
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
