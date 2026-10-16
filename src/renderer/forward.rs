use std::ops::Range;

use super::model::GpuModel;
use crate::assets::Vertex;
use crate::gpu::{Texture, create_render_pipeline};

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
            &[Some(Vertex::desc())],
            shader,
        );

        Self { pipeline }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &GpuModel,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material_index];
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &material.bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, light_bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, instances.clone());
        }
    }
}
