use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    #[allow(dead_code)]
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Computes a bounding sphere containing all given 3D positions.
    pub fn from_points<I>(points: I) -> Self
    where
        I: IntoIterator<Item = Vec3>,
    {
        let pts: Vec<Vec3> = points.into_iter().collect();
        if pts.is_empty() {
            return Self {
                center: Vec3::ZERO,
                radius: 0.0,
            };
        }

        let sum: Vec3 = pts.iter().copied().sum();
        let center = sum / pts.len() as f32;

        let max_dist_sq = pts
            .iter()
            .map(|p| center.distance_squared(*p))
            .fold(0.0f32, f32::max);

        Self {
            center,
            radius: max_dist_sq.sqrt(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    #[allow(dead_code)]
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    /// Creates a normalized plane from a Vec4 (A, B, C, D).
    pub fn from_vec4(v: Vec4) -> Self {
        let normal = Vec3::new(v.x, v.y, v.z);
        let len = normal.length();
        if len > 0.0 {
            Self {
                normal: normal / len,
                distance: v.w / len,
            }
        } else {
            Self {
                normal: Vec3::ZERO,
                distance: 0.0,
            }
        }
    }

    /// Signed distance from the plane to point `p`.
    /// Positive if on the normal side (inside), negative if behind.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.distance
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extracts the 6 frustum planes from a view-projection matrix (Gribb-Hartmann method).
    /// Compatible with glam perspective_rh convention where clip space z is in [0, w].
    pub fn from_view_projection(m: Mat4) -> Self {
        let row0 = m.row(0);
        let row1 = m.row(1);
        let row2 = m.row(2);
        let row3 = m.row(3);

        let planes = [
            Plane::from_vec4(row3 + row0), // Left:   w + x >= 0
            Plane::from_vec4(row3 - row0), // Right:  w - x >= 0
            Plane::from_vec4(row3 + row1), // Bottom: w + y >= 0
            Plane::from_vec4(row3 - row1), // Top:    w - y >= 0
            Plane::from_vec4(row2),        // Near:   z >= 0
            Plane::from_vec4(row3 - row2), // Far:    w - z >= 0
        ];

        Self { planes }
    }

    /// Checks if a bounding sphere intersects or is contained within the frustum.
    /// Returns false if the sphere is completely behind any plane.
    pub fn contains_sphere(&self, sphere: &BoundingSphere) -> bool {
        for plane in &self.planes {
            if plane.signed_distance(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }
}

/// A spherical body (such as a planet) that can occlude objects behind its curvature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonOccluder {
    pub center: Vec3,
    pub radius: f32,
}

#[allow(dead_code)]
impl HorizonOccluder {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Precomputes camera horizon parameters once per frame.
    /// Returns None if the camera is inside or on the surface of the occluder.
    pub fn prepare_camera(&self, camera_pos: Vec3) -> Option<(Vec3, f32)> {
        let cam_rel = camera_pos - self.center;
        let cam_dist = cam_rel.length();
        if cam_dist <= self.radius {
            return None;
        }
        let cam_dir = cam_rel / cam_dist;
        let cam_horizon_angle = (self.radius / cam_dist).clamp(0.0, 1.0).acos();
        Some((cam_dir, cam_horizon_angle))
    }

    /// Tests whether a bounding sphere is occluded behind the horizon using precomputed camera parameters.
    pub fn is_sphere_behind_horizon(
        &self,
        cam_dir: Vec3,
        cam_horizon_angle: f32,
        sphere: &BoundingSphere,
    ) -> bool {
        let p_rel = sphere.center - self.center;
        let p_dist = p_rel.length();
        if p_dist <= 0.0 {
            return false;
        }

        let p_dir = p_rel / p_dist;
        // Terrain max height is 1.035 (elevation scale is 0.03 above unit sphere)
        let max_terrain_radius = 1.035f32;
        let theta_patch = (self.radius / max_terrain_radius).clamp(0.0, 1.0).acos();
        let alpha_patch = (sphere.radius / p_dist).clamp(0.0, 1.0).asin();

        let max_angle = cam_horizon_angle + theta_patch + alpha_patch;
        if max_angle >= std::f32::consts::PI {
            return false;
        }

        cam_dir.dot(p_dir) < max_angle.cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horizon_culling() {
        let occluder = HorizonOccluder::new(Vec3::ZERO, 1.0);
        let cam_pos = Vec3::new(0.0, 0.0, 5.0); // Looking from +Z towards origin
        let (cam_dir, cam_horizon_angle) = occluder.prepare_camera(cam_pos).unwrap();

        // Front of Earth (+Z): facing camera, should be visible
        let front_sphere = BoundingSphere::new(Vec3::new(0.0, 0.0, 1.0), 0.05);
        assert!(!occluder.is_sphere_behind_horizon(cam_dir, cam_horizon_angle, &front_sphere));

        // Back of Earth (-Z, 180 degrees): facing completely away, should be culled
        let back_sphere = BoundingSphere::new(Vec3::new(0.0, 0.0, -1.0), 0.05);
        assert!(occluder.is_sphere_behind_horizon(cam_dir, cam_horizon_angle, &back_sphere));

        // Patch at 120 degrees (behind horizon):
        let angle_120 = 120.0f32.to_radians();
        let behind_horizon_pos = Vec3::new(angle_120.sin(), 0.0, angle_120.cos());
        let behind_horizon_sphere = BoundingSphere::new(behind_horizon_pos, 0.05);
        assert!(occluder.is_sphere_behind_horizon(
            cam_dir,
            cam_horizon_angle,
            &behind_horizon_sphere
        ));

        // Patch at 45 degrees: visible
        let angle_45_pos = Vec3::new(2.0f32.sqrt() / 2.0, 0.0, 2.0f32.sqrt() / 2.0);
        let angle_45_sphere = BoundingSphere::new(angle_45_pos, 0.05);
        assert!(!occluder.is_sphere_behind_horizon(cam_dir, cam_horizon_angle, &angle_45_sphere));
    }

    #[test]
    fn test_bounding_sphere_from_points() {
        let points = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ];
        let sphere = BoundingSphere::from_points(points);
        assert!((sphere.center - Vec3::ZERO).length() < 1e-5);
        assert!((sphere.radius - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_frustum_culling() {
        // Camera at (0, 0, 5) looking at (0, 0, 0), up is Y
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), 1.0, 0.1, 100.0);
        let vp = proj * view;
        let frustum = Frustum::from_view_projection(vp);

        // Target at origin should be inside
        let center_sphere = BoundingSphere::new(Vec3::ZERO, 0.5);
        assert!(frustum.contains_sphere(&center_sphere));

        // Sphere behind camera (z = 10) should be culled
        let behind_sphere = BoundingSphere::new(Vec3::new(0.0, 0.0, 10.0), 0.5);
        assert!(!frustum.contains_sphere(&behind_sphere));

        // Sphere far beyond far plane (z = -150) should be culled
        let far_sphere = BoundingSphere::new(Vec3::new(0.0, 0.0, -150.0), 0.5);
        assert!(!frustum.contains_sphere(&far_sphere));

        // Sphere far to the right should be culled
        let right_sphere = BoundingSphere::new(Vec3::new(50.0, 0.0, 0.0), 0.5);
        assert!(!frustum.contains_sphere(&right_sphere));
    }
}
