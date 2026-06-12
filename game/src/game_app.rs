use std::collections::HashSet;
use std::mem;

use enum_map::EnumMap;
use macroquad::prelude::*;
use quad_gamepad::GamepadContext;

use crate::enum_all::EnumAll;
use crate::game::{Game, PlayState};
use crate::grid::Player;
use crate::input::{
    ClickAction, FrameOutput, GameContext, InputProcessor, MetaInput, read_frame_input,
};
use crate::level_stack::LevelStack;
use crate::levels;
use crate::render::progress_buttons::{ExportOverlay, ProgressAction};
use crate::render::{
    ButtonAction, ConfirmDialog, InputHints, UiState, button_bar_y, button_rects, render,
};
use crate::screen_wake;
use crate::sprites::Sprites;
use crate::storage::{load_completed_levels, save_completed_levels};

fn load_level(name: &str, completed_levels: &mut HashSet<String>) -> Game {
    let level = levels::get_level(name).unwrap_or_else(|| panic!("Level not found: {}", name));
    Game::new(level.grid.clone(), mem::take(completed_levels))
}

pub struct App {
    game: Game,
    stack: LevelStack,
    input: InputProcessor,
    gamepad: GamepadContext,
    sprites: Sprites,
    confirm_dialog: ConfirmDialog,
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
            input: InputProcessor::default(),
            gamepad: GamepadContext::new(),
            sprites,
            confirm_dialog: ConfirmDialog::None,
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

