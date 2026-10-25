use glam::Vec2;
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase};
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub start_pos: Vec2,
    pub prev_pos: Vec2,
    pub curr_pos: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub enum TouchGesture {
    None,
    /// Single-finger surface-pinned great-circle globe pan
    SinglePan {
        prev_pos: Vec2,
        curr_pos: Vec2,
        just_started: bool,
    },
    /// Double-tap + 1-finger vertical drag to zoom in/out
    SingleZoomDrag {
        prev_y: f32,
        curr_y: f32,
        anchor_pos: Vec2,
    },
    /// Quick double-tap impulse zoom toward tapped location
    DoubleTap {
        anchor_pos: Vec2,
    },
    /// Two-finger pinch-to-zoom, twist-to-rotate bearing, and vertical drag to tilt pitch
    TwoFinger {
        prev0: Vec2,
        curr0: Vec2,
        prev1: Vec2,
        curr1: Vec2,
    },
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    cursor_position: Option<Vec2>,
    cursor_delta: Vec2,
    wheel_delta: f32,
    pinch_delta: f32,
    modifiers: ModifiersState,

    // Multi-touch state (deterministic order by touch id)
    touches: BTreeMap<u64, TouchPoint>,
    single_pan_just_started: bool,
    multi_touch_latched: bool,
    single_touch_start_time: Option<Instant>,
    last_single_tap: Option<(Instant, Vec2)>,
    double_tap_active: bool,
    double_tap_moved: bool,
    double_tap_anchor: Vec2,
    pending_double_tap: Option<Vec2>,
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
        let now = Instant::now();

        match touch.phase {
            TouchPhase::Started => {
                if self.touches.is_empty() {
                    let is_double = if let Some((tap_time, tap_pos)) = self.last_single_tap {
                        now.duration_since(tap_time).as_millis() < 320
                            && pos.distance(tap_pos) < 80.0
                    } else {
                        false
                    };

                    if is_double {
                        self.double_tap_active = true;
                        self.double_tap_moved = false;
                        self.double_tap_anchor = pos;
                        self.last_single_tap = None;
                        self.single_pan_just_started = false;
                    } else {
                        self.double_tap_active = false;
                        self.double_tap_moved = false;
                        self.single_pan_just_started = true;
                        self.single_touch_start_time = Some(now);
                    }
                    self.multi_touch_latched = false;
                }

                self.touches.insert(
                    touch.id,
                    TouchPoint {
                        start_pos: pos,
                        prev_pos: pos,
                        curr_pos: pos,
                    },
                );

                if self.touches.len() >= 2 {
                    self.multi_touch_latched = true;
                    self.double_tap_active = false;
                    self.single_pan_just_started = false;
                    for t in self.touches.values_mut() {
                        t.prev_pos = t.curr_pos;
                        t.start_pos = t.curr_pos;
                    }
                }
            }
            TouchPhase::Moved => {
                if let Some(t) = self.touches.get_mut(&touch.id) {
                    t.curr_pos = pos;
                    if self.double_tap_active && t.curr_pos.distance(t.start_pos) > 12.0 {
                        self.double_tap_moved = true;
                    }
                }

                // If 1 finger remains after a 2-finger gesture and purposefully drags > 35px,
                // smoothly transition into a fresh 1-finger pan without jumping.
                if self.touches.len() == 1 && self.multi_touch_latched {
                    if let Some(t) = self.touches.values_mut().next() {
                        if t.curr_pos.distance(t.start_pos) > 35.0 {
                            self.multi_touch_latched = false;
                            self.single_pan_just_started = true;
                            t.start_pos = t.curr_pos;
                            t.prev_pos = t.curr_pos;
                        }
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let removed = self.touches.remove(&touch.id);

                if self.touches.is_empty() {
                    if self.double_tap_active && !self.double_tap_moved {
                        self.pending_double_tap = Some(self.double_tap_anchor);
                    } else if !self.multi_touch_latched && !self.double_tap_active {
                        if let (Some(start_time), Some(pt)) = (self.single_touch_start_time, removed)
                        {
                            if now.duration_since(start_time).as_millis() < 260
                                && pt.curr_pos.distance(pt.start_pos) < 30.0
                            {
                                self.last_single_tap = Some((now, pt.curr_pos));
                            }
                        }
                    }
                    self.multi_touch_latched = false;
                    self.double_tap_active = false;
                    self.single_pan_just_started = false;
                    self.single_touch_start_time = None;
                } else if self.touches.len() == 1 {
                    // Transitioning from 2 fingers down to 1 finger: latch to prevent post-pinch jump
                    self.multi_touch_latched = true;
                    self.single_pan_just_started = false;
                    if let Some(t) = self.touches.values_mut().next() {
                        t.start_pos = t.curr_pos;
                        t.prev_pos = t.curr_pos;
                    }
                }
            }
        }
    }

    pub fn touch_gesture(&self) -> TouchGesture {
        if let Some(anchor_pos) = self.pending_double_tap {
            return TouchGesture::DoubleTap { anchor_pos };
        }

        match self.touches.len() {
            0 => TouchGesture::None,
            1 => {
                if self.multi_touch_latched {
                    return TouchGesture::None;
                }
                let t = self.touches.values().next().unwrap();
                if self.double_tap_active {
                    TouchGesture::SingleZoomDrag {
                        prev_y: t.prev_pos.y,
                        curr_y: t.curr_pos.y,
                        anchor_pos: self.double_tap_anchor,
                    }
                } else {
                    TouchGesture::SinglePan {
                        prev_pos: t.prev_pos,
                        curr_pos: t.curr_pos,
                        just_started: self.single_pan_just_started,
                    }
                }
            }
            _ => {
                let mut iter = self.touches.values();
                let t0 = iter.next().unwrap();
                let t1 = iter.next().unwrap();
                TouchGesture::TwoFinger {
                    prev0: t0.prev_pos,
                    curr0: t0.curr_pos,
                    prev1: t1.prev_pos,
                    curr1: t1.curr_pos,
                }
            }
        }
    }

    pub fn has_active_touches(&self) -> bool {
        !self.touches.is_empty()
    }

    pub fn clear_frame_deltas(&mut self) {
        self.cursor_delta = Vec2::ZERO;
        self.wheel_delta = 0.0;
        self.pinch_delta = 0.0;
        self.single_pan_just_started = false;
        self.pending_double_tap = None;
        for t in self.touches.values_mut() {
            t.prev_pos = t.curr_pos;
        }
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
