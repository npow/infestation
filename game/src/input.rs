use std::collections::HashSet;
use std::mem;

use enum_map::{Enum, EnumMap};
use macroquad::prelude::*;
use quad_gamepad::{GamepadAxis, GamepadButton, GamepadContext, GamepadState};
use serde::{Deserialize, Serialize};

use crate::direction::Dir4;
use crate::enum_all::EnumAll;
use crate::game::{Action, PlayState};
use crate::grid::Player;

const INITIAL_DELAY: f32 = 0.25;
const REPEAT_INTERVAL: f32 = 0.05;
const AUTO_SYNC_DELAY: f32 = 0.05;
const STICK_DEADZONE: f32 = 0.5;
const SWIPE_THRESHOLD: f32 = 30.0;

// --- Output ---

/// A meta input action (not per-player).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetaInput {
    Undo,
    Restart,
    Exit,
    Confirm(Option<Player>), // None for spacebar if no controller is plugged in
    Export,
    Import,
    OverlayCopy,
    EmailSolution,
}

/// What the input system produces each frame.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameOutput {
    /// No actionable input this frame.
    None,
    /// Player actions ready to execute (sync resolved).
    PlayerActions(Vec<Action>),
    /// A single meta action.
    #[serde(rename = "meta")]
    MetaAction(MetaInput),
}

/// A single player's input for one frame: a direction and whether it's synced.
#[derive(Clone, Copy)]
struct PlayerInput {
    dir: Dir4,
    synced: bool,
}

/// What the input system collected this frame (after polling all sources).
#[derive(Clone, Copy)]
enum HeldInput {
    /// A player-agnostic "stall" (spacebar, click-stall, gamepad A with no dirs).
    Stall,
    /// Per-player directional inputs. `None` = that player provided nothing.
    Independent(EnumMap<Player, Option<PlayerInput>>),
}

// --- Input ---

/// All raw input for a single frame. Constructed by [`read_frame_input`].
pub struct FrameInput {
    /// Keys that transitioned to down this frame.
    pub keys_pressed: HashSet<KeyCode>,
    /// Keys currently held down.
    pub keys_down: HashSet<KeyCode>,
    /// Gamepad states (indices 0 and 1), cloned from GamepadContext.
    pub gamepads: EnumMap<GamepadSource, GamepadState>,
    /// Touch events this frame.
    pub touches: Vec<Touch>,
    /// Mouse left-click position (if pressed this frame).
    pub mouse_click: Option<Vec2>,
    /// Frame delta time.
    pub dt: f32,
}

// --- Game context ---

/// What a click/tap on a UI element should do.
#[derive(Clone, Copy)]
pub(crate) enum ClickAction {
    Meta(MetaInput),
    Stall,
}

/// Subset of game state the input processor needs to make decisions.
#[derive(Clone, Deserialize)]
pub struct GameContext {
    pub player_count: usize,
    #[serde(default)]
    pub is_animating: bool,
    #[serde(default = "default_play_state")]
    pub play_state: PlayState,
    #[serde(default)]
    pub can_undo: bool,
    #[serde(default)]
    pub can_exit: bool,
    /// A confirmation dialog is showing — suppress player actions,
    /// only produce Confirm/Undo/Exit meta actions.
    #[serde(default)]
    pub dialog_active: bool,
    /// An overlay is showing — suppress everything except Exit and
    /// click-target hits (e.g. copy button).
    #[serde(default)]
    pub overlay_active: bool,
    /// Which players are currently standing on a completed portal.
    #[serde(skip)]
    pub(crate) on_completed_portal: EnumMap<Player, bool>,
    /// Clickable UI regions and their actions, computed by game_app
    /// from the current layout each frame.
    #[serde(skip)]
    pub(crate) click_targets: Vec<(Rect, ClickAction)>,
}

fn default_play_state() -> PlayState {
    PlayState::Playing
}

// --- Processor ---

#[derive(Clone, Copy, Enum)]
enum KeyboardSource {
    Arrows,
    Wasd,
}

#[derive(Clone, Copy, Enum)]
pub enum GamepadSource {
    Gamepad0,
    Gamepad1,
}
impl GamepadSource {
    fn from_player(player: Player) -> GamepadSource {
        match player {
            Player::Player1 => Self::Gamepad0,
            Player::Player2 => Self::Gamepad1,
        }
    }

    fn index(&self) -> usize {
        match self {
            GamepadSource::Gamepad0 => 0,
            GamepadSource::Gamepad1 => 1,
        }
    }
}

