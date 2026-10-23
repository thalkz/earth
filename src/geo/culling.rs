use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
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
