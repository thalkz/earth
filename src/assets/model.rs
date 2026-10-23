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
    pub fn new(
        name: String,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        material_index: usize,
    ) -> Self {
        let count = indices.len() as u32;
        Self {
            name,
            vertices,
            indices,
            lod_ranges: vec![0..count],
            material_index,
        }
    }

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
    pub mip_levels: Vec<Vec<u8>>,
}

impl ImageAsset {
    pub fn from_color(rgba: [f32; 4]) -> Self {
        Self {
            width: 1,
            height: 1,
            mip_levels: vec![rgba.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8).to_vec()],
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
