use crate::assets::{ImageAsset, dds};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> =
    std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn init_android_app(app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

pub fn load_dds_image(filename: &str) -> anyhow::Result<ImageAsset> {
    let data = load_binary(filename)?;
    dds::parse_dds(data)
}

fn resolve_resource_path(filename: &str) -> PathBuf {
    let relative = Path::new("res").join(filename);
    if relative.exists() {
        return relative;
    }

    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let exe_relative = exe_dir.join("res").join(filename);
        if exe_relative.exists() {
            return exe_relative;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join(filename)
}

pub fn load_binary(filename: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_os = "android")]
    if let Some(app) = ANDROID_APP.get() {
        use std::ffi::CString;
        use std::io::Read;

        let asset_manager = app.asset_manager();
        let c_path = CString::new(filename)?;
        let mut asset = asset_manager
            .open(&c_path)
            .ok_or_else(|| anyhow::anyhow!("Android asset not found in APK: {filename}"))?;
        let mut buf = Vec::with_capacity(asset.length());
        asset.read_to_end(&mut buf)?;
        return Ok(buf);
    }

    let path = resolve_resource_path(filename);
    Ok(fs::read(path)?)
}
