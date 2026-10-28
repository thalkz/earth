mod app_handler;
mod framerate_logger;
mod input;
mod state;

use crate::app::app_handler::App;
pub use framerate_logger::FramerateLogger;
pub use input::{InputState, TouchGesture};
pub use state::State;
use winit::event_loop::EventLoop;

pub fn run() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(target_os = "android")]
pub fn run_android(android_app: winit::platform::android::activity::AndroidApp) -> anyhow::Result<()> {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let _ = env_logger::try_init();
    crate::assets::init_android_app(android_app.clone());

    let event_loop = EventLoop::builder()
        .with_android_app(android_app)
        .build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
