use crate::assets::Model;
use crate::scene::{self, Camera, CameraActionState, CameraController, Gizmos, Light};
use std::time::Duration;

pub struct Scene {
    pub config: SceneConfig,
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub light: Light,
    pub earth: Model,
    pub gizmos: Gizmos,
}

pub struct SceneConfig {
    pub width: u32,
    pub height: u32,
    pub enable_gizmos: bool,
}

impl Scene {
    pub fn new(config: SceneConfig, earth: Model) -> Self {
        let camera = Camera::new(config.width, config.height);
        let camera_controller = CameraController::new(10.0);
        let light = Light::new(glam::vec3(2.0, 2.0, 2.0), glam::vec3(1.0, 1.0, 1.0));
        let gizmos = scene::Gizmos::new();

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
        let rotation =
            glam::Quat::from_axis_angle(glam::Vec3::Y, rotation_speed * dt.as_secs_f32());
        self.light.direction = (rotation * self.light.direction).normalize();

        self.gizmos.clear();
        if self.config.enable_gizmos && actions.is_active(scene::Action::ShowGizmos) {
            let white = [1.0; 4];
            self.gizmos.add_model(&self.earth, white, &self.camera);
        }
    }
}
