use crate::assets;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmosVertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl GizmosVertex {
    fn new(position: [f32; 3], color: [f32; 4]) -> GizmosVertex {
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

    pub fn add_model(&mut self, model: &assets::Model, color: [f32; 4]) {
        for mesh in &model.meshes {
            let mut i = 0;
            while i < (mesh.indices.len() - 2) {
                let a = mesh.vertices[mesh.indices[i] as usize].position;
                let b = mesh.vertices[mesh.indices[i + 1] as usize].position;
                let c = mesh.vertices[mesh.indices[i + 2] as usize].position;
                self.add_line(a, b, color);
                self.add_line(b, c, color);
                self.add_line(c, a, color);
                i += 3;
            }
        }
    }

    pub fn add_line(&mut self, from: [f32; 3], to: [f32; 3], color: [f32; 4]) {
        self.vertices.push(GizmosVertex::new(from, color));
        self.vertices.push(GizmosVertex::new(to, color));
    }

    pub fn add_point(&mut self, at: [f32; 3], color: [f32; 4]) {
        todo!()
    }
}
