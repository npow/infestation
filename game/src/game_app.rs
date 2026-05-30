use std::collections::HashSet;
use std::mem;

use macroquad::prelude::*;
use quad_gamepad::GamepadContext;

use enum_map::EnumMap;

use crate::game::{Action, Game, PlayState};
use crate::grid::Player;
use crate::input::{InputState, MetaInput, TouchGesture};
use crate::level_stack::LevelStack;
use crate::levels;
use crate::render::progress_buttons::ExportOverlay;
use crate::render::{
    ButtonAction, ConfirmDialog, InputHints, UiState, button_at_position, button_bar_y, render,
};
use crate::screen_wake;
use crate::sprites::Sprites;
use crate::storage::{load_completed_levels, save_completed_levels};

/// Grace period for the second player to input before a solo action fires.
const SYNC_GRACE_PERIOD: f32 = 0.064;

fn load_level(name: &str, completed_levels: &mut HashSet<String>) -> Game {
    let level = levels::get_level(name).unwrap_or_else(|| panic!("Level not found: {}", name));
    Game::new(level.grid.clone(), mem::take(completed_levels))
}

#[derive(Clone, Copy)]
struct PendingAction {
    action: Action,
    synced: bool,
    /// Whether this was from a fresh key press (not a held-repeat).
    /// Held-repeat pending is cleared when the key is released.
    fresh: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetaInputOutcome {
    Handled,
    ChangedLevel,
}

pub struct App {
    game: Game,
    stack: LevelStack,
    input: InputState,
    gamepad: GamepadContext,
    sprites: Sprites,
    confirm_dialog: ConfirmDialog,
    /// Per-player buffered actions (sync or immediate).
    pending: EnumMap<Player, Option<PendingAction>>,
    /// Time elapsed since the first pending action was set, for the 2P grace period.
    pending_timer: f32,
    /// Whether a clipboard import is in progress (async on WASM).
    import_pending: bool,
    /// Export overlay showing the encoded progress string.
    export_overlay: ExportOverlay,
}

impl App {
    pub fn new(sprites: Sprites, level_name: Option<&str>) -> Self {
        screen_wake::request();

        let level_name = level_name.unwrap_or("world");
        let mut completed = load_completed_levels();
        let game = load_level(level_name, &mut completed);
        let stack = LevelStack::new(level_name.to_string());
        Self {
            game,
            stack,
            input: InputState::new(),
            gamepad: GamepadContext::new(),
            sprites,
            confirm_dialog: ConfirmDialog::None,
            pending: EnumMap::default(),
            pending_timer: 0.0,
            import_pending: false,
            export_overlay: ExportOverlay::default(),
        }
    }

    fn input_hints(&self) -> InputHints {
        if let Some(gp) = self.gamepad.gamepad(0)
            && gp.is_connected()
        {
            return InputHints::Controller(gp.controller_type());
        }
        if quad_touch::is_touch_device() {
            InputHints::Touch
        } else {
            InputHints::Keyboard
        }
    }

    fn ui_state(&self) -> UiState {
        UiState {
            can_reset: self.game.state.history.len() > 1,
            can_undo: self.game.state.history.len() > 1,
            can_exit: true,
            on_portal: self.game.state.standing_on_portal().is_some(),
        }
    }

    /// Exit the current level and return to parent. Caller must ensure `stack.can_exit()` is true.
    fn exit_level(&mut self) {
        let was_won = self.game.state.play_state() == PlayState::Won;
        if let Some(restored) = self.stack.exit_level(&self.game) {
            if was_won {
                save_completed_levels(&restored.state.completed_levels);
            }
            self.game = restored;
            self.input.reset();
            self.pending = EnumMap::default();
        }
    }

    fn handle_progress_action(&mut self, action: crate::render::progress_buttons::ProgressAction) {
        use crate::render::progress_buttons::ProgressAction;
        use crate::storage::progress;

        match action {
            ProgressAction::Export => {
                let encoded = progress::encode(&self.game.state.completed_levels);
                self.export_overlay = ExportOverlay::Showing(encoded);
            }
            ProgressAction::Import => {
                progress::start_import();
                self.import_pending = true;
            }
        }
    }

