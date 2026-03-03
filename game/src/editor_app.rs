use std::collections::HashSet;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use std::str;

use macroquad::prelude::*;

use crate::direction::{Dir4, Dir8};
use crate::enum_all::EnumAll;
use enum_map::EnumMap;

use crate::game::{Action, Game, PlayState};
use crate::grid::{Cell, Grid, LevelMetadata, NoteText, Player};
use crate::position::{Position, PositionDelta};
use crate::sprites::Sprites;
use crate::testing::ScenarioInput;

const PADDING: f32 = 8.0;
const TOOLBAR_WIDTH: f32 = 150.0;

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Move,
    Wall,
    Player,
    Player2,
    Rat,
    CyborgRat,
    Portal,
    Note,
    Plank,
    Spiderweb,
    BlackHole,
    Explosive,
    Trigger,
}

impl Tool {
    fn all() -> [Tool; 13] {
        [
            Tool::Move,
            Tool::Wall,
            Tool::Player,
            Tool::Player2,
            Tool::Rat,
            Tool::CyborgRat,
            Tool::Portal,
            Tool::Note,
            Tool::Plank,
            Tool::Spiderweb,
            Tool::BlackHole,
            Tool::Explosive,
            Tool::Trigger,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            Tool::Move => "Move",
            Tool::Wall => "Wall",
            Tool::Player => "Player1",
            Tool::Player2 => "Player2",
            Tool::Rat => "Rat",
            Tool::CyborgRat => "Cyborg",
            Tool::Portal => "Portal",
            Tool::Note => "Note",
            Tool::Plank => "Plank",
            Tool::Spiderweb => "Web",
            Tool::BlackHole => "Hole",
            Tool::Explosive => "Bomb",
            Tool::Trigger => "Trigger",
        }
    }

    fn key(&self) -> &'static str {
        match self {
            Tool::Move => "m",
            Tool::Wall => "#",
            Tool::Player => "1",
            Tool::Player2 => "2",
            Tool::Rat => "",
            Tool::CyborgRat => "c",
            Tool::Portal => "g",
            Tool::Note => "n",
            Tool::Plank => "=",
            Tool::Spiderweb => "",
            Tool::BlackHole => "o",
            Tool::Explosive => "x",
            Tool::Trigger => "t",
        }
    }

    fn to_cell(self, player_dir: Dir4, trigger_digit: u8) -> Option<Cell> {
        match self {
            Tool::Move | Tool::Portal | Tool::Note => None,
            Tool::Wall => Some(Cell::Wall),
            Tool::Player => Some(Cell::Player(Player::Player1, player_dir)),
            Tool::Player2 => Some(Cell::Player(Player::Player2, player_dir)),
            Tool::Rat => Some(Cell::Rat(Dir8::Northeast)),
            Tool::CyborgRat => Some(Cell::CyborgRat(Dir8::Northeast)),
            Tool::Plank => Some(Cell::Plank),
            Tool::Spiderweb => Some(Cell::Spiderweb),
            Tool::BlackHole => Some(Cell::BlackHole),
            Tool::Explosive => Some(Cell::Explosive),
            Tool::Trigger => Some(Cell::Trigger(trigger_digit)),
        }
    }
}

/// An item being dragged as part of a bulk selection
struct DraggedItem {
    delta: PositionDelta,
    cell: Cell,
    portal: Option<String>,
    note: Option<NoteText>,
}

#[derive(Clone, Copy)]
struct PendingAction {
    action: Action,
    synced: bool,
}

enum EditorMode {
    Level {
        game: Box<Game>,
        input_history: Vec<Vec<Action>>,
    },
    Scenario {
        after_grid: Grid,
        p1_action: Action,
        p2_action: Action,
        expected_state: PlayState,
    },
}

struct Editor {
    before_grid: Grid,
    mode: EditorMode,
    tool: Tool,
    player_dir: Dir4,  // Direction for placing new players
    trigger_digit: u8, // Current digit for Trigger tool (1-9)
    portal_dialog: Option<(Position, usize, String)>, // (position, pane, current text) when entering portal level
    note_dialog: Option<(Position, usize, String)>, // (position, pane, current text) when entering note text
    sprites: Sprites,
    dragging: Option<(Position, Cell)>, // Source position and cell being dragged
    last_paint_pos: Option<Position>,   // Last position painted/erased (for drag painting)
    // Rectangle selection state
    selecting_rect: Option<(Position, Position)>, // (start, current) corners while dragging
    selection: HashSet<Position>,                 // Currently selected cell positions
    // Multi-item dragging state (anchor_pos, items with offsets from anchor)
    dragging_selection: Option<(Position, Vec<DraggedItem>)>,
    // Two-player sync buffering
    pending: EnumMap<Player, Option<PendingAction>>,
    // Solution replay: pre-loaded actions to step through
    replay_actions: Option<Vec<Vec<Action>>>,
    replay_index: usize,
}

impl Editor {
    fn new_level(grid: Grid, sprites: Sprites) -> Self {
        let game = Game::new(grid.clone(), HashSet::new());
        Self {
            before_grid: grid,
            mode: EditorMode::Level {
                game: Box::new(game),
                input_history: Vec::new(),
            },
            tool: Tool::Move,
            player_dir: Dir4::North,
            trigger_digit: 1,
            portal_dialog: None,
            note_dialog: None,
            sprites,
            dragging: None,
            last_paint_pos: None,
            selecting_rect: None,
            selection: HashSet::new(),
            dragging_selection: None,
            pending: EnumMap::default(),
            replay_actions: None,
            replay_index: 0,
        }
    }

    fn new_scenario(
        before_grid: Grid,
        after_grid: Grid,
        p1_action: Action,
        p2_action: Action,
        expected_state: PlayState,
        sprites: Sprites,
    ) -> Self {
        Self {
            before_grid,
            mode: EditorMode::Scenario {
                after_grid,
                p1_action,
                p2_action,
                expected_state,
            },
            tool: Tool::Move,
            player_dir: Dir4::North,
            trigger_digit: 1,
            portal_dialog: None,
            note_dialog: None,
            sprites,
            dragging: None,
            last_paint_pos: None,
            selecting_rect: None,
            selection: HashSet::new(),
            dragging_selection: None,
            pending: EnumMap::default(),
            replay_actions: None,
            replay_index: 0,
        }
    }

    fn step_replay_forward(&mut self) {
        if let Some(ref actions) = self.replay_actions
            && self.replay_index < actions.len()
        {
            let action = actions[self.replay_index].clone();
            self.replay_index += 1;
            self.add_actions(action);
        }
    }

    fn replay_inputs(&mut self) {
        if let EditorMode::Level {
            ref mut game,
            ref input_history,
        } = self.mode
        {
            **game = Game::new(self.before_grid.clone(), HashSet::new());
            for actions in input_history {
                if game.state.play_state() == PlayState::Playing {
                    game.apply_actions(actions);
                }
            }
        }
    }

    /// Get the grid displayed in the right pane
    fn after_grid(&self) -> &Grid {
        match &self.mode {
            EditorMode::Level { game, .. } => &game.state.grid,
            EditorMode::Scenario { after_grid, .. } => after_grid,
        }
    }

    /// Get mutable reference to the grid for a given pane
    fn grid_for_pane_mut(&mut self, pane: usize) -> &mut Grid {
        if pane == 0 {
            &mut self.before_grid
        } else {
            match &mut self.mode {
                EditorMode::Level { .. } => &mut self.before_grid, // Level mode: edits go to before_grid
                EditorMode::Scenario { after_grid, .. } => after_grid,
            }
        }
    }

    /// Get reference to the grid for a given pane
    fn grid_for_pane(&self, pane: usize) -> &Grid {
        if pane == 0 {
            &self.before_grid
        } else {
            self.after_grid()
        }
    }

