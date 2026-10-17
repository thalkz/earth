use super::render_model::RenderModel;
use crate::assets::Vertex;
use crate::gpu::Texture;

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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Forward Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("forward.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Forward Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_earth: &RenderModel,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        for mesh in &render_earth.meshes {
            let material = &render_earth.materials[mesh.material_index];
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &material.bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, light_bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
