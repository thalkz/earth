use winit::event_loop::EventLoop;

use crate::app::App;

mod app;
mod assets;
mod gpu;
mod renderer;
mod scene;

pub fn run() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
