use crate::assets::{Model, ModelVertex, Vertex};
use crate::gpu::{create_render_pipeline, Texture};

pub struct GizmoPass {
    pub pipeline: wgpu::RenderPipeline,
}

impl GizmoPass {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        light_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gizmo Pipeline Layout"),
            bind_group_layouts: &[Some(camera_layout), Some(light_layout)],
            immediate_size: 0,
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gizmo.wgsl").into()),
        };

        let pipeline = create_render_pipeline(
            "Gizmo Render Pipeline",
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
        for mesh in &model.meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, light_bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
