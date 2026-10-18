use glam::{Vec2, Vec3};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLng {
    pub lat: f32,
    pub lng: f32,
}

impl LatLng {
    pub fn new(lat: f32, lng: f32) -> LatLng {
        LatLng { lat, lng }
    }

    pub fn to_uv(&self) -> Vec2 {
        let u = (self.lng / (2.0 * PI)) + 0.5;
        let v = 0.5 - (self.lat / PI);
        Vec2::new(1.0 - u, v)
    }

    pub fn from_uv(uv: Vec2) -> LatLng {
        let lng = (uv.x - 0.5) * 2.0 * PI;
        let lat = (0.5 - uv.y) * PI;
        LatLng { lat, lng }
    }
}

impl From<Vec3> for LatLng {
    fn from(value: Vec3) -> Self {
        let lat = value.y.asin();
        let lng = value.x.atan2(-value.z);
        LatLng { lat, lng }
    }
}

impl From<LatLng> for Vec3 {
    fn from(value: LatLng) -> Self {
        let y = value.lat.sin();
        let r = value.lat.cos();
        let x = value.lng.sin() * r;
        let z = -value.lng.cos() * r;
        Self { x, y, z }
    }
}

pub fn sphere_to_uv(p: Vec3) -> Vec2 {
    LatLng::from(p).to_uv()
}
