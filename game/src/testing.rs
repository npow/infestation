//! Public testing API for scenario tests.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::game::{Action, Game, PlayState};
use crate::grid::Grid;

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

impl Game {
    pub fn grid_to_csv(&self) -> String {
        self.state.grid.to_csv()
    }

    pub fn play_state(&self) -> PlayState {
        self.state.play_state()
    }
}
