use crate::{
    renderer::{render_gizmos, render_model},
    scene,
};
use wgpu::util::DeviceExt;

pub struct RenderScene {
    pub render_earth: render_model::RenderModel,
    pub render_gizmos: render_gizmos::RenderGizmos,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,
}

impl RenderScene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &scene::Scene,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let render_earth = render_model::RenderModel::from_model(
            device,
            queue,
            &texture_bind_group_layout,
            sampler,
            &scene.earth,
        );

        let mut render_gizmos = render_gizmos::RenderGizmos::new(device);
        render_gizmos.update(device, queue, &scene.gizmos);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&scene.camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform: LightUniform = scene.light.into();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        Self {
            render_earth,
            render_gizmos,
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
        }
    }

    /// Called every frame to write updated positions/matrices to the existing buffers
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &scene::Scene) {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&scene.camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let light_uniform: LightUniform = scene.light.into();
        queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );

        self.render_gizmos.update(device, queue, &scene.gizmos);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &scene::Camera) {
        self.view_position = camera.eye.extend(1.0).to_array();
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 3],
    _padding: u32,
    pub color: [f32; 3],
    _padding2: u32,
}

impl LightUniform {
    pub fn new(direction: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            direction,
            _padding: 0,
            color,
            _padding2: 0,
        }
    }
}

impl From<scene::Light> for LightUniform {
    fn from(value: scene::Light) -> Self {
        Self::new(value.direction.to_array(), value.color.to_array())
    }
}
