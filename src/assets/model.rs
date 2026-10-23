use std::ops::Range;

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
    pub lod_ranges: Vec<Range<u32>>,
    pub material_index: usize,
}

impl Mesh {
    pub fn with_lods(
        name: String,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        lod_ranges: Vec<Range<u32>>,
        material_index: usize,
    ) -> Self {
        Self {
            name,
            vertices,
            indices,
            lod_ranges,
            material_index,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Material {
    pub name: String,
    pub diffuse: ImageAsset,
}

#[derive(Clone, Debug)]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub mip_levels: Vec<Vec<u8>>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}
