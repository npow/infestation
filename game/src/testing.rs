//! Public testing API for scenario tests.

use std::collections::HashSet;

use crate::grid::Grid;

pub use crate::direction::Dir4;
pub use crate::game::{Action, Game, PlayState};

/// Create a game from CSV content.
pub fn game_from_csv(csv: &str) -> Game {
    Game::new(Grid::from_csv(csv), HashSet::new())
}

/// Get the grid as CSV.
pub fn grid_to_csv(game: &Game) -> String {
    game.state.grid.to_csv()
}

/// Get the current play state.
pub fn play_state(game: &Game) -> PlayState {
    game.state.play_state()
}
