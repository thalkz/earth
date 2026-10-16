mod input;
mod state;
mod time;

pub use input::{Action, InputState};
pub use state::State;
pub use time::Clock;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler, event::*, event_loop::ActiveEventLoop, keyboard::PhysicalKey,
    window::Window,
};

pub struct App {
    state: Option<State>,
    input: InputState,
    clock: Clock,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            input: InputState::new(),
            clock: Clock::new(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();
        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );
        self.state =
            Some(pollster::block_on(State::new(window)).expect("should block on state creation"));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let dt = self.clock.tick();
                state.update(dt, &self.input);
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                self.input.update_key(code, key_state.is_pressed());
                if self.input.is_active(Action::Exit) {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