    fn game_context(&self) -> GameContext {
        let ui = self.ui_state();
        let hints = self.input_hints();
        let play_state = self.game.state.play_state();
        let is_playing = play_state == PlayState::Playing;

        let mut click_targets = Vec::new();

        // Main UI buttons
        let bar_y = button_bar_y();
        for (bx, by, bw, bh, action) in
            button_rects(ui.on_portal, hints, bar_y, self.sprites.font())
        {
            let enabled = match action {
                ButtonAction::Reset => ui.can_reset,
                ButtonAction::Undo => ui.can_undo,
                ButtonAction::Stall => is_playing,
                ButtonAction::Exit => ui.can_exit,
            };
            if enabled {
                let click_action = match action {
                    ButtonAction::Reset => ClickAction::Meta(MetaInput::Restart),
                    ButtonAction::Undo => ClickAction::Meta(MetaInput::Undo),
                    ButtonAction::Exit => ClickAction::Meta(MetaInput::Exit),
                    ButtonAction::Stall => ClickAction::Stall,
                };
                click_targets.push((Rect::new(bx, by, bw, bh), click_action));
            }
        }

        // Progress export/import buttons
        for (bx, by, bw, bh, action) in
            crate::render::progress_buttons::button_rects(self.sprites.font())
        {
            let meta = match action {
                ProgressAction::Export => MetaInput::Export,
                ProgressAction::Import => MetaInput::Import,
            };
            click_targets.push((Rect::new(bx, by, bw, bh), ClickAction::Meta(meta)));
        }

        // Overlay copy button
        if let ExportOverlay::Showing(ref encoded) = self.export_overlay {
            let (bx, by, bw, bh) =
                crate::render::progress_buttons::overlay_copy_rect(encoded, self.sprites.font());
            click_targets.push((
                Rect::new(bx, by, bw, bh),
                ClickAction::Meta(MetaInput::OverlayCopy),
            ));
        }

        // Email solution button (WASM only, when won)
        #[cfg(target_arch = "wasm32")]
        if !self.game.is_animating() && play_state == PlayState::Won {
            let (offset_x, offset_y) = crate::render::grid_offset(&self.game);
            let cell = crate::render::cell_size(&self.game);
            let grid_w = self.game.grid_width() as f32 * cell;
            let grid_h = self.game.grid_height() as f32 * cell;
            let grid_center_x = offset_x + grid_w / 2.0;
            let grid_center_y = offset_y + grid_h / 2.0;
            let (bx, by, bw, bh) = crate::render::email_button::button_rect(
                grid_center_x,
                grid_center_y,
                self.sprites.font(),
            );
            click_targets.push((
                Rect::new(bx, by, bw, bh),
                ClickAction::Meta(MetaInput::EmailSolution),
            ));
        }

        GameContext {
            player_count: self.game.state.player_count(),
            is_animating: self.game.is_animating(),
            play_state,
            can_undo: ui.can_undo,
            can_exit: self.stack.can_exit(),
            dialog_active: self.confirm_dialog != ConfirmDialog::None,
            overlay_active: self.export_overlay.is_visible(),
            on_completed_portal: EnumMap::from_fn(|p| {
                self.game.state.player_on_completed_portal(p)
            }),
            click_targets,
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
                .mark_level_completed(&self.stack.current_level);
            save_completed_levels(&self.game.state.completed_levels);
        }
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

    /// Handle a meta action. Returns false if the game should exit.
    fn handle_meta(&mut self, action: MetaInput) -> bool {
        match action {
            MetaInput::Undo => {
                if self.confirm_dialog != ConfirmDialog::None {
                    self.confirm_dialog = ConfirmDialog::None;
                } else {
                    self.game.undo();
                }
            }
            MetaInput::Restart => {
                if self.game.state.history.len() > 1 {
                    self.confirm_dialog = ConfirmDialog::Restart;
                }
            }
            MetaInput::Exit => {
                if self.export_overlay.is_visible() {
                    self.export_overlay = ExportOverlay::Hidden;
                } else if self.confirm_dialog != ConfirmDialog::None {
                    self.confirm_dialog = ConfirmDialog::None;
                } else {
                    self.confirm_dialog = if self.stack.can_exit() {
                        ConfirmDialog::Exit
                    } else {
                        ConfirmDialog::QuitGame
                    };
                }
            }
            MetaInput::Confirm(player) => {
                if self.confirm_dialog != ConfirmDialog::None {
                    match self.confirm_dialog {
                        ConfirmDialog::Restart => {
                            self.game.restart();
                        }
                        ConfirmDialog::Exit => {
                            self.exit_level();
                        }
                        ConfirmDialog::QuitGame => {
                            self.confirm_dialog = ConfirmDialog::None;
                            return false;
                        }
                        ConfirmDialog::None => {}
                    }
                    self.confirm_dialog = ConfirmDialog::None;
                } else {
                    let play_state = self.game.state.play_state();
                    if !self.game.is_animating()
                        && play_state == PlayState::Won
                        && self.stack.can_exit()
                    {
                        self.exit_level();
                    } else if play_state == PlayState::Playing {
                        let level = match player {
                            Some(player) => self.game.enter_portal(player).map(str::to_string),
                            None => {
                                let mut level = None;
                                for player in Player::iter_all() {
                                    if let Some(dest) = self.game.enter_portal(player) {
                                        level = Some(dest.to_string());
                                        break;
                                    }
                                }
                                level
                            }
                        };
                        if let Some(level) = level {
                            self.do_portal_transition(&level);
                        }
                    }
                }
            }
            MetaInput::Export => {
                let encoded = crate::storage::progress::encode(&self.game.state.completed_levels);
                self.export_overlay = ExportOverlay::Showing(encoded);
            }
            MetaInput::Import => {
                crate::storage::progress::start_import();
                self.import_pending = true;
            }
            MetaInput::OverlayCopy => {
                if let ExportOverlay::Showing(ref encoded) = self.export_overlay {
                    crate::storage::progress::copy_to_clipboard(encoded);
                    let encoded = encoded.clone();
                    self.export_overlay = ExportOverlay::Copied(encoded);
                }
            }
            MetaInput::EmailSolution => {
                #[cfg(target_arch = "wasm32")]
                {
                    let csv = self.game.state.initial_grid.to_csv();
                    let level_name = &self.stack.current_level;
                    let encoded = crate::solution::encode_solution(
                        level_name,
                        &csv,
                        &self.game.state.action_history,
                    );
                    let url = crate::solution::mailto_url(level_name, &encoded);
                    crate::open_url::open(&url);
                }
            }
        }
        true
    }

    /// Run one frame of the game loop. Returns false if the game should exit.
    pub fn tick(&mut self) -> bool {
        self.gamepad.poll();
        let frame_input = read_frame_input(&self.gamepad);

        self.handle_portal_transition();

        let ctx = self.game_context();
        match self.input.process(&frame_input, &ctx) {
            FrameOutput::None => {}
            FrameOutput::PlayerActions(actions) => {
                self.game.try_begin_actions(actions);
            }
            FrameOutput::MetaAction(action) => {
                if !self.handle_meta(action) {
                    return false;
                }
            }
        }

        self.game.animate(frame_input.dt);
        self.handle_portal_transition();

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
