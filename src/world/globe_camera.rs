use crate::geo::LatLng;
use crate::world::Camera;
use glam::{Vec2, Vec3};
use std::f32::consts::PI;

pub const MIN_DISTANCE: f32 = 0.015; // Safe distance above mountains (1.015 altitude)
pub const MAX_DISTANCE: f32 = 10.0;
pub const MAX_PITCH: f32 = 80.0 * PI / 180.0; // 80 degrees

#[derive(Debug, Clone)]
pub struct GlobeCamera {
    /// Focal point on the sphere surface (radius = 1.0)
    pub target: LatLng,
    /// Distance from target point along the gaze line
    pub distance: f32,
    /// Tilt angle from nadir (0.0 = top-down, MAX_PITCH = oblique horizon view)
    pub pitch: f32,
    /// Azimuth angle in radians (0.0 = North, PI/2 = East)
    pub heading: f32,
    /// Dynamic velocities for smooth inertia
    pub vel_target: Vec2,
    pub vel_distance: f32,
    pub vel_heading: f32,
    pub vel_pitch: f32,
}

impl Default for GlobeCamera {
    fn default() -> Self {
        Self {
            target: LatLng::new(0.0, 0.0),
            distance: 3.5,
            pitch: 0.0,
            heading: 0.0,
            vel_target: Vec2::ZERO,
            vel_distance: 0.0,
            vel_heading: 0.0,
            vel_pitch: 0.0,
        }
    }
}

#[allow(dead_code)]
impl GlobeCamera {
    pub fn new(target: LatLng, distance: f32) -> Self {
        Self {
            target,
            distance: distance.clamp(MIN_DISTANCE, MAX_DISTANCE),
            ..Default::default()
        }
    }

    /// Computes the local East-North-Up orthonormal basis at the current target.
    /// Returns (east, north, up_normal).
    pub fn local_enu_frame(&self) -> (Vec3, Vec3, Vec3) {
        let up_normal = Vec3::from(self.target).normalize();

        // Cross with North axis (Vec3::Y in our geo coordinate system)
        let mut east = up_normal.cross(Vec3::Y);
        if east.length_squared() < 1e-6 {
            // At poles, choose a stable arbitrary east vector
            east = Vec3::X;
        } else {
            east = east.normalize();
        }

        let north = east.cross(up_normal).normalize();
        (east, north, up_normal)
    }

    /// Converts spherical globe parameters into Cartesian eye, target, and up vectors.
    pub fn to_cartesian(&self) -> (Vec3, Vec3, Vec3) {
        let (east, north, up_normal) = self.local_enu_frame();
        let target_pos = up_normal; // On unit sphere, target position is up_normal

        // Direction along the ground at current heading
        let dir_ground = north * self.heading.cos() + east * self.heading.sin();

        // Eye position offset relative to target
        let offset = (up_normal * self.pitch.cos() - dir_ground * self.pitch.sin()) * self.distance;
        let eye = target_pos + offset;

        // Camera up vector maintains horizontal horizon
        let camera_up = (up_normal * self.pitch.sin() + dir_ground * self.pitch.cos()).normalize();

        (eye, target_pos, camera_up)
    }

    /// Updates an existing Camera with current GlobeCamera state and adaptive clip planes.
    pub fn update_camera(&self, camera: &mut Camera) {
        let (eye, target, up) = self.to_cartesian();
        camera.eye = eye;
        camera.target = target;
        camera.up = up;
        camera.update_clip_planes();
    }

    /// Resets orientation to look North.
    pub fn reset_north(&mut self) {
        self.heading = 0.0;
        self.vel_heading = 0.0;
    }

    /// Resets camera to default space view.
    pub fn reset_view(&mut self) {
        self.distance = 3.5;
        self.pitch = 0.0;
        self.heading = 0.0;
        self.vel_target = Vec2::ZERO;
        self.vel_distance = 0.0;
        self.vel_heading = 0.0;
        self.vel_pitch = 0.0;
    }
}
