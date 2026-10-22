use crate::gpu::{GpuContext, SurfaceHandler, Texture};
use crate::renderer::ForwardPass;
use crate::renderer::gizmos::GizmosPass;
use crate::renderer::render_scene::{CameraUniform, LightUniform, RenderScene};
use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, Color, CommandEncoderDescriptor, Device, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, Sampler,
    SamplerBindingType, ShaderStages, StoreOp, TextureFormat, TextureSampleType,
    TextureViewDescriptor, TextureViewDimension,
};

pub struct Renderer {
    forward_pass: ForwardPass,
    gizmos_pass: GizmosPass,
    pub bind_group_layouts: RendererBindGroupLayouts,
}

pub struct RendererBindGroupLayouts {
    pub texture_bind_group_layout: BindGroupLayout,
    pub elevation_bind_group_layout: BindGroupLayout,
    pub camera_bind_group_layout: BindGroupLayout,
    pub light_bind_group_layout: BindGroupLayout,
    pub default_sampler: Sampler,
}

impl Renderer {
    pub fn new(device: &Device, color_format: TextureFormat) -> Self {
        let default_sampler = Texture::create_sampler(device);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&Texture::bind_group_layout());

        let elevation_bind_group_layout =
            device.create_bind_group_layout(&Texture::elevation_bind_group_layout());

        let camera_bind_group_layout =
            device.create_bind_group_layout(&CameraUniform::bind_group_layout());

        let light_bind_group_layout =
            device.create_bind_group_layout(&LightUniform::bind_group_layout());

        let forward_pass = ForwardPass::new(
            device,
            color_format,
            &texture_bind_group_layout,
            &elevation_bind_group_layout,
            &camera_bind_group_layout,
            &light_bind_group_layout,
        );

        let gizmos_pass = GizmosPass::new(device, color_format, &camera_bind_group_layout);

        Self {
            forward_pass,
            gizmos_pass,
            bind_group_layouts: RendererBindGroupLayouts {
                texture_bind_group_layout,
                elevation_bind_group_layout,
                camera_bind_group_layout,
                light_bind_group_layout,
                default_sampler,
            },
        }
    }

    pub fn render(
        &self,
        gpu: &GpuContext,
        surface: &mut SurfaceHandler,
        render_scene: &RenderScene,
    ) -> anyhow::Result<()> {
        let _span = tracing::trace_span!("Renderer::render").entered();

        let output = {
            let _span = tracing::trace_span!("get_current_texture").entered();
            match surface.get_current_texture(&gpu.device)? {
                Some(output) => output,
                None => return Ok(()),
            }
        };

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _pass_span = tracing::trace_span!("render_pass").entered();
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &surface.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            {
                let _forward_span = tracing::trace_span!(
                    "forward_pass",
                    visible_meshes = render_scene.visible_earth_meshes.len()
                )
                .entered();
                self.forward_pass.draw(
                    &mut render_pass,
                    &render_scene.render_earth,
                    &render_scene.visible_earth_meshes,
                    &render_scene.camera_bind_group,
                    &render_scene.light_bind_group,
                    &render_scene.elevation_bind_group,
                );
            }

            if render_scene.render_gizmos.vertex_count > 0 {
                let _gizmos_span = tracing::trace_span!(
                    "gizmos_pass",
                    vertex_count = render_scene.render_gizmos.vertex_count
                )
                .entered();
                self.gizmos_pass.draw(
                    &mut render_pass,
                    &render_scene.render_gizmos,
                    &render_scene.camera_bind_group,
                );
            }
        }

        {
            let _span = tracing::trace_span!("queue_submit").entered();
            gpu.queue.submit(std::iter::once(encoder.finish()));
        }

        {
            let _span = tracing::trace_span!("queue_present").entered();
            gpu.queue.present(output);
        }

        Ok(())
    }
}

impl Texture {
    fn bind_group_layout<'a>() -> BindGroupLayoutDescriptor<'a> {
        BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        }
    }

    fn elevation_bind_group_layout<'a>() -> BindGroupLayoutDescriptor<'a> {
        BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("elevation_bind_group_layout"),
        }
    }
}

impl CameraUniform {
    fn bind_group_layout<'a>() -> BindGroupLayoutDescriptor<'a> {
        BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        }
    }
}

impl LightUniform {
    fn bind_group_layout<'a>() -> BindGroupLayoutDescriptor<'a> {
        BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("light_bind_group_layout"),
        }
    }
}
