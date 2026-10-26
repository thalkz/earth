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
pub enum ImageAsset {
    Cubemap {
        width: u32,
        #[allow(dead_code)]
        height: u32,
        format: wgpu::TextureFormat,
        data: Vec<u8>,
        mip_offsets: Vec<Range<usize>>,
    },
    Texture2D {
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        data: Vec<u8>,
        mip_offsets: Vec<Range<usize>>,
    },
}

impl ImageAsset {
    pub fn mip_levels(&self) -> impl Iterator<Item = &[u8]> {
        let (data, mip_offsets) = match self {
            ImageAsset::Cubemap {
                data, mip_offsets, ..
            }
            | ImageAsset::Texture2D {
                data, mip_offsets, ..
            } => (data, mip_offsets),
        };
        mip_offsets.iter().map(|range| &data[range.clone()])
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}
