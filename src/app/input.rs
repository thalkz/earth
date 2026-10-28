use glam::Vec2;
use std::collections::{HashMap, HashSet};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase};
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
    active_touches: HashMap<u64, Vec2>,
    prev_touch_distance: Option<f32>,
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

    pub fn update_touch(&mut self, touch: Touch) {
        let pos = Vec2::new(touch.location.x as f32, touch.location.y as f32);

        match touch.phase {
            TouchPhase::Started => {
                self.active_touches.insert(touch.id, pos);
                self.sync_touch_mode();
            }
            TouchPhase::Moved => {
                self.active_touches.insert(touch.id, pos);
                match self.active_touches.len() {
                    1 => {
                        self.update_cursor_position(pos);
                    }
                    2 => {
                        let mut iter = self.active_touches.values();
                        if let (Some(&p0), Some(&p1)) = (iter.next(), iter.next()) {
                            let midpoint = (p0 + p1) * 0.5;
                            let dist = p0.distance(p1);
                            self.update_cursor_position(midpoint);
                            if let Some(prev_dist) = self.prev_touch_distance {
                                let delta_dist = dist - prev_dist;
                                self.update_pinch(delta_dist * 0.004);
                            }
                            self.prev_touch_distance = Some(dist);
                        }
                    }
                    _ => {}
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.active_touches.remove(&touch.id);
                self.sync_touch_mode();
            }
        }
    }

    fn sync_touch_mode(&mut self) {
        self.prev_touch_distance = None;
        match self.active_touches.len() {
            0 => {
                self.pressed_mouse_buttons.remove(&MouseButton::Left);
                self.pressed_mouse_buttons.remove(&MouseButton::Right);
                self.cursor_position = None;
            }
            1 => {
                self.pressed_mouse_buttons.insert(MouseButton::Left);
                self.pressed_mouse_buttons.remove(&MouseButton::Right);
                if let Some(&pos) = self.active_touches.values().next() {
                    self.cursor_position = Some(pos);
                }
            }
            _ => {
                self.pressed_mouse_buttons.remove(&MouseButton::Left);
                self.pressed_mouse_buttons.insert(MouseButton::Right);
                let mut iter = self.active_touches.values();
                if let (Some(&p0), Some(&p1)) = (iter.next(), iter.next()) {
                    self.cursor_position = Some((p0 + p1) * 0.5);
                    self.prev_touch_distance = Some(p0.distance(p1));
                }
            }
        }
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

    pub fn is_exit_pressed(&self) -> bool {
        self.is_key_pressed(KeyCode::Escape)
    }
}
