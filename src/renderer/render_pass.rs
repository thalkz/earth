use crate::gpu::{GpuContext, SurfaceHandler, Texture};
use crate::renderer::gizmos::GizmosPass;
use crate::renderer::render_scene::{CameraUniform, LightUniform, RenderScene};
use crate::renderer::{ForwardPass, SkyboxPass};
use wgpu::{
    AddressMode, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, Color, CommandEncoderDescriptor, Device, LoadOp, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, Sampler,
    SamplerBindingType, ShaderStages, StoreOp, TextureFormat, TextureSampleType,
    TextureViewDescriptor, TextureViewDimension,
};

pub struct Renderer {
    forward_pass: ForwardPass,
    skybox_pass: SkyboxPass,
    gizmos_pass: GizmosPass,
    pub bind_group_layouts: RendererBindGroupLayouts,
}

pub struct RendererBindGroupLayouts {
    pub texture_bind_group_layout: BindGroupLayout,
    pub elevation_bind_group_layout: BindGroupLayout,
    pub camera_bind_group_layout: BindGroupLayout,
    pub light_bind_group_layout: BindGroupLayout,
    pub default_sampler: Sampler,
    pub repeat_sampler: Sampler,
}

impl Renderer {
    pub fn new(device: &Device, color_format: TextureFormat) -> Self {
        let default_sampler = Texture::create_sampler(device, AddressMode::ClampToEdge);
        let repeat_sampler = Texture::create_sampler(device, AddressMode::Repeat);

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

        let skybox_pass = SkyboxPass::new(device, color_format, &camera_bind_group_layout);
        let gizmos_pass = GizmosPass::new(device, color_format, &camera_bind_group_layout);

        Self {
            forward_pass,
            skybox_pass,
            gizmos_pass,
            bind_group_layouts: RendererBindGroupLayouts {
                texture_bind_group_layout,
                elevation_bind_group_layout,
                camera_bind_group_layout,
                light_bind_group_layout,
                default_sampler,
                repeat_sampler,
            },
        }
    }

    pub fn render(
        &self,
        gpu: &GpuContext,
        surface: &mut SurfaceHandler,
        render_scene: &RenderScene,
    ) -> anyhow::Result<()> {
        let Some(output) = surface.get_current_texture(&gpu.device)? else {
            return Ok(());
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
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
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

            self.forward_pass.draw(
                &mut render_pass,
                &render_scene.render_earth,
                &render_scene.visible_earth_meshes,
                &render_scene.camera_bind_group,
                &render_scene.light_bind_group,
                &render_scene.elevation_bind_group,
            );

            self.skybox_pass
                .draw(&mut render_pass, &render_scene.camera_bind_group);

            if render_scene.render_gizmos.vertex_count > 0 {
                self.gizmos_pass.draw(
                    &mut render_pass,
                    &render_scene.render_gizmos,
                    &render_scene.camera_bind_group,
                );
            }
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        gpu.queue.present(output);

        Ok(())
    }
}

const fn tex_entry(
    binding: u32,
    visibility: ShaderStages,
    view_dimension: TextureViewDimension,
) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Texture {
            multisampled: false,
            view_dimension,
            sample_type: TextureSampleType::Float { filterable: true },
        },
        count: None,
    }
}

const fn sampler_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Sampler(SamplerBindingType::Filtering),
        count: None,
    }
}

const fn uniform_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl Texture {
    const fn bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        BindGroupLayoutDescriptor {
            entries: const {
                &[
                    tex_entry(0, ShaderStages::FRAGMENT, TextureViewDimension::Cube),
                    sampler_entry(1, ShaderStages::FRAGMENT),
                ]
            },
            label: Some("texture_bind_group_layout"),
        }
    }

    const fn elevation_bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        BindGroupLayoutDescriptor {
            entries: const {
                &[
                    tex_entry(0, ShaderStages::VERTEX_FRAGMENT, TextureViewDimension::Cube),
                    sampler_entry(1, ShaderStages::VERTEX_FRAGMENT),
                    tex_entry(2, ShaderStages::FRAGMENT, TextureViewDimension::Cube),
                    tex_entry(3, ShaderStages::FRAGMENT, TextureViewDimension::D2),
                    tex_entry(4, ShaderStages::FRAGMENT, TextureViewDimension::D2),
                    sampler_entry(5, ShaderStages::FRAGMENT),
                ]
            },
            label: Some("elevation_bind_group_layout"),
        }
    }
}

impl CameraUniform {
    const fn bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        BindGroupLayoutDescriptor {
            entries: const { &[uniform_entry(0, ShaderStages::VERTEX_FRAGMENT)] },
            label: Some("camera_bind_group_layout"),
        }
    }
}

impl LightUniform {
    const fn bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        BindGroupLayoutDescriptor {
            entries: const { &[uniform_entry(0, ShaderStages::FRAGMENT)] },
            label: Some("light_bind_group_layout"),
        }
    }
}
