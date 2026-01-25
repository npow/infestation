use std::borrow::BorrowMut;
use std::collections::HashSet;

use crate::direction::Dir4;
use crate::grid::{Cell, Grid};
use crate::levels;
use crate::position::Position;
use crate::storage::strip_path_prefix;

mod animation;
mod cyborg_distance;
mod cyborg_rat;
mod explosion;
mod player;
mod rat;
mod zap;

const MOVE_SPEED: f32 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PlayState {
    Playing,
    GameOver,
    Won,
}

#[derive(Clone, Copy)]
pub(crate) struct Moving {
    pub(crate) cell: Cell,
    pub(crate) from: Position,
    pub(crate) progress: f32,
    pub(crate) to: Position,
}

#[derive(Clone, Copy)]
pub(crate) struct Exploding {
    pub(crate) pos: Position,
    pub(crate) progress: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct Zapping {
    pub(crate) pos: Position,
    pub(crate) progress: f32,
}

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Move(Dir4),
    Stall,
}

/// Handles move resolution and animation.
/// Used for both instant resolution and animated playback.
#[derive(Clone)]
pub(crate) struct MoveHandler<G = Grid> {
    /// Grid being modified (also used for rendering during animation).
    pub(crate) grid: G,
    /// Movement animations in progress.
    pub(crate) moving: Vec<Moving>,
    /// Zap animations in progress.
    pub(crate) zapping: Vec<Zapping>,
    /// Trigger numbers that have been activated and need processing.
    pub(crate) triggered_numbers: Vec<u8>,
    /// Explosion animations in progress.
    pub(crate) exploding: Vec<Exploding>,
    /// Explosions queued for the next wave.
    pub(crate) pending_explosions: Vec<Position>,
}

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    pub(crate) fn new(grid: G) -> Self {
        Self {
            grid,
            moving: Vec::new(),
            zapping: Vec::new(),
            triggered_numbers: Vec::new(),
            exploding: Vec::new(),
            pending_explosions: Vec::new(),
        }
    }

    /// Check if there's anything to animate.
    pub(crate) fn is_empty(&self) -> bool {
        self.moving.is_empty()
            && self.zapping.is_empty()
            && self.exploding.is_empty()
            && self.triggered_numbers.is_empty()
            && self.pending_explosions.is_empty()
    }

    fn begin_move(&mut self, moving: Moving) {
        let grid = self.grid.borrow_mut();
        *grid.at_mut(moving.from) = Cell::Empty;
        self.moving.push(moving);
        let dest_entity = grid.at_mut(moving.to);
        if !matches!(*dest_entity, Cell::BlackHole) {
            // The grid changes will get overwritten when we replace the grid with the previous one.
            // This is just for sequential blocking checks.
            *dest_entity = moving.cell;
        }
    }
}

/// Core game state without animation.
#[derive(Clone)]
pub(crate) struct GameState {
    pub(crate) grid: Grid,
    pub(crate) initial_grid: Grid,
    pub(crate) history: Vec<Grid>,
    pub(crate) queued_move: Option<Action>,
    pub(crate) completed_levels: HashSet<String>,
}

/// Game wrapper combining state and move handling.
#[derive(Clone)]
pub(crate) struct Game {
    pub(crate) state: GameState,
    /// Animation state. When Some, render from handler.prev_grid.
    /// state.grid always has the final resolved state.
    pub(crate) animation: Option<MoveHandler>,
}

impl GameState {
    pub(crate) fn new(grid: Grid, completed_levels: HashSet<String>) -> Self {
        Self {
            initial_grid: grid.clone(),
            grid: grid.clone(),
            history: vec![grid],
            queued_move: None,
            completed_levels,
        }
    }

    /// Returns the portal destination if the player is currently standing on a portal.
    pub(crate) fn standing_on_portal(&self) -> Option<&str> {
        let (player_pos, _) = self.find_player()?;
        self.grid.get_portal(player_pos)
    }

    /// Returns the note text if the player is currently standing on a note cell.
    pub(crate) fn standing_on_note(&self) -> Option<&str> {
        let (player_pos, _) = self.find_player()?;
        self.grid.get_note(player_pos)
    }

    fn is_level_completed(&self, level: &str) -> bool {
        self.completed_levels.contains(strip_path_prefix(level))
    }

