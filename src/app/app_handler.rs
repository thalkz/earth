use crate::app::{Clock, FramerateLogger, InputState, State};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

pub struct App {
    state: Option<State>,
    input: InputState,
    clock: Clock,
    framerate_logger: FramerateLogger,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            input: InputState::new(),
            clock: Clock::new(),
            framerate_logger: FramerateLogger::new(),
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
        _window_id: WindowId,
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
                self.framerate_logger.tick();
                state.update(dt, &self.input);
                self.input.clear_frame_deltas();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.update_cursor_position(glam::Vec2::new(
                    position.x as f32,
                    position.y as f32,
                ));
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                self.input.update_mouse_button(button, btn_state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.update_scroll(delta);
            }
            WindowEvent::PinchGesture { delta, .. } => {
                self.input.update_pinch(delta as f32);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input.update_modifiers(modifiers.state());
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
                if self.input.is_exit_pressed() {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

