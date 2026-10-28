mod app;
mod assets;
mod geo;
mod gpu;
mod renderer;
mod world;

pub use app::run;
pub use assets::{DDS_MAGIC, DdsHeader, ImageAsset, parse_dds};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    app::run_android(app).expect("failed to run on android");
}