    fn add_actions(&mut self, actions: Vec<Action>) {
        if let EditorMode::Level {
            ref mut game,
            ref mut input_history,
        } = self.mode
            && game.state.play_state() == PlayState::Playing
        {
            game.apply_actions(&actions);
            input_history.push(actions);
        }
    }

    fn register_action(&mut self, player: Player, action: Action, synced: bool) {
        self.pending[player] = Some(PendingAction { action, synced });
        self.try_execute_pending();
    }

    fn try_execute_pending(&mut self) {
        let player_count = self
            .before_grid
            .entries()
            .filter(|(_, cell)| matches!(cell, Cell::Player(..)))
            .count();
        if player_count == 0 {
            return;
        }

        let players = || Player::iter_all().take(player_count);
        let has_non_synced = players().any(|p| self.pending[p].is_some_and(|pa| !pa.synced));
        let all_synced = players().all(|p| self.pending[p].is_some_and(|pa| pa.synced));

        if !has_non_synced && !all_synced {
            return;
        }

        let actions: Vec<Action> = players()
            .map(|p| {
                self.pending[p]
                    .take()
                    .map(|pa| pa.action)
                    .unwrap_or(Action::Stall)
            })
            .collect();
        self.add_actions(actions);
    }

    fn remove_last_input(&mut self) {
        if let EditorMode::Level {
            ref mut input_history,
            ..
        } = self.mode
            && input_history.pop().is_some()
        {
            if self.replay_index > 0 {
                self.replay_index -= 1;
            }
            self.replay_inputs();
        }
    }

    /// Q-pick: sample the cell under the cursor and set the tool accordingly
    fn q_pick(&mut self, pos: Position, pane: usize) {
        let grid = self.grid_for_pane(pane);

        // Check for portal/note first (they overlay cells)
        if grid.get_portal(pos).is_some() {
            self.tool = Tool::Portal;
            return;
        }
        if grid.get_note(pos).is_some() {
            self.tool = Tool::Note;
            return;
        }

        self.tool = match grid.at(pos) {
            Cell::Empty => Tool::Move,
            Cell::Wall => Tool::Wall,
            Cell::Player(Player::Player1, dir) => {
                self.player_dir = dir;
                Tool::Player
            }
            Cell::Player(Player::Player2, dir) => {
                self.player_dir = dir;
                Tool::Player2
            }
            Cell::Rat(_) => Tool::Rat,
            Cell::CyborgRat(_) => Tool::CyborgRat,
            Cell::Plank => Tool::Plank,
            Cell::Spiderweb => Tool::Spiderweb,
            Cell::BlackHole => Tool::BlackHole,
            Cell::Explosive => Tool::Explosive,
            Cell::Trigger(n) => {
                self.trigger_digit = n;
                Tool::Trigger
            }
        };
    }

    fn place_cell(&mut self, pos: Position, cell: Cell, pane: usize) {
        let grid = self.grid_for_pane_mut(pane);
        // If placing player, remove existing player of same type first
        if let Cell::Player(who, _) = cell {
            let existing: Vec<_> = grid
                .entries()
                .filter(|(_, c)| matches!(c, Cell::Player(p, _) if *p == who))
                .map(|(p, _)| p)
                .collect();
            for p in existing {
                *grid.at_mut(p) = Cell::Empty;
            }
        }

        *grid.at_mut(pos) = cell;
        self.replay_inputs();
    }

    fn erase_cell(&mut self, pos: Position, pane: usize) {
        let grid = self.grid_for_pane_mut(pane);
        *grid.at_mut(pos) = Cell::Empty;
        grid.remove_portal(pos);
        grid.remove_note(pos);
        self.replay_inputs();
    }

    fn place_portal(&mut self, pos: Position, level: String, pane: usize) {
        self.grid_for_pane_mut(pane).insert_portal(pos, level);
        self.replay_inputs();
    }

    fn place_note(&mut self, pos: Position, text: String, pane: usize) {
        self.grid_for_pane_mut(pane)
            .insert_note(pos, NoteText::Simple(text));
        self.replay_inputs();
    }

    fn start_drag(&mut self, pos: Position, pane: usize) {
        // If clicking on a selected cell, drag the entire selection
        if self.selection.contains(&pos) {
            // First pass: collect items (immutable borrow)
            let grid = self.grid_for_pane(pane);
            let mut items = Vec::new();
            let mut positions_to_clear = Vec::new();
            for &sel_pos in &self.selection {
                let cell = grid.at(sel_pos);
                let portal = grid.get_portal(sel_pos).map(String::from);
                let note = grid.get_note(sel_pos).cloned();
                // Include position if it has a non-empty cell, portal, or note
                if !matches!(cell, Cell::Empty) || portal.is_some() || note.is_some() {
                    items.push(DraggedItem {
                        delta: sel_pos - pos,
                        cell,
                        portal,
                        note,
                    });
                    positions_to_clear.push(sel_pos);
                }
            }

            // Second pass: clear cells (mutable borrow)
            let grid = self.grid_for_pane_mut(pane);
            for sel_pos in positions_to_clear {
                *grid.at_mut(sel_pos) = Cell::Empty;
                grid.remove_portal(sel_pos);
                grid.remove_note(sel_pos);
            }

            if !items.is_empty() {
                self.dragging_selection = Some((pos, items));
                self.replay_inputs();
            }
            return;
        }

        // Otherwise, single-cell drag as before
        let cell = self.grid_for_pane(pane).at(pos);
        if !matches!(cell, Cell::Empty) {
            self.selection.clear();
            self.dragging = Some((pos, cell));
            *self.grid_for_pane_mut(pane).at_mut(pos) = Cell::Empty;
            self.replay_inputs();
        }
    }

    fn end_drag(&mut self, pos: Position, pane: usize) {
        // Handle multi-item drag
        if let Some((_, items)) = self.dragging_selection.take() {
            let grid = self.grid_for_pane_mut(pane);
            let bounds = (grid.width(), grid.height());
            for item in items {
                let target = pos + item.delta;
                if target.in_bounds(bounds) {
                    if !matches!(item.cell, Cell::Empty) {
                        *grid.at_mut(target) = item.cell;
                    }
                    if let Some(level) = item.portal {
                        grid.insert_portal(target, level);
                    }
                    if let Some(text) = item.note {
                        grid.insert_note(target, text);
                    }
                }
            }
            self.replay_inputs();
            self.selection.clear();
            return;
        }

        // Single-item drag
        if let Some((_, cell)) = self.dragging.take() {
            self.place_cell(pos, cell, pane);
        }
    }

    fn cancel_drag(&mut self, pane: usize) {
        // Handle multi-item drag cancellation
        if let Some((anchor, items)) = self.dragging_selection.take() {
            let grid = self.grid_for_pane_mut(pane);
            for item in items {
                let target = anchor + item.delta;
                *grid.at_mut(target) = item.cell;
                if let Some(level) = item.portal {
                    grid.insert_portal(target, level);
                }
                if let Some(text) = item.note {
                    grid.insert_note(target, text);
                }
            }
            self.replay_inputs();
            return;
        }

        // Single-item drag cancellation
        if let Some((src_pos, cell)) = self.dragging.take() {
            // Put it back
            *self.grid_for_pane_mut(pane).at_mut(src_pos) = cell;
            self.replay_inputs();
        }
    }

    fn start_selection(&mut self, pos: Position) {
        self.selecting_rect = Some((pos, pos));
        self.selection.clear();
    }

    fn update_selection(&mut self, pos: Position) {
        if let Some((start, _)) = self.selecting_rect {
            self.selecting_rect = Some((start, pos));
        }
    }

