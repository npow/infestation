use macroquad::prelude::*;
use quad_gamepad::{GamepadAxis, GamepadButton, GamepadContext};

use crate::direction::Dir4;
use crate::game::Action;

const REPEAT_DELAY: f32 = 0.5;
const REPEAT_RATE: f32 = 0.05;
const STICK_THRESHOLD: f32 = 0.5;
const SWIPE_THRESHOLD: f32 = 30.0;

/// Number of input sources: [0]=arrows, [1]=WASD, [2]=gamepad0, [3]=gamepad1.
const NUM_SOURCES: usize = 4;

const ARROW_KEYS: [KeyCode; 4] = [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right];
const WASD_KEYS: [KeyCode; 4] = [KeyCode::W, KeyCode::S, KeyCode::A, KeyCode::D];
const ARROW_STALL: KeyCode = KeyCode::Space;
const WASD_STALL: KeyCode = KeyCode::Tab;
const ARROW_SYNC: KeyCode = KeyCode::RightAlt;
const WASD_SYNC: KeyCode = KeyCode::LeftAlt;

const DIRS: [Dir4; 4] = [Dir4::North, Dir4::South, Dir4::West, Dir4::East];
const DPAD_BUTTONS: [GamepadButton; 4] = [
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
];

/// A meta input action (not per-player).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MetaInput {
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
    /// Per-source directional held timers: [arrows, WASD, gamepad0, gamepad1][dir].
    held_move: [[f32; 4]; NUM_SOURCES],
    /// Per-source stall held timers: [arrows(Space), WASD(Tab), gp0, gp1].
    held_stall: [f32; NUM_SOURCES],
    /// Meta action held timers.
    held_undo: f32,
    held_confirm: f32,
    /// Per-gamepad analog stick edge detection.
    stick_active: [[bool; 4]; 2],
    touch_start: Option<(u64, Vec2)>,
    touch_handled_this_frame: bool,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            held_move: [[0.0; 4]; NUM_SOURCES],
            held_stall: [0.0; NUM_SOURCES],
            held_undo: 0.0,
            held_confirm: 0.0,
            stick_active: [[false; 4]; 2],
            touch_start: None,
            touch_handled_this_frame: false,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.held_move = [[0.0; 4]; NUM_SOURCES];
        self.held_stall = [0.0; NUM_SOURCES];
        self.held_undo = 0.0;
        self.held_confirm = 0.0;
        self.stick_active = [[false; 4]; 2];
    }

    /// Poll meta actions (undo, restart, exit, confirm) from keyboard + any gamepad.
    pub(crate) fn poll_meta_inputs(
        &mut self,
        gamepad: &GamepadContext,
        dt: f32,
    ) -> Vec<MetaInput> {
        let mut inputs = Vec::new();

        // Restart (R / LB / LT on any gamepad)
        if is_key_pressed(KeyCode::R)
            || any_gp_pressed_multi(
                gamepad,
                &[GamepadButton::LeftShoulder, GamepadButton::LeftTrigger],
            )
        {
            inputs.push(MetaInput::Restart);
        }

        // Undo with repeat (U / X / Y on any gamepad)
        let undo_down = is_key_down(KeyCode::U)
            || any_gp_down_multi(gamepad, &[GamepadButton::West, GamepadButton::North]);
        let undo_pressed = is_key_pressed(KeyCode::U)
            || any_gp_pressed_multi(gamepad, &[GamepadButton::West, GamepadButton::North]);
        if input_repeat(undo_down, undo_pressed, &mut self.held_undo, dt) {
            inputs.push(MetaInput::Undo);
        }

        // Confirm with repeat (Space / A / B on any gamepad)
        let confirm_down = is_key_down(KeyCode::Space)
            || any_gp_down_multi(gamepad, &[GamepadButton::South, GamepadButton::East]);
        let confirm_pressed = is_key_pressed(KeyCode::Space)
            || any_gp_pressed_multi(gamepad, &[GamepadButton::South, GamepadButton::East]);
        if input_repeat(confirm_down, confirm_pressed, &mut self.held_confirm, dt) {
            inputs.push(MetaInput::Confirm);
        }

        // Exit (Escape / Start on any gamepad)
        if is_key_pressed(KeyCode::Escape) || any_gp_pressed(gamepad, GamepadButton::Start) {
            inputs.push(MetaInput::Exit);
        }

        inputs
    }

    /// Poll all input sources and return per-player actions.
    /// Returns `[Option<(Action, synced)>; 2]` indexed by player (0=P1, 1=P2).
    /// Arrow keys are P1 normally, but become P2 if gamepad 0 is connected.
    pub(crate) fn poll_player_actions(
        &mut self,
        gamepad: &GamepadContext,
        dt: f32,
    ) -> [Option<(Action, bool)>; 2] {
        let controller_connected = gamepad.gamepad(0).is_some_and(|g| g.is_connected());
        let arrow_player: usize = if controller_connected { 1 } else { 0 };

        let mut result: [Option<(Action, bool)>; 2] = [None; 2];

        // Source 0: Arrow keys → arrow_player
        if let Some(action) = poll_keys_action(
            &ARROW_KEYS,
            ARROW_STALL,
            &mut self.held_move[0],
            &mut self.held_stall[0],
            dt,
        ) {
            let synced = is_key_down(ARROW_SYNC);
            result[arrow_player] = Some((action, synced));
        }

        // Source 1: WASD → always P2
        if let Some(action) = poll_keys_action(
            &WASD_KEYS,
            WASD_STALL,
            &mut self.held_move[1],
            &mut self.held_stall[1],
            dt,
        ) {
            let synced = is_key_down(WASD_SYNC);
            result[1] = Some((action, synced));
        }

        // Source 2: Gamepad 0 → always P1
        if let Some(action) = poll_gamepad_action(
            gamepad,
            0,
            &mut self.held_move[2],
            &mut self.held_stall[2],
            &mut self.stick_active[0],
            dt,
        ) {
            let synced = gp_btn_down(gamepad, 0, GamepadButton::RightShoulder);
            result[0] = Some((action, synced));
        }

        // Source 3: Gamepad 1 → always P2
        if let Some(action) = poll_gamepad_action(
            gamepad,
            1,
            &mut self.held_move[3],
            &mut self.held_stall[3],
            &mut self.stick_active[1],
            dt,
        ) {
            let synced = gp_btn_down(gamepad, 1, GamepadButton::RightShoulder);
            result[1] = Some((action, synced));
        }

        result
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

// --- Shared polling functions ---

/// Poll keyboard keys for a directional action or stall.
fn poll_keys_action(
    keys: &[KeyCode; 4],
    stall_key: KeyCode,
    held_move: &mut [f32; 4],
    held_stall: &mut f32,
    dt: f32,
) -> Option<Action> {
    for (i, &key) in keys.iter().enumerate() {
        let down = is_key_down(key);
        let pressed = is_key_pressed(key);
        if input_repeat(down, pressed, &mut held_move[i], dt) {
            return Some(Action::Move(DIRS[i]));
        }
    }

    let stall_down = is_key_down(stall_key);
    let stall_pressed = is_key_pressed(stall_key);
    if input_repeat(stall_down, stall_pressed, held_stall, dt) {
        return Some(Action::Stall);
    }

    None
}

/// Poll a gamepad for a directional action or stall.
fn poll_gamepad_action(
    gp: &GamepadContext,
    index: usize,
    held_move: &mut [f32; 4],
    held_stall: &mut f32,
    stick_active: &mut [bool; 4],
    dt: f32,
) -> Option<Action> {
    // D-pad with repeat
    for i in 0..4 {
        let down = gp_btn_down(gp, index, DPAD_BUTTONS[i]);
        let pressed = gp_btn_pressed(gp, index, DPAD_BUTTONS[i]);
        if input_repeat(down, pressed, &mut held_move[i], dt) {
            return Some(Action::Move(DIRS[i]));
        }
    }

    // Left analog stick (edge-triggered, no repeat)
    let stick_y = gp_stick_value(gp, index, GamepadAxis::LeftY);
    let stick_x = gp_stick_value(gp, index, GamepadAxis::LeftX);

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

    let mut result = None;
    if let Some((dir, idx)) = stick_dir
        && !stick_active[idx]
    {
        result = Some(Action::Move(dir));
    }
    let active_idx = stick_dir.map(|(_, idx)| idx);
    for (i, active) in stick_active.iter_mut().enumerate() {
        *active = active_idx == Some(i);
    }
    if result.is_some() {
        return result;
    }

    // Stall (South/East buttons)
    let stall_down = gp_btn_down(gp, index, GamepadButton::South)
        || gp_btn_down(gp, index, GamepadButton::East);
    let stall_pressed = gp_btn_pressed(gp, index, GamepadButton::South)
        || gp_btn_pressed(gp, index, GamepadButton::East);
    if input_repeat(stall_down, stall_pressed, held_stall, dt) {
        return Some(Action::Stall);
    }

    None
}

// --- Gamepad helpers (parameterized by index) ---

fn gp_btn_down(gp: &GamepadContext, index: usize, btn: GamepadButton) -> bool {
    gp.gamepad(index).is_some_and(|g| g.is_button_down(btn))
}

fn gp_btn_pressed(gp: &GamepadContext, index: usize, btn: GamepadButton) -> bool {
    gp.gamepad(index).is_some_and(|g| g.is_button_pressed(btn))
}

fn gp_stick_value(gp: &GamepadContext, index: usize, axis: GamepadAxis) -> f32 {
    let v = gp.gamepad(index).map(|g| g.axis(axis)).unwrap_or(0.0);
    if v.abs() > STICK_THRESHOLD { v } else { 0.0 }
}

/// Check if a button is down on any gamepad (0 or 1).
fn any_gp_down(gp: &GamepadContext, btn: GamepadButton) -> bool {
    (0..2).any(|i| gp_btn_down(gp, i, btn))
}

/// Check if a button was pressed on any gamepad (0 or 1).
fn any_gp_pressed(gp: &GamepadContext, btn: GamepadButton) -> bool {
    (0..2).any(|i| gp_btn_pressed(gp, i, btn))
}

/// Check if any of multiple buttons are down on any gamepad.
fn any_gp_down_multi(gp: &GamepadContext, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| any_gp_down(gp, btn))
}

/// Check if any of multiple buttons were pressed on any gamepad.
fn any_gp_pressed_multi(gp: &GamepadContext, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| any_gp_pressed(gp, btn))
}

// --- Utilities ---

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
