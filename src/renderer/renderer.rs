use crate::gpu::{self, GpuContext, SurfaceHandler};
use crate::renderer::gizmos::GizmosPass;
use crate::renderer::{ForwardPass, render_scene};

pub struct Renderer {
    forward_pass: ForwardPass,
    gizmos_pass: GizmosPass,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&gpu::Texture::bind_group_layout());

        let camera_bind_group_layout =
            device.create_bind_group_layout(&render_scene::CameraUniform::bind_layout_group());

        let light_bind_group_layout =
            device.create_bind_group_layout(&render_scene::LightUniform::bind_layout_group());

        let forward_pass = ForwardPass::new(
            device,
            color_format,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
            &light_bind_group_layout,
        );

        let gizmos_pass = GizmosPass::new(device, color_format, &camera_bind_group_layout);

        Self {
            forward_pass,
            gizmos_pass,
            texture_bind_group_layout,
            camera_bind_group_layout,
            light_bind_group_layout,
        }
    }

    pub fn render(
        &self,
        gpu: &GpuContext,
        surface: &mut SurfaceHandler,
        render_scene: &render_scene::RenderScene,
    ) -> anyhow::Result<()> {
        let Some(output) = surface.get_current_texture(&gpu.device)? else {
            return Ok(());
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &surface.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            self.forward_pass.draw(
                &mut render_pass,
                &render_scene.render_earth,
                &render_scene.camera_bind_group,
                &render_scene.light_bind_group,
            );

            self.gizmos_pass.draw(
                &mut render_pass,
                &render_scene.render_gizmos,
                &render_scene.camera_bind_group,
            );
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        gpu.queue.present(output);

        Ok(())
    }
}

impl gpu::Texture {
    fn bind_group_layout<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        }
    }
}

impl render_scene::CameraUniform {
    fn bind_layout_group<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        }
    }
}

impl render_scene::LightUniform {
    fn bind_layout_group<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("light_bind_group_layout"),
        }
    }
}
