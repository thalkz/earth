use glam::Vec3;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub indices: Vec<u32>,
    pub lod_ranges: [Range<u32>; 4],
    pub diffuse: ImageAsset,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
}

#[derive(Clone, Debug)]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub is_cubemap: bool,
    pub data: Vec<u8>,
    pub mip_offsets: Vec<Range<usize>>,
}

impl ImageAsset {
    pub fn mip_levels(&self) -> impl Iterator<Item = &[u8]> {
        self.mip_offsets
            .iter()
            .map(|range| &self.data[range.clone()])
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: Vec3,
}
