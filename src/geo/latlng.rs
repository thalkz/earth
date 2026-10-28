use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLng {
    pub lat: f32,
    pub lng: f32,
}

impl LatLng {
    pub fn new(lat: f32, lng: f32) -> LatLng {
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
        let (y, r) = value.lat.sin_cos();
        let (sin_lng, cos_lng) = value.lng.sin_cos();
        Self {
            x: sin_lng * r,
            y,
            z: -cos_lng * r,
        }
    }
}
