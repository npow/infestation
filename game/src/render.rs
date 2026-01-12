use crate::game::{Game, PlayState};
use crate::grid::Cell;
use crate::sprites::Sprites;
use macroquad::prelude::*;
use quad_gamepad::ControllerType;

const PADDING: f32 = 4.0;
const DIALOGUE_HEIGHT: f32 = 160.0;
const DIALOGUE_PADDING: f32 = 12.0;
const BUTTON_BAR_HEIGHT: f32 = 70.0;
const BUTTON_HEIGHT: f32 = 28.0;
const BUTTON_SPACING: f32 = 6.0;

fn text_params(font: &Font, size: u16, color: Color) -> TextParams<'_> {
    TextParams {
        font: Some(font),
        font_size: size,
        color,
        ..Default::default()
    }
}

fn draw_text_f(text: &str, x: f32, y: f32, font: &Font, size: u16, color: Color) {
    draw_text_ex(text, x, y, text_params(font, size, color));
}

fn measure_text_f(text: &str, font: &Font, size: u16) -> TextDimensions {
    measure_text(text, Some(font), size, 1.0)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ButtonAction {
    Reset,
    Undo,
    Stall,
    Exit,
}

pub(crate) struct UiState {
    pub(crate) can_reset: bool,
    pub(crate) can_undo: bool,
    pub(crate) can_exit: bool,
    pub(crate) on_portal: bool,
}

/// Confirmation dialog that requires user acknowledgment.
#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) enum ConfirmDialog {
    #[default]
    None,
    Restart,
    Exit,
    QuitGame,
}

/// Input hint style based on platform and connected controller.
#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) enum InputHints {
    #[default]
    Keyboard,
    Touch,
    Controller(ControllerType),
}

impl InputHints {
    /// Hint text for game over screen.
    pub(crate) fn game_over_hint(self) -> &'static str {
        use ControllerType::*;
        match self {
            InputHints::Keyboard => "U to undo | R to restart",
            InputHints::Touch => "Tap to undo",
            InputHints::Controller(Xbox | Generic) => "X to undo | LB to restart",
            InputHints::Controller(PlayStation) => "□ to undo | L1 to restart",
            InputHints::Controller(Nintendo) => "Y to undo | L to restart",
        }
    }

    /// Hint text for level complete screen.
    pub(crate) fn level_complete_hint(self) -> &'static str {
        use ControllerType::*;
        match self {
            InputHints::Keyboard => "Space to continue",
            InputHints::Touch => "Tap to continue",
            InputHints::Controller(Xbox | Generic) => "A to continue",
            InputHints::Controller(PlayStation) => "✕ to continue",
            InputHints::Controller(Nintendo) => "B to continue",
        }
    }
}

fn draw_cell(cell: Cell, px: f32, py: f32, size: f32, sprites: &Sprites) {
    match cell {
        Cell::Trigger(n) => {
            // Draw digit centered in cell
            let text = &n.to_string();
            let font_size = (size * 0.8) as u16;
            let dims = measure_text_f(text, sprites.font(), font_size);
            let tx = px + (size - dims.width) / 2.0;
            let ty = py + (size + dims.height) / 2.0;
            draw_text_f(text, tx, ty, sprites.font(), font_size, WHITE);
        }
        Cell::Empty => {}
        _ => {
            let texture = match cell {
                Cell::Player(dir) => sprites.player(dir),
                Cell::Rat(dir) => sprites.rat(dir),
                Cell::CyborgRat(dir) => sprites.cyborg_rat(dir),
                Cell::Wall => sprites.wall(),
                Cell::Plank => sprites.planks(),
                Cell::Spiderweb => sprites.spiderweb(),
                Cell::BlackHole => sprites.blackhole(),
                Cell::Explosive => sprites.explosive(),
                _ => return,
            };
            draw_texture_ex(
                texture,
                px,
                py,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    ..Default::default()
                },
            );
        }
    }
}