    fn handle_overlay_click(&mut self, pos: Vec2) {
        use crate::render::progress_buttons::overlay_copy_hit;

        if let ExportOverlay::Showing(ref encoded) = self.export_overlay
            && overlay_copy_hit(pos, encoded, self.sprites.font())
        {
            crate::storage::progress::copy_to_clipboard(encoded);
            let encoded = encoded.clone();
            self.export_overlay = ExportOverlay::Copied(encoded);
            return;
        }
        self.export_overlay = ExportOverlay::Hidden;
    }

    #[cfg(target_arch = "wasm32")]
    fn email_solution(&self) {
        let csv = self.game.state.initial_grid.to_csv();
        let level_name = &self.stack.current_level;
        let encoded =
            crate::solution::encode_solution(level_name, &csv, &self.game.state.action_history);
        let url = crate::solution::mailto_url(level_name, &encoded);
        crate::open_url::open(&url);
    }

    #[must_use]
    fn handle_meta_input(&mut self, action: MetaInput) -> MetaInputOutcome {
        match action {
            MetaInput::Restart => {
                if self.game.state.history.len() > 1 {
                    self.confirm_dialog = ConfirmDialog::Restart;
                }
            }
            MetaInput::Undo => {
                self.game.undo();
                self.pending = EnumMap::default();
            }
            MetaInput::Exit => {
                self.confirm_dialog = if self.stack.can_exit() {
                    ConfirmDialog::Exit
                } else {
                    ConfirmDialog::QuitGame
                };
            }
            MetaInput::Confirm(player) => {
                let play_state = self.game.state.play_state();
                if !self.game.is_animating()
                    && play_state == PlayState::Won
                    && self.stack.can_exit()
                {
                    self.exit_level();
                    return MetaInputOutcome::ChangedLevel;
                } else if play_state == PlayState::Playing
                    && let Some(level) = self.game.enter_portal(player).map(str::to_string)
                {
                    self.do_portal_transition(&level);
                    return MetaInputOutcome::ChangedLevel;
                }
            }
        }
        MetaInputOutcome::Handled
    }

    fn handle_button_action(&mut self, action: ButtonAction) {
        match action {
            ButtonAction::Reset => {
                assert_eq!(
                    self.handle_meta_input(MetaInput::Restart),
                    MetaInputOutcome::Handled
                );
            }
            ButtonAction::Undo => {
                assert_eq!(
                    self.handle_meta_input(MetaInput::Undo),
                    MetaInputOutcome::Handled
                );
            }
            ButtonAction::Exit => {
                assert_eq!(
                    self.handle_meta_input(MetaInput::Exit),
                    MetaInputOutcome::Handled
                );
            }
            ButtonAction::Stall => {
                // UI stall button acts as P1 non-synced stall
                if self.game.state.play_state() == PlayState::Playing {
                    self.pending[Player::Player1] = Some(PendingAction {
                        action: Action::Stall,
                        synced: false,
                        fresh: true,
                    });
                }
            }
        }
    }

    fn handle_tap_or_click(&mut self, pos: Vec2) {
        let hints = self.input_hints();
        let ui = self.ui_state();

        let play_state = self.game.state.play_state();
        let is_playing = play_state == PlayState::Playing;
        let bar_y = button_bar_y();
        if let Some(action) =
            button_at_position(pos, &ui, is_playing, hints, bar_y, self.sprites.font())
        {
            self.handle_button_action(action);
            return;
        }

        // Email Solution button on win screen (WASM only)
        #[cfg(target_arch = "wasm32")]
        if !self.game.is_animating()
            && play_state == PlayState::Won
            && crate::render::email_button::hit(pos, &self.game, self.sprites.font())
        {
            self.email_solution();
            return;
        }

        // Progress export/import buttons
        if let Some(action) = crate::render::progress_buttons::hit(pos, self.sprites.font()) {
            self.handle_progress_action(action);
            return;
        }

        // Tap on overlay screens (not on a button)
        if !self.game.is_animating() {
            if play_state == PlayState::Won && self.stack.can_exit() {
                self.exit_level();
            } else if play_state == PlayState::GameOver {
                self.game.undo();
                self.pending = EnumMap::default();
            }
        }
    }

