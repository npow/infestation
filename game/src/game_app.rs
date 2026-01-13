use std::collections::HashSet;
use std::mem;

use macroquad::prelude::*;
use quad_gamepad::GamepadContext;

use crate::game::{Action, Game, PlayState};
use crate::input::{Input, InputState, TouchGesture};
use crate::level_stack::LevelStack;
use crate::levels;
use crate::render::{ButtonAction, ConfirmDialog, InputHints, UiState, button_at_position, render};
use crate::screen_wake;
use crate::sprites::Sprites;
use crate::storage::{load_completed_levels, save_completed_levels};

fn load_level(name: &str, completed_levels: &mut HashSet<String>) -> Game {
    let level = levels::get_level(name).unwrap_or_else(|| panic!("Level not found: {}", name));
    Game::new(level.grid.clone(), mem::take(completed_levels))
}

fn button_action_to_input(action: ButtonAction) -> Input {
    match action {
        ButtonAction::Reset => Input::Restart,
        ButtonAction::Undo => Input::Undo,
        ButtonAction::Exit => Input::Exit,
        ButtonAction::Stall => Input::Confirm,
    }
}

pub struct App {
    game: Game,
    stack: LevelStack,
    input: InputState,
    gamepad: GamepadContext,
    sprites: Sprites,
    confirm_dialog: ConfirmDialog,
}

impl App {
    pub fn new(sprites: Sprites) -> Self {
        screen_wake::request();

        let mut completed = load_completed_levels();
        let game = load_level("world", &mut completed);
        let stack = LevelStack::new("world".to_string());
        Self {
            game,
            stack,
            input: InputState::new(),
            gamepad: GamepadContext::new(),
            sprites,
            confirm_dialog: ConfirmDialog::None,
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
        }
    }

    fn handle_input(&mut self, action: Input) {
        match action {
            Input::Restart => {
                if self.game.state.history.len() > 1 {
                    self.confirm_dialog = ConfirmDialog::Restart;
                }
            }
            Input::Undo => {
                self.game.undo();
            }
            Input::Exit => {
                self.confirm_dialog = if self.stack.can_exit() {
                    ConfirmDialog::Exit
                } else {
                    ConfirmDialog::QuitGame
                };
            }
            Input::Confirm => {
                let play_state = self.game.state.play_state();
                if !self.game.is_animating()
                    && play_state == PlayState::Won
                    && self.stack.can_exit()
                {
                    self.exit_level();
                } else if play_state == PlayState::Playing {
                    if let Some(level) = self.game.enter_portal().map(str::to_string) {
                        self.do_portal_transition(&level);
                    } else {
                        self.game.try_begin_action(Action::Stall);
                    }
                }
            }
            Input::Move(dir) => {
                if self.game.state.play_state() == PlayState::Playing {
                    self.game.try_begin_action(Action::Move(dir));
                }
            }
        }
    }

    fn handle_tap_or_click(&mut self, pos: Vec2) {
        let hints = self.input_hints();
        let ui = self.ui_state();

        let play_state = self.game.state.play_state();
        let is_playing = play_state == PlayState::Playing;
        if let Some(action) = button_at_position(pos, &ui, is_playing, hints, self.sprites.font()) {
            self.handle_input(button_action_to_input(action));
            return;
        }

        // Tap on overlay screens (not on a button)
        if !self.game.is_animating() {
            if play_state == PlayState::Won && self.stack.can_exit() {
                self.exit_level();
            } else if play_state == PlayState::GameOver {
                self.game.undo();
            }
        }
    }

    fn do_portal_transition(&mut self, level: &str) {
        let level = level.to_string();

        self.stack.enter_level(&self.game, level.clone());
        self.game = load_level(&level, &mut self.game.state.completed_levels);

        // Auto-complete levels with no rats
        if !self.game.initial_has_rats() {
            self.game
                .state
                .completed_levels
                .insert(self.stack.current_level.clone());
            save_completed_levels(&self.game.state.completed_levels);
        }

        self.input.reset();
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
            for action in self.input.poll_keyboard_gamepad(&self.gamepad, dt) {
                match action {
                    Input::Confirm => should_confirm = true,
                    Input::Undo | Input::Exit => self.confirm_dialog = ConfirmDialog::None,
                    _ => {}
                }
            }
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
            for action in self.input.poll_keyboard_gamepad(&self.gamepad, dt) {
                self.handle_input(action);
            }

            if let Some(gesture) = self.input.poll_touch() {
                match gesture {
                    TouchGesture::Swipe(dir) => self.handle_input(Input::Move(dir)),
                    TouchGesture::Tap(pos) => self.handle_tap_or_click(pos),
                }
            }

            if let Some(pos) = self.input.poll_mouse_click() {
                self.handle_tap_or_click(pos);
            }
        }

        self.game.animate(dt);
        self.render();

        true
    }

    fn render(&mut self) {
        let hints = self.input_hints();
        let ui = self.ui_state();

        // Priority: note text > completed portal name > current level name
        let note_text = self.game.state.standing_on_note();
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
