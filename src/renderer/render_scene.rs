use crate::assets::{Earth, ImageAsset};
use crate::geo::{Frustum, HorizonOccluder};
use crate::gpu::Texture;
use crate::renderer::render_pass::RendererBindGroupLayouts;
use crate::renderer::{RenderGizmos, RenderModel};
use crate::world::{Camera, Light, Scene};
use glam::Mat4;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferUsages, Device,
    Queue,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleMesh {
    pub mesh_index: usize,
    pub lod: usize,
}

pub const LOD_DISTANCE_THRESHOLDS: [f32; 3] = [0.2, 0.4, 1.0];

pub fn select_lod(distance: f32) -> usize {
    if distance < LOD_DISTANCE_THRESHOLDS[0] {
        0
    } else if distance < LOD_DISTANCE_THRESHOLDS[1] {
        1
    } else if distance < LOD_DISTANCE_THRESHOLDS[2] {
        2
    } else {
        3
    }
}

pub struct RenderScene {
    pub render_earth: RenderModel,
    pub render_gizmos: RenderGizmos,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub light_buffer: Buffer,
    pub light_bind_group: BindGroup,
    pub elevation_bind_group: BindGroup,
    pub frustum: Frustum,
    pub visible_earth_meshes: Vec<VisibleMesh>,
}

impl RenderScene {
    pub fn new(
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        layouts: &RendererBindGroupLayouts,
    ) -> Self {
        let render_earth = RenderModel::from_model(
            device,
            queue,
            &layouts.texture_bind_group_layout,
            &layouts.default_sampler,
            &scene.earth.model,
        );

        let elevation_bind_group =
            create_elevation_texture(device, queue, layouts, &scene.earth);

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
            layout: &layouts.camera_bind_group_layout,
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
            layout: &layouts.light_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        let frustum = Frustum::from_view_projection(scene.camera.build_view_projection_matrix());
        let visible_earth_meshes = Vec::with_capacity(render_earth.meshes.len());

        let mut render_scene = Self {
            render_earth,
            render_gizmos,
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
            elevation_bind_group,
            frustum,
            visible_earth_meshes,
        };

        render_scene.update(device, queue, scene);
        render_scene
    }

    /// Called every frame to write updated positions/matrices to the existing buffers
    pub fn update(&mut self, device: &Device, queue: &Queue, scene: &Scene) {
        self.frustum = Frustum::from_view_projection(scene.camera.build_view_projection_matrix());
        let earth_occluder = HorizonOccluder::new(glam::Vec3::ZERO, 0.998);
        let camera_horizon = earth_occluder.prepare_camera(scene.camera.eye);

        self.visible_earth_meshes.clear();
        for (i, mesh) in self.render_earth.meshes.iter().enumerate() {
            if let Some((camera_dir, cam_horizon_angle)) = camera_horizon
                && earth_occluder.is_sphere_behind_horizon(
                    camera_dir,
                    cam_horizon_angle,
                    &mesh.bounding_sphere,
                )
            {
                continue;
            }
            if !self.frustum.contains_sphere(&mesh.bounding_sphere) {
                continue;
            }
            let dist_to_center = scene.camera.eye.distance(mesh.bounding_sphere.center);
            let closest_dist = (dist_to_center - mesh.bounding_sphere.radius).max(0.0);
            let lod = select_lod(closest_dist);
            self.visible_earth_meshes
                .push(VisibleMesh { mesh_index: i, lod });
        }

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&scene.camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );

        let mut light_uniform: LightUniform = scene.light.into();
        light_uniform.time = scene.elapsed_time;
        queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );

        self.render_gizmos.update(device, queue, &scene.gizmos);
    }
}

fn create_2d_from_asset(
    device: &Device,
    queue: &Queue,
    image: &ImageAsset,
    label: &str,
) -> Texture {
    let mip_levels: Vec<&[u8]> = image.mip_levels().collect();
    match image {
        ImageAsset::Texture2D {
            width,
            height,
            format,
            ..
        } => Texture::create_2d(device, queue, *width, *height, *format, &mip_levels, Some(label)),
        ImageAsset::Cubemap { width, format, .. } => {
            Texture::create_cube(device, queue, *width, *format, &mip_levels, Some(label))
        }
    }
}

fn create_elevation_texture(
    device: &Device,
    queue: &Queue,
    layouts: &RendererBindGroupLayouts,
    earth: &Earth,
) -> BindGroup {
    let elevation_texture = create_2d_from_asset(device, queue, &earth.elevation, "elevation");
    let ocean_texture = create_2d_from_asset(device, queue, &earth.ocean, "ocean_diffuse");
    let water_normal_1_texture =
        create_2d_from_asset(device, queue, &earth.water_normal_1, "water_normal_1");
    let water_normal_2_texture =
        create_2d_from_asset(device, queue, &earth.water_normal_2, "water_normal_2");

    device.create_bind_group(&BindGroupDescriptor {
        layout: &layouts.elevation_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&elevation_texture.view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&layouts.default_sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&ocean_texture.view),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&water_normal_1_texture.view),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::TextureView(&water_normal_2_texture.view),
            },
            BindGroupEntry {
                binding: 5,
                resource: BindingResource::Sampler(&layouts.repeat_sampler),
            },
        ],
        label: Some("Elevation Bind Group"),
    })
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        let view_proj = camera.build_view_projection_matrix();
        self.view_position = camera.eye.extend(1.0).to_array();
        self.view_proj = view_proj.to_cols_array_2d();
        self.inv_view_proj = view_proj.inverse().to_cols_array_2d();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 3],
    pub time: f32,
    pub color: [f32; 3],
    _padding2: u32,
}

impl LightUniform {
    pub fn new(direction: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            direction,
            time: 0.0,
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