pub(crate) fn render(
    game: &Game,
    sprites: &Sprites,
    description: Option<&str>,
    ui: &UiState,
    hints: InputHints,
    confirm_dialog: ConfirmDialog,
) {
    let cell = cell_size(game);
    let grid_w = game.grid_width() as f32 * cell;
    let grid_h = game.grid_height() as f32 * cell;
    let (offset_x, offset_y) = grid_offset(game);

    clear_background(Color::from_rgba(30, 30, 40, 255));

    let play_state = game.state.play_state();

    // Button bar at top
    render_button_bar(ui, play_state == PlayState::Playing, hints, sprites.font());

    // Grid lines
    for i in 0..=game.grid_width() {
        let offset = i as f32 * cell;
        draw_line(
            offset_x + offset,
            offset_y,
            offset_x + offset,
            offset_y + grid_h,
            1.0,
            DARKGRAY,
        );
    }
    for i in 0..=game.grid_height() {
        let offset = i as f32 * cell;
        draw_line(
            offset_x,
            offset_y + offset,
            offset_x + grid_w,
            offset_y + offset,
            1.0,
            DARKGRAY,
        );
    }

    // Draw portals first (underneath everything else)
    for (pos, level) in game.state.grid.portals() {
        let texture = sprites.portal(game.is_level_completed(level));
        draw_texture_ex(
            texture,
            offset_x + pos.x as f32 * cell,
            offset_y + pos.y as f32 * cell,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(cell, cell)),
                ..Default::default()
            },
        );
    }

    // Draw note indicators (underneath entities)
    for (pos, _) in game.state.grid.notes() {
        draw_texture_ex(
            sprites.note(),
            offset_x + pos.x as f32 * cell,
            offset_y + pos.y as f32 * cell,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(cell, cell)),
                ..Default::default()
            },
        );
    }

    if let Some(handler) = &game.animation {
        // During animation: render background grid
        // (moving entities are cleared from grid, so no need to skip their source positions)
        for (pos, grid_cell) in handler.grid.entries() {
            draw_cell(
                grid_cell,
                offset_x + pos.x as f32 * cell,
                offset_y + pos.y as f32 * cell,
                cell,
                sprites,
            );
        }

        // Render moving entities at interpolated positions
        for m in &handler.moving {
            let x = m.from.x as f32 + (m.to.x - m.from.x) as f32 * m.progress;
            let y = m.from.y as f32 + (m.to.y - m.from.y) as f32 * m.progress;
            draw_cell(
                m.cell,
                offset_x + x * cell,
                offset_y + y * cell,
                cell,
                sprites,
            );
        }

        // Render zaps - scale from 1 cell to 3x3
        for z in &handler.zapping {
            let scale = 1.0 + 2.0 * z.progress;
            let zap_size = cell * scale;
            let cx = offset_x + z.pos.x as f32 * cell + cell / 2.0;
            let cy = offset_y + z.pos.y as f32 * cell + cell / 2.0;
            let px = cx - zap_size / 2.0;
            let py = cy - zap_size / 2.0;

            draw_texture_ex(
                sprites.zap(),
                px,
                py,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(zap_size, zap_size)),
                    ..Default::default()
                },
            );
        }

        // Render explosions - scale from 1 cell to 3x3
        for e in &handler.exploding {
            // Scale from 1.0 to 3.0 as progress goes from 0.0 to 1.0
            let scale = 1.0 + 2.0 * e.progress;
            let explosion_size = cell * scale;
            // Center the explosion on the cell
            let cx = offset_x + e.pos.x as f32 * cell + cell / 2.0;
            let cy = offset_y + e.pos.y as f32 * cell + cell / 2.0;
            let px = cx - explosion_size / 2.0;
            let py = cy - explosion_size / 2.0;

            draw_texture_ex(
                sprites.explosion(),
                px,
                py,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(explosion_size, explosion_size)),
                    ..Default::default()
                },
            );
        }
    } else {
        // Not animating: render current grid
        for (pos, grid_cell) in game.state.grid.entries() {
            draw_cell(
                grid_cell,
                offset_x + pos.x as f32 * cell,
                offset_y + pos.y as f32 * cell,
                cell,
                sprites,
            );
        }
    }

    // Grid center for overlay text
    let grid_center_x = offset_x + grid_w / 2.0;
    let grid_center_y = offset_y + grid_h / 2.0;

    // Game over overlay
    if play_state == PlayState::GameOver && !game.is_animating() {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 180),
        );

        let font = sprites.font();
        let line1 = "GAME";
        let line2 = "OVER";
        let dims1 = measure_text_f(line1, font, 64);
        let dims2 = measure_text_f(line2, font, 64);
        draw_text_f(
            line1,
            grid_center_x - dims1.width / 2.0,
            grid_center_y - 20.0,
            font,
            64,
            RED,
        );
        draw_text_f(
            line2,
            grid_center_x - dims2.width / 2.0,
            grid_center_y + 50.0,
            font,
            64,
            RED,
        );

        let hint = hints.game_over_hint();
        let hint_dims = measure_text_f(hint, font, 32);
        draw_text_f(
            hint,
            grid_center_x - hint_dims.width / 2.0,
            grid_center_y + 110.0,
            font,
            32,
            WHITE,
        );
    }

    // Level complete overlay
    if play_state == PlayState::Won && !game.is_animating() {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 180),
        );

        let font = sprites.font();
        let line1 = "LEVEL";
        let line2 = "COMPLETE";
        let dims1 = measure_text_f(line1, font, 64);
        let dims2 = measure_text_f(line2, font, 64);
        draw_text_f(
            line1,
            grid_center_x - dims1.width / 2.0,
            grid_center_y - 20.0,
            font,
            64,
            GREEN,
        );
        draw_text_f(
            line2,
            grid_center_x - dims2.width / 2.0,
            grid_center_y + 50.0,
            font,
            64,
            GREEN,
        );

        let hint = hints.level_complete_hint();
        let hint_dims = measure_text_f(hint, font, 32);
        draw_text_f(
            hint,
            grid_center_x - hint_dims.width / 2.0,
            grid_center_y + 110.0,
            font,
            32,
            WHITE,
        );
    }

    // Dialogue area at bottom
    render_dialogue(description, dialogue_y(game), sprites.font());

    // Confirmation dialog on top of everything
    if confirm_dialog != ConfirmDialog::None {
        render_confirm_dialog(confirm_dialog, hints, sprites.font());
    }
}

