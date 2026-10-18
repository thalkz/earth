use crate::world::{Action, CameraActionState};
use glam::Vec2;
use std::collections::HashSet;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    cursor_position: Option<Vec2>,
    cursor_delta: Vec2,
    wheel_delta: f32,
    pinch_delta: f32,
    modifiers: ModifiersState,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_key(&mut self, code: KeyCode, is_pressed: bool) {
        if is_pressed {
            self.pressed_keys.insert(code);
        } else {
            self.pressed_keys.remove(&code);
        }
    }

    pub fn update_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.pressed_mouse_buttons.insert(button);
            }
            ElementState::Released => {
                self.pressed_mouse_buttons.remove(&button);
            }
        }
    }

    pub fn update_cursor_position(&mut self, new_pos: Vec2) {
        if let Some(prev) = self.cursor_position {
            self.cursor_delta += new_pos - prev;
        }
        self.cursor_position = Some(new_pos);
    }

    pub fn update_scroll(&mut self, delta: MouseScrollDelta) {
        let amount = match delta {
            MouseScrollDelta::LineDelta(_x, y) => y,
            MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) / 20.0,
        };
        self.wheel_delta += amount;
    }

    pub fn update_pinch(&mut self, delta: f32) {
        self.pinch_delta += delta;
    }

    pub fn update_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn clear_frame_deltas(&mut self) {
        self.cursor_delta = Vec2::ZERO;
        self.wheel_delta = 0.0;
        self.pinch_delta = 0.0;
    }

    pub fn cursor_position(&self) -> Option<Vec2> {
        self.cursor_position
    }

    pub fn cursor_delta(&self) -> Vec2 {
        self.cursor_delta
    }

    pub fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }

    pub fn pinch_delta(&self) -> f32 {
        self.pinch_delta
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    pub fn is_key_pressed(&self, code: KeyCode) -> bool {
        self.pressed_keys.contains(&code)
    }

    pub fn is_shift_down(&self) -> bool {
        self.modifiers.shift_key()
    }

    pub fn is_ctrl_down(&self) -> bool {
        self.modifiers.control_key() || self.modifiers.super_key()
    }

    #[allow(dead_code)]
    pub fn is_alt_down(&self) -> bool {
        self.modifiers.alt_key()
    }

    pub fn is_exit_pressed(&self) -> bool {
        self.is_key_pressed(KeyCode::Escape)
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

