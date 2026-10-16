use std::collections::HashSet;
use winit::keyboard::KeyCode;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Exit,
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    pub fn update_key(&mut self, code: KeyCode, is_pressed: bool) {
        if is_pressed {
            self.pressed_keys.insert(code);
        } else {
            self.pressed_keys.remove(&code);
        }
    }

    pub fn is_key_pressed(&self, code: KeyCode) -> bool {
        self.pressed_keys.contains(&code)
    }

    pub fn is_active(&self, action: Action) -> bool {
        match action {
            Action::MoveForward => {
                self.is_key_pressed(KeyCode::KeyW) || self.is_key_pressed(KeyCode::ArrowUp)
            }
            Action::MoveBackward => {
                self.is_key_pressed(KeyCode::KeyS) || self.is_key_pressed(KeyCode::ArrowDown)
            }
            Action::MoveLeft => {
                self.is_key_pressed(KeyCode::KeyA) || self.is_key_pressed(KeyCode::ArrowLeft)
            }
            Action::MoveRight => {
                self.is_key_pressed(KeyCode::KeyD) || self.is_key_pressed(KeyCode::ArrowRight)
            }
            Action::Exit => self.is_key_pressed(KeyCode::Escape),
        }
    }
}
