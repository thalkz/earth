use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

const RAW_MAGIC: &[u8; 4] = b"EIMG";

fn convert_png_to_raw_16(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    println!("Converting {:?} to 16-bit raw...", input_path);
    let start = Instant::now();
    let img = image::open(input_path)?;
    let luma = img.into_luma16();
    let (width, height) = luma.dimensions();
    let raw_u16 = luma.into_raw();
    let raw_bytes: &[u8] = bytemuck::cast_slice(&raw_u16);

    let mut file = File::create(output_path)?;
    file.write_all(RAW_MAGIC)?;
    file.write_all(&width.to_le_bytes())?;
    file.write_all(&height.to_le_bytes())?;
    file.write_all(&2u32.to_le_bytes())?; // 2 bytes per pixel (16-bit unorm)
    file.write_all(raw_bytes)?;
    println!(
        "Done in {:?}! Saved {:?} ({}x{}, {} bytes)",
        start.elapsed(),
        output_path,
        width,
        height,
        raw_bytes.len() + 16
    );
    Ok(())
}

fn convert_png_to_raw_rgba8(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    println!("Converting {:?} to RGBA8 raw...", input_path);
    let start = Instant::now();
    let img = image::open(input_path)?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    let raw_bytes = rgba.into_raw();

    let mut file = File::create(output_path)?;
    file.write_all(RAW_MAGIC)?;
    file.write_all(&width.to_le_bytes())?;
    file.write_all(&height.to_le_bytes())?;
    file.write_all(&4u32.to_le_bytes())?; // 4 bytes per pixel (RGBA8)
    file.write_all(&raw_bytes)?;
    println!(
        "Done in {:?}! Saved {:?} ({}x{}, {} bytes)",
        start.elapsed(),
        output_path,
        width,
        height,
        raw_bytes.len() + 16
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let elev_png = Path::new("res/earth/elevation.png");
    let elev_raw = Path::new("res/earth/elevation.raw");
    if elev_png.exists() {
        convert_png_to_raw_16(elev_png, elev_raw)?;
    }

    let diff_png = Path::new("res/earth/diffuse.png");
    let diff_raw = Path::new("res/earth/diffuse.raw");
    if diff_png.exists() {
        convert_png_to_raw_rgba8(diff_png, diff_raw)?;
    }

    Ok(())
}
