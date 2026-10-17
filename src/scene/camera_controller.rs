use crate::scene::{Action, Camera, CameraActionState};
use std::time::Duration;

pub struct CameraController {
    pub speed: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self { speed }
    }

    pub fn update_camera(
        &self,
        camera: &mut Camera,
        actions: &CameraActionState,
        dt: Duration,
    ) {
        let dt_secs = dt.as_secs_f32();
        let move_amount = self.speed * dt_secs;

        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.length();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if actions.is_active(Action::MoveForward) && forward_mag > move_amount {
            camera.eye += forward_norm * move_amount;
        }
        if actions.is_active(Action::MoveBackward) {
            camera.eye -= forward_norm * move_amount;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case forward/backward was pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.length();

        if actions.is_active(Action::MoveRight) {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * move_amount).normalize() * forward_mag;
        }
        if actions.is_active(Action::MoveLeft) {
            camera.eye = camera.target - (forward - right * move_amount).normalize() * forward_mag;
        }
    }
}
