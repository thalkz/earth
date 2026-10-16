use std::ops::Range;

use super::model::{model_vertex_layout, GpuModel};
use crate::gpu::{create_render_pipeline, Texture};
use crate::scene::Instance;
use wgpu::BufferAddress;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl From<&Instance> for InstanceRaw {
    fn from(value: &Instance) -> Self {
        Self {
            model: value.transform.to_mat4().to_cols_array_2d(),
        }
    }
}

pub struct ForwardPass {
    pub pipeline: wgpu::RenderPipeline,
}

impl ForwardPass {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
        camera_layout: &wgpu::BindGroupLayout,
        light_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Forward Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(texture_layout),
                    Some(camera_layout),
                    Some(light_layout),
                ],
                immediate_size: 0,
            });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Forward Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("forward.wgsl").into()),
        };

        let pipeline = create_render_pipeline(
            "Forward Render Pipeline",
            device,
            &render_pipeline_layout,
            color_format,
            Some(Texture::DEPTH_FORMAT),
            &[Some(model_vertex_layout()), Some(InstanceRaw::desc())],
            shader,
        );

        Self { pipeline }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a GpuModel,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &material.bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, light_bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, instances.clone());
        }
    }
}