    /// Advance the grace period timer whenever any pending action exists.
    /// Called every frame regardless of animation state.
    fn advance_pending_timer(&mut self, dt: f32) {
        let player_count = self.game.state.player_count();
        let any_pending = self
            .pending
            .values()
            .take(player_count)
            .any(|p| p.is_some());
        if any_pending {
            self.pending_timer += dt;
        } else {
            self.pending_timer = 0.0;
        }
    }

    /// Try to execute pending actions if the trigger condition is met.
    fn try_execute_pending(&mut self) {
        if self.game.is_animating() {
            return;
        }

        let player_count = self.game.state.player_count();
        if player_count == 0 {
            return;
        }

        let players = &[Player::Player1, Player::Player2][..player_count];
        let any_pending = players.iter().any(|&p| self.pending[p].is_some());
        let all_pending = players.iter().all(|&p| self.pending[p].is_some());

        if !any_pending {
            return;
        }

        let any_synced = players
            .iter()
            .any(|&p| self.pending[p].is_some_and(|pa| pa.synced));

        if !all_pending {
            // Explicitly synced: wait indefinitely for all players
            if any_synced {
                return;
            }
            // In 2-player, grace period for natural simultaneous input
            if player_count > 1 && self.pending_timer < SYNC_GRACE_PERIOD {
                return;
            }
        }

        // Fire
        self.pending_timer = 0.0;
        let actions: Vec<Action> = players
            .iter()
            .map(|&p| {
                self.pending[p]
                    .take()
                    .map(|pa| pa.action)
                    .unwrap_or(Action::Stall)
            })
            .collect();

        self.game.try_begin_actions(actions);
    }

    fn do_portal_transition(&mut self, level: &str) {
        let level = level.to_string();

        self.stack.enter_level(&self.game, level.clone());
        self.game = load_level(&level, &mut self.game.state.completed_levels);

        // Auto-complete levels with no rats
        if !self.game.initial_has_rats() {
            self.game
                .state
                .mark_level_completed(&self.stack.current_level);
            save_completed_levels(&self.game.state.completed_levels);
        }

        self.input.reset();
        self.pending = EnumMap::default();
    }

    fn handle_portal_transition(&mut self) {
        if self.game.is_animating() {
            return;
        }

        let Some(level) = self.game.state.portal_destination().map(str::to_string) else {
            return;
        };

        self.do_portal_transition(&level);
    }

