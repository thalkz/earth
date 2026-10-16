use std::ops::Range;

use crate::assets::{DrawModel, Model, ModelVertex, Vertex};
use crate::gpu::{Texture, create_render_pipeline};
use crate::instance::InstanceRaw;

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
            &[Some(ModelVertex::desc()), Some(InstanceRaw::desc())],
            shader,
        );

        Self { pipeline }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw_model_instanced(model, instances, camera_bind_group, light_bind_group);
    }
}
