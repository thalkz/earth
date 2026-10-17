use glam::{Mat4, Vec3};

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

    pub fn forward_axis(&self) -> glam::Vec3 {
        (self.target - self.eye).normalize()
    }

    pub fn right_axis(&self) -> glam::Vec3 {
        let foreward = self.forward_axis();
        foreward.cross(self.up).normalize()
    }

    pub fn up_axis(&self) -> glam::Vec3 {
        let foreward = self.forward_axis();
        let right = foreward.cross(self.up).normalize();
        right.cross(foreward)
    }
}
