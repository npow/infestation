//! Public testing API for scenario tests.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub use crate::direction::Dir4;
use crate::game::MoveHandler;
pub use crate::game::{Action, Game, PlayState};
use crate::grid::Cell;
pub use crate::grid::{CellKind, Grid};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScenarioInput {
    TwoPlayer {
        p1: Action,
        p2: Action,
        state: PlayState,
    },
    SinglePlayer {
        #[serde(rename = "move")]
        action: Action,
        state: PlayState,
    },
}

/// Create a game from CSV content.
pub fn game_from_csv(csv: &str) -> Game {
    Game::new(Grid::from_csv(csv), HashSet::new())
}

/// Create a grid from CSV content.
pub fn grid_from_csv(csv: &str) -> Grid {
    Grid::from_csv(csv)
}

/// Get the grid as CSV.
pub fn grid_to_csv(game: &Game) -> String {
    game.state.grid.to_csv()
}

/// Get the current play state.
pub fn play_state(game: &Game) -> PlayState {
    game.state.play_state()
}

impl Game {
    pub fn grid_to_csv(&self) -> String {
        self.state.grid.to_csv()
    }

    pub fn play_state(&self) -> PlayState {
        self.state.play_state()
    }
}

/// Apply multiple player actions to a game.
#[must_use]
pub fn apply_actions(game: &mut Game, actions: &[Action]) -> bool {
    game.apply_actions(actions)
}

/// Apply actions directly to a grid without CSV serialization.
#[must_use]
pub fn step_grid(grid: &Grid, actions: &[Action]) -> (Grid, PlayState) {
    let initial_player_count = count_players(grid);
    let initial_had_rats = has_rats(grid);
    let initial_play_state = play_state_from_grid(grid, initial_player_count, initial_had_rats);
    if initial_play_state != PlayState::Playing {
        return (grid.clone(), initial_play_state);
    }

    let mut next = grid.clone();
    let mut resolver = MoveHandler::new(&mut next);
    if resolver.find_players().is_empty() {
        return (grid.clone(), initial_play_state);
    }

    resolver.do_player_moves(actions);
    resolver.resolve_all();

    let play_state = play_state_from_grid(&next, initial_player_count, initial_had_rats);
    (next, play_state)
}

/// Apply actions to a state the caller already knows is Playing.
///
/// Search callers expand only live frontier states and already know the level's
/// player count, so this avoids two full-grid scans before every candidate move.
#[must_use]
pub fn step_grid_assume_playing(
    grid: &Grid,
    actions: &[Action],
    initial_player_count: usize,
    initial_had_rats: bool,
) -> (Grid, PlayState) {
    let mut next = grid.clone();
    let mut resolver = MoveHandler::new(&mut next);
    resolver.do_player_moves(actions);
    resolver.resolve_all();

    let play_state = play_state_from_grid(&next, initial_player_count, initial_had_rats);
    (next, play_state)
}

fn count_players(grid: &Grid) -> usize {
    grid.entries()
        .filter(|(_, cell)| matches!(cell, Cell::Player(..)))
        .count()
}

fn has_rats(grid: &Grid) -> bool {
    grid.entries()
        .any(|(_, cell)| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)))
}

fn play_state_from_grid(
    grid: &Grid,
    initial_player_count: usize,
    initial_had_rats: bool,
) -> PlayState {
    let mut player_count = 0usize;
    let mut has_rats = false;
    for (_, cell) in grid.entries() {
        match cell {
            Cell::Player(..) => player_count += 1,
            Cell::Rat(_) | Cell::CyborgRat(_) => has_rats = true,
            _ => {}
        }
    }

    if player_count < initial_player_count {
        return PlayState::GameOver;
    }

    if !has_rats && initial_had_rats {
        PlayState::Won
    } else {
        PlayState::Playing
    }
}