    fn end_selection(&mut self, pane: usize) {
        if let Some((start, end)) = self.selecting_rect.take() {
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);

            // First pass: collect positions with content (immutable borrow)
            let grid = self.grid_for_pane(pane);
            let mut selected_positions = Vec::new();
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let pos = Position { x, y };
                    // Select if non-empty cell, or has a portal or note
                    let has_content = !matches!(grid.at(pos), Cell::Empty)
                        || grid.get_portal(pos).is_some()
                        || grid.get_note(pos).is_some();
                    if has_content {
                        selected_positions.push(pos);
                    }
                }
            }

            // Second pass: update selection (mutable borrow)
            self.selection.clear();
            for pos in selected_positions {
                self.selection.insert(pos);
            }
        }
    }

    fn clear_selection(&mut self) {
        self.selection.clear();
        self.selecting_rect = None;
    }

    fn pane_layout(&self) -> (f32, f32, f32) {
        let available_width = screen_width() - TOOLBAR_WIDTH - PADDING * 3.0;
        let pane_width = available_width / 2.0;
        let available_height = screen_height() - PADDING * 2.0;

        let cell_w = pane_width / self.before_grid.width() as f32;
        let cell_h = available_height / self.before_grid.height() as f32;
        let cell_size = cell_w.min(cell_h);

        (pane_width, available_height, cell_size)
    }

    fn grid_offset(&self, pane: usize) -> (f32, f32) {
        let (pane_width, _, cell_size) = self.pane_layout();
        let pane_x = TOOLBAR_WIDTH + PADDING + pane as f32 * (pane_width + PADDING);
        let grid_w = self.before_grid.width() as f32 * cell_size;
        let grid_h = self.before_grid.height() as f32 * cell_size;
        let offset_x = pane_x + (pane_width - grid_w) / 2.0;
        let offset_y = PADDING + (screen_height() - PADDING * 2.0 - grid_h) / 2.0;
        (offset_x, offset_y)
    }

    fn grid_to_screen(&self, pos: Position, pane: usize) -> (f32, f32) {
        let (_, _, cell_size) = self.pane_layout();
        let (offset_x, offset_y) = self.grid_offset(pane);
        (
            offset_x + pos.x as f32 * cell_size,
            offset_y + pos.y as f32 * cell_size,
        )
    }

    fn screen_to_grid(&self, mx: f32, my: f32) -> Option<(Position, usize)> {
        let (_, _, cell_size) = self.pane_layout();

        for pane in 0..2 {
            let (offset_x, offset_y) = self.grid_offset(pane);

            let gx = ((mx - offset_x) / cell_size).floor() as i32;
            let gy = ((my - offset_y) / cell_size).floor() as i32;

            if gx >= 0
                && gy >= 0
                && (gx as usize) < self.before_grid.width()
                && (gy as usize) < self.before_grid.height()
            {
                return Some((Position::new(gx as usize, gy as usize), pane));
            }
        }
        None
    }

    fn render(&self) {
        clear_background(Color::from_rgba(30, 30, 40, 255));

        let (pane_width, _, cell_size) = self.pane_layout();

        let (left_label, right_label) = match &self.mode {
            EditorMode::Level { .. } => ("Initial", "After Moves"),
            EditorMode::Scenario { .. } => ("Before", "After"),
        };

        // Draw left pane (before grid)
        self.render_grid(&self.before_grid, 0, pane_width, cell_size, left_label);

        // Draw right pane
        self.render_grid(self.after_grid(), 1, pane_width, cell_size, right_label);

        // Draw selection highlights and rectangle
        self.render_selection(cell_size);

        // Draw cursor preview
        self.render_cursor_preview(cell_size);

        // Draw toolbar
        self.render_toolbar();

        // Draw portal dialog if active
        self.render_portal_dialog();

        // Draw note dialog if active
        self.render_note_dialog();
    }

    fn render_portal_dialog(&self) {
        let Some((_, _, ref text)) = self.portal_dialog else {
            return;
        };

        let dialog_w = 300.0;
        let dialog_h = 100.0;
        let dialog_x = (screen_width() - dialog_w) / 2.0;
        let dialog_y = (screen_height() - dialog_h) / 2.0;

        // Dim background
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 150),
        );

        // Dialog box
        draw_rectangle(
            dialog_x,
            dialog_y,
            dialog_w,
            dialog_h,
            Color::from_rgba(40, 40, 50, 255),
        );
        draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 2.0, WHITE);

        // Title
        draw_text(
            "Enter level name:",
            dialog_x + 10.0,
            dialog_y + 30.0,
            26.0,
            WHITE,
        );

        // Text input box
        let input_x = dialog_x + 10.0;
        let input_y = dialog_y + 45.0;
        let input_w = dialog_w - 20.0;
        let input_h = 30.0;
        draw_rectangle(
            input_x,
            input_y,
            input_w,
            input_h,
            Color::from_rgba(20, 20, 30, 255),
        );
        draw_rectangle_lines(input_x, input_y, input_w, input_h, 1.0, GRAY);
        draw_text(text, input_x + 5.0, input_y + 22.0, 22.0, WHITE);

        // Hint
        draw_text(
            "Enter to confirm, Esc to cancel",
            dialog_x + 10.0,
            dialog_y + 90.0,
            14.0,
            GRAY,
        );
    }

    fn render_note_dialog(&self) {
        let Some((_, _, ref text)) = self.note_dialog else {
            return;
        };

        let dialog_w = 400.0;
        let dialog_h = 150.0;
        let dialog_x = (screen_width() - dialog_w) / 2.0;
        let dialog_y = (screen_height() - dialog_h) / 2.0;

        // Dim background
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 150),
        );

        // Dialog box
        draw_rectangle(
            dialog_x,
            dialog_y,
            dialog_w,
            dialog_h,
            Color::from_rgba(40, 40, 50, 255),
        );
        draw_rectangle_lines(dialog_x, dialog_y, dialog_w, dialog_h, 2.0, WHITE);

        // Title
        draw_text(
            "Enter note text:",
            dialog_x + 10.0,
            dialog_y + 30.0,
            26.0,
            WHITE,
        );

        // Text input box (larger for notes)
        let input_x = dialog_x + 10.0;
        let input_y = dialog_y + 45.0;
        let input_w = dialog_w - 20.0;
        let input_h = 60.0;
        draw_rectangle(
            input_x,
            input_y,
            input_w,
            input_h,
            Color::from_rgba(20, 20, 30, 255),
        );
        draw_rectangle_lines(input_x, input_y, input_w, input_h, 1.0, GRAY);

        // Draw text with word wrap
        let line_height = 18.0;
        let max_chars_per_line = 45;
        let lines: Vec<&str> = text
            .as_bytes()
            .chunks(max_chars_per_line)
            .map(|chunk| str::from_utf8(chunk).unwrap())
            .collect();
        for (i, line) in lines.iter().take(3).enumerate() {
            draw_text(
                line,
                input_x + 5.0,
                input_y + 16.0 + i as f32 * line_height,
                16.0,
                WHITE,
            );
        }

        // Hint
        draw_text(
            "Enter to confirm, Esc to cancel",
            dialog_x + 10.0,
            dialog_y + 135.0,
            14.0,
            GRAY,
        );
    }

    fn render_selection(&self, cell_size: f32) {
        // Draw selection highlights on pane 0 (initial grid)
        let highlight_color = Color::from_rgba(100, 150, 255, 80);
        let border_color = Color::from_rgba(100, 150, 255, 200);

        for &pos in &self.selection {
            let (x, y) = self.grid_to_screen(pos, 0);
            draw_rectangle(x, y, cell_size, cell_size, highlight_color);
            draw_rectangle_lines(x, y, cell_size, cell_size, 2.0, border_color);
        }

        // Draw selection rectangle while selecting
        if let Some((start, end)) = self.selecting_rect {
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);

            let (sx, sy) = self.grid_to_screen(Position { x: min_x, y: min_y }, 0);
            let width = (max_x - min_x + 1) as f32 * cell_size;
            let height = (max_y - min_y + 1) as f32 * cell_size;

            draw_rectangle(sx, sy, width, height, Color::from_rgba(100, 150, 255, 40));
            draw_rectangle_lines(
                sx,
                sy,
                width,
                height,
                2.0,
                Color::from_rgba(100, 150, 255, 180),
            );
        }
    }

    fn render_cursor_preview(&self, cell_size: f32) {
        let (mx, my) = mouse_position();

        // Determine grid position
        let Some((pos, current_pane)) = self.screen_to_grid(mx, my) else {
            return;
        };

        // Handle multi-item drag preview
        if let Some((_, ref items)) = self.dragging_selection {
            for item in items {
                let target_x = mx + item.delta.dx as f32 * cell_size;
                let target_y = my + item.delta.dy as f32 * cell_size;

                self.draw_cell_preview(item.cell, target_x, target_y, cell_size, 180);

                // Ghost on opposite pane
                let other_pane = 1 - current_pane;
                let target_pos = pos + item.delta;
                let bounds = (self.before_grid.width(), self.before_grid.height());
                if target_pos.in_bounds(bounds) {
                    let (gx, gy) = self.grid_to_screen(target_pos, other_pane);
                    self.draw_cell_preview(item.cell, gx, gy, cell_size, 100);
                }
            }
            return;
        }

        // Single-item preview
        let preview_cell = if let Some((_, cell)) = self.dragging {
            Some(cell)
        } else {
            self.tool.to_cell(self.player_dir, self.trigger_digit)
        };

        let Some(cell) = preview_cell else {
            return;
        };

        // Draw on cursor pane (follows mouse)
        self.draw_cell_preview(
            cell,
            mx - cell_size / 2.0,
            my - cell_size / 2.0,
            cell_size,
            180,
        );

        // Draw ghost on opposite pane (snapped to grid)
        let other_pane = 1 - current_pane;
        let (gx, gy) = self.grid_to_screen(pos, other_pane);
        self.draw_cell_preview(cell, gx, gy, cell_size, 100);
    }

    fn draw_cell_preview(&self, cell: Cell, x: f32, y: f32, cell_size: f32, alpha: u8) {
        if let Cell::Trigger(n) = cell {
            let text = &n.to_string();
            let font_size = cell_size * 0.8;
            let dims = measure_text(text, None, font_size as u16, 1.0);
            let tx = x + (cell_size - dims.width) / 2.0;
            let ty = y + (cell_size + dims.height) / 2.0;
            draw_text(
                text,
                tx,
                ty,
                font_size,
                Color::from_rgba(255, 255, 255, alpha),
            );
        } else if let Some(texture) = match cell {
            Cell::Player(Player::Player1, dir) => Some(self.sprites.player(dir)),
            Cell::Player(Player::Player2, dir) => Some(self.sprites.player2(dir)),
            Cell::Rat(dir) => Some(self.sprites.rat(dir)),
            Cell::CyborgRat(dir) => Some(self.sprites.cyborg_rat(dir)),
            Cell::Wall => Some(self.sprites.wall()),
            Cell::Plank => Some(self.sprites.planks()),
            Cell::Spiderweb => Some(self.sprites.spiderweb()),
            Cell::BlackHole => Some(self.sprites.blackhole()),
            Cell::Explosive => Some(self.sprites.explosive()),
            Cell::Empty | Cell::Trigger(_) => None,
        } {
            draw_texture_ex(
                texture,
                x,
                y,
                Color::from_rgba(255, 255, 255, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(cell_size, cell_size)),
                    ..Default::default()
                },
            );
        }
    }

    fn render_grid(&self, grid: &Grid, pane: usize, pane_width: f32, cell_size: f32, label: &str) {
        let pane_x = TOOLBAR_WIDTH + PADDING + pane as f32 * (pane_width + PADDING);

        // Pane background
        draw_rectangle(
            pane_x,
            PADDING,
            pane_width,
            screen_height() - PADDING * 2.0,
            Color::from_rgba(40, 40, 50, 255),
        );

        // Label
        draw_text(label, pane_x + 4.0, PADDING + 20.0, 26.0, WHITE);

        let grid_w = grid.width() as f32 * cell_size;
        let grid_h = grid.height() as f32 * cell_size;
        let offset_x = pane_x + (pane_width - grid_w) / 2.0;
        let offset_y = PADDING + (screen_height() - PADDING * 2.0 - grid_h) / 2.0;

        // Grid lines
        for i in 0..=grid.width() {
            let x = offset_x + i as f32 * cell_size;
            draw_line(x, offset_y, x, offset_y + grid_h, 1.0, DARKGRAY);
        }
        for i in 0..=grid.height() {
            let y = offset_y + i as f32 * cell_size;
            draw_line(offset_x, y, offset_x + grid_w, y, 1.0, DARKGRAY);
        }

        // Draw portals
        for (pos, level) in grid.portals() {
            let completed = match &self.mode {
                EditorMode::Level { game, .. } => game.is_level_completed(level),
                EditorMode::Scenario { .. } => false,
            };
            let texture = self.sprites.portal(completed);
            draw_texture_ex(
                texture,
                offset_x + pos.x as f32 * cell_size,
                offset_y + pos.y as f32 * cell_size,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(cell_size, cell_size)),
                    ..Default::default()
                },
            );
        }

        // Draw notes
        for (pos, _) in grid.notes() {
            draw_texture_ex(
                self.sprites.note(),
                offset_x + pos.x as f32 * cell_size,
                offset_y + pos.y as f32 * cell_size,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(cell_size, cell_size)),
                    ..Default::default()
                },
            );
        }

        // Draw cells
        for (pos, cell) in grid.entries() {
            let px = offset_x + pos.x as f32 * cell_size;
            let py = offset_y + pos.y as f32 * cell_size;

            if let Cell::Trigger(n) = cell {
                let text = &n.to_string();
                let font_size = cell_size * 0.8;
                let dims = measure_text(text, None, font_size as u16, 1.0);
                let tx = px + (cell_size - dims.width) / 2.0;
                let ty = py + (cell_size + dims.height) / 2.0;
                draw_text(text, tx, ty, font_size, WHITE);
            } else {
                let texture = match cell {
                    Cell::Player(Player::Player1, dir) => self.sprites.player(dir),
                    Cell::Player(Player::Player2, dir) => self.sprites.player2(dir),
                    Cell::Rat(dir) => self.sprites.rat(dir),
                    Cell::CyborgRat(dir) => self.sprites.cyborg_rat(dir),
                    Cell::Wall => self.sprites.wall(),
                    Cell::Plank => self.sprites.planks(),
                    Cell::Spiderweb => self.sprites.spiderweb(),
                    Cell::BlackHole => self.sprites.blackhole(),
                    Cell::Explosive => self.sprites.explosive(),
                    Cell::Empty | Cell::Trigger(_) => continue,
                };
                draw_texture_ex(
                    texture,
                    px,
                    py,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(cell_size, cell_size)),
                        ..Default::default()
                    },
                );
            }
        }

        // Show game state on right pane (level mode only - scenario mode shows in toolbar)
        if pane == 1
            && let EditorMode::Level { ref game, .. } = self.mode
        {
            let play_state = game.state.play_state();
            let state_text = match play_state {
                PlayState::Playing => "",
                PlayState::GameOver => "GAME OVER",
                PlayState::Won => "WON",
            };
            if !state_text.is_empty() {
                let dims = measure_text(state_text, None, 32, 1.0);
                draw_text(
                    state_text,
                    pane_x + (pane_width - dims.width) / 2.0,
                    offset_y + grid_h + 30.0,
                    32.0,
                    if play_state == PlayState::Won {
                        GREEN
                    } else {
                        RED
                    },
                );
            }
        }
    }

    fn toolbar_button_rect(index: usize) -> (f32, f32, f32, f32) {
        let y = PADDING + index as f32 * 40.0;
        (PADDING, y, TOOLBAR_WIDTH - PADDING * 2.0, 35.0)
    }

    fn click_toolbar(&mut self, mx: f32, my: f32) -> bool {
        for (i, tool) in Tool::all().iter().enumerate() {
            let (bx, by, bw, bh) = Self::toolbar_button_rect(i);
            if mx >= bx && mx < bx + bw && my >= by && my < by + bh {
                self.tool = *tool;
                return true;
            }
        }

        // Check scenario-mode buttons (action selectors, state selector)
        if let EditorMode::Scenario {
            ref mut p1_action,
            ref mut p2_action,
            ref mut expected_state,
            ..
        } = self.mode
        {
            let content_y = PADDING + Tool::all().len() as f32 * 40.0 + 20.0;
            let btn_w = 16.0;
            let btn_h = 20.0;
            let p1_y = content_y + 5.0;
            let p2_y = p1_y + 25.0;

            let actions: [Action; 5] = [
                Action::Move(Dir4::North),
                Action::Move(Dir4::South),
                Action::Move(Dir4::East),
                Action::Move(Dir4::West),
                Action::Stall,
            ];

            // P1 action buttons
            for (i, act) in actions.iter().enumerate() {
                let btn_x = PADDING + 20.0 + i as f32 * (btn_w + 1.0);
                if mx >= btn_x && mx < btn_x + btn_w && my >= p1_y && my < p1_y + btn_h {
                    *p1_action = *act;
                    return true;
                }
            }

            // P2 action buttons
            for (i, act) in actions.iter().enumerate() {
                let btn_x = PADDING + 20.0 + i as f32 * (btn_w + 1.0);
                if mx >= btn_x && mx < btn_x + btn_w && my >= p2_y && my < p2_y + btn_h {
                    *p2_action = *act;
                    return true;
                }
            }

            // State buttons
            let state_y = p2_y + 30.0;
            let state_btn_y = state_y + 5.0;
            let state_btn_w = 32.0;
            let states = [PlayState::Playing, PlayState::GameOver, PlayState::Won];

            for (i, state) in states.iter().enumerate() {
                let btn_x = PADDING + i as f32 * (state_btn_w + 2.0);
                if mx >= btn_x
                    && mx < btn_x + state_btn_w
                    && my >= state_btn_y
                    && my < state_btn_y + btn_h
                {
                    *expected_state = *state;
                    return true;
                }
            }
        }

        // Check size buttons
        let [w_minus, w_plus, h_minus, h_plus] = self.size_button_rects();

        if w_minus.contains(mx, my) {
            self.resize(-1, 0);
            return true;
        }
        if w_plus.contains(mx, my) {
            self.resize(1, 0);
            return true;
        }
        if h_minus.contains(mx, my) {
            self.resize(0, -1);
            return true;
        }
        if h_plus.contains(mx, my) {
            self.resize(0, 1);
            return true;
        }

        false
    }

    fn render_toolbar(&self) {
        // Background
        draw_rectangle(
            0.0,
            0.0,
            TOOLBAR_WIDTH,
            screen_height(),
            Color::from_rgba(20, 20, 30, 255),
        );

        // Tools
        for (i, tool) in Tool::all().iter().enumerate() {
            let (bx, by, bw, bh) = Self::toolbar_button_rect(i);
            let selected = self.tool == *tool;
            let bg_color = if selected {
                Color::from_rgba(80, 80, 100, 255)
            } else {
                Color::from_rgba(50, 50, 60, 255)
            };

            draw_rectangle(bx, by, bw, bh, bg_color);

            let label = format!("[{}] {}", tool.key(), tool.name());
            let dims = measure_text(&label, None, 22, 1.0);
            draw_text(
                &label,
                bx + (bw - dims.width) / 2.0,
                by + (bh + dims.offset_y) / 2.0,
                22.0,
                WHITE,
            );
        }

        // Mode-specific content below tools
        let content_y = PADDING + Tool::all().len() as f32 * 40.0 + 20.0;

        match &self.mode {
            EditorMode::Level { input_history, .. } => {
                // Move count
                let moves_text = format!("Moves: {}", input_history.len());
                draw_text(&moves_text, PADDING, content_y, 26.0, WHITE);

                // Replay progress
                if let Some(ref actions) = self.replay_actions {
                    let text = format!("Replay: {}/{}", self.replay_index, actions.len());
                    draw_text(&text, PADDING, content_y + 25.0, 22.0, YELLOW);
                }
            }
            EditorMode::Scenario {
                p1_action,
                p2_action,
                expected_state,
                ..
            } => {
                // Action options: None (no input), or specific action
                let actions: [(Action, &str); 5] = [
                    (Action::Move(Dir4::North), "N"),
                    (Action::Move(Dir4::South), "S"),
                    (Action::Move(Dir4::East), "E"),
                    (Action::Move(Dir4::West), "W"),
                    (Action::Stall, "-"),
                ];
                let btn_w = 16.0;
                let btn_h = 20.0;

                // P1 Action selector
                draw_text("P1:", PADDING, content_y, 16.0, WHITE);
                let p1_y = content_y + 5.0;
                for (i, (act, label)) in actions.iter().enumerate() {
                    let btn_x = PADDING + 20.0 + i as f32 * (btn_w + 1.0);
                    let selected = p1_action == act;
                    let bg = if selected {
                        Color::from_rgba(80, 120, 80, 255)
                    } else {
                        Color::from_rgba(50, 50, 60, 255)
                    };
                    draw_rectangle(btn_x, p1_y, btn_w, btn_h, bg);
                    draw_text(label, btn_x + 3.0, p1_y + 15.0, 14.0, WHITE);
                }

                // P2 Action selector
                let p2_y = p1_y + 25.0;
                draw_text("P2:", PADDING, p2_y + 15.0, 16.0, WHITE);
                for (i, (act, label)) in actions.iter().enumerate() {
                    let btn_x = PADDING + 20.0 + i as f32 * (btn_w + 1.0);
                    let selected = p2_action == act;
                    let bg = if selected {
                        Color::from_rgba(80, 120, 80, 255)
                    } else {
                        Color::from_rgba(50, 50, 60, 255)
                    };
                    draw_rectangle(btn_x, p2_y, btn_w, btn_h, bg);
                    draw_text(label, btn_x + 3.0, p2_y + 15.0, 14.0, WHITE);
                }

                // State selector
                let state_y = p2_y + 30.0;
                draw_text("State:", PADDING, state_y, 20.0, WHITE);
                let states = [
                    (PlayState::Playing, "Play"),
                    (PlayState::GameOver, "Over"),
                    (PlayState::Won, "Won"),
                ];
                let state_btn_y = state_y + 5.0;
                let state_btn_w = 32.0;
                for (i, (state, label)) in states.iter().enumerate() {
                    let btn_x = PADDING + i as f32 * (state_btn_w + 2.0);
                    let selected = expected_state == state;
                    let bg = if selected {
                        Color::from_rgba(80, 120, 80, 255)
                    } else {
                        Color::from_rgba(50, 50, 60, 255)
                    };
                    draw_rectangle(btn_x, state_btn_y, state_btn_w, btn_h, bg);
                    let dims = measure_text(label, None, 14, 1.0);
                    draw_text(
                        label,
                        btn_x + (state_btn_w - dims.width) / 2.0,
                        state_btn_y + 15.0,
                        14.0,
                        WHITE,
                    );
                }
            }
        }

        // Size controls
        let size_y = content_y + 110.0;
        draw_text("Size:", PADDING, size_y, 22.0, WHITE);

        // Width row
        let width_y = size_y + 25.0;
        let (w_minus, w_plus) = self.render_size_row("W", self.before_grid.width(), width_y);
        let _ = (w_minus, w_plus); // Positions used in click handler

        // Height row
        let height_y = width_y + 30.0;
        let (h_minus, h_plus) = self.render_size_row("H", self.before_grid.height(), height_y);
        let _ = (h_minus, h_plus); // Positions used in click handler
    }

    fn render_size_row(&self, label: &str, value: usize, y: f32) -> ((f32, f32), (f32, f32)) {
        let btn_size = 24.0;
        let minus_x = PADDING;
        let plus_x = TOOLBAR_WIDTH - PADDING - btn_size;

        // Minus button
        draw_rectangle(
            minus_x,
            y,
            btn_size,
            btn_size,
            Color::from_rgba(50, 50, 60, 255),
        );
        draw_text("-", minus_x + 7.0, y + 18.0, 22.0, WHITE);

        // Value
        let text = format!("{}: {}", label, value);
        let dims = measure_text(&text, None, 20, 1.0);
        draw_text(
            &text,
            PADDING + (TOOLBAR_WIDTH - PADDING * 2.0 - dims.width) / 2.0,
            y + 18.0,
            20.0,
            WHITE,
        );

        // Plus button
        draw_rectangle(
            plus_x,
            y,
            btn_size,
            btn_size,
            Color::from_rgba(50, 50, 60, 255),
        );
        draw_text("+", plus_x + 5.0, y + 18.0, 22.0, WHITE);

        ((minus_x, y), (plus_x, y))
    }

    fn size_button_rects(&self) -> [Rect; 4] {
        let btn_size = 24.0;
        let minus_x = PADDING;
        let plus_x = TOOLBAR_WIDTH - PADDING - btn_size;

        let content_y = PADDING + Tool::all().len() as f32 * 40.0 + 20.0;
        let size_y = content_y + 110.0;
        let width_y = size_y + 25.0;
        let height_y = width_y + 30.0;

        [
            Rect {
                x: minus_x,
                y: width_y,
                w: btn_size,
                h: btn_size,
            }, // w_minus
            Rect {
                x: plus_x,
                y: width_y,
                w: btn_size,
                h: btn_size,
            }, // w_plus
            Rect {
                x: minus_x,
                y: height_y,
                w: btn_size,
                h: btn_size,
            }, // h_minus
            Rect {
                x: plus_x,
                y: height_y,
                w: btn_size,
                h: btn_size,
            }, // h_plus
        ]
    }

    fn save_level(&self, csv_path: &str, json_path: &str, level_name: &str) {
        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(csv_path).parent() {
            let _ = create_dir_all(parent);
        }

        let csv = self.before_grid.to_csv();
        write(csv_path, csv).expect("Failed to save CSV");

        let json = self.before_grid.to_json(level_name);
        write(json_path, json).expect("Failed to save JSON");
    }

    fn save_scenario(&self, before_path: &str, after_path: &str, json_path: &str) {
        if let EditorMode::Scenario {
            ref after_grid,
            p1_action,
            p2_action,
            expected_state,
        } = self.mode
        {
            // Create parent directories if they don't exist
            if let Some(parent) = Path::new(before_path).parent() {
                let _ = create_dir_all(parent);
            }

            // Save before grid
            write(before_path, self.before_grid.to_csv()).expect("Failed to save before CSV");

            // Save after grid
            write(after_path, after_grid.to_csv()).expect("Failed to save after CSV");

            // Use two-player format (p1/p2) for two-player scenarios
            let is_two_player = self
                .before_grid
                .entries()
                .any(|(_, c)| matches!(c, Cell::Player(Player::Player2, _)));

            let input = if is_two_player {
                ScenarioInput::TwoPlayer {
                    p1: p1_action,
                    p2: p2_action,
                    state: expected_state,
                }
            } else {
                ScenarioInput::SinglePlayer {
                    action: p1_action,
                    state: expected_state,
                }
            };
            let json =
                serde_json::to_string_pretty(&input).expect("Failed to serialize scenario") + "\n";
            write(json_path, json).expect("Failed to save JSON");
        }
    }

    fn resize(&mut self, delta_w: i32, delta_h: i32) {
        let new_w = (self.before_grid.width() as i32 + delta_w).max(1) as usize;
        let new_h = (self.before_grid.height() as i32 + delta_h).max(1) as usize;
        self.before_grid.resize(new_w, new_h);
        if let EditorMode::Scenario {
            ref mut after_grid, ..
        } = self.mode
        {
            after_grid.resize(new_w, new_h);
        }
        self.replay_inputs();
    }
}

