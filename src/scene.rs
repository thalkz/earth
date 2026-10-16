mod camera;
mod instance;
mod light;
mod transform;

pub use camera::{Camera, CameraController, CameraUniform};
pub use instance::{build_instances, Instance};
pub use light::{Light, LightUniform};
pub use transform::Transform;

use crate::app::InputState;
use std::time::Duration;

pub struct Scene {
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub light: Light,
    pub instances: Vec<Instance>,
}

impl Scene {
    pub fn new(width: u32, height: u32) -> Self {
        let camera = Camera::new(width, height);
        let camera_controller = CameraController::new(4.0);
        let light = Light::new(glam::vec3(2.0, 2.0, 2.0), glam::vec3(1.0, 1.0, 1.0));
        let instances = build_instances();

        Self {
            camera,
            camera_controller,
            light,
            instances,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width, height);
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        self.camera_controller.update_camera(&mut self.camera, input, dt);

        let rotation_speed = 60.0_f32.to_radians();
        let rotation = glam::Quat::from_axis_angle(glam::Vec3::Y, rotation_speed * dt.as_secs_f32());
        self.light.position = rotation * self.light.position;
    }
}
