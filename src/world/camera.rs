use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone)]
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
        let mut camera = Camera {
            eye: Vec3::new(0.0, 2.0, 4.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            up: Vec3::Y,
            aspect: width as f32 / height.max(1) as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        camera.update_clip_planes();
        camera
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
        proj * view
    }

    /// Adapts znear and zfar to current altitude above the sphere surface to avoid z-fighting.
    pub fn update_clip_planes(&mut self) {
        let altitude = (self.eye.length() - 1.0).max(0.0005);
        self.znear = (altitude * 0.1).clamp(0.0001, 0.5);
        self.zfar = (self.eye.length() + 2.5).clamp(2.0, 50.0);
    }

    /// Unprojects a 2D screen coordinate (pixels from top-left) into a 3D ray (origin, normalized direction).
    pub fn screen_to_ray(&self, screen_pos: Vec2, viewport_size: Vec2) -> (Vec3, Vec3) {
        let ndc_x = (2.0 * screen_pos.x / viewport_size.x.max(1.0)) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_pos.y / viewport_size.y.max(1.0));

        let inv_view_proj = self.build_view_projection_matrix().inverse();

        let p_near = inv_view_proj.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let p_far = inv_view_proj.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

        let ray_dir = (p_far - p_near).normalize();
        (p_near, ray_dir)
    }

    /// Intersects a ray with a sphere. Returns the closest surface intersection if it hits.
    pub fn ray_intersect_sphere(
        ray_origin: Vec3,
        ray_dir: Vec3,
        center: Vec3,
        radius: f32,
    ) -> Option<Vec3> {
        let oc = ray_origin - center;
        let a = ray_dir.dot(ray_dir);
        let b = 2.0 * oc.dot(ray_dir);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                Some(ray_origin + ray_dir * t)
            } else {
                let t2 = (-b + discriminant.sqrt()) / (2.0 * a);
                if t2 > 0.0 {
                    Some(ray_origin + ray_dir * t2)
                } else {
                    None
                }
            }
        }
    }

    /// Intersects ray with sphere; if ray misses, projects onto the sphere's horizon limb circle
    /// as seen from `ray_origin` so there is no discontinuity when a drag crosses the limb.
    pub fn ray_intersect_sphere_or_silhouette(
        ray_origin: Vec3,
        ray_dir: Vec3,
        center: Vec3,
        radius: f32,
    ) -> Vec3 {
        if let Some(hit) = Self::ray_intersect_sphere(ray_origin, ray_dir, center, radius) {
            hit
        } else {
            let oc = ray_origin - center;
            let dist = oc.length();
            if dist <= radius {
                return center + ray_dir * radius;
            }
            let normal = oc / dist;
            let cos_limb = (radius / dist).clamp(0.0, 1.0);
            let sin_limb = (1.0 - cos_limb * cos_limb).sqrt();
            let limb_center = center + normal * (radius * cos_limb);
            let denom = ray_dir.dot(normal);
            let plane_hit = if denom.abs() > 1e-4 {
                let t = (limb_center - ray_origin).dot(normal) / denom;
                ray_origin + ray_dir * t.max(0.0)
            } else {
                ray_origin + ray_dir
            };
            let radial = plane_hit - limb_center;
            let radial_dir = (radial - normal * radial.dot(normal))
                .try_normalize()
                .unwrap_or_else(|| {
                    (ray_dir - normal * ray_dir.dot(normal))
                        .try_normalize()
                        .unwrap_or(Vec3::X)
                });
            center + (normal * cos_limb + radial_dir * sin_limb) * radius
        }
    }
}
