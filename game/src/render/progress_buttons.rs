use macroquad::prelude::*;

use super::{
    BOTTOM_SAFE_AREA, BUTTON_BAR_HEIGHT, BUTTON_HEIGHT, BUTTON_SPACING, draw_text_f, measure_text_f,
};

const FONT_SIZE: u16 = 20;
const RIGHT_MARGIN: f32 = 12.0;

const EXPORT_LABEL: &str = "Export";
const IMPORT_LABEL: &str = "Import";

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ProgressAction {
    Export,
    Import,
}

fn button_rects(font: &Font) -> [(Rect, ProgressAction); 2] {
    let export_w = measure_text_f(EXPORT_LABEL, font, FONT_SIZE).width + 20.0;
    let import_w = measure_text_f(IMPORT_LABEL, font, FONT_SIZE).width + 20.0;
    let btn_w = export_w.max(import_w);
    let x = screen_width() - btn_w - RIGHT_MARGIN;
    let bar_y = screen_height() - BUTTON_BAR_HEIGHT - BOTTOM_SAFE_AREA;
    let y_start = bar_y + (BUTTON_BAR_HEIGHT - 2.0 * BUTTON_HEIGHT - BUTTON_SPACING) / 2.0;

    [
        (
            Rect::new(x, y_start, btn_w, BUTTON_HEIGHT),
            ProgressAction::Export,
        ),
        (
            Rect::new(
                x,
                y_start + BUTTON_HEIGHT + BUTTON_SPACING,
                btn_w,
                BUTTON_HEIGHT,
            ),
            ProgressAction::Import,
        ),
    ]
}

pub(crate) fn draw(font: &Font) {
    let bg = Color::from_rgba(40, 40, 55, 255);
    let border = Color::from_rgba(70, 70, 85, 255);
    let text_color = Color::from_rgba(180, 180, 200, 255);

    for (rect, action) in button_rects(font) {
        let label = match action {
            ProgressAction::Export => EXPORT_LABEL,
            ProgressAction::Import => IMPORT_LABEL,
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, border);
        let dims = measure_text_f(label, font, FONT_SIZE);
        draw_text_f(
            label,
            rect.x + (rect.w - dims.width) / 2.0,
            rect.y + (rect.h + dims.height) / 2.0 - 2.0,
            font,
            FONT_SIZE,
            text_color,
        );
    }
}

pub(crate) fn hit(pos: Vec2, font: &Font) -> Option<ProgressAction> {
    button_rects(font)
        .into_iter()
        .find(|(rect, _)| rect.contains(pos))
        .map(|(_, action)| action)
}

// === Export Overlay ===

#[derive(Default)]
pub(crate) enum ExportOverlay {
    #[default]
    Hidden,
    Showing(String),
    Copied(String),
}

impl ExportOverlay {
    pub(crate) fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }
}

const OVL_CODE_FONT: u16 = 16;
const OVL_BTN_FONT: u16 = 22;
const OVL_PADDING: f32 = 16.0;
const OVL_MAX_WIDTH: f32 = 480.0;
const OVL_COPY_BTN_H: f32 = 36.0;
const OVL_TEXT_PAD: f32 = 6.0;
const OVL_GAP: f32 = 14.0;

