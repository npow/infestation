use macroquad::prelude::*;
use quad_gamepad::{GamepadAxis, GamepadButton, GamepadContext};

use crate::direction::Dir4;

const REPEAT_DELAY: f32 = 0.5;
const REPEAT_RATE: f32 = 0.05;
const STICK_THRESHOLD: f32 = 0.5;
const SWIPE_THRESHOLD: f32 = 30.0;

/// A parsed input action from keyboard, gamepad, or gesture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Input {
    Move(Dir4),
    Undo,
    Restart,
    Exit,
    Confirm,
}

/// A touch gesture result.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TouchGesture {
    Swipe(Dir4),
    Tap(Vec2),
}

/// Tracks held state for input repeat and touch gesture detection.
pub(crate) struct InputState {
    held: [f32; 4],
    held_undo: f32,
    held_confirm: f32,
    stick_active: [bool; 4],
    touch_start: Option<(u64, Vec2)>,
    touch_handled_this_frame: bool,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            held: [0.0; 4],
            held_undo: 0.0,
            held_confirm: 0.0,
            stick_active: [false; 4],
            touch_start: None,
            touch_handled_this_frame: false,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.held = [0.0; 4];
        self.held_undo = 0.0;
        self.held_confirm = 0.0;
        self.stick_active = [false; 4];
    }

    /// Poll keyboard and gamepad for inputs this frame.
    pub(crate) fn poll_keyboard_gamepad(
        &mut self,
        gamepad: &GamepadContext,
        dt: f32,
    ) -> Vec<Input> {
        let mut inputs = Vec::new();

        // Restart (R / LB / LT)
        if is_pressed_multi(
            KeyCode::R,
            &[GamepadButton::LeftShoulder, GamepadButton::LeftTrigger],
            gamepad,
        ) {
            inputs.push(Input::Restart);
        }

        // Undo with repeat (U / X / Y)
        let undo_down = is_down_multi(
            KeyCode::U,
            &[GamepadButton::West, GamepadButton::North],
            gamepad,
        );
        let undo_pressed = is_pressed_multi(
            KeyCode::U,
            &[GamepadButton::West, GamepadButton::North],
            gamepad,
        );
        if input_repeat(undo_down, undo_pressed, &mut self.held_undo, dt) {
            inputs.push(Input::Undo);
        }

        // Confirm with repeat (Space / A / B)
        let confirm_down = is_down_multi(
            KeyCode::Space,
            &[GamepadButton::South, GamepadButton::East],
            gamepad,
        );
        let confirm_pressed = is_pressed_multi(
            KeyCode::Space,
            &[GamepadButton::South, GamepadButton::East],
            gamepad,
        );
        if input_repeat(confirm_down, confirm_pressed, &mut self.held_confirm, dt) {
            inputs.push(Input::Confirm);
        }

        // Exit (Escape / Menu/Start)
        if is_pressed_multi(KeyCode::Escape, &[GamepadButton::Start], gamepad) {
            inputs.push(Input::Exit);
        }

        // D-pad and keyboard arrows with repeat
        for (key, btn, dir, idx) in [
            (KeyCode::Up, GamepadButton::DPadUp, Dir4::North, 0),
            (KeyCode::Down, GamepadButton::DPadDown, Dir4::South, 1),
            (KeyCode::Left, GamepadButton::DPadLeft, Dir4::West, 2),
            (KeyCode::Right, GamepadButton::DPadRight, Dir4::East, 3),
        ] {
            let down = is_down(key, btn, gamepad);
            let pressed = is_pressed(key, btn, gamepad);
            if input_repeat(down, pressed, &mut self.held[idx], dt) {
                inputs.push(Input::Move(dir));
            }
        }

        // Left analog stick (edge-triggered, no repeat, pick dominant axis)
        let stick_y = stick_value(GamepadAxis::LeftY, gamepad);
        let stick_x = stick_value(GamepadAxis::LeftX, gamepad);

        let stick_dir = if stick_y.abs() > stick_x.abs() {
            if stick_y > 0.0 {
                Some((Dir4::North, 0))
            } else if stick_y < 0.0 {
                Some((Dir4::South, 1))
            } else {
                None
            }
        } else if stick_x < 0.0 {
            Some((Dir4::West, 2))
        } else if stick_x > 0.0 {
            Some((Dir4::East, 3))
        } else {
            None
        };

        if let Some((dir, idx)) = stick_dir
            && !self.stick_active[idx]
        {
            inputs.push(Input::Move(dir));
        }
        let active_idx = stick_dir.map(|(_, idx)| idx);
        for i in 0..4 {
            self.stick_active[i] = active_idx == Some(i);
        }

        inputs
    }

    /// Poll touch input for gestures. Call once per frame before poll_mouse_click.
    pub(crate) fn poll_touch(&mut self) -> Option<TouchGesture> {
        self.touch_handled_this_frame = false;

        for touch in touches() {
            match touch.phase {
                TouchPhase::Started => {
                    self.touch_start = Some((touch.id, touch.position));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if let Some((start_id, start_pos)) = self.touch_start.take() {
                        if start_id != touch.id {
                            continue;
                        }
                        self.touch_handled_this_frame = true;
                        let delta = touch.position - start_pos;

                        return Some(if delta.length() >= SWIPE_THRESHOLD {
                            TouchGesture::Swipe(swipe_to_direction(delta))
                        } else {
                            TouchGesture::Tap(start_pos)
                        });
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Poll mouse click. Returns click position if clicked and touch isn't active.
    pub(crate) fn poll_mouse_click(&self) -> Option<Vec2> {
        if self.touch_start.is_some() || self.touch_handled_this_frame {
            return None;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();
            Some(Vec2::new(x, y))
        } else {
            None
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

fn is_down(key: KeyCode, btn: GamepadButton, gp: &GamepadContext) -> bool {
    is_key_down(key) || gp.gamepad(0).is_some_and(|g| g.is_button_down(btn))
}

fn is_pressed(key: KeyCode, btn: GamepadButton, gp: &GamepadContext) -> bool {
    is_key_pressed(key) || gp.gamepad(0).is_some_and(|g| g.is_button_pressed(btn))
}

fn is_down_multi(key: KeyCode, btns: &[GamepadButton], gp: &GamepadContext) -> bool {
    is_key_down(key)
        || gp
            .gamepad(0)
            .is_some_and(|g| btns.iter().any(|&btn| g.is_button_down(btn)))
}

fn is_pressed_multi(key: KeyCode, btns: &[GamepadButton], gp: &GamepadContext) -> bool {
    is_key_pressed(key)
        || gp
            .gamepad(0)
            .is_some_and(|g| btns.iter().any(|&btn| g.is_button_pressed(btn)))
}

fn stick_value(axis: GamepadAxis, gp: &GamepadContext) -> f32 {
    let v = gp.gamepad(0).map(|g| g.axis(axis)).unwrap_or(0.0);
    if v.abs() > STICK_THRESHOLD { v } else { 0.0 }
}

fn input_repeat(down: bool, pressed: bool, held: &mut f32, dt: f32) -> bool {
    if down {
        *held += dt;
        pressed || (*held > REPEAT_DELAY && *held % REPEAT_RATE < dt)
    } else {
        *held = 0.0;
        false
    }
}

fn swipe_to_direction(delta: Vec2) -> Dir4 {
    if delta.x.abs() > delta.y.abs() {
        if delta.x > 0.0 {
            Dir4::East
        } else {
            Dir4::West
        }
    } else if delta.y > 0.0 {
        Dir4::South
    } else {
        Dir4::North
    }
}
