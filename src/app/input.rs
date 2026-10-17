use crate::world::{Action, CameraActionState};
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

fn to_action(key: KeyCode) -> Option<Action> {
    match key {
        KeyCode::KeyW | KeyCode::ArrowUp => Some(Action::MoveForward),
        KeyCode::KeyS | KeyCode::ArrowDown => Some(Action::MoveBackward),
        KeyCode::KeyA | KeyCode::ArrowLeft => Some(Action::MoveLeft),
        KeyCode::KeyD | KeyCode::ArrowRight => Some(Action::MoveRight),
        KeyCode::Space => Some(Action::ShowGizmos),
        _ => None,
    }
}