impl KeyboardSource {
    fn move_keys(&self) -> EnumMap<Dir4, KeyCode> {
        match self {
            Self::Arrows => enum_map::enum_map! {
                Dir4::North => KeyCode::Up,
                Dir4::South => KeyCode::Down,
                Dir4::East => KeyCode::Right,
                Dir4::West => KeyCode::Left,
            },
            Self::Wasd => enum_map::enum_map! {
                Dir4::North => KeyCode::W,
                Dir4::South => KeyCode::S,
                Dir4::East => KeyCode::D,
                Dir4::West => KeyCode::A,
            },
        }
    }

    fn sync_key(&self) -> KeyCode {
        match self {
            Self::Arrows => KeyCode::RightShift,
            Self::Wasd => KeyCode::LeftShift,
        }
    }
}

struct RepeatTracker {
    dir: Dir4,
    delay: f32,
}

struct TouchState {
    id: u64,
    start: Vec2,
    swiped: bool,
}

/// Processes raw input into game actions. Holds internal state for
/// sync buffering, animation buffering, key repeat, and touch tracking.
#[derive(Default)]
pub struct InputProcessor {
    pending_sync: EnumMap<Player, Option<Dir4>>,
    animation_buffer: Option<HeldInput>,
    /// Per-player: true if the buffer entry came from an edge trigger (not a repeat).
    buffer_from_edge: EnumMap<Player, bool>,
    /// True if the buffered stall came from an edge trigger (not a repeat).
    stall_buffer_from_edge: bool,
    auto_sync_timer: Option<f32>,
    /// When true, the auto-sync timer was started from an edge trigger, so
    /// both players pairing up should emit immediately (no original delay to preserve).
    auto_sync_from_edge: bool,
    repeat: EnumMap<Player, Option<RepeatTracker>>,
    touch: Option<TouchState>,
    stall_repeat: Option<f32>,
    /// Suppress stall until all stall keys are released (after a meta action consumed the press).
    suppress_stall: bool,
    undo_repeat: Option<f32>,
}

impl InputProcessor {
    fn emit_meta(&mut self, meta: MetaInput) -> FrameOutput {
        self.animation_buffer = None;
        self.pending_sync = EnumMap::default();
        self.auto_sync_timer = None;
        self.suppress_stall = true;
        FrameOutput::MetaAction(meta)
    }

    /// Process one frame of input and return what happened.
    pub fn process(&mut self, input: &FrameInput, ctx: &GameContext) -> FrameOutput {
        let (swipe_dir, tap_pos) = self.update_touch(input);

        // Click/tap on UI targets → meta actions take priority
        let click_pos = input.mouse_click.or(tap_pos);
        let mut stall_from_click = false;
        if let Some(pos) = click_pos {
            match hit_click_target(pos, &ctx.click_targets) {
                Some(ClickAction::Meta(meta)) => return self.emit_meta(meta),
                Some(ClickAction::Stall) => stall_from_click = true,
                None => {}
            }
        }

        // Keyboard/gamepad meta actions
        if let Some(meta) = check_meta(input, ctx) {
            if matches!(meta, MetaInput::Undo) {
                self.undo_repeat = Some(INITIAL_DELAY);
            }
            return self.emit_meta(meta);
        }

        // Undo repeat (key held after initial press)
        if ctx.can_undo && !ctx.dialog_active && !ctx.overlay_active && undo_keys_held(input) {
            if let Some(ref mut delay) = self.undo_repeat {
                *delay -= input.dt;
                if *delay <= 0.0 {
                    *delay += REPEAT_INTERVAL;
                    return self.emit_meta(MetaInput::Undo);
                }
            }
        } else {
            self.undo_repeat = None;
        }

        // Player actions only when playing
        if ctx.play_state != PlayState::Playing {
            return FrameOutput::None;
        }

        // Collect held state from all input sources
        let mut held = Self::collect_held(input, ctx.player_count);

        // Suppress stall until all stall keys are released (after a meta action consumed the press)
        if matches!(held, HeldInput::Stall) {
            if self.suppress_stall {
                held = HeldInput::Independent(EnumMap::default());
            }
        } else {
            self.suppress_stall = false;
        }

        // Inject swipe for P1 (overrides stall if both happen)
        if let Some(dir) = swipe_dir {
            match &mut held {
                HeldInput::Independent(dirs) if dirs[Player::Player1].is_none() => {
                    dirs[Player::Player1] = Some(PlayerInput { dir, synced: false });
                }
                HeldInput::Stall => {
                    held = HeldInput::Independent(enum_map::enum_map! {
                        Player::Player1 => Some(PlayerInput { dir, synced: false }),
                        Player::Player2 => None,
                    });
                }
                _ => {}
            }
        }

        // Inject stall-click (only if no directions from any source)
        if stall_from_click
            && let HeldInput::Independent(ref dirs) = held
            && dirs.values().all(|v| v.is_none())
        {
            held = HeldInput::Stall;
        }

        match self.player_actions(held, ctx.player_count, ctx.is_animating, input.dt) {
            Some(actions) => {
                FrameOutput::PlayerActions(actions.into_values().take(ctx.player_count).collect())
            }
            None => FrameOutput::None,
        }
    }

