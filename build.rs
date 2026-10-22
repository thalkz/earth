use anyhow::Result;
use fs_extra::dir::CopyOptions;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // This tells Cargo to rerun this script if something in /res/ changes.
    println!("cargo:rerun-if-changed=res/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let paths_to_copy = vec!["res/"];
    fs_extra::copy_items(&paths_to_copy, &out_dir, &copy_options)?;

    convert_elevation_if_missing(&out_dir)?;
    convert_diffuse_if_missing(&out_dir)?;

    Ok(())
}

fn convert_elevation_if_missing(out_dir: &str) -> Result<()> {
    let out_dds = Path::new(out_dir).join("res").join("earth").join("elevation.dds");
    let src_png = Path::new("res").join("earth").join("elevation.png");
    if !out_dds.exists() && src_png.exists() {
        println!("cargo:warning=res/earth/elevation.dds is missing in OUT_DIR. Please run `cargo run --release --bin convert_textures` to precompute normals.");
    }
    Ok(())
}

fn convert_diffuse_if_missing(out_dir: &str) -> Result<()> {
    let out_dds = Path::new(out_dir).join("res").join("earth").join("diffuse.dds");
    let src_png = Path::new("res").join("earth").join("diffuse.png");
    if !out_dds.exists() && src_png.exists() {
        println!("cargo:warning=res/earth/diffuse.dds is missing in OUT_DIR. Please run `cargo run --release --bin convert_textures`.");
    }
    Ok(())
}

