use crate::app::InputState;
use crate::geo::LatLng;
use crate::world::{Camera, GlobeCamera};
use glam::{Quat, Vec2, Vec3};
use std::f32::consts::PI;
use std::time::Duration;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

pub struct CameraController {
    pub globe_camera: GlobeCamera,
    drag_start_hit: Option<Vec3>,
    is_dragging_surface: bool,
}

impl CameraController {
    pub fn new(globe_camera: GlobeCamera) -> Self {
        Self {
            globe_camera,
            drag_start_hit: None,
            is_dragging_surface: false,
        }
    }

    pub fn update_camera(
        &mut self,
        camera: &mut Camera,
        input: &InputState,
        dt: Duration,
        viewport_size: Vec2,
    ) {
        let dt_secs = dt.as_secs_f32();

        // 1. Orbit (Heading/Bearing) & Tilt (Pitch)
        // Activated by Right Click, Middle Click, or Left Click + (Ctrl or Shift)
        let is_orbit_down = input.is_mouse_pressed(MouseButton::Right)
            || input.is_mouse_pressed(MouseButton::Middle)
            || (input.is_mouse_pressed(MouseButton::Left)
                && (input.is_ctrl_down() || input.is_shift_down()));

        if is_orbit_down {
            self.is_dragging_surface = false;
            self.drag_start_hit = None;

            let delta = input.cursor_delta();
            let rot_sens = 0.005;

            // Horizontal drag rotates heading
            self.globe_camera.heading += delta.x * rot_sens;
            if self.globe_camera.heading > PI {
                self.globe_camera.heading -= 2.0 * PI;
            } else if self.globe_camera.heading < -PI {
                self.globe_camera.heading += 2.0 * PI;
            }

            // Vertical drag tilts pitch (drag down tilts up toward nadir, drag up tilts toward horizon)
            self.globe_camera.pitch = (self.globe_camera.pitch - delta.y * rot_sens)
                .clamp(0.0, crate::world::globe_camera::MAX_PITCH);
        } else if input.is_mouse_pressed(MouseButton::Left) {
            // 2. Surface Pinned Dragging (Great Circle)
            if let Some(cursor) = input.cursor_position() {
                let (ray_origin, ray_dir) = camera.screen_to_ray(cursor, viewport_size);
                let hit = Camera::ray_intersect_sphere_or_silhouette(
                    ray_origin,
                    ray_dir,
                    Vec3::ZERO,
                    1.0,
                );

                if !self.is_dragging_surface {
                    self.is_dragging_surface = true;
                    self.drag_start_hit = Some(hit);
                } else if let Some(p0) = self.drag_start_hit {
                    let p1 = hit;
                    let axis = p1.cross(p0);
                    let len = axis.length();
                    if len > 1e-6 {
                        let angle = p1.dot(p0).clamp(-1.0, 1.0).acos();
                        let rot = Quat::from_axis_angle(axis / len, angle);
                        let curr_pos = Vec3::from(self.globe_camera.target);
                        let new_pos = (rot * curr_pos).normalize();
                        self.globe_camera.target = LatLng::from(new_pos);
                    }

                    // Update camera so subsequent ray-cast aligns with current cursor
                    self.globe_camera.update_camera(camera);
                    let (r_o, r_d) = camera.screen_to_ray(cursor, viewport_size);
                    self.drag_start_hit = Some(Camera::ray_intersect_sphere_or_silhouette(
                        r_o,
                        r_d,
                        Vec3::ZERO,
                        1.0,
                    ));
                }
            }
        } else {
            self.is_dragging_surface = false;
            self.drag_start_hit = None;
        }

        // 3. Anchor-Aware Zoom (Mouse Wheel & Trackpad Pinch)
        let zoom_input = input.wheel_delta() + input.pinch_delta() * 5.0;
        if zoom_input.abs() > 1e-4 {
            let zoom_factor = (-zoom_input * 0.1).exp();
            let old_dist = self.globe_camera.distance;
            let new_dist = (old_dist * zoom_factor).clamp(
                crate::world::globe_camera::MIN_DISTANCE,
                crate::world::globe_camera::MAX_DISTANCE,
            );
            self.globe_camera.distance = new_dist;

            // Zoom toward cursor anchor point if cursor is on globe
            if let Some(cursor) = input.cursor_position() {
                let (ray_origin, ray_dir) = camera.screen_to_ray(cursor, viewport_size);
                if let Some(hit) =
                    Camera::ray_intersect_sphere(ray_origin, ray_dir, Vec3::ZERO, 1.0)
                {
                    // Shift target toward cursor proportional to zoom delta
                    let anchor_bias = (1.0 - (new_dist / old_dist)).clamp(0.0, 0.4);
                    let curr_pos = Vec3::from(self.globe_camera.target);
                    let new_pos = curr_pos.lerp(hit, anchor_bias).normalize();
                    self.globe_camera.target = LatLng::from(new_pos);
                }
            }
        }

        // 4. Keyboard Controls
        if input.is_key_pressed(KeyCode::KeyW) || input.is_key_pressed(KeyCode::ArrowUp) {
            if input.is_ctrl_down() || input.is_shift_down() {
                self.globe_camera.pitch = (self.globe_camera.pitch + 1.5 * dt_secs)
                    .clamp(0.0, crate::world::globe_camera::MAX_PITCH);
            } else {
                self.globe_camera.distance = (self.globe_camera.distance * (1.0 - 1.5 * dt_secs))
                    .clamp(
                        crate::world::globe_camera::MIN_DISTANCE,
                        crate::world::globe_camera::MAX_DISTANCE,
                    );
            }
        }
        if input.is_key_pressed(KeyCode::KeyS) || input.is_key_pressed(KeyCode::ArrowDown) {
            if input.is_ctrl_down() || input.is_shift_down() {
                self.globe_camera.pitch = (self.globe_camera.pitch - 1.5 * dt_secs)
                    .clamp(0.0, crate::world::globe_camera::MAX_PITCH);
            } else {
                self.globe_camera.distance = (self.globe_camera.distance * (1.0 + 1.5 * dt_secs))
                    .clamp(
                        crate::world::globe_camera::MIN_DISTANCE,
                        crate::world::globe_camera::MAX_DISTANCE,
                    );
            }
        }
        if input.is_key_pressed(KeyCode::KeyA) || input.is_key_pressed(KeyCode::ArrowLeft) {
            self.globe_camera.heading -= 1.5 * dt_secs;
        }
        if input.is_key_pressed(KeyCode::KeyD) || input.is_key_pressed(KeyCode::ArrowRight) {
            self.globe_camera.heading += 1.5 * dt_secs;
        }
        if input.is_key_pressed(KeyCode::KeyN) {
            // Smoothly reset North
            self.globe_camera.heading *= (0.01_f32).powf(dt_secs);
        }
        if input.is_key_pressed(KeyCode::KeyR) {
            self.globe_camera.reset_view();
        }

        // Synchronize Camera with GlobeCamera
        self.globe_camera.update_camera(camera);
    }
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new(GlobeCamera::default())
    }
}
