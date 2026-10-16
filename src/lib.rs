use winit::event_loop::EventLoop;

use crate::app::App;

mod app;
mod camera;
mod data;
mod instance;
mod light;
mod model;
mod resources;
mod state;
mod gpu;

pub fn run() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
