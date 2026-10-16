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

    pub fn to_uniform(&self) -> LightUniform {
        LightUniform::new(self.direction.to_array(), self.color.to_array())
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 3],
    _padding: u32,
    pub color: [f32; 3],
    _padding2: u32,
}

impl LightUniform {
    pub fn new(direction: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            direction,
            _padding: 0,
            color,
            _padding2: 0,
        }
    }
}
