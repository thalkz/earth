use crate::assets::ImageAsset;
use crate::geo::{Frustum, HorizonOccluder};
use crate::gpu::Texture;
use crate::renderer::{RenderGizmos, RenderModel};
use crate::world::{Camera, Light, Scene};
use glam::Mat4;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferUsages, Device, Queue, Sampler, TextureFormat,
};

pub struct RenderScene {
    pub render_earth: RenderModel,
    pub render_gizmos: RenderGizmos,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub light_buffer: Buffer,
    pub light_bind_group: BindGroup,
    pub elevation_bind_group: BindGroup,
    pub frustum: Frustum,
    pub visible_earth_meshes: Vec<usize>,
}

impl RenderScene {
    pub fn new(
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        texture_bind_group_layout: &BindGroupLayout,
        sampler: &Sampler,
        elevation_bind_group_layout: &BindGroupLayout,
        elevation_sampler: &Sampler,
        camera_bind_group_layout: &BindGroupLayout,
        light_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let render_earth = RenderModel::from_model(
            device,
            queue,
            texture_bind_group_layout,
            sampler,
            &scene.earth.model,
        );

        let elevation_bind_group = create_elevation_texture(
            device,
            queue,
            elevation_bind_group_layout,
            elevation_sampler,
            &scene.earth.elevation,
        );

        let mut render_gizmos = RenderGizmos::new(device);
        render_gizmos.update(device, queue, &scene.gizmos);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&scene.camera);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform: LightUniform = scene.light.into();
        let light_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: light_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        let frustum = Frustum::from_view_projection(scene.camera.build_view_projection_matrix());
        let earth_occluder = HorizonOccluder::new(glam::Vec3::ZERO, 0.998);
        let cam_horizon = earth_occluder.prepare_camera(scene.camera.eye);

        let mut visible_earth_meshes = Vec::with_capacity(render_earth.meshes.len());
        for (i, mesh) in render_earth.meshes.iter().enumerate() {
            if let Some((cam_dir, cam_horizon_angle)) = cam_horizon {
                if earth_occluder.is_sphere_behind_horizon(cam_dir, cam_horizon_angle, &mesh.bounding_sphere) {
                    continue;
                }
            }
            if !frustum.contains_sphere(&mesh.bounding_sphere) {
                continue;
            }
            visible_earth_meshes.push(i);
        }

        Self {
            render_earth,
            render_gizmos,
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
            elevation_bind_group,
            frustum,
            visible_earth_meshes,
        }
    }

    /// Called every frame to write updated positions/matrices to the existing buffers
    pub fn update(&mut self, device: &Device, queue: &Queue, scene: &Scene) {
        self.frustum = Frustum::from_view_projection(scene.camera.build_view_projection_matrix());
        let earth_occluder = HorizonOccluder::new(glam::Vec3::ZERO, 0.998);
        let cam_horizon = earth_occluder.prepare_camera(scene.camera.eye);

        self.visible_earth_meshes.clear();
        for (i, mesh) in self.render_earth.meshes.iter().enumerate() {
            if let Some((cam_dir, cam_horizon_angle)) = cam_horizon {
                if earth_occluder.is_sphere_behind_horizon(cam_dir, cam_horizon_angle, &mesh.bounding_sphere) {
                    continue;
                }
            }
            if !self.frustum.contains_sphere(&mesh.bounding_sphere) {
                continue;
            }
            self.visible_earth_meshes.push(i);
        }

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

fn create_elevation_texture(
    device: &Device,
    queue: &Queue,
    texture_bind_group_layout: &BindGroupLayout,
    sampler: &Sampler,
    image: &ImageAsset,
) -> BindGroup {
    let elevation_texture = Texture::create_2d(
        device,
        queue,
        image.width,
        image.height,
        TextureFormat::Rgba8Unorm,
        &image.data,
        Some("elevation"),
    );

    let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&elevation_texture.view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
        ],
        label: Some("Elevation Bind Group"),
    });
    texture_bind_group
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
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
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

impl From<Light> for LightUniform {
    fn from(value: Light) -> Self {
        Self::new(value.direction.to_array(), value.color.to_array())
    }
}