fn wrap_chars(text: &str, font: &Font, font_size: u16, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for ch in text.chars() {
        line.push(ch);
        if measure_text_f(&line, font, font_size).width > max_width {
            let last = line.pop().unwrap();
            lines.push(line);
            line = String::from(last);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

struct OverlayLayout {
    box_rect: Rect,
    wrapped: Vec<String>,
    line_h: f32,
    button: Rect,
}

fn overlay_layout(encoded: &str, font: &Font) -> OverlayLayout {
    let box_w = (screen_width() - 40.0).min(OVL_MAX_WIDTH);
    let inner_w = box_w - OVL_PADDING * 2.0;
    let text_inner_w = inner_w - OVL_TEXT_PAD * 2.0;
    let wrapped = wrap_chars(encoded, font, OVL_CODE_FONT, text_inner_w);
    let line_h = OVL_CODE_FONT as f32 * 1.4;
    let text_area_h = wrapped.len() as f32 * line_h + OVL_TEXT_PAD * 2.0;

    // Copy button width: use the wider label so button doesn't jump between states
    let copy_w = measure_text_f("Copy", font, OVL_BTN_FONT).width;
    let copied_w = measure_text_f("Copied!", font, OVL_BTN_FONT).width;
    let btn_w = copy_w.max(copied_w) + 24.0;
    let btn_h = OVL_COPY_BTN_H;

    let box_h = OVL_PADDING + text_area_h + OVL_GAP + btn_h + OVL_PADDING;
    let box_x = (screen_width() - box_w) / 2.0;
    let box_y = (screen_height() - box_h) / 2.0;

    let btn_x = box_x + (box_w - btn_w) / 2.0;
    let btn_y = box_y + box_h - OVL_PADDING - btn_h;

    OverlayLayout {
        box_rect: Rect::new(box_x, box_y, box_w, box_h),
        wrapped,
        line_h,
        button: Rect::new(btn_x, btn_y, btn_w, btn_h),
    }
}

pub(crate) fn overlay_copy_hit(pos: Vec2, encoded: &str, font: &Font) -> bool {
    overlay_layout(encoded, font).button.contains(pos)
}

pub(crate) fn draw_overlay(overlay: &ExportOverlay, font: &Font) {
    let (encoded, is_copied) = match overlay {
        ExportOverlay::Hidden => return,
        ExportOverlay::Showing(s) => (s.as_str(), false),
        ExportOverlay::Copied(s) => (s.as_str(), true),
    };

    // Full-screen dim
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 200),
    );

    let layout = overlay_layout(encoded, font);
    let OverlayLayout {
        box_rect,
        wrapped,
        line_h,
        button,
    } = &layout;
    let inner_w = box_rect.w - OVL_PADDING * 2.0;
    let text_area_h = wrapped.len() as f32 * line_h + OVL_TEXT_PAD * 2.0;

    // Box background
    draw_rectangle(
        box_rect.x,
        box_rect.y,
        box_rect.w,
        box_rect.h,
        Color::from_rgba(30, 30, 45, 255),
    );
    draw_rectangle_lines(
        box_rect.x,
        box_rect.y,
        box_rect.w,
        box_rect.h,
        2.0,
        Color::from_rgba(80, 80, 100, 255),
    );

    // Text area with "selected" background
    let text_area_x = box_rect.x + OVL_PADDING;
    let text_area_y = box_rect.y + OVL_PADDING;
    draw_rectangle(
        text_area_x,
        text_area_y,
        inner_w,
        text_area_h,
        Color::from_rgba(35, 45, 65, 255),
    );

    let text_color = Color::from_rgba(190, 200, 220, 255);
    let mut y = text_area_y + OVL_TEXT_PAD + OVL_CODE_FONT as f32;
    for line in wrapped {
        draw_text_f(
            line,
            text_area_x + OVL_TEXT_PAD,
            y,
            font,
            OVL_CODE_FONT,
            text_color,
        );
        y += line_h;
    }

    // Copy button
    let (btn_bg, label_color) = if is_copied {
        (
            Color::from_rgba(35, 75, 35, 255),
            Color::from_rgba(100, 220, 100, 255),
        )
    } else {
        (
            Color::from_rgba(50, 50, 65, 255),
            Color::from_rgba(200, 200, 220, 255),
        )
    };
    draw_rectangle(button.x, button.y, button.w, button.h, btn_bg);
    draw_rectangle_lines(
        button.x,
        button.y,
        button.w,
        button.h,
        1.0,
        Color::from_rgba(80, 80, 100, 255),
    );

    let label = if is_copied { "Copied!" } else { "Copy" };
    let label_dims = measure_text_f(label, font, OVL_BTN_FONT);
    draw_text_f(
        label,
        button.x + (button.w - label_dims.width) / 2.0,
        button.y + (button.h + label_dims.height) / 2.0 - 2.0,
        font,
        OVL_BTN_FONT,
        label_color,
    );
}