    /// Resolve held inputs into actions, handling edge detection,
    /// key repeat, animation buffering, and sync.
    /// Returns `None` if no actions this frame.
    fn player_actions(
        &mut self,
        held: HeldInput,
        player_count: usize,
        is_animating: bool,
        dt: f32,
    ) -> Option<EnumMap<Player, Action>> {
        match held {
            HeldInput::Stall => {
                // Clear directional repeat trackers
                self.repeat = EnumMap::default();

                // Repeat: fire immediately, then after INITIAL_DELAY, then every REPEAT_INTERVAL
                let stall_is_edge;
                match &mut self.stall_repeat {
                    None => {
                        self.stall_repeat = Some(INITIAL_DELAY);
                        stall_is_edge = true;
                    }
                    Some(delay) => {
                        *delay -= dt;
                        if *delay > 0.0 {
                            return None;
                        }
                        *delay += REPEAT_INTERVAL;
                        stall_is_edge = false;
                    }
                }

                if is_animating {
                    self.animation_buffer = Some(HeldInput::Stall);
                    self.stall_buffer_from_edge = stall_is_edge;
                    if let Some(ref mut timer) = self.auto_sync_timer {
                        *timer -= dt;
                    }
                    return None;
                }

                // Stall overrides any pending sync
                self.pending_sync = EnumMap::default();
                self.auto_sync_timer = None;
                Some(EnumMap::from_fn(|_| Action::Stall))
            }
            HeldInput::Independent(dirs) => {
                self.stall_repeat = None;

                // Edge detection + repeat per player
                // In 2-player mode, shorten repeat delays to compensate for auto-sync delay
                let initial_delay = if player_count > 1 {
                    (INITIAL_DELAY - AUTO_SYNC_DELAY).max(0.0)
                } else {
                    INITIAL_DELAY
                };
                let repeat_interval = if player_count > 1 {
                    (REPEAT_INTERVAL - AUTO_SYNC_DELAY).max(0.0)
                } else {
                    REPEAT_INTERVAL
                };
                let mut edge: EnumMap<Player, Option<PlayerInput>> = EnumMap::default();
                let mut is_edge: EnumMap<Player, bool> = EnumMap::default();
                for player in Player::iter_all() {
                    let (result, edge_flag) = apply_repeat(
                        &mut self.repeat[player],
                        dirs[player],
                        dt,
                        initial_delay,
                        repeat_interval,
                    );
                    edge[player] = result;
                    is_edge[player] = edge_flag;
                }

                if is_animating {
                    // Buffer per-player dirs during animation
                    match &mut self.animation_buffer {
                        None => {
                            if edge.values().any(|v| v.is_some()) {
                                self.animation_buffer = Some(HeldInput::Independent(edge));
                                for player in Player::iter_all() {
                                    if edge[player].is_some() {
                                        self.buffer_from_edge[player] = is_edge[player];
                                    }
                                }
                            }
                        }
                        Some(HeldInput::Independent(buf)) => {
                            for (player, slot) in edge {
                                if slot.is_some() {
                                    buf[player] = slot;
                                    self.buffer_from_edge[player] = is_edge[player];
                                }
                            }
                        }
                        Some(HeldInput::Stall) => {
                            if edge.values().any(|v| v.is_some()) {
                                self.animation_buffer = Some(HeldInput::Independent(edge));
                                for player in Player::iter_all() {
                                    if edge[player].is_some() {
                                        self.buffer_from_edge[player] = is_edge[player];
                                    }
                                }
                            }
                        }
                    }

                    // Clear repeat-sourced buffer entries when key released
                    match &mut self.animation_buffer {
                        Some(HeldInput::Independent(buf)) => {
                            for player in Player::iter_all() {
                                if dirs[player].is_none() && !self.buffer_from_edge[player] {
                                    buf[player] = None;
                                }
                            }
                            if buf.values().all(|v| v.is_none()) {
                                self.animation_buffer = None;
                            }
                        }
                        Some(HeldInput::Stall) => {
                            if !self.stall_buffer_from_edge {
                                self.animation_buffer = None;
                            }
                        }
                        None => {}
                    }

                    // Start auto-sync timer during animation so it counts down
                    // in parallel with the animation.
                    if player_count > 1
                        && self.auto_sync_timer.is_none()
                        && edge.values().any(|v| v.is_some())
                    {
                        self.auto_sync_timer = Some(AUTO_SYNC_DELAY);
                        self.auto_sync_from_edge = is_edge.values().any(|&v| v);
                    }
                    if let Some(ref mut timer) = self.auto_sync_timer {
                        *timer -= dt;
                    }
                    return None;
                }

                // Flush animation buffer
                match self.animation_buffer.take() {
                    Some(HeldInput::Stall) => {
                        self.pending_sync = EnumMap::default();
                        return Some(EnumMap::from_fn(|_| Action::Stall));
                    }
                    Some(HeldInput::Independent(buf)) => {
                        for (player, slot) in buf {
                            if slot.is_some() && edge[player].is_none() {
                                edge[player] = slot;
                            }
                        }
                    }
                    None => {}
                }

                // Resolve sync and emit
                let any_edge = is_edge.values().any(|&v| v);
                self.resolve(edge, player_count, dt, any_edge)
            }
        }
    }

