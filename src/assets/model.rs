#[derive(Clone, Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_index: usize,
}

#[derive(Clone, Debug)]
pub enum MaterialDiffuse {
    Color([f32; 4]),
    Texture(ImageAsset),
}

#[derive(Clone, Debug)]
pub struct Material {
    pub name: String,
    pub diffuse: MaterialDiffuse,
}

#[derive(Clone, Debug)]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImageAsset {
    pub fn from_color(rgba: [f32; 4]) -> Self {
        Self {
            width: 1,
            height: 1,
            data: rgba.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8).to_vec(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}