fn render_confirm_dialog(dialog: ConfirmDialog, hints: InputHints, font: &Font) {
    use ControllerType::*;

    // Semi-transparent overlay
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 200),
    );

    let (title, subtitle) = match dialog {
        ConfirmDialog::Restart => ("RESTART", "level?"),
        ConfirmDialog::Exit => ("EXIT", "level?"),
        ConfirmDialog::QuitGame => ("QUIT", "game?"),
        ConfirmDialog::None => return,
    };

    let confirm_hint = match hints {
        InputHints::Keyboard => "Space to confirm, Esc to cancel",
        InputHints::Touch => "Tap to confirm",
        InputHints::Controller(Xbox | Generic) => "A/B to confirm, X/Y to cancel",
        InputHints::Controller(PlayStation) => "✕/○ to confirm, □/△ to cancel",
        InputHints::Controller(Nintendo) => "B/A to confirm, Y/X to cancel",
    };

    let center_x = screen_width() / 2.0;
    let center_y = screen_height() / 2.0;

    // Title
    let title_dims = measure_text_f(title, font, 64);
    draw_text_f(
        title,
        center_x - title_dims.width / 2.0,
        center_y - 30.0,
        font,
        64,
        YELLOW,
    );

    // Subtitle
    let subtitle_dims = measure_text_f(subtitle, font, 48);
    draw_text_f(
        subtitle,
        center_x - subtitle_dims.width / 2.0,
        center_y + 30.0,
        font,
        48,
        YELLOW,
    );

    // Hint
    let hint_dims = measure_text_f(confirm_hint, font, 28);
    draw_text_f(
        confirm_hint,
        center_x - hint_dims.width / 2.0,
        center_y + 100.0,
        font,
        28,
        WHITE,
    );
}

