use crate::game::{Game, PlayState};
use macroquad::prelude::*;

use super::{cell_size, draw_text_f, grid_offset, measure_text_f};

const LABEL: &str = "Show Solution Code";
const FONT_SIZE: u16 = 28;

fn button_rect(grid_center_x: f32, grid_center_y: f32, font: &Font) -> Rect {
    let dims = measure_text_f(LABEL, font, FONT_SIZE);
    let w = dims.width + 24.0;
    let h = dims.height + 16.0;
    Rect::new(grid_center_x - w / 2.0, grid_center_y + 140.0, w, h)
}

pub(crate) fn draw(grid_center_x: f32, grid_center_y: f32, font: &Font) {
    let rect = button_rect(grid_center_x, grid_center_y, font);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::from_rgba(50, 120, 50, 255),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(80, 180, 80, 255),
    );
    let dims = measure_text_f(LABEL, font, FONT_SIZE);
    draw_text_f(
        LABEL,
        rect.x + (rect.w - dims.width) / 2.0,
        rect.y + (rect.h + dims.height) / 2.0 - 2.0,
        font,
        FONT_SIZE,
        WHITE,
    );
}

pub(crate) fn hit(pos: Vec2, game: &Game, font: &Font, dialogue_height: f32) -> bool {
    if game.state.play_state() != PlayState::Won || game.is_animating() {
        return false;
    }

    let (offset_x, offset_y) = grid_offset(game, dialogue_height);
    let cell = cell_size(game, dialogue_height);
    let grid_w = game.grid_width() as f32 * cell;
    let grid_h = game.grid_height() as f32 * cell;
    let grid_center_x = offset_x + grid_w / 2.0;
    let grid_center_y = offset_y + grid_h / 2.0;

    button_rect(grid_center_x, grid_center_y, font).contains(pos)
}