    /// Run one frame of the game loop. Returns false if the game should exit.
    pub fn tick(&mut self) -> bool {
        self.gamepad.poll();
        let dt = get_frame_time();

        self.handle_portal_transition();

        // Export overlay: consume all input, handle tap/click/esc to copy or dismiss
        if self.export_overlay.is_visible() {
            for action in self.input.poll_meta_inputs(&self.gamepad, dt) {
                if matches!(action, MetaInput::Exit) {
                    self.export_overlay = ExportOverlay::Hidden;
                }
            }
            self.input.poll_player_actions(&self.gamepad, dt);
            if let Some(TouchGesture::Tap(pos)) = self.input.poll_touch() {
                self.handle_overlay_click(pos);
            }
            if let Some(pos) = self.input.poll_mouse_click() {
                self.handle_overlay_click(pos);
            }
        // Handle confirmation dialog input
        } else if self.confirm_dialog != ConfirmDialog::None {
            let mut should_confirm = false;
            for action in self.input.poll_meta_inputs(&self.gamepad, dt) {
                match action {
                    MetaInput::Confirm(_) => should_confirm = true,
                    MetaInput::Undo | MetaInput::Exit => {
                        self.confirm_dialog = ConfirmDialog::None;
                    }
                    _ => {}
                }
            }
            // Consume player actions during dialog (don't let them queue)
            self.input.poll_player_actions(&self.gamepad, dt);

            if let Some(gesture) = self.input.poll_touch()
                && matches!(gesture, TouchGesture::Tap(_))
            {
                should_confirm = true;
            }
            self.input.poll_mouse_click();

            if should_confirm {
                match self.confirm_dialog {
                    ConfirmDialog::Restart => {
                        self.game.restart();
                        self.input.reset();
                        self.pending = EnumMap::default();
                    }
                    ConfirmDialog::Exit => {
                        self.exit_level();
                    }
                    ConfirmDialog::QuitGame => {
                        return false;
                    }
                    ConfirmDialog::None => {}
                }
                self.confirm_dialog = ConfirmDialog::None;
            }
        } else {
            // Poll meta inputs
            for action in self.input.poll_meta_inputs(&self.gamepad, dt) {
                match self.handle_meta_input(action) {
                    MetaInputOutcome::Handled => {}
                    MetaInputOutcome::ChangedLevel => break,
                }
            }

            // Poll per-player actions
            if self.game.state.play_state() == PlayState::Playing {
                let player_actions = self.input.poll_player_actions(&self.gamepad, dt);
                let player_count = self.game.state.player_count();
                for (player, player_action) in player_actions.iter().take(player_count) {
                    if let Some(input) = player_action {
                        self.pending[player] = Some(PendingAction {
                            action: input.action,
                            synced: input.synced,
                            fresh: input.fresh,
                        });
                    } else if self.pending[player].is_some_and(|p| !p.fresh) {
                        // Key was released — clear held-repeat pending so it doesn't
                        // fire as a stale buffered move after animation finishes.
                        self.pending[player] = None;
                    }
                }
            }

            // Touch gestures: swipe → P1 non-synced move, tap → button/overlay
            if let Some(gesture) = self.input.poll_touch() {
                match gesture {
                    TouchGesture::Swipe(dir) => {
                        if self.game.state.play_state() == PlayState::Playing {
                            self.pending[Player::Player1] = Some(PendingAction {
                                action: Action::Move(dir),
                                synced: false,
                                fresh: true,
                            });
                        }
                    }
                    TouchGesture::Tap(pos) => self.handle_tap_or_click(pos),
                }
            }

            if let Some(pos) = self.input.poll_mouse_click() {
                self.handle_tap_or_click(pos);
            }

            self.try_execute_pending();
        }

        self.advance_pending_timer(dt);
        self.game.animate(dt);
        self.handle_portal_transition();
        self.try_execute_pending();

        // Poll for async clipboard import result
        if self.import_pending
            && let Some(imported) = crate::storage::progress::poll_import()
        {
            self.game.state.completed_levels.extend(imported);
            save_completed_levels(&self.game.state.completed_levels);
            self.import_pending = false;
        }

        self.render();

        true
    }

    fn render(&mut self) {
        let hints = self.input_hints();
        let ui = self.ui_state();

        // Priority: note text > completed portal name > current level name
        let gamepad_count = self.gamepad.connected_count();
        let note_text = self
            .game
            .state
            .standing_on_note()
            .map(|t| t.resolve(hints, gamepad_count));
        let portal_name = self.game.state.standing_on_completed_portal();
        let level_name =
            levels::get_level(&self.stack.current_level).map(|l| l.display_name.as_str());
        let description = note_text.or(portal_name).or(level_name);

        render(
            &self.game,
            &self.sprites,
            description,
            &ui,
            hints,
            self.confirm_dialog,
        );
        crate::render::progress_buttons::draw_overlay(&self.export_overlay, self.sprites.font());
        self.gamepad.end_frame();
    }
}
