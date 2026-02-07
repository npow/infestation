use std::borrow::BorrowMut;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::direction::Dir4;
use crate::grid::{Cell, Grid, NoteText};
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

/// Information about a player for movement resolution.
#[derive(Clone, Copy)]
pub struct PlayerInfo {
    pub pos: Position,
    pub dir: Dir4,
    pub acted: bool,
    pub player_index: usize,
}

const MOVE_SPEED: f32 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayState {
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

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
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
pub struct GameState {
    pub(crate) grid: Grid,
    pub(crate) initial_grid: Grid,
    pub(crate) history: Vec<Grid>,
    pub(crate) queued_actions: Option<Vec<Option<Action>>>,
    pub(crate) completed_levels: HashSet<String>,
}

/// Game wrapper combining state and move handling.
#[derive(Clone)]
pub struct Game {
    pub(crate) state: GameState,
    /// Animation state. When Some, render from handler.prev_grid.
    /// state.grid always has the final resolved state.
    pub(crate) animation: Option<MoveHandler>,
}

impl GameState {
    pub(crate) fn player_count(&self) -> usize {
        Self::count_players(&self.initial_grid)
    }

    pub(crate) fn new(grid: Grid, completed_levels: HashSet<String>) -> Self {
        Self {
            initial_grid: grid.clone(),
            grid: grid.clone(),
            history: vec![grid],
            queued_actions: None,
            completed_levels,
        }
    }

    /// Returns the portal destination if the player is currently standing on a portal.
    pub(crate) fn standing_on_portal(&self) -> Option<&str> {
        let (player_pos, _) = self.find_player()?;
        self.grid.get_portal(player_pos)
    }

    /// Returns the note text if the player is currently standing on a note cell.
    pub(crate) fn standing_on_note(&self) -> Option<&NoteText> {
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

    fn count_players(grid: &Grid) -> usize {
        grid.entries()
            .filter(|(_, cell)| cell.as_player().is_some())
            .count()
    }

    /// Compute play state from grid: GameOver if any player died, Won if no rats (and started with rats).
    pub(crate) fn play_state(&self) -> PlayState {
        // GameOver if any player has died
        let initial_player_count = Self::count_players(&self.initial_grid);
        let current_player_count = Self::count_players(&self.grid);
        if current_player_count < initial_player_count {
            return PlayState::GameOver;
        }

        // Check for win condition (no rats left)
        let has_rats = self
            .grid
            .entries()
            .any(|(_, cell)| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)));

        if !has_rats && self.initial_has_rats() {
            PlayState::Won
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
        self.state.queued_actions = None;
    }

    pub(crate) fn undo(&mut self) {
        if self.state.history.len() > 1 {
            self.state.history.pop();
            self.state.grid = self.state.history.last().unwrap().clone();
            self.animation = None;
            self.state.queued_actions = None;
        }
    }

    pub(crate) fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub(crate) fn begin_actions(&mut self, actions: &[Option<Action>]) {
        let prev_grid = self.state.grid.clone();

        // Handler #1: resolve instantly
        if !self.apply_actions(actions) {
            return;
        }

        // Handler #2: for animation
        let mut animator = MoveHandler::new(prev_grid);
        animator.do_player_moves(actions);

        if !animator.is_empty() {
            self.animation = Some(animator);
        }
    }

    pub(crate) fn try_begin_actions(&mut self, actions: Vec<Option<Action>>) {
        if self.is_animating() {
            if self.state.queued_actions.is_none() {
                self.state.queued_actions = Some(actions);
            }
        } else {
            self.begin_actions(&actions);
        }
    }

    /// Apply an input immediately without animation (for editor replay)
    pub fn apply_action(&mut self, m: Action) -> bool {
        self.apply_actions(&[Some(m)])
    }

    /// Apply multiple player actions immediately without animation.
    /// Takes one Option<Action> per player in the grid.
    pub fn apply_actions(&mut self, actions: &[Option<Action>]) -> bool {
        let play_state = self.state.play_state();

        if play_state != PlayState::Playing {
            return false;
        }

        // Check that at least one player exists and at least one action is provided
        let mut resolver = MoveHandler::new(&mut self.state.grid);
        if resolver.find_players().is_empty() {
            return false;
        }

        resolver.do_player_moves(actions);
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
