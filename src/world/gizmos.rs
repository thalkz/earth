use crate::assets::Model;
use glam::Vec3;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmosVertex {
    pub position: Vec3,
    pub color: [f32; 4],
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

    pub fn add_model(&mut self, model: &Model, color: [f32; 4]) {
        for mesh in &model.meshes {
            for triangle in mesh.indices.as_chunks::<3>().0 {
                let a = mesh.vertices[triangle[0] as usize].position.into();
                let b = mesh.vertices[triangle[1] as usize].position.into();
                self.add_line(a, b, color);
            }
        }
    }

    pub fn add_line(&mut self, from: Vec3, to: Vec3, color: [f32; 4]) {
        self.vertices.push(GizmosVertex::new(from, color));
        self.vertices.push(GizmosVertex::new(to, color));
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }
}