    /// Report what each player is currently holding (no edge detection or repeat).
    fn collect_held(input: &FrameInput, player_count: usize) -> HeldInput {
        let gamepad0_connected = input.gamepads[GamepadSource::Gamepad0].is_connected();
        let single = player_count == 1;

        let mut dirs: EnumMap<Player, Option<PlayerInput>> = EnumMap::default();

        // Keyboard sources (arrows + WASD)
        let kb_sources = [
            (
                KeyboardSource::Arrows,
                if single || !gamepad0_connected {
                    Player::Player1
                } else {
                    Player::Player2
                },
            ),
            (
                KeyboardSource::Wasd,
                if single {
                    Player::Player1
                } else {
                    Player::Player2
                },
            ),
        ];
        for (src, player) in kb_sources {
            if let Some(dir) = dir_from_keys(&input.keys_down, src.move_keys()) {
                let synced = !single && input.keys_down.contains(&src.sync_key());
                if dirs[player].is_none() {
                    dirs[player] = Some(PlayerInput { dir, synced });
                }
            }
        }

        // Gamepads (D-pad + right stick)
        for player in Player::iter_all() {
            let src = GamepadSource::from_player(player);
            let gp = &input.gamepads[src];
            if !gp.is_connected() {
                continue;
            }

            let dir = gamepad_dir_held(gp).or_else(|| stick_direction(gp));
            if let Some(dir) = dir {
                let synced = !single && gp.is_button_down(GamepadButton::RightShoulder);
                if dirs[player].is_none() {
                    dirs[player] = Some(PlayerInput { dir, synced });
                }
            }
        }

        // If any player has a direction, return Independent
        if dirs.values().any(|v| v.is_some()) {
            return HeldInput::Independent(dirs);
        }

        // Check stall sources: gamepad A button, spacebar
        let gamepad_a_down = input
            .gamepads
            .values()
            .any(|gp| gp.is_button_down(GamepadButton::South));
        let space_down = input.keys_down.contains(&KeyCode::Space);

        if gamepad_a_down || space_down {
            return HeldInput::Stall;
        }

        HeldInput::Independent(dirs) // all None
    }

    fn update_touch(&mut self, input: &FrameInput) -> (Option<Dir4>, Option<Vec2>) {
        let mut swipe_dir = None;
        let mut tap_pos = None;

        for touch in &input.touches {
            match touch.phase {
                TouchPhase::Started => {
                    self.touch = Some(TouchState {
                        id: touch.id,
                        start: touch.position,
                        swiped: false,
                    });
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if let Some(ref mut state) = self.touch
                        && state.id == touch.id
                        && !state.swiped
                    {
                        let delta = touch.position - state.start;
                        if delta.x.abs() > SWIPE_THRESHOLD || delta.y.abs() > SWIPE_THRESHOLD {
                            state.swiped = true;
                            swipe_dir = Some(if delta.x.abs() > delta.y.abs() {
                                if delta.x > 0.0 {
                                    Dir4::East
                                } else {
                                    Dir4::West
                                }
                            } else {
                                // Screen Y down = South
                                if delta.y > 0.0 {
                                    Dir4::South
                                } else {
                                    Dir4::North
                                }
                            });
                        }
                    }
                }
                TouchPhase::Ended => {
                    if let Some(ref state) = self.touch
                        && state.id == touch.id
                    {
                        if !state.swiped {
                            tap_pos = Some(touch.position);
                        }
                        self.touch = None;
                    }
                }
                TouchPhase::Cancelled => {
                    if self.touch.as_ref().is_some_and(|s| s.id == touch.id) {
                        self.touch = None;
                    }
                }
            }
        }

        (swipe_dir, tap_pos)
    }

