use crate::assets::{DrawLight, Model, ModelVertex, Vertex};
use crate::gpu::{create_render_pipeline, Texture};

pub struct LightVizPass {
    pub pipeline: wgpu::RenderPipeline,
}

impl LightVizPass {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        light_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Light Viz Pipeline Layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(light_layout),
            ],
            immediate_size: 0,
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Light Viz Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../light.wgsl").into()),
        };

        let pipeline = create_render_pipeline(
            "Light Viz Render Pipeline",
            device,
            &layout,
            color_format,
            Some(Texture::DEPTH_FORMAT),
            &[Some(ModelVertex::desc())],
            shader,
        );

        Self { pipeline }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw_light_model(model, camera_bind_group, light_bind_group);
    }
}
