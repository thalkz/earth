use crate::app::{Action, InputState};
use glam::{Mat4, Vec3};
use std::time::Duration;

pub struct CameraController {
    pub speed: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self { speed }
    }

    pub fn update_camera(&self, camera: &mut Camera, input: &InputState, dt: Duration) {
        let dt_secs = dt.as_secs_f32();
        let move_amount = self.speed * dt_secs;

        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.length();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if input.is_active(Action::MoveForward) && forward_mag > move_amount {
            camera.eye += forward_norm * move_amount;
        }
        if input.is_active(Action::MoveBackward) {
            camera.eye -= forward_norm * move_amount;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case forward/backward was pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.length();

        if input.is_active(Action::MoveRight) {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * move_amount).normalize() * forward_mag;
        }
        if input.is_active(Action::MoveLeft) {
            camera.eye = camera.target - (forward - right * move_amount).normalize() * forward_mag;
        }
    }
}

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Camera {
        Camera {
            eye: Vec3::new(0.0, 2.0, 4.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            up: Vec3::Y,
            aspect: width as f32 / height.max(1) as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
        proj * view
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
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.eye.extend(1.0).to_array();
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}