    fn resolve(
        &mut self,
        dirs: EnumMap<Player, Option<PlayerInput>>,
        player_count: usize,
        dt: f32,
        any_edge: bool,
    ) -> Option<EnumMap<Player, Action>> {
        if player_count == 1 {
            let PlayerInput { dir, .. } = dirs[Player::Player1]?;
            return Some(EnumMap::from_fn(|_| Action::Move(dir)));
        }

        // Accumulate new inputs into pending_sync
        let mut new_unsynced_input = false;
        for (player, slot) in dirs {
            if let Some(PlayerInput { dir, synced }) = slot {
                self.pending_sync[player] = Some(dir);
                if !synced {
                    new_unsynced_input = true;
                }
            }
        }

        // If both players have pending actions, emit immediately if:
        // - no timer running, OR
        // - timer was started from an edge (no original delay to preserve), OR
        // - timer already expired
        if self.pending_sync.values().all(|v| v.is_some())
            && (self.auto_sync_timer.is_none()
                || self.auto_sync_from_edge
                || self.auto_sync_timer.is_some_and(|t| t <= 0.0))
        {
            self.auto_sync_timer = None;
            return Some(mem::take(&mut self.pending_sync).map(|_, slot| match slot {
                Some(dir) => Action::Move(dir),
                None => Action::Stall,
            }));
        }

        // If any player has a pending action, start or tick timer
        if self.pending_sync.values().any(|v| v.is_some()) {
            if new_unsynced_input && self.auto_sync_timer.is_none() {
                self.auto_sync_timer = Some(AUTO_SYNC_DELAY);
                self.auto_sync_from_edge = any_edge;
            }
            if let Some(ref mut timer) = self.auto_sync_timer {
                *timer -= dt;
                if *timer <= 0.0 {
                    self.auto_sync_timer = None;
                    return Some(mem::take(&mut self.pending_sync).map(|_, slot| match slot {
                        Some(dir) => Action::Move(dir),
                        None => Action::Stall,
                    }));
                }
            }
        }

        None
    }
}

// --- Stateless helpers ---

fn dir_from_keys(keys: &HashSet<KeyCode>, mapping: EnumMap<Dir4, KeyCode>) -> Option<Dir4> {
    mapping
        .iter()
        .find(|(_, key)| keys.contains(key))
        .map(|(dir, _)| dir)
}

fn undo_keys_held(input: &FrameInput) -> bool {
    input.keys_down.contains(&KeyCode::U)
        || input.keys_down.contains(&KeyCode::Backspace)
        || input
            .gamepads
            .values()
            .any(|gp| gp.is_button_down(GamepadButton::West))
}

