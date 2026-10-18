use crate::assets::Earth;
use crate::world::{Action, Camera, CameraActionState, CameraController, Gizmos, Light};
use glam::{Quat, Vec3};
use std::time::Duration;

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
    pub fn new(config: SceneConfig, earth: Earth) -> Self {
        let camera = Camera::new(config.width, config.height);
        let camera_controller = CameraController::new(10.0);
        let light = Light::new(glam::vec3(2.0, 2.0, 2.0), glam::vec3(1.0, 1.0, 1.0));
        let gizmos = Gizmos::new();

        Self {
            config,
            camera,
            camera_controller,
            light,
            earth,
            gizmos,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width, height);
    }

    pub fn update(&mut self, dt: Duration, actions: &CameraActionState) {
        self.camera_controller
            .update_camera(&mut self.camera, actions, dt);

        let rotation_speed = 60.0_f32.to_radians();
        let rotation = Quat::from_axis_angle(Vec3::Y, rotation_speed * dt.as_secs_f32());
        self.light.direction = (rotation * self.light.direction).normalize();

        self.gizmos.clear();
        if self.config.enable_gizmos && actions.is_active(Action::ShowGizmos) {
            let white = [1.0; 4];
            self.gizmos
                .add_model(&self.earth.model, white, &self.camera);
        }
    }
}
