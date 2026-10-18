use glam::Vec3;

struct LatLng {
    lat: f32,
    lng: f32,
}

impl LatLng {
    fn new(lat: f32, lng: f32) -> LatLng {
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
