use crate::assets::Model;
use glam::Vec3;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmosVertex {
    pub position: Vec3,
    pub color: [f32; 4],
}

#[derive(Default)]
pub struct Gizmos {
    pub vertices: Vec<GizmosVertex>,
}

impl Gizmos {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_model(&mut self, model: &Model, color: [f32; 4]) {
        let lod0 = &model.lod_ranges[0];
        let indices = &model.indices[lod0.start as usize..lod0.end as usize];
        for mesh in &model.meshes {
            for &[i0, i1, i2] in indices.as_chunks::<3>().0 {
                let a = mesh.vertices[i0 as usize].position;
                let b = mesh.vertices[i1 as usize].position;
                let c = mesh.vertices[i2 as usize].position;
                self.add_line(a, b, color);
                self.add_line(b, c, color);
                self.add_line(c, a, color);
            }
        }
    }

    pub fn add_line(&mut self, from: Vec3, to: Vec3, color: [f32; 4]) {
        self.vertices.push(GizmosVertex {
            position: from,
            color,
        });
        self.vertices.push(GizmosVertex {
            position: to,
            color,
        });
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }
}
