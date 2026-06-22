use std::collections::HashSet;
use std::mem;

use macroquad::prelude::*;
use quad_gamepad::GamepadContext;

use enum_map::EnumMap;

use crate::game::{Action, Game, PlayState};
use crate::grid::{Cell, Player};
use crate::input::{InputState, MetaInput, PlayerInput, PointerEvent, TouchGesture};
use crate::level_stack::{LevelStack, TOP};
use crate::levels;
use crate::path::{NextMove, PathDrag, PathFollower};
use crate::position::Position;
use crate::render::progress_buttons::ExportOverlay;
use crate::render::{
    ButtonAction, ConfirmDialog, InputHints, Overlays, UiState, button_at_position, button_bar_y,
    confirm_dialog_hit, render, screen_to_grid,
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
    /// Path being dragged out with a finger or mouse button held down.
    drag: Option<PathDrag>,
    /// Active paths the players are following.
    paths: EnumMap<Player, Option<PathFollower>>,
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

        let level_name = level_name.unwrap_or(TOP);
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
            drag: None,
            paths: EnumMap::default(),
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

    /// Text for the dialogue strip.
    /// Priority: note text > completed portal name > current level name.
    fn description(&self) -> Option<String> {
        let note_text = self
            .game
            .state
            .standing_on_note()
            .map(|t| t.resolve(self.input_hints(), self.gamepad.connected_count()));
        note_text
            .or_else(|| self.game.state.standing_on_completed_portal())
            .map(String::from)
            .or_else(|| {
                levels::get_level(&self.stack.current_level).map(|l| l.display_name.clone())
            })
    }

    /// Current height of the dialogue strip, which the grid layout depends on.
    fn dialogue_height(&self) -> f32 {
        crate::render::dialogue_height(self.description().as_deref(), self.sprites.font())
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
            self.reset_input();
        }
    }

    /// Drop the in-progress drag and all paths being followed.
    fn clear_paths(&mut self) {
        self.drag = None;
        self.paths = EnumMap::default();
    }

    /// Reset all transient input: held keys, buffered actions, and paths.
    fn reset_input(&mut self) {
        self.input.reset();
        self.pending = EnumMap::default();
        self.clear_paths();
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

    /// Show the encoded solution in the copyable overlay.
    fn show_solution(&mut self) {
        let csv = self.game.state.initial_grid.to_csv();
        let level_name = &self.stack.current_level;
        let encoded =
            crate::solution::encode_solution(level_name, &csv, &self.game.state.action_history);
        self.export_overlay = ExportOverlay::Showing(encoded);
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
                self.clear_paths();
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

    /// Hit-test a tap/click against the on-screen button bar.
    fn button_at(&self, pos: Vec2) -> Option<ButtonAction> {
        let is_playing = self.game.state.play_state() == PlayState::Playing;
        button_at_position(
            pos,
            &self.ui_state(),
            is_playing,
            self.input_hints(),
            button_bar_y(),
            self.sprites.font(),
        )
    }

    /// Handle a tap/click that didn't hit a button-bar button.
    fn handle_tap_or_click(&mut self, pos: Vec2) {
        let play_state = self.game.state.play_state();

        // Show Solution button on win screen
        if !self.game.is_animating()
            && play_state == PlayState::Won
            && crate::render::solution_button::hit(
                pos,
                &self.game,
                self.sprites.font(),
                self.dialogue_height(),
            )
        {
            self.show_solution();
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

    /// A finger/mouse press: stops any path being followed, and starts a
    /// path drag if it lands on a player.
    fn pointer_down(&mut self, pos: Vec2) {
        self.clear_paths();
        if self.game.state.play_state() != PlayState::Playing {
            return;
        }
        let Some(cell) = screen_to_grid(&self.game, self.dialogue_height(), pos) else {
            return;
        };
        if let Some((player, _)) = self.game.state.grid.at(cell).as_player() {
            self.drag = Some(PathDrag::new(player, cell));
        }
    }

    fn pointer_moved(&mut self, pos: Vec2) {
        if self.drag.is_none() {
            return;
        }
        let Some(cell) = screen_to_grid(&self.game, self.dialogue_height(), pos) else {
            return;
        };
        let grid = &self.game.state.grid;
        if let Some(drag) = &mut self.drag {
            // Only walls are undrawable; anything else (planks included)
            // might be gone by the time the player gets there.
            drag.extend_to(cell, |p| grid.at(p) == Cell::Wall);
        }
    }

    /// Feed the next move from each player's active path into this frame's
    /// actions. Manual input for a player abandons that player's path, as
    /// does a wall ahead or the player having strayed off the path.
    fn feed_path_moves(&mut self, player_actions: &mut EnumMap<Player, Option<PlayerInput>>) {
        for (player, follower) in self.paths.iter_mut() {
            let Some(active) = follower else {
                continue;
            };
            if player_actions[player].is_some() {
                *follower = None;
                continue;
            }
            if self.game.is_animating() || self.pending[player].is_some() {
                continue;
            }
            let next = self
                .game
                .state
                .player_position(player)
                .map(|pos| (pos, active.next_move(pos)));
            if let Some((pos, NextMove::Move(dir))) = next
                && !self.game.state.grid.at(pos + dir.delta()).blocks_player()
            {
                player_actions[player] = Some(PlayerInput {
                    action: Action::Move(dir),
                    synced: false,
                    fresh: true,
                });
            } else {
                *follower = None;
            }
        }
    }

    /// Cell paths to draw: the path being dragged out, plus the remaining
    /// path of each player still following one.
    fn path_overlays(&self) -> Vec<Vec<Position>> {
        let drag = self.drag.iter().map(|d| d.cells.clone());
        let following = self.paths.iter().filter_map(|(player, follower)| {
            let pos = self.game.state.player_position(player)?;
            Some(follower.as_ref()?.preview(pos))
        });
        drag.chain(following).collect()
    }

    /// Ghosts for explicitly synced (shift-held) preregistered moves: each
    /// shows the player translucently where it will end up (in place if the
    /// move is blocked). Momentary pendings from normal keypresses (the 2P
    /// grace period, animation buffering) don't ghost.
    fn pending_ghosts(&self) -> Vec<(Position, Cell)> {
        self.pending
            .iter()
            .filter_map(|(player, pending)| {
                let Some(PendingAction {
                    action: Action::Move(dir),
                    synced: true,
                    ..
                }) = *pending
                else {
                    return None;
                };
                let pos = self.game.state.player_position(player)?;
                let dest = pos + dir.delta();
                let dest = if self.game.state.grid.at(dest).blocks_player() {
                    pos
                } else {
                    dest
                };
                Some((dest, Cell::Player(player, dir)))
            })
            .collect()
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

        self.reset_input();
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
            for event in self.input.poll_pointer() {
                if let Some(TouchGesture::Tap(pos)) = event.gesture() {
                    self.handle_overlay_click(pos);
                }
            }
        // Handle confirmation dialog input
        } else if self.confirm_dialog != ConfirmDialog::None {
            let mut should_confirm = false;
            let mut should_cancel = false;
            for action in self.input.poll_meta_inputs(&self.gamepad, dt) {
                match action {
                    MetaInput::Confirm(_) => should_confirm = true,
                    MetaInput::Undo | MetaInput::Exit => should_cancel = true,
                    _ => {}
                }
            }
            // Consume player actions during dialog (don't let them queue)
            self.input.poll_player_actions(&self.gamepad, dt);

            // Yes confirms; No or a tap anywhere else cancels
            for event in self.input.poll_pointer() {
                if let Some(TouchGesture::Tap(pos)) = event.gesture() {
                    match confirm_dialog_hit(pos) {
                        Some(true) => should_confirm = true,
                        Some(false) | None => should_cancel = true,
                    }
                }
            }

            if should_cancel {
                self.confirm_dialog = ConfirmDialog::None;
            } else if should_confirm {
                match self.confirm_dialog {
                    ConfirmDialog::Restart => {
                        self.game.restart();
                        self.reset_input();
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
            // Touch/mouse: a drag starting on a player drags out a path for
            // it to follow; swipes and button-bar taps feed the same
            // meta/player input streams as the keyboard; other taps are
            // handled per-overlay.
            let mut swipe = None;
            let mut tap_pos = None;
            for event in self.input.poll_pointer() {
                match event {
                    PointerEvent::Down(pos) => self.pointer_down(pos),
                    PointerEvent::Moved(pos) => self.pointer_moved(pos),
                    PointerEvent::Up { .. } => {
                        if let Some(drag) = self.drag.take() {
                            let player = drag.player;
                            self.paths[player] = drag.into_follower();
                        } else {
                            match event.gesture() {
                                Some(TouchGesture::Swipe(dir)) => swipe = Some(dir),
                                Some(TouchGesture::Tap(pos)) => tap_pos = Some(pos),
                                None => {}
                            }
                        }
                    }
                }
            }
            let tapped_button = tap_pos.and_then(|pos| self.button_at(pos));

            let mut meta_inputs = self.input.poll_meta_inputs(&self.gamepad, dt);
            match tapped_button {
                Some(ButtonAction::Reset) => meta_inputs.push(MetaInput::Restart),
                Some(ButtonAction::Undo) => meta_inputs.push(MetaInput::Undo),
                Some(ButtonAction::Exit) => meta_inputs.push(MetaInput::Exit),
                // Like Space: enter a portal if a player is standing on one
                Some(ButtonAction::Stall) => meta_inputs.extend([
                    MetaInput::Confirm(Player::Player1),
                    MetaInput::Confirm(Player::Player2),
                ]),
                None => {}
            }

            let mut changed_level = false;
            for action in meta_inputs {
                match self.handle_meta_input(action) {
                    MetaInputOutcome::Handled => {}
                    MetaInputOutcome::ChangedLevel => {
                        changed_level = true;
                        break;
                    }
                }
            }

            // Poll per-player actions
            if self.game.state.play_state() == PlayState::Playing {
                let mut player_actions = self.input.poll_player_actions(&self.gamepad, dt);
                if !changed_level {
                    if let Some(dir) = swipe {
                        player_actions[Player::Player1] = Some(PlayerInput {
                            action: Action::Move(dir),
                            synced: false,
                            fresh: true,
                        });
                    }
                    // Like Space: stall any player without a move this frame
                    if matches!(tapped_button, Some(ButtonAction::Stall)) {
                        let stall = PlayerInput {
                            action: Action::Stall,
                            synced: false,
                            fresh: true,
                        };
                        for (_, action) in player_actions.iter_mut() {
                            *action = action.or(Some(stall));
                        }
                    }
                    self.feed_path_moves(&mut player_actions);
                }
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

            if !changed_level
                && tapped_button.is_none()
                && let Some(pos) = tap_pos
            {
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
        let description = self.description();

        render(
            &self.game,
            &self.sprites,
            description.as_deref(),
            &ui,
            hints,
            self.confirm_dialog,
            &Overlays {
                paths: self.path_overlays(),
                ghosts: self.pending_ghosts(),
            },
        );
        crate::render::progress_buttons::draw_overlay(&self.export_overlay, self.sprites.font());
        self.gamepad.end_frame();
    }
}
