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

    let paths_to_copy = vec!["res/earth"];
    let existing_paths: Vec<_> = paths_to_copy
        .into_iter()
        .filter(|p| Path::new(p).exists())
        .collect();
    fs_extra::copy_items(&existing_paths, &dest, &copy_options)?;

    Ok(())
}
