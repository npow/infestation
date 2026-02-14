use crate::game::{Game, PlayState};
use macroquad::prelude::*;

use super::{cell_size, draw_text_f, grid_offset, measure_text_f};

const LABEL: &str = "Email Solution To Author";
const FONT_SIZE: u16 = 28;

fn button_rect(grid_center_x: f32, grid_center_y: f32, font: &Font) -> (f32, f32, f32, f32) {
    let dims = measure_text_f(LABEL, font, FONT_SIZE);
    let w = dims.width + 24.0;
    let h = dims.height + 16.0;
    let x = grid_center_x - w / 2.0;
    let y = grid_center_y + 140.0;
    (x, y, w, h)
}

pub(crate) fn draw(grid_center_x: f32, grid_center_y: f32, font: &Font) {
    let (btn_x, btn_y, btn_w, btn_h) = button_rect(grid_center_x, grid_center_y, font);
    draw_rectangle(
        btn_x,
        btn_y,
        btn_w,
        btn_h,
        Color::from_rgba(50, 120, 50, 255),
    );
    draw_rectangle_lines(
        btn_x,
        btn_y,
        btn_w,
        btn_h,
        1.0,
        Color::from_rgba(80, 180, 80, 255),
    );
    let dims = measure_text_f(LABEL, font, FONT_SIZE);
    draw_text_f(
        LABEL,
        btn_x + (btn_w - dims.width) / 2.0,
        btn_y + (btn_h + dims.height) / 2.0 - 2.0,
        font,
        FONT_SIZE,
        WHITE,
    );
}

pub(crate) fn hit(pos: Vec2, game: &Game, font: &Font) -> bool {
    if game.state.play_state() != PlayState::Won || game.is_animating() {
        return false;
    }

    let (offset_x, offset_y) = grid_offset(game);
    let cell = cell_size(game);
    let grid_w = game.grid_width() as f32 * cell;
    let grid_h = game.grid_height() as f32 * cell;
    let grid_center_x = offset_x + grid_w / 2.0;
    let grid_center_y = offset_y + grid_h / 2.0;

    let (bx, by, bw, bh) = button_rect(grid_center_x, grid_center_y, font);
    pos.x >= bx && pos.x < bx + bw && pos.y >= by && pos.y < by + bh
}
