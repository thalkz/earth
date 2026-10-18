use anyhow::Result;
use fs_extra::dir::CopyOptions;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const RAW_MAGIC: &[u8; 4] = b"EIMG";

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
    let out_raw = Path::new(out_dir).join("res").join("earth").join("elevation.raw");
    let src_png = Path::new("res").join("earth").join("elevation.png");
    if !out_raw.exists() && src_png.exists() {
        println!("cargo:warning=Generating elevation.raw in OUT_DIR...");
        let img = image::open(&src_png)?;
        let luma = img.into_luma16();
        let (width, height) = luma.dimensions();
        let raw_u16 = luma.into_raw();
        let raw_bytes: &[u8] = bytemuck::cast_slice(&raw_u16);

        let mut file = File::create(&out_raw)?;
        file.write_all(RAW_MAGIC)?;
        file.write_all(&width.to_le_bytes())?;
        file.write_all(&height.to_le_bytes())?;
        file.write_all(&2u32.to_le_bytes())?; // 2 bytes per pixel
        file.write_all(raw_bytes)?;
    }
    Ok(())
}

fn convert_diffuse_if_missing(out_dir: &str) -> Result<()> {
    let out_raw = Path::new(out_dir).join("res").join("earth").join("diffuse.raw");
    let src_png = Path::new("res").join("earth").join("diffuse.png");
    if !out_raw.exists() && src_png.exists() {
        println!("cargo:warning=Generating diffuse.raw in OUT_DIR...");
        let img = image::open(&src_png)?;
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        let raw_bytes = rgba.into_raw();

        let mut file = File::create(&out_raw)?;
        file.write_all(RAW_MAGIC)?;
        file.write_all(&width.to_le_bytes())?;
        file.write_all(&height.to_le_bytes())?;
        file.write_all(&4u32.to_le_bytes())?; // 4 bytes per pixel
        file.write_all(&raw_bytes)?;
    }
    Ok(())
}