fn check_meta(input: &FrameInput, ctx: &GameContext) -> Option<MetaInput> {
    let gamepad_a = input
        .gamepads
        .values()
        .any(|gp| gp.is_button_pressed(GamepadButton::South));
    let gamepad_x = input
        .gamepads
        .values()
        .any(|gp| gp.is_button_pressed(GamepadButton::West));

    if ctx.overlay_active {
        if input.keys_pressed.contains(&KeyCode::Escape) || gamepad_x {
            return Some(MetaInput::Exit);
        }
        return None;
    }

    if ctx.dialog_active {
        if input.keys_pressed.contains(&KeyCode::Escape)
            || input.keys_pressed.contains(&KeyCode::Backspace)
            || gamepad_x
        {
            return Some(MetaInput::Exit);
        }
        if input.keys_pressed.contains(&KeyCode::Enter)
            || input.keys_pressed.contains(&KeyCode::Space)
            || gamepad_a
        {
            return Some(MetaInput::Confirm(None));
        }
        return None;
    }

    let gamepad_start = input
        .gamepads
        .values()
        .any(|gp| gp.is_button_pressed(GamepadButton::Start));
    if input.keys_pressed.contains(&KeyCode::Escape) || gamepad_start {
        return Some(MetaInput::Exit);
    }
    if (input.keys_pressed.contains(&KeyCode::Backspace)
        || input.keys_pressed.contains(&KeyCode::U)
        || gamepad_x)
        && ctx.can_undo
    {
        return Some(MetaInput::Undo);
    }
    let gamepad_lb = input
        .gamepads
        .values()
        .any(|gp| gp.is_button_pressed(GamepadButton::LeftShoulder));
    if input.keys_pressed.contains(&KeyCode::R) || gamepad_lb {
        return Some(MetaInput::Restart);
    }

    // Won screen → confirm to continue
    if ctx.play_state == PlayState::Won
        && (input.keys_pressed.contains(&KeyCode::Space)
            || input.keys_pressed.contains(&KeyCode::Enter)
            || gamepad_a)
    {
        return Some(MetaInput::Confirm(None));
    }

    // A button / spacebar on a completed portal → confirm (instead of stall)
    if ctx.play_state == PlayState::Playing {
        let single = ctx.player_count == 1;
        for default_player in Player::iter_all() {
            let p = if single {
                Player::Player1
            } else {
                default_player
            };
            if input.gamepads[GamepadSource::from_player(default_player)]
                .is_button_pressed(GamepadButton::South)
                && ctx.on_completed_portal[p]
            {
                return Some(MetaInput::Confirm(Some(p)));
            }
        }
        if input.keys_pressed.contains(&KeyCode::Space)
            && ctx.on_completed_portal.values().any(|&v| v)
        {
            return Some(MetaInput::Confirm(None));
        }
    }

    None
}

fn hit_click_target(pos: Vec2, targets: &[(Rect, ClickAction)]) -> Option<ClickAction> {
    targets
        .iter()
        .find(|(rect, _)| rect.contains(pos))
        .map(|(_, action)| *action)
}

/// Returns `(input, is_edge)` — `is_edge` is true on the initial press,
/// false on held-key repeats.
fn apply_repeat(
    tracker: &mut Option<RepeatTracker>,
    held: Option<PlayerInput>,
    dt: f32,
    initial_delay: f32,
    repeat_interval: f32,
) -> (Option<PlayerInput>, bool) {
    let Some(PlayerInput { dir, synced }) = held else {
        *tracker = None;
        return (None, false);
    };
    match tracker {
        Some(t) if t.dir == dir => {
            // Same direction held — repeat
            t.delay -= dt;
            if t.delay <= 0.0 {
                t.delay += repeat_interval;
                (Some(PlayerInput { dir, synced }), false)
            } else {
                (None, false)
            }
        }
        _ => {
            // New direction — fire immediately
            *tracker = Some(RepeatTracker {
                dir,
                delay: initial_delay,
            });
            (Some(PlayerInput { dir, synced }), true)
        }
    }
}

fn gamepad_dir_held(gp: &GamepadState) -> Option<Dir4> {
    for (btn, dir) in [
        (GamepadButton::DPadUp, Dir4::North),
        (GamepadButton::DPadDown, Dir4::South),
        (GamepadButton::DPadRight, Dir4::East),
        (GamepadButton::DPadLeft, Dir4::West),
    ] {
        if gp.is_button_down(btn) {
            return Some(dir);
        }
    }
    None
}

fn stick_direction(gp: &GamepadState) -> Option<Dir4> {
    let x = gp.axis(GamepadAxis::RightX);
    let y = gp.axis(GamepadAxis::RightY);
    if x.abs() < STICK_DEADZONE && y.abs() < STICK_DEADZONE {
        return None;
    }
    Some(if x.abs() > y.abs() {
        if x > 0.0 { Dir4::East } else { Dir4::West }
    } else {
        // gilrs/quad-gamepad convention: +Y = up = North
        if y > 0.0 { Dir4::North } else { Dir4::South }
    })
}

/// Read all raw input from macroquad globals and the gamepad context.
/// This is the only function that touches macroquad's input APIs.
pub(crate) fn read_frame_input(gamepad: &GamepadContext) -> FrameInput {
    let mouse_click = if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        Some(Vec2::new(x, y))
    } else {
        None
    };

    FrameInput {
        keys_pressed: get_keys_pressed(),
        keys_down: get_keys_down(),
        gamepads: EnumMap::from_fn(|source: GamepadSource| {
            gamepad.gamepad(source.index()).cloned().unwrap_or_default()
        }),
        touches: touches(),
        mouse_click,
        dt: get_frame_time(),
    }
}
