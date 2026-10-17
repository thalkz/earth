use crate::assets::Model;
use crate::scene::Camera;
use glam::Vec3;
use std::f32::consts::TAU;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmosVertex {
    position: Vec3,
    color: [f32; 4],
}

impl GizmosVertex {
    fn new(position: Vec3, color: [f32; 4]) -> GizmosVertex {
        GizmosVertex { position, color }
    }
}

pub struct Gizmos {
    pub vertices: Vec<GizmosVertex>,
}

impl Gizmos {
    pub fn new() -> Gizmos {
        Gizmos {
            vertices: Vec::new(),
        }
    }

    pub fn add_model(&mut self, model: &Model, color: [f32; 4], camera: &Camera) {
        for mesh in &model.meshes {
            for triangle in mesh.indices.chunks_exact(3) {
                let a = mesh.vertices[triangle[0] as usize].position.into();
                let b = mesh.vertices[triangle[1] as usize].position.into();
                let c = mesh.vertices[triangle[2] as usize].position.into();
                self.add_line(a, b, color);
                self.add_line(b, c, color);
                self.add_line(c, a, color);
                self.add_point(a, color, camera);
            }
        }
    }

    pub fn add_line(&mut self, from: Vec3, to: Vec3, color: [f32; 4]) {
        self.vertices.push(GizmosVertex::new(from, color));
        self.vertices.push(GizmosVertex::new(to, color));
    }

    pub fn add_point(&mut self, at: Vec3, color: [f32; 4], camera: &Camera) {
        let right = camera.right_axis();
        let up = camera.up_axis();
        let step = TAU / CIRCLE_SEGMENTS as f32;

        for i in 0..CIRCLE_SEGMENTS {
            let t0 = i as f32 * step;
            let t1 = (i + 1) as f32 * step;

            let p0 = at + (right * t0.cos() + up * t0.sin()) * RADIUS;
            let p1 = at + (right * t1.cos() + up * t1.sin()) * RADIUS;

            self.add_line(p0, p1, color);
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }
}

const CIRCLE_SEGMENTS: u32 = 8;
const RADIUS: f32 = 0.02;
