use std::collections::HashSet;
use std::mem;

use macroquad::prelude::*;
use quad_gamepad::GamepadContext;

use crate::game::{Action, Game, PlayState};
use crate::input::{InputState, MetaInput, TouchGesture};
use crate::level_stack::LevelStack;
use crate::levels;
use crate::render::{
    ButtonAction, ConfirmDialog, InputHints, UiState, button_at_position, button_bar_y, render,
};
use crate::screen_wake;
use crate::sprites::Sprites;
use crate::storage::{load_completed_levels, save_completed_levels};

fn load_level(name: &str, completed_levels: &mut HashSet<String>) -> Game {
    let level = levels::get_level(name).unwrap_or_else(|| panic!("Level not found: {}", name));
    Game::new(level.grid.clone(), mem::take(completed_levels))
}

#[derive(Clone, Copy)]
struct PendingAction {
    action: Action,
    synced: bool,
}

pub struct App {
    game: Game,
    stack: LevelStack,
    input: InputState,
    gamepad: GamepadContext,
    sprites: Sprites,
    confirm_dialog: ConfirmDialog,
    /// Per-player buffered actions (sync or immediate).
    pending: [Option<PendingAction>; 2],
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
            pending: [None; 2],
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
            self.pending = [None; 2];
        }
    }

    fn handle_meta_input(&mut self, action: MetaInput) {
        match action {
            MetaInput::Restart => {
                if self.game.state.history.len() > 1 {
                    self.confirm_dialog = ConfirmDialog::Restart;
                }
            }
            MetaInput::Undo => {
                self.game.undo();
                self.pending = [None; 2];
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
                } else if play_state == PlayState::Playing
                    && let Some(level) = self.game.enter_portal(player).map(str::to_string)
                {
                    self.do_portal_transition(&level);
                }
            }
        }
    }

    fn handle_button_action(&mut self, action: ButtonAction) {
        match action {
            ButtonAction::Reset => self.handle_meta_input(MetaInput::Restart),
            ButtonAction::Undo => self.handle_meta_input(MetaInput::Undo),
            ButtonAction::Exit => self.handle_meta_input(MetaInput::Exit),
            ButtonAction::Stall => {
                // UI stall button acts as P1 non-synced stall
                if self.game.state.play_state() == PlayState::Playing {
                    self.pending[0] = Some(PendingAction {
                        action: Action::Stall,
                        synced: false,
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

        // Tap on overlay screens (not on a button)
        if !self.game.is_animating() {
            if play_state == PlayState::Won && self.stack.can_exit() {
                self.exit_level();
            } else if play_state == PlayState::GameOver {
                self.game.undo();
                self.pending = [None; 2];
            }
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

        let has_non_synced = self.pending[..player_count]
            .iter()
            .any(|p| p.is_some_and(|pa| !pa.synced));
        let all_synced = self.pending[..player_count]
            .iter()
            .all(|p| p.is_some_and(|pa| pa.synced));

        if !has_non_synced && !all_synced {
            return;
        }

        // Build actions and consume all pending
        let actions: Vec<Option<Action>> = (0..player_count)
            .map(|i| self.pending[i].take().map(|pa| pa.action))
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
        self.pending = [None; 2];
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

        // Handle confirmation dialog input
        if self.confirm_dialog != ConfirmDialog::None {
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
                        self.pending = [None; 2];
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
                self.handle_meta_input(action);
            }

            // Poll per-player actions
            if self.game.state.play_state() == PlayState::Playing {
                let player_actions = self.input.poll_player_actions(&self.gamepad, dt);
                let player_count = self.game.state.player_count();
                for (i, player_action) in
                    player_actions.iter().enumerate().take(player_count.min(2))
                {
                    if let Some((action, synced)) = player_action {
                        self.pending[i] = Some(PendingAction {
                            action: *action,
                            synced: *synced,
                        });
                    }
                }
            }

            // Touch gestures: swipe → P1 non-synced move, tap → button/overlay
            if let Some(gesture) = self.input.poll_touch() {
                match gesture {
                    TouchGesture::Swipe(dir) => {
                        if self.game.state.play_state() == PlayState::Playing {
                            self.pending[0] = Some(PendingAction {
                                action: Action::Move(dir),
                                synced: false,
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

        self.game.animate(dt);
        self.handle_portal_transition();
        self.try_execute_pending();
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
        self.gamepad.end_frame();
    }
}
