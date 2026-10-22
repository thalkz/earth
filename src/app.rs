mod app_handler;
mod framerate_logger;
mod input;
mod state;
mod time;
mod tracing;

use crate::app::app_handler::App;
pub use framerate_logger::FramerateLogger;
pub use input::InputState;
pub use state::State;
pub use time::Clock;
pub use tracing::init_tracing;
use winit::event_loop::EventLoop;

pub fn run() -> anyhow::Result<()> {
    init_tracing()?;
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
