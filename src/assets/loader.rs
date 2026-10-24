use crate::assets::{ImageAsset, dds};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_dds_image(filename: &str) -> anyhow::Result<ImageAsset> {
    let data = load_binary(filename)?;
    dds::parse_dds(data)
}

fn resolve_resource_path(filename: &str) -> PathBuf {
    // 1. Check relative to current working directory: ./res/<filename>
    let relative = Path::new("res").join(filename);
    if relative.exists() {
        return relative;
    }

    // 2. Check relative to executable directory: <exe_dir>/res/<filename>
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_relative = exe_dir.join("res").join(filename);
            if exe_relative.exists() {
                return exe_relative;
            }
        }
    }

    // 3. Fallback to compile-time crate root (e.g. for tests running in different directories)
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join(filename)
}

pub fn load_binary(filename: &str) -> anyhow::Result<Vec<u8>> {
    let path = resolve_resource_path(filename);
    let data = fs::read(&path)?;
    Ok(data)
}
