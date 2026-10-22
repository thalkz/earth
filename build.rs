use anyhow::Result;
use fs_extra::dir::CopyOptions;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // This tells Cargo to rerun this script if something in res/earth changes.
    println!("cargo:rerun-if-changed=res/earth/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;

    let dest = Path::new(&out_dir).join("res");
    std::fs::create_dir_all(&dest)?;

    let paths_to_copy = vec!["res/earth", "res/cube"];
    let existing_paths: Vec<_> = paths_to_copy
        .into_iter()
        .filter(|p| Path::new(p).exists())
        .collect();
    fs_extra::copy_items(&existing_paths, &dest, &copy_options)?;

    convert_elevation_if_missing(&out_dir)?;
    convert_diffuse_if_missing(&out_dir)?;

    Ok(())
}

fn convert_elevation_if_missing(out_dir: &str) -> Result<()> {
    let out_dds = Path::new(out_dir).join("res").join("earth").join("elevation.dds");
    let src_gebco = Path::new("res").join("in").join("gebco_2026_geotiff");
    let src_gebco_alt = Path::new("in").join("gebco_2026_geotiff");
    if !out_dds.exists() && (src_gebco.exists() || src_gebco_alt.exists()) {
        println!("cargo:warning=res/earth/elevation.dds is missing in OUT_DIR. Please run `cargo run --release --bin convert_textures` to precompute normals.");
    }
    Ok(())
}

fn convert_diffuse_if_missing(out_dir: &str) -> Result<()> {
    let out_dds = Path::new(out_dir).join("res").join("earth").join("diffuse.dds");
    let src_tif = Path::new("res").join("in").join("diffuse.tif");
    let src_tif_alt = Path::new("in").join("diffuse.tif");
    if !out_dds.exists() && (src_tif.exists() || src_tif_alt.exists()) {
        println!("cargo:warning=res/earth/diffuse.dds is missing in OUT_DIR. Please run `cargo run --release --bin convert_textures`.");
    }
    Ok(())
}