enum AppMode {
    Level {
        csv_path: String,
        json_path: String,
        level_name: String,
    },
    Scenario {
        scenario_names: Vec<String>,
        current_index: usize,
    },
}

pub struct App {
    editor: Editor,
    mode: AppMode,
}

impl App {
    pub fn new_level(sprites: Sprites, level_name: &str) -> Self {
        let csv_path = format!("levels/{}.csv", level_name);
        let json_path = format!("levels/{}.json", level_name);

        // Load existing level or create empty grid
        let (grid, display_name) = if let Ok(csv) = read_to_string(&csv_path) {
            let json_str = read_to_string(&json_path).unwrap();
            let metadata = LevelMetadata::parse(&json_str);
            let name = metadata.name.clone();
            (Grid::from_csv_and_metadata(&csv, &metadata), name)
        } else {
            let mut grid = Grid::create_empty(10, 10);
            *grid.at_mut(Position::new(5, 5)) = Cell::Player(Player::Player1, Dir4::North);
            (grid, level_name.to_string())
        };

        let editor = Editor::new_level(grid, sprites);

        Self {
            editor,
            mode: AppMode::Level {
                csv_path,
                json_path,
                level_name: display_name,
            },
        }
    }

    pub fn new_solution(sprites: Sprites, decoded: crate::solution::DecodedSolution) -> Self {
        let grid = Grid::from_csv(&decoded.grid);
        let mut editor = Editor::new_level(grid, sprites);
        editor.replay_actions = Some(decoded.actions);
        Self {
            editor,
            mode: AppMode::Level {
                csv_path: String::new(),
                json_path: String::new(),
                level_name: decoded.level,
            },
        }
    }