    pub(crate) fn mark_level_completed(&mut self, level: &str) {
        self.completed_levels
            .insert(strip_path_prefix(level).to_string());
    }

    /// Returns the display name of the portal if standing on a completed portal.
    pub(crate) fn standing_on_completed_portal(&self) -> Option<&str> {
        let portal = self.standing_on_portal()?;
        self.is_level_completed(portal)
            .then(|| levels::get_level(portal).map(|l| l.display_name.as_str()))?
    }

    /// Returns the portal destination if the player just stepped onto an unvisited portal (auto-enter).
    pub(crate) fn portal_destination(&self) -> Option<&str> {
        // Check for auto-enter: player just stepped onto an unvisited portal
        let (player_pos, _) = self.find_player()?;
        let current_portal = self.grid.get_portal(player_pos)?;

        // Don't auto-enter if already completed
        if self.is_level_completed(current_portal) {
            return None;
        }

        // Check if player was on a different position before (just moved onto portal)
        if self.history.len() >= 2 {
            let prev_grid = &self.history[self.history.len() - 2];
            let prev_player_pos = prev_grid.entries().find_map(|(pos, cell)| {
                if matches!(cell, Cell::Player(_)) {
                    Some(pos)
                } else {
                    None
                }
            });

            // Only auto-enter if player moved to this position
            if prev_player_pos != Some(player_pos) {
                return Some(current_portal);
            }
        }

        None
    }

    pub(crate) fn initial_has_rats(&self) -> bool {
        self.initial_grid
            .entries()
            .any(|(_, cell)| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)))
    }

    /// Compute play state from grid: GameOver if no player, Won if no rats (and started with rats).
    pub(crate) fn play_state(&self) -> PlayState {
        let play_state = self.grid.play_state();
        if play_state == PlayState::GameOver {
            return PlayState::GameOver;
        }
        if self.initial_has_rats() {
            play_state
        } else {
            PlayState::Playing
        }
    }
}

impl Game {
    pub(crate) fn new(grid: Grid, completed_levels: HashSet<String>) -> Self {
        Self {
            state: GameState::new(grid, completed_levels),
            animation: None,
        }
    }

    pub(crate) fn is_level_completed(&self, level: &str) -> bool {
        self.state.is_level_completed(level)
    }

    pub(crate) fn restart(&mut self) {
        self.state.grid = self.state.initial_grid.clone();
        self.state.history = vec![self.state.grid.clone()];
        self.animation = None;
        self.state.queued_move = None;
    }

    pub(crate) fn undo(&mut self) {
        if self.state.history.len() > 1 {
            self.state.history.pop();
            self.state.grid = self.state.history.last().unwrap().clone();
            self.animation = None;
            self.state.queued_move = None;
        }
    }

    pub(crate) fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub(crate) fn begin_action(&mut self, m: Action) {
        let prev_grid = self.state.grid.clone();

        // Handler #1: resolve instantly
        if !self.apply_action(m) {
            return;
        }

        // Handler #2: for animation
        let mut animator = MoveHandler::new(prev_grid);
        animator.do_player_move(m);

        if !animator.is_empty() {
            self.animation = Some(animator);
        }
    }

    pub(crate) fn try_begin_action(&mut self, m: Action) {
        if self.is_animating() {
            if self.state.queued_move.is_none() {
                self.state.queued_move = Some(m);
            }
        } else {
            self.begin_action(m);
        }
    }

    /// Apply an input immediately without animation (for editor replay)
    pub(crate) fn apply_action(&mut self, m: Action) -> bool {
        let play_state = self.state.play_state();

        if play_state != PlayState::Playing || self.state.find_player().is_none() {
            return false;
        };

        let mut resolver = MoveHandler::new(&mut self.state.grid);
        resolver.do_player_move(m);
        resolver.resolve_all();

        self.state.history.push(self.state.grid.clone());

        true
    }

    pub(crate) fn initial_has_rats(&self) -> bool {
        self.state.initial_has_rats()
    }

    pub(crate) fn grid_width(&self) -> usize {
        self.state.grid.width()
    }

    pub(crate) fn grid_height(&self) -> usize {
        self.state.grid.height()
    }
}

#[cfg(test)]
mod tests;
