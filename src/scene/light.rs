use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Light {
    pub direction: Vec3,
    pub color: Vec3,
}

impl Light {
    pub fn new(direction: Vec3, color: Vec3) -> Self {
        Self { direction, color }
    }
}