    pub fn new_scenario(sprites: Sprites, filter: &[&str]) -> Self {
        let scenario_dirs = [
            "scenario_tests/scenarios",
            "scenario_tests/two_player_scenarios",
        ];

        // Collect scenario names from all directories
        let mut scenario_names: Vec<String> = scenario_dirs
            .iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let stem = path.file_stem()?.to_str()?;
                // Look for _i.json files to identify scenarios
                if path.extension()?.to_str()? == "json" && stem.ends_with("_i") {
                    Some(stem.strip_suffix("_i")?.to_string())
                } else {
                    None
                }
            })
            .collect();
        scenario_names.sort();
        scenario_names.dedup();

        // Add any filtered names that don't exist yet (will be created)
        for &name in filter {
            if !scenario_names.iter().any(|s| s == name) {
                scenario_names.push(name.to_string());
            }
        }
        scenario_names.sort();
        scenario_names.dedup();

        // Filter to specified scenarios if any provided
        if !filter.is_empty() {
            scenario_names.retain(|name| filter.contains(&name.as_str()));
        }

        assert!(
            !scenario_names.is_empty(),
            "No scenarios found in {:?}",
            scenario_dirs
        );

        let editor = Self::load_or_create_scenario(&scenario_names[0], sprites);

        Self {
            editor,
            mode: AppMode::Scenario {
                scenario_names,
                current_index: 0,
            },
        }
    }

    fn load_or_create_scenario(name: &str, sprites: Sprites) -> Editor {
        Self::load_scenario_from_dir(name, "scenario_tests/scenarios", sprites.clone())
            .or_else(|| {
                Self::load_scenario_from_dir(
                    name,
                    "scenario_tests/two_player_scenarios",
                    sprites.clone(),
                )
            })
            .unwrap_or_else(|| {
                let mut grid = Grid::create_empty(5, 5);
                *grid.at_mut(Position::new(2, 2)) = Cell::Player(Player::Player1, Dir4::North);
                Editor::new_scenario(
                    grid.clone(),
                    grid,
                    Action::Stall,
                    Action::Stall,
                    PlayState::Playing,
                    sprites,
                )
            })
    }

    fn load_scenario_from_dir(name: &str, scenarios_dir: &str, sprites: Sprites) -> Option<Editor> {
        let before_path = format!("{}/{}_1.csv", scenarios_dir, name);
        let after_path = format!("{}/{}_2.csv", scenarios_dir, name);
        let json_path = format!("{}/{}_i.json", scenarios_dir, name);

        let before_csv = read_to_string(&before_path).ok()?;
        let after_csv = read_to_string(&after_path).ok()?;
        let json_str = read_to_string(&json_path).ok()?;

        let before_grid = Grid::from_csv(&before_csv);
        let after_grid = Grid::from_csv(&after_csv);

        let input: ScenarioInput =
            serde_json::from_str(&json_str).expect("Failed to parse scenario JSON");

        let (p1_action, p2_action, expected_state) = match input {
            ScenarioInput::TwoPlayer { p1, p2, state } => (p1, p2, state),
            ScenarioInput::SinglePlayer { action, state } => (action, Action::Stall, state),
        };

        Some(Editor::new_scenario(
            before_grid,
            after_grid,
            p1_action,
            p2_action,
            expected_state,
            sprites,
        ))
    }

    /// Run one frame of the editor loop. Returns true to continue.
    pub fn tick(&mut self) -> bool {
        // Handle portal dialog input first (blocks other input)
        if let Some((pos, pane, ref mut text)) = self.editor.portal_dialog {
            if is_key_pressed(KeyCode::Escape) {
                self.editor.portal_dialog = None;
            } else if is_key_pressed(KeyCode::Enter) && !text.is_empty() {
                let level = text.clone();
                self.editor.portal_dialog = None;
                self.editor.place_portal(pos, level, pane);
            } else if is_key_pressed(KeyCode::Backspace) && !text.is_empty() {
                text.pop();
            } else if let Some(c) = get_char_pressed()
                && (c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' || c == '/')
            {
                text.push(c);
            }

            self.editor.render();
            return true;
        }

        // Handle note dialog input (blocks other input)
        if let Some((pos, pane, ref mut text)) = self.editor.note_dialog {
            if is_key_pressed(KeyCode::Escape) {
                self.editor.note_dialog = None;
            } else if is_key_pressed(KeyCode::Enter) && !text.is_empty() {
                let note_text = text.clone();
                self.editor.note_dialog = None;
                self.editor.place_note(pos, note_text, pane);
            } else if is_key_pressed(KeyCode::Backspace) && !text.is_empty() {
                text.pop();
            } else if let Some(c) = get_char_pressed()
                && (c.is_alphanumeric() || c.is_ascii_punctuation() || c == ' ')
            {
                text.push(c);
            }

            self.editor.render();
            return true;
        }

        // Tool selection via character input
        if let Some(c) = get_char_pressed() {
            match c.to_ascii_lowercase() {
                'm' => self.editor.tool = Tool::Move,
                '#' => self.editor.tool = Tool::Wall,
                '1' => self.editor.tool = Tool::Player,
                '2' => self.editor.tool = Tool::Player2,
                'c' => self.editor.tool = Tool::CyborgRat,
                'g' => self.editor.tool = Tool::Portal,
                'n' => {
                    if self.editor.replay_actions.is_some() {
                        self.editor.step_replay_forward();
                    } else {
                        self.editor.tool = Tool::Note;
                    }
                }
                '=' => self.editor.tool = Tool::Plank,
                'o' => self.editor.tool = Tool::BlackHole,
                'x' => self.editor.tool = Tool::Explosive,
                't' => self.editor.tool = Tool::Trigger,
                'q' => {
                    // Q-pick: sample the cell under the cursor
                    let (mx, my) = mouse_position();
                    if let Some((pos, pane)) = self.editor.screen_to_grid(mx, my) {
                        self.editor.q_pick(pos, pane);
                    }
                }
                _ => {}
            }
        }

        // Scroll wheel changes trigger digit when Trigger tool is selected
        let (_, scroll_y) = mouse_wheel();
        if self.editor.tool == Tool::Trigger && scroll_y != 0.0 {
            if scroll_y > 0.0 {
                self.editor.trigger_digit = if self.editor.trigger_digit >= 9 {
                    1
                } else {
                    self.editor.trigger_digit + 1
                };
            } else {
                self.editor.trigger_digit = if self.editor.trigger_digit <= 1 {
                    9
                } else {
                    self.editor.trigger_digit - 1
                };
            }
        }

        if !is_key_down(KeyCode::LeftControl) && !is_key_down(KeyCode::RightControl) {
            // Movement input - behavior depends on mode (skip when Ctrl held)
            match &mut self.editor.mode {
                EditorMode::Level { .. } => {
                    let synced_p1 = is_key_down(KeyCode::RightShift);
                    let synced_p2 = is_key_down(KeyCode::LeftShift);

                    // P1: arrow keys + space
                    if is_key_pressed(KeyCode::Up) {
                        self.editor.register_action(
                            Player::Player1,
                            Action::Move(Dir4::North),
                            synced_p1,
                        );
                    }
                    if is_key_pressed(KeyCode::Down) {
                        self.editor.register_action(
                            Player::Player1,
                            Action::Move(Dir4::South),
                            synced_p1,
                        );
                    }
                    if is_key_pressed(KeyCode::Left) {
                        self.editor.register_action(
                            Player::Player1,
                            Action::Move(Dir4::West),
                            synced_p1,
                        );
                    }
                    if is_key_pressed(KeyCode::Right) {
                        self.editor.register_action(
                            Player::Player1,
                            Action::Move(Dir4::East),
                            synced_p1,
                        );
                    }
                    if is_key_pressed(KeyCode::Space) {
                        self.editor
                            .register_action(Player::Player1, Action::Stall, synced_p1);
                    }

                    // P2: WASD + tab
                    if is_key_pressed(KeyCode::W) {
                        self.editor.register_action(
                            Player::Player2,
                            Action::Move(Dir4::North),
                            synced_p2,
                        );
                    }
                    if is_key_pressed(KeyCode::S) {
                        self.editor.register_action(
                            Player::Player2,
                            Action::Move(Dir4::South),
                            synced_p2,
                        );
                    }
                    if is_key_pressed(KeyCode::A) {
                        self.editor.register_action(
                            Player::Player2,
                            Action::Move(Dir4::West),
                            synced_p2,
                        );
                    }
                    if is_key_pressed(KeyCode::D) {
                        self.editor.register_action(
                            Player::Player2,
                            Action::Move(Dir4::East),
                            synced_p2,
                        );
                    }
                    if is_key_pressed(KeyCode::Tab) {
                        self.editor
                            .register_action(Player::Player2, Action::Stall, synced_p2);
                    }
                }
                EditorMode::Scenario { .. } => {
                    // Scenario mode: use toolbar buttons for P1/P2 actions
                }
            }
        }

        // Undo last move (u or backspace) - level mode only
        if is_key_pressed(KeyCode::U) || is_key_pressed(KeyCode::Backspace) {
            self.editor.pending = EnumMap::default();
            self.editor.remove_last_input();
        }

        // Escape: cancel drag/selection
        if is_key_pressed(KeyCode::Escape) {
            if self.editor.dragging.is_some() || self.editor.dragging_selection.is_some() {
                self.editor.cancel_drag(0);
            } else if !self.editor.selection.is_empty() || self.editor.selecting_rect.is_some() {
                self.editor.clear_selection();
            }
        }

        // R: restart (clear moves) in level mode
        if is_key_pressed(KeyCode::R)
            && let EditorMode::Level {
                ref mut input_history,
                ..
            } = self.editor.mode
        {
            input_history.clear();
            self.editor.pending = EnumMap::default();
            self.editor.replay_index = 0;
            self.editor.replay_inputs();
        }

        // Save
        if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::S) {
            self.save();
        }

        // Scroll wheel to rotate player direction (when Player or Player2 tool selected)
        if self.editor.tool == Tool::Player || self.editor.tool == Tool::Player2 {
            let (_, scroll_y) = mouse_wheel();
            if scroll_y < 0.0 {
                self.editor.player_dir = self.editor.player_dir.rotate_cw();
            } else if scroll_y > 0.0 {
                self.editor.player_dir = self.editor.player_dir.rotate_ccw();
            }
        }

        // Mouse handling
        let (mx, my) = mouse_position();

        // Right-click-and-drag to erase (also clears selection)
        if is_mouse_button_down(MouseButton::Right) {
            self.editor.clear_selection();
            if let Some((pos, pane)) = self.editor.screen_to_grid(mx, my)
                && self.editor.last_paint_pos != Some(pos)
            {
                self.editor.erase_cell(pos, pane);
                self.editor.last_paint_pos = Some(pos);
            }
        } else if is_mouse_button_down(MouseButton::Left) {
            // Left-click handling
            if is_mouse_button_pressed(MouseButton::Left) {
                // Check toolbar first on initial press
                if self.editor.click_toolbar(mx, my) {
                    // Toolbar clicked, don't do grid actions
                } else if let Some((pos, pane)) = self.editor.screen_to_grid(mx, my) {
                    match self.editor.tool {
                        Tool::Move => {
                            let cell = self.editor.grid_for_pane(pane).at(pos);
                            if matches!(cell, Cell::Empty) && !self.editor.selection.contains(&pos)
                            {
                                // Click on empty cell: start rectangle selection
                                self.editor.start_selection(pos);
                            } else {
                                // Click on a cell (or selected cell): start dragging
                                self.editor.start_drag(pos, pane);
                            }
                        }
                        Tool::Portal => {
                            // Open portal dialog
                            self.editor.portal_dialog = Some((pos, pane, String::new()));
                        }
                        Tool::Note => {
                            // Open note dialog
                            self.editor.note_dialog = Some((pos, pane, String::new()));
                        }
                        tool => {
                            // Place cell and start tracking drag-painting
                            if let Some(cell) =
                                tool.to_cell(self.editor.player_dir, self.editor.trigger_digit)
                            {
                                self.editor.place_cell(pos, cell, pane);
                                self.editor.last_paint_pos = Some(pos);
                            }
                        }
                    }
                } else {
                    // Clicked outside grid: clear selection
                    self.editor.clear_selection();
                }
            } else {
                // Mouse held down - update selection rectangle or continue drag-painting
                if self.editor.selecting_rect.is_some() {
                    if let Some((pos, _)) = self.editor.screen_to_grid(mx, my) {
                        self.editor.update_selection(pos);
                    }
                } else if !matches!(self.editor.tool, Tool::Move | Tool::Portal | Tool::Note)
                    && self.editor.dragging.is_none()
                    && let Some((pos, pane)) = self.editor.screen_to_grid(mx, my)
                    && self.editor.last_paint_pos != Some(pos)
                    && let Some(cell) = self
                        .editor
                        .tool
                        .to_cell(self.editor.player_dir, self.editor.trigger_digit)
                {
                    // Continue drag-painting for non-Move/Portal tools
                    self.editor.place_cell(pos, cell, pane);
                    self.editor.last_paint_pos = Some(pos);
                }
            }
        } else {
            // No mouse button down - reset paint tracking and handle drag/selection release
            if self.editor.selecting_rect.is_some() {
                if let Some((_, pane)) = self.editor.screen_to_grid(mx, my) {
                    self.editor.end_selection(pane);
                } else {
                    self.editor.end_selection(0); // Default to pane 0 if outside grid
                }
            }
            if self.editor.dragging.is_some() || self.editor.dragging_selection.is_some() {
                if let Some((pos, pane)) = self.editor.screen_to_grid(mx, my) {
                    self.editor.end_drag(pos, pane);
                } else {
                    self.editor.cancel_drag(0); // Default to pane 0 if outside grid
                }
            }
            self.editor.last_paint_pos = None;
        }

        // Scenario navigation (Ctrl+Left/Right)
        if let AppMode::Scenario {
            ref scenario_names,
            ref mut current_index,
        } = self.mode
            && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
        {
            let mut new_index = None;
            if is_key_pressed(KeyCode::Left) && *current_index > 0 {
                new_index = Some(*current_index - 1);
            }
            if is_key_pressed(KeyCode::Right) && *current_index < scenario_names.len() - 1 {
                new_index = Some(*current_index + 1);
            }
            if let Some(idx) = new_index {
                *current_index = idx;
                let name = &scenario_names[idx];
                self.editor = Self::load_or_create_scenario(name, self.editor.sprites.clone());
            }
        }

        self.editor.render();

        // Draw scenario name at the bottom
        if let AppMode::Scenario {
            ref scenario_names,
            current_index,
            ..
        } = self.mode
        {
            let name = &scenario_names[current_index];
            let text = format!("{} ({}/{})", name, current_index + 1, scenario_names.len());
            draw_text(
                &text,
                TOOLBAR_WIDTH + PADDING,
                screen_height() - 10.0,
                20.0,
                WHITE,
            );
        }

        true
    }

    fn save(&self) {
        match &self.mode {
            AppMode::Level {
                csv_path,
                json_path,
                level_name,
            } => {
                self.editor.save_level(csv_path, json_path, level_name);
            }
            AppMode::Scenario {
                scenario_names,
                current_index,
                ..
            } => {
                let name = &scenario_names[*current_index];
                // Determine which directory the scenario belongs to
                let is_two_player = self
                    .editor
                    .before_grid
                    .entries()
                    .any(|(_, c)| matches!(c, Cell::Player(Player::Player2, _)));
                let scenarios_dir = if Path::new(&format!(
                    "scenario_tests/two_player_scenarios/{}_i.json",
                    name
                ))
                .exists()
                    || is_two_player
                {
                    "scenario_tests/two_player_scenarios"
                } else {
                    "scenario_tests/scenarios"
                };
                let before_path = format!("{}/{}_1.csv", scenarios_dir, name);
                let after_path = format!("{}/{}_2.csv", scenarios_dir, name);
                let json_path = format!("{}/{}_i.json", scenarios_dir, name);
                self.editor
                    .save_scenario(&before_path, &after_path, &json_path);

                // Clean up files from the other directory if the scenario moved
                let other_dir = if is_two_player {
                    "scenario_tests/scenarios"
                } else {
                    "scenario_tests/two_player_scenarios"
                };
                for suffix in ["_1.csv", "_2.csv", "_i.json"] {
                    let old_path = format!("{}/{}{}", other_dir, name, suffix);
                    let _ = std::fs::remove_file(&old_path);
                }
            }
        }
    }
}