fn render_dialogue(description: Option<&str>, dialogue_y: f32, font: &Font) {
    let dialogue_height = screen_height() - dialogue_y;

    // Background
    draw_rectangle(
        0.0,
        dialogue_y,
        screen_width(),
        dialogue_height,
        Color::from_rgba(20, 20, 30, 255),
    );

    // Top border
    draw_line(
        0.0,
        dialogue_y,
        screen_width(),
        dialogue_y,
        2.0,
        Color::from_rgba(60, 60, 80, 255),
    );

    // Text with word wrap, preserving explicit newlines
    if let Some(text) = description {
        let font_size: u16 = 26;
        let line_height = font_size as f32 * 1.2;
        let max_width = screen_width() - DIALOGUE_PADDING * 2.0;
        let color = Color::from_rgba(200, 200, 220, 255);

        let wrapped = wrap_text(text, font, font_size, max_width);

        let mut y = dialogue_y + DIALOGUE_PADDING + font_size as f32;
        for line in &wrapped {
            draw_text_f(line, DIALOGUE_PADDING, y, font, font_size, color);
            y += line_height;
        }
    }
}

fn wrap_text(text: &str, font: &Font, font_size: u16, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let test = if line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", line, word)
            };
            if measure_text_f(&test, font, font_size).width > max_width && !line.is_empty() {
                lines.push(line);
                line = word.to_string();
            } else {
                line = test;
            }
        }
        lines.push(line);
    }
    lines
}

fn cell_size(game: &Game) -> f32 {
    let width = screen_width();
    let height = screen_height() - DIALOGUE_HEIGHT - BUTTON_BAR_HEIGHT;
    let cell_w = (width - PADDING * 2.0) / game.grid_width() as f32;
    let cell_h = (height - PADDING * 2.0) / game.grid_height() as f32;
    cell_w.min(cell_h)
}

fn grid_offset(game: &Game) -> (f32, f32) {
    let cell = cell_size(game);
    let grid_w = game.grid_width() as f32 * cell;
    let offset_x = (screen_width() - grid_w) / 2.0;
    let offset_y = BUTTON_BAR_HEIGHT + PADDING; // Grid below button bar
    (offset_x, offset_y)
}

fn dialogue_y(game: &Game) -> f32 {
    let cell = cell_size(game);
    let grid_h = game.grid_height() as f32 * cell;
    let grid_bottom = BUTTON_BAR_HEIGHT + PADDING + grid_h + PADDING;
    // Dialogue starts at grid bottom, but no higher than (screen_height - DIALOGUE_HEIGHT)
    grid_bottom.min(screen_height() - DIALOGUE_HEIGHT)
}

