#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GizmosVertex {
    position: [f32; 3],
    color: [f32; 4],
}
pub const GIZMOS_VERTEX_SIZE: usize = 7;

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

    pub fn add_line(&mut self, from: [f32; 3], to: [f32; 3], color: [f32; 4]) {
        self.vertices.push(GizmosVertex::new(from, color));
        self.vertices.push(GizmosVertex::new(to, color));
    }

    pub fn add_point(&mut self, at: [f32; 3], color: [f32; 4]) {
        todo!()
    }
}
