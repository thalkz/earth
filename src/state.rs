use crate::app::InputState;
use crate::assets::{self, Model};
use crate::gpu::{GpuContext, SurfaceHandler};
use crate::instance::InstanceRaw;
use crate::renderer::Renderer;
use crate::scene::{CameraUniform, LightUniform, Scene};
use std::sync::Arc;
use std::time::Duration;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub struct State {
    window: Arc<Window>,
    gpu: GpuContext,
    surface: SurfaceHandler,
    renderer: Renderer,
    scene: Scene,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    obj_model: Model,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let (gpu, surface) = GpuContext::init_window(window.clone()).await?;

        let renderer = Renderer::new(&gpu.device, surface.config.format);

        let scene = Scene::new(surface.config.width, surface.config.height);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&scene.camera);
        let camera_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &renderer.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform = scene.light.to_uniform();
        let light_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("light_uniform_buffer"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let light_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &renderer.light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let obj_model = assets::load_model(
            "cube.obj",
            &gpu.device,
            &gpu.queue,
            &renderer.texture_bind_group_layout,
        )
        .await
        .unwrap();

        let instance_data: Vec<InstanceRaw> = scene.instances.iter().map(|i| i.into()).collect();
        let instance_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Ok(Self {
            window,
            gpu,
            surface,
            renderer,
            scene,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            instance_buffer,
            obj_model,
            light_uniform,
            light_buffer,
            light_bind_group,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.gpu.device, width, height);
        self.scene.resize(width, height);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();
        self.renderer.render(
            &self.gpu,
            &mut self.surface,
            &self.scene,
            &self.obj_model,
            &self.camera_bind_group,
            &self.light_bind_group,
            &self.instance_buffer,
        )
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        self.scene.update(dt, input);

        self.camera_uniform.update_view_proj(&self.scene.camera);
        self.gpu.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.light_uniform = self.scene.light.to_uniform();
        self.gpu.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }
}
