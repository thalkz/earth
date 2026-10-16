use crate::assets::Model;
use crate::scene::{Camera, CameraActionState, CameraController, Light};
use std::time::Duration;

pub struct Scene {
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub light: Light,
    pub earth: Model,
}

impl Scene {
    pub fn new(width: u32, height: u32, earth: Model) -> Self {
        let camera = Camera::new(width, height);
        let camera_controller = CameraController::new(10.0);
        let light = Light::new(glam::vec3(2.0, 2.0, 2.0), glam::vec3(1.0, 1.0, 1.0));

        Self {
            camera,
            camera_controller,
            light,
            earth,
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
    }
}
