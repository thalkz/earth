use crate::app::{InputState, TouchGesture};
use crate::geo::LatLng;
use crate::world::globe_camera::{MAX_DISTANCE, MAX_PITCH, MIN_DISTANCE};
use crate::world::{Camera, GlobeCamera};
use glam::{Quat, Vec2, Vec3};
use std::f32::consts::PI;
use std::time::Duration;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

fn wrap_pi(mut angle: f32) -> f32 {
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle < -PI {
        angle += 2.0 * PI;
    }
    angle
}

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
        let dt_secs = dt.as_secs_f32().clamp(1e-4, 0.1);
        let friction = 8.0f32;
        let decay = (-friction * dt_secs).exp();

        let touch_gesture = input.touch_gesture();
        if !matches!(touch_gesture, TouchGesture::None) || input.has_active_touches() {
            self.handle_touch_gesture(camera, touch_gesture, dt_secs, viewport_size);
            self.globe_camera.update_camera(camera);
            return;
        }

        // 1. Desktop Orbit (Heading/Bearing) & Tilt (Pitch)
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

            let d_heading = delta.x * rot_sens;
            let d_pitch = -delta.y * rot_sens;

            self.globe_camera.heading = wrap_pi(self.globe_camera.heading + d_heading);
            self.globe_camera.pitch = (self.globe_camera.pitch + d_pitch).clamp(0.0, MAX_PITCH);

            self.globe_camera.vel_heading = d_heading / dt_secs;
            self.globe_camera.vel_pitch = d_pitch / dt_secs;
        } else {
            // Apply inertial orbit when not actively dragging orbit
            if self.globe_camera.vel_heading.abs() > 1e-4 {
                self.globe_camera.heading =
                    wrap_pi(self.globe_camera.heading + self.globe_camera.vel_heading * dt_secs);
                self.globe_camera.vel_heading *= decay;
            } else {
                self.globe_camera.vel_heading = 0.0;
            }

            if self.globe_camera.vel_pitch.abs() > 1e-4 {
                self.globe_camera.pitch = (self.globe_camera.pitch
                    + self.globe_camera.vel_pitch * dt_secs)
                    .clamp(0.0, MAX_PITCH);
                self.globe_camera.vel_pitch *= decay;
            } else {
                self.globe_camera.vel_pitch = 0.0;
            }
        }

        // 2. Desktop Surface Pinned Dragging (Great Circle)
        if !is_orbit_down && input.is_mouse_pressed(MouseButton::Left) {
            if let Some(cursor) = input.cursor_position() {
                self.apply_surface_drag(camera, cursor, !self.is_dragging_surface, dt_secs, viewport_size);
            }
        } else {
            self.is_dragging_surface = false;
            self.drag_start_hit = None;

            // Apply inertial surface panning when released
            let speed = self.globe_camera.vel_target.length();
            if speed > 1e-4 {
                let axis = self.globe_camera.vel_target / speed;
                let angle = speed * dt_secs;
                let rot = Quat::from_axis_angle(axis, angle);
                let curr_pos = Vec3::from(self.globe_camera.target);
                self.globe_camera.target = LatLng::from((rot * curr_pos).normalize());

                self.globe_camera.vel_target *= decay;
            } else {
                self.globe_camera.vel_target = Vec3::ZERO;
            }
        }

        // 3. Desktop Anchor-Aware Zoom (Mouse Wheel & Trackpad Pinch)
        let zoom_input = input.wheel_delta() + input.pinch_delta() * 5.0;
        if zoom_input.abs() > 1e-4 {
            let zoom_factor = (-zoom_input * 0.1).exp();
            let old_dist = self.globe_camera.distance;
            let new_dist = (old_dist * zoom_factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
            self.globe_camera.distance = new_dist;

            self.globe_camera.vel_distance = -zoom_input * 1.5;

            if let Some(cursor) = input.cursor_position() {
                self.anchor_zoom_toward(camera, cursor, old_dist, new_dist, viewport_size);
            }
        } else if self.globe_camera.vel_distance.abs() > 1e-3 {
            let zoom_factor = (self.globe_camera.vel_distance * dt_secs).exp();
            self.globe_camera.distance =
                (self.globe_camera.distance * zoom_factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
            self.globe_camera.vel_distance *= decay;
        } else {
            self.globe_camera.vel_distance = 0.0;
        }

        // 4. Keyboard Controls
        if input.is_key_pressed(KeyCode::KeyW) || input.is_key_pressed(KeyCode::ArrowUp) {
            if input.is_ctrl_down() || input.is_shift_down() {
                self.globe_camera.pitch =
                    (self.globe_camera.pitch + 1.5 * dt_secs).clamp(0.0, MAX_PITCH);
            } else {
                self.globe_camera.distance = (self.globe_camera.distance * (1.0 - 1.5 * dt_secs))
                    .clamp(MIN_DISTANCE, MAX_DISTANCE);
            }
        }
        if input.is_key_pressed(KeyCode::KeyS) || input.is_key_pressed(KeyCode::ArrowDown) {
            if input.is_ctrl_down() || input.is_shift_down() {
                self.globe_camera.pitch =
                    (self.globe_camera.pitch - 1.5 * dt_secs).clamp(0.0, MAX_PITCH);
            } else {
                self.globe_camera.distance = (self.globe_camera.distance * (1.0 + 1.5 * dt_secs))
                    .clamp(MIN_DISTANCE, MAX_DISTANCE);
            }
        }
        if input.is_key_pressed(KeyCode::KeyA) || input.is_key_pressed(KeyCode::ArrowLeft) {
            self.globe_camera.heading = wrap_pi(self.globe_camera.heading - 1.5 * dt_secs);
        }
        if input.is_key_pressed(KeyCode::KeyD) || input.is_key_pressed(KeyCode::ArrowRight) {
            self.globe_camera.heading = wrap_pi(self.globe_camera.heading + 1.5 * dt_secs);
        }
        if input.is_key_pressed(KeyCode::KeyN) {
            self.globe_camera.heading *= (0.01_f32).powf(dt_secs);
        }
        if input.is_key_pressed(KeyCode::KeyR) {
            self.globe_camera.reset_view();
        }

        self.globe_camera.update_camera(camera);
    }

    fn handle_touch_gesture(
        &mut self,
        camera: &mut Camera,
        gesture: TouchGesture,
        dt_secs: f32,
        viewport_size: Vec2,
    ) {
        match gesture {
            TouchGesture::None => {
                // Finger lifted during multi-touch latch window: damp any residual spin
                self.is_dragging_surface = false;
                self.drag_start_hit = None;
                self.globe_camera.vel_target = Vec3::ZERO;
                self.globe_camera.vel_heading = 0.0;
                self.globe_camera.vel_pitch = 0.0;
            }
            TouchGesture::DoubleTap { anchor_pos } => {
                self.is_dragging_surface = false;
                self.drag_start_hit = None;
                self.globe_camera.vel_target = Vec3::ZERO;

                let old_dist = self.globe_camera.distance;
                let new_dist = (old_dist * 0.55).clamp(MIN_DISTANCE, MAX_DISTANCE);
                self.anchor_zoom_toward(camera, anchor_pos, old_dist, new_dist, viewport_size);
                self.globe_camera.vel_distance = -4.0;
            }
            TouchGesture::SingleZoomDrag {
                prev_y,
                curr_y,
                anchor_pos,
            } => {
                self.is_dragging_surface = false;
                self.drag_start_hit = None;
                self.globe_camera.vel_target = Vec3::ZERO;

                let dy = curr_y - prev_y;
                if dy.abs() > 0.1 {
                    // Drag down zooms in, drag up zooms out (standard mobile map one-handed zoom)
                    let scale = (-dy * 0.006).exp();
                    let old_dist = self.globe_camera.distance;
                    let new_dist = (old_dist * scale).clamp(MIN_DISTANCE, MAX_DISTANCE);
                    self.globe_camera.distance = new_dist;
                    self.anchor_zoom_toward(camera, anchor_pos, old_dist, new_dist, viewport_size);
                }
            }
            TouchGesture::SinglePan {
                prev_pos,
                curr_pos,
                just_started,
            } => {
                let start_new = just_started || !self.is_dragging_surface;
                if start_new {
                    self.globe_camera.vel_heading = 0.0;
                    self.globe_camera.vel_pitch = 0.0;
                    self.globe_camera.vel_distance = 0.0;
                }

                if start_new || curr_pos.distance_squared(prev_pos) > 0.04 {
                    self.apply_surface_drag(camera, curr_pos, start_new, dt_secs, viewport_size);
                } else {
                    // Finger held stationary on screen: smoothly bleed off velocity
                    self.globe_camera.vel_target *= (-20.0 * dt_secs).exp();
                }
            }
            TouchGesture::TwoFinger {
                prev0,
                curr0,
                prev1,
                curr1,
            } => {
                self.is_dragging_surface = false;
                self.drag_start_hit = None;
                self.globe_camera.vel_target = Vec3::ZERO;
                self.globe_camera.vel_heading = 0.0;
                self.globe_camera.vel_pitch = 0.0;
                self.globe_camera.vel_distance = 0.0;

                let v_prev = prev1 - prev0;
                let v_curr = curr1 - curr0;
                let prev_dist = v_prev.length();
                let curr_dist = v_curr.length();

                let prev_mid = (prev0 + prev1) * 0.5;
                let curr_mid = (curr0 + curr1) * 0.5;
                let d0 = curr0 - prev0;
                let d1 = curr1 - prev1;

                // 1. Detect parallel 2-finger vertical drag for Tilt (Pitch)
                let abs_y_sum = d0.y.abs() + d1.y.abs();
                let parallel_y = if d0.y * d1.y > 0.0 && abs_y_sum > 0.2 {
                    (2.0 * d0.y.abs().min(d1.y.abs()) / abs_y_sum).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                let mid_dy = 0.5 * (d0.y + d1.y);
                let dist_delta = (curr_dist - prev_dist).abs();
                let tilt_dominance = if mid_dy.abs() + dist_delta > 0.2 {
                    mid_dy.abs() / (mid_dy.abs() + dist_delta * 0.9)
                } else {
                    0.0
                };
                let tilt_weight = parallel_y * tilt_dominance;

                if tilt_weight > 0.1 {
                    let d_pitch = -mid_dy * tilt_weight * (2.4 / viewport_size.y.max(1.0));
                    self.globe_camera.pitch =
                        (self.globe_camera.pitch + d_pitch).clamp(0.0, MAX_PITCH);
                    self.globe_camera.update_camera(camera);
                }

                // 2. Two-finger Pinch-to-Zoom (suppressed during pure vertical tilt)
                let non_tilt = 1.0 - tilt_weight * 0.85;
                if prev_dist > 15.0 && curr_dist > 15.0 {
                    let raw_scale = prev_dist / curr_dist;
                    let scale = 1.0 + (raw_scale - 1.0) * non_tilt;
                    let old_dist = self.globe_camera.distance;
                    let new_dist = (old_dist * scale).clamp(MIN_DISTANCE, MAX_DISTANCE);
                    self.globe_camera.distance = new_dist;
                    self.anchor_zoom_toward(camera, curr_mid, old_dist, new_dist, viewport_size);
                    self.globe_camera.update_camera(camera);
                }

                // 3. Two-finger Twist-to-Rotate Compass Heading
                if prev_dist > 25.0 && curr_dist > 25.0 {
                    let cross = v_prev.x * v_curr.y - v_prev.y * v_curr.x;
                    let dot = v_prev.dot(v_curr);
                    let d_heading = cross.atan2(dot) * non_tilt;
                    if d_heading.abs() > 1e-4 {
                        self.globe_camera.heading = wrap_pi(self.globe_camera.heading + d_heading);
                        self.globe_camera.update_camera(camera);
                    }
                }

                // 4. Two-finger Midpoint Pan (keeps ground between fingers anchored)
                if non_tilt > 0.3 && curr_mid.distance_squared(prev_mid) > 0.1 {
                    let (ro0, rd0) = camera.screen_to_ray(prev_mid, viewport_size);
                    let (ro1, rd1) = camera.screen_to_ray(curr_mid, viewport_size);
                    if let (Some(h0), Some(h1)) = (
                        Camera::ray_intersect_sphere(ro0, rd0, Vec3::ZERO, 1.0),
                        Camera::ray_intersect_sphere(ro1, rd1, Vec3::ZERO, 1.0),
                    ) {
                        let axis = h1.cross(h0);
                        let len = axis.length();
                        if len > 1e-6 {
                            let angle = h1.dot(h0).clamp(-1.0, 1.0).acos() * non_tilt;
                            let rot = Quat::from_axis_angle(axis / len, angle);
                            let curr_pos = Vec3::from(self.globe_camera.target);
                            self.globe_camera.target = LatLng::from((rot * curr_pos).normalize());
                            self.globe_camera.update_camera(camera);
                        }
                    }
                }
            }
        }
    }

    fn apply_surface_drag(
        &mut self,
        camera: &mut Camera,
        cursor: Vec2,
        start_new: bool,
        dt_secs: f32,
        viewport_size: Vec2,
    ) {
        let (ray_origin, ray_dir) = camera.screen_to_ray(cursor, viewport_size);
        let hit = Camera::ray_intersect_sphere_or_silhouette(ray_origin, ray_dir, Vec3::ZERO, 1.0);

        if start_new {
            self.is_dragging_surface = true;
            self.drag_start_hit = Some(hit);
            self.globe_camera.vel_target = Vec3::ZERO;
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

                let inst_vel = (axis / len) * (angle / dt_secs);
                self.globe_camera.vel_target = self.globe_camera.vel_target.lerp(inst_vel, 0.5);
            }

            // Re-sync camera and update surface anchor point
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

    fn anchor_zoom_toward(
        &mut self,
        camera: &Camera,
        anchor_screen: Vec2,
        old_dist: f32,
        new_dist: f32,
        viewport_size: Vec2,
    ) {
        if new_dist >= old_dist {
            return;
        }
        let (ray_origin, ray_dir) = camera.screen_to_ray(anchor_screen, viewport_size);
        if let Some(hit) = Camera::ray_intersect_sphere(ray_origin, ray_dir, Vec3::ZERO, 1.0) {
            let anchor_bias = (1.0 - (new_dist / old_dist)).clamp(0.0, 0.45);
            let curr_pos = Vec3::from(self.globe_camera.target);
            let new_pos = curr_pos.lerp(hit, anchor_bias).normalize();
            self.globe_camera.target = LatLng::from(new_pos);
        }
    }
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new(GlobeCamera::default())
    }
}
