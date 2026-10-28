use crate::assets::Vertex;
use crate::gpu::Texture;
use crate::renderer::{RenderModel, VisibleMesh};
use wgpu::{
    BindGroup, BindGroupLayout, BlendState, ColorTargetState, ColorWrites, CompareFunction,
    DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, IndexFormat,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, StencilState, TextureFormat, VertexState,
};

pub struct ForwardPass {
    pub pipeline: RenderPipeline,
}

impl ForwardPass {
    pub fn new(
        device: &Device,
        color_format: TextureFormat,
        texture_layout: &BindGroupLayout,
        elevation_layout: &BindGroupLayout,
        camera_layout: &BindGroupLayout,
        light_layout: &BindGroupLayout,
    ) -> Self {
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Forward Render Pipeline Layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(light_layout),
                Some(elevation_layout),
                Some(texture_layout),
            ],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Forward Shader"),
            source: ShaderSource::Wgsl(include_str!("forward.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Forward Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: color_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
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
        render_pass: &mut RenderPass<'a>,
        render_earth: &'a RenderModel,
        visible_meshes: &[VisibleMesh],
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
        elevation_bind_group: &'a BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, light_bind_group, &[]);
        render_pass.set_bind_group(2, elevation_bind_group, &[]);
        render_pass.set_bind_group(3, &render_earth.diffuse_bind_group, &[]);
        render_pass.set_index_buffer(render_earth.index_buffer.slice(..), IndexFormat::Uint32);

        for visible in visible_meshes {
            let mesh = &render_earth.meshes[visible.mesh_index];
            let range = render_earth.lod_range(visible.lod);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.draw_indexed(range, 0, 0..1);
        }
    }
}
