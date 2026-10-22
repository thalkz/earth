use crate::app::InputState;
use crate::assets::Earth;
use crate::world::{Camera, CameraController, Gizmos, GlobeCamera, Light};
use glam::{Quat, Vec2, Vec3};
use std::time::Duration;
use winit::keyboard::KeyCode;

pub struct Scene {
    pub config: SceneConfig,
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub light: Light,
    pub earth: Earth,
    pub gizmos: Gizmos,
}

pub struct SceneConfig {
    pub width: u32,
    pub height: u32,
    pub enable_gizmos: bool,
}

impl Scene {
    pub async fn load(config: SceneConfig) -> anyhow::Result<Self> {
        let mut camera = Camera::new(config.width, config.height);

        let globe_camera = GlobeCamera::default();
        globe_camera.update_camera(&mut camera);

        let camera_controller = CameraController::new(globe_camera);
        let light = Light::new(glam::vec3(2.0, 0.0, 2.0), glam::vec3(1.0, 1.0, 1.0));
        let gizmos = Gizmos::new();

        let earth = Earth::load(1024, "earth/elevation.raw", "earth/diffuse.raw").await?;

        Ok(Self {
            config,
            camera,
            camera_controller,
            light,
            earth,
            gizmos,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.camera.resize(width, height);
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        let viewport_size = Vec2::new(self.config.width as f32, self.config.height as f32);
        self.camera_controller
            .update_camera(&mut self.camera, input, dt, viewport_size);

        let rotation_speed = 60.0_f32.to_radians();
        let rotation = Quat::from_axis_angle(Vec3::Y, rotation_speed * dt.as_secs_f32());
        self.light.direction = (rotation * self.light.direction).normalize();

        self.gizmos.clear();
        if self.config.enable_gizmos && input.is_key_pressed(KeyCode::Space) {
            let white = [1.0; 4];
            self.gizmos
                .add_model(&self.earth.model, white, &self.camera);
        }
    }
}
