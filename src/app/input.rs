use crate::scene::{CameraAction, CameraActionState};
use std::collections::HashSet;
use winit::keyboard::KeyCode;

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

    pub fn is_exit_pressed(&self) -> bool {
        self.is_key_pressed(KeyCode::Escape)
    }
}

impl From<&InputState> for CameraActionState {
    fn from(value: &InputState) -> Self {
        // TODO avoid allocating a new hashmap every frame
        let actions = value
            .pressed_keys
            .iter()
            .copied()
            .filter_map(to_action)
            .collect();
        CameraActionState { actions }
    }
}

fn to_action(key: KeyCode) -> Option<CameraAction> {
    match key {
        KeyCode::KeyW | KeyCode::ArrowUp => Some(CameraAction::MoveForward),
        KeyCode::KeyS | KeyCode::ArrowDown => Some(CameraAction::MoveBackward),
        KeyCode::KeyA | KeyCode::ArrowLeft => Some(CameraAction::MoveLeft),
        KeyCode::KeyD | KeyCode::ArrowRight => Some(CameraAction::MoveRight),
        _ => None,
    }
}