fn button_labels(on_portal: bool, hints: InputHints) -> [(&'static str, ButtonAction); 4] {
    use ControllerType::*;
    use InputHints::*;

    let reset = match hints {
        Keyboard => "Reset (R)",
        Touch => "Reset",
        Controller(Xbox | Generic) => "Reset (LB)",
        Controller(PlayStation) => "Reset (L1)",
        Controller(Nintendo) => "Reset (L)",
    };

    let undo = match hints {
        Keyboard => "Undo (U)",
        Touch => "Undo",
        Controller(Xbox | Generic) => "Undo (X)",
        Controller(PlayStation) => "Undo (□)",
        Controller(Nintendo) => "Undo (Y)",
    };

    let stall = match (on_portal, hints) {
        (true, Keyboard) => "Enter (Space)",
        (false, Keyboard) => "Stall (Space)",
        (true, Touch) => "Enter",
        (false, Touch) => "Stall",
        (true, Controller(Xbox | Generic)) => "Enter (A)",
        (false, Controller(Xbox | Generic)) => "Stall (A)",
        (true, Controller(PlayStation)) => "Enter (✕)",
        (false, Controller(PlayStation)) => "Stall (✕)",
        (true, Controller(Nintendo)) => "Enter (B)",
        (false, Controller(Nintendo)) => "Stall (B)",
    };

    let exit = match hints {
        Keyboard => "Exit (Esc)",
        Touch => "Exit",
        Controller(Xbox | Generic) => "Exit (Menu)",
        Controller(PlayStation) => "Exit (Options)",
        Controller(Nintendo) => "Exit (+)",
    };

    [
        (reset, ButtonAction::Reset),
        (undo, ButtonAction::Undo),
        (stall, ButtonAction::Stall),
        (exit, ButtonAction::Exit),
    ]
}

fn button_rects(
    on_portal: bool,
    hints: InputHints,
    font: &Font,
) -> [(f32, f32, f32, f32, ButtonAction); 4] {
    let buttons = button_labels(on_portal, hints);

    // 2x2 grid: row 0 = buttons 0,1; row 1 = buttons 2,3
    let row_height = BUTTON_HEIGHT + BUTTON_SPACING;
    let y_start = (BUTTON_BAR_HEIGHT - 2.0 * BUTTON_HEIGHT - BUTTON_SPACING) / 2.0;

    let mut rects = [(0.0, 0.0, 0.0, 0.0, ButtonAction::Reset); 4];
    for row in 0..2 {
        let row_buttons: Vec<_> = buttons[row * 2..(row + 1) * 2].to_vec();
        let row_width: f32 = row_buttons
            .iter()
            .map(|(label, _)| measure_text_f(label, font, 22).width + 20.0)
            .sum::<f32>()
            + BUTTON_SPACING;

        let mut x = (screen_width() - row_width) / 2.0;
        let y = y_start + row as f32 * row_height;

        for (i, (label, action)) in row_buttons.iter().enumerate() {
            let w = measure_text_f(label, font, 22).width + 20.0;
            rects[row * 2 + i] = (x, y, w, BUTTON_HEIGHT, *action);
            x += w + BUTTON_SPACING;
        }
    }
    rects
}

fn render_button_bar(ui: &UiState, is_playing: bool, hints: InputHints, font: &Font) {
    // Background
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        BUTTON_BAR_HEIGHT,
        Color::from_rgba(20, 20, 30, 255),
    );

    // Bottom border
    draw_line(
        0.0,
        BUTTON_BAR_HEIGHT,
        screen_width(),
        BUTTON_BAR_HEIGHT,
        2.0,
        Color::from_rgba(60, 60, 80, 255),
    );

    let labels = button_labels(ui.on_portal, hints);
    let enabled_states = [
        (ButtonAction::Reset, ui.can_reset),
        (ButtonAction::Undo, ui.can_undo),
        (ButtonAction::Stall, is_playing),
        (ButtonAction::Exit, ui.can_exit),
    ];

    for (x, y, w, h, action) in button_rects(ui.on_portal, hints, font) {
        let (label, _) = labels.iter().find(|&&(_, a)| a == action).unwrap();
        let enabled = enabled_states
            .iter()
            .find(|&&(a, _)| a == action)
            .map(|&(_, e)| e)
            .unwrap();

        let (bg_color, text_color) = if enabled {
            (
                Color::from_rgba(50, 50, 60, 255),
                Color::from_rgba(220, 220, 230, 255),
            )
        } else {
            (
                Color::from_rgba(35, 35, 42, 255),
                Color::from_rgba(80, 80, 90, 255),
            )
        };

        draw_rectangle(x, y, w, h, bg_color);
        draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(70, 70, 85, 255));

        let dims = measure_text_f(label, font, 22);
        draw_text_f(
            label,
            x + (w - dims.width) / 2.0,
            y + (h + dims.height) / 2.0 - 2.0,
            font,
            22,
            text_color,
        );
    }
}

/// Returns the button action at the given screen position, if any
pub(crate) fn button_at_position(
    pos: Vec2,
    ui: &UiState,
    is_playing: bool,
    hints: InputHints,
    font: &Font,
) -> Option<ButtonAction> {
    for (bx, by, bw, bh, action) in button_rects(ui.on_portal, hints, font) {
        if pos.x >= bx && pos.x < bx + bw && pos.y >= by && pos.y < by + bh {
            let enabled = match action {
                ButtonAction::Reset => ui.can_reset,
                ButtonAction::Undo => ui.can_undo,
                ButtonAction::Stall => is_playing,
                ButtonAction::Exit => ui.can_exit,
            };
            if enabled {
                return Some(action);
            }
        }
    }
    None
}
