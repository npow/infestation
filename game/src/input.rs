use enum_map::{Enum, EnumMap};
use macroquad::prelude::*;
use quad_gamepad::{GamepadAxis, GamepadButton, GamepadContext};

use crate::direction::Dir4;
use crate::game::Action;
use crate::grid::Player;

#[derive(Debug, Clone, Copy, Enum)]
enum Gamepad {
    Gamepad1,
    Gamepad2,
}

impl Gamepad {
    fn index(self) -> usize {
        match self {
            Gamepad::Gamepad1 => 0,
            Gamepad::Gamepad2 => 1,
        }
    }
}

const REPEAT_DELAY: f32 = 0.2;
const STICK_THRESHOLD: f32 = 0.5;
const SWIPE_THRESHOLD: f32 = 30.0;

/// Number of input sources: [0]=arrows, [1]=WASD, [2]=gamepad0, [3]=gamepad1.
const NUM_SOURCES: usize = 4;

const ARROW_KEYS: [KeyCode; 4] = [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right];
const WASD_KEYS: [KeyCode; 4] = [KeyCode::W, KeyCode::S, KeyCode::A, KeyCode::D];
const ARROW_SYNC: KeyCode = KeyCode::RightShift;
const WASD_SYNC: KeyCode = KeyCode::LeftShift;

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
    /// Confirm action from a specific player.
    Confirm(Player),
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
    /// Stall held timer for spacebar.
    held_stall_space: f32,
    /// Stall held timers per gamepad.
    held_stall_gp: EnumMap<Gamepad, f32>,
    /// Meta action held timers.
    held_undo: f32,
    held_confirm: EnumMap<Player, f32>,
    /// Per-gamepad analog stick edge detection.
    stick_active: EnumMap<Gamepad, [bool; 4]>,
    touch_start: Option<(u64, Vec2)>,
    touch_handled_this_frame: bool,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            held_move: [[0.0; 4]; NUM_SOURCES],
            held_stall_space: 0.0,
            held_stall_gp: EnumMap::default(),
            held_undo: 0.0,
            held_confirm: EnumMap::default(),
            stick_active: EnumMap::default(),
            touch_start: None,
            touch_handled_this_frame: false,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.held_move = [[0.0; 4]; NUM_SOURCES];
        self.held_stall_space = 0.0;
        self.held_stall_gp = EnumMap::default();
        self.held_undo = 0.0;
        self.held_confirm = EnumMap::default();
        self.stick_active = EnumMap::default();
    }

    /// Poll meta actions (undo, restart, exit, confirm) from keyboard + any gamepad.
    pub(crate) fn poll_meta_inputs(&mut self, gamepad: &GamepadContext, dt: f32) -> Vec<MetaInput> {
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

        // Per-player confirm with repeat: Space → both, gp0 → P1, gp1 → P2
        let space_down = is_key_down(KeyCode::Space);
        let space_pressed = is_key_pressed(KeyCode::Space);
        let confirm_sources: [(bool, bool, Player); 4] = [
            (space_down, space_pressed, Player::Player1),
            (space_down, space_pressed, Player::Player2),
            (
                gp_btn_down_multi(
                    gamepad,
                    Gamepad::Gamepad1.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                gp_btn_pressed_multi(
                    gamepad,
                    Gamepad::Gamepad1.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                Player::Player1,
            ),
            (
                gp_btn_down_multi(
                    gamepad,
                    Gamepad::Gamepad2.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                gp_btn_pressed_multi(
                    gamepad,
                    Gamepad::Gamepad2.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                Player::Player2,
            ),
        ];
        for (down, pressed, player) in confirm_sources {
            if input_repeat(down, pressed, &mut self.held_confirm[player], dt) {
                inputs.push(MetaInput::Confirm(player));
            }
        }

        // Exit (Escape / Start on any gamepad)
        if is_key_pressed(KeyCode::Escape) || any_gp_pressed(gamepad, GamepadButton::Start) {
            inputs.push(MetaInput::Exit);
        }

        inputs
    }

    /// Poll all input sources and return per-player actions.
    /// Arrow keys are P1 normally, but become P2 if gamepad 1 is connected.
    pub(crate) fn poll_player_actions(
        &mut self,
        gamepad: &GamepadContext,
        dt: f32,
    ) -> EnumMap<Player, Option<(Action, bool)>> {
        let controller_connected = gamepad
            .gamepad(Gamepad::Gamepad1.index())
            .is_some_and(|g| g.is_connected());
        let arrow_player = if controller_connected {
            Player::Player2
        } else {
            Player::Player1
        };

        let mut result: EnumMap<Player, Option<(Action, bool)>> = EnumMap::default();

        // Source 0: Arrow keys → arrow_player
        if let Some(action) = poll_keys_dir(&ARROW_KEYS, &mut self.held_move[0], dt) {
            let synced = is_key_down(ARROW_SYNC);
            result[arrow_player] = Some((action, synced));
        }

        // Source 1: WASD → always P2
        if let Some(action) = poll_keys_dir(&WASD_KEYS, &mut self.held_move[1], dt) {
            let synced = is_key_down(WASD_SYNC);
            result[Player::Player2] = Some((action, synced));
        }

        // Spacebar stalls both players (not synced)
        if input_held(
            is_key_down(KeyCode::Space),
            is_key_pressed(KeyCode::Space),
            &mut self.held_stall_space,
            dt,
        ) {
            result[Player::Player1] = result[Player::Player1].or(Some((Action::Stall, false)));
            result[Player::Player2] = result[Player::Player2].or(Some((Action::Stall, false)));
        }

        // Source 2: Gamepad 1 → always P1
        if let Some(action) = poll_gamepad_action(
            gamepad,
            Gamepad::Gamepad1.index(),
            &mut self.held_move[2],
            &mut self.held_stall_gp[Gamepad::Gamepad1],
            &mut self.stick_active[Gamepad::Gamepad1],
            dt,
        ) {
            let synced = gp_btn_down(
                gamepad,
                Gamepad::Gamepad1.index(),
                GamepadButton::RightShoulder,
            );
            result[Player::Player1] = Some((action, synced));
        }

        // Source 3: Gamepad 2 → always P2
        if let Some(action) = poll_gamepad_action(
            gamepad,
            Gamepad::Gamepad2.index(),
            &mut self.held_move[3],
            &mut self.held_stall_gp[Gamepad::Gamepad2],
            &mut self.stick_active[Gamepad::Gamepad2],
            dt,
        ) {
            let synced = gp_btn_down(
                gamepad,
                Gamepad::Gamepad2.index(),
                GamepadButton::RightShoulder,
            );
            result[Player::Player2] = Some((action, synced));
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

/// Poll keyboard keys for a directional action.
fn poll_keys_dir(keys: &[KeyCode; 4], held_move: &mut [f32; 4], dt: f32) -> Option<Action> {
    for (i, &key) in keys.iter().enumerate() {
        let down = is_key_down(key);
        let pressed = is_key_pressed(key);
        if input_held(down, pressed, &mut held_move[i], dt) {
            return Some(Action::Move(DIRS[i]));
        }
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
        if input_held(down, pressed, &mut held_move[i], dt) {
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
    let stall_down =
        gp_btn_down(gp, index, GamepadButton::South) || gp_btn_down(gp, index, GamepadButton::East);
    let stall_pressed = gp_btn_pressed(gp, index, GamepadButton::South)
        || gp_btn_pressed(gp, index, GamepadButton::East);
    if input_held(stall_down, stall_pressed, held_stall, dt) {
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

fn gp_btn_down_multi(gp: &GamepadContext, index: usize, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| gp_btn_down(gp, index, btn))
}

fn gp_btn_pressed_multi(gp: &GamepadContext, index: usize, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| gp_btn_pressed(gp, index, btn))
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

const UNDO_REPEAT_RATE: f32 = 0.05;

fn input_held(down: bool, pressed: bool, held: &mut f32, dt: f32) -> bool {
    if down {
        *held += dt;
        pressed || *held > REPEAT_DELAY
    } else {
        *held = 0.0;
        false
    }
}

fn input_repeat(down: bool, pressed: bool, held: &mut f32, dt: f32) -> bool {
    if down {
        *held += dt;
        pressed || (*held > REPEAT_DELAY && *held % UNDO_REPEAT_RATE < dt)
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
