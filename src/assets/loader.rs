use crate::assets::{ImageAsset, dds};
use std::fs;
use std::path::Path;

const OUT_DIR: &str = env!("OUT_DIR");

pub fn load_dds_image(filename: &str) -> anyhow::Result<ImageAsset> {
    let data = load_binary(filename)?;
    dds::parse_dds(data)
}

pub fn load_binary(filename: &str) -> anyhow::Result<Vec<u8>> {
    let base_path = Path::new(OUT_DIR).join("res");
    let path = base_path.join(filename);
    let data = fs::read(path)?;
    Ok(data)
}
