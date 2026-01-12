use crate::direction::Dir4;
use crate::grid::Cell;
use crate::position::Position;

use super::{Action, Game, GameState, MoveHandler, Moving};

impl MoveHandler {
    pub(crate) fn find_player(&self) -> Option<(Position, Dir4)> {
        for (pos, cell) in self.grid.entries() {
            if let Cell::Player(dir) = cell {
                return Some((pos, dir));
            }
        }
        None
    }

    pub(crate) fn do_player_move(&mut self, m: Action) {
        let Some((player_pos, current_dir)) = self.find_player() else {
            return;
        };

        let prev_grid = self.grid.clone();

        let new_pos;
        let new_dir;
        match m {
            Action::Move(dir) => {
                let candidate = player_pos + dir.delta();
                let target_cell = self.grid.at(candidate);
                let blocked = target_cell.blocks_player();
                new_pos = if blocked { player_pos } else { candidate };
                new_dir = dir;

                self.begin_move(Moving {
                    cell: Cell::Player(dir),
                    from: player_pos,
                    progress: if blocked { 1.0 } else { 0.0 },
                    to: new_pos,
                });
            }
            Action::Stall => {
                new_pos = player_pos;
                new_dir = current_dir;
            }
        }

        self.move_cyborg_rats(new_pos, new_dir);
        self.move_rats(new_pos, new_dir);
        // Don't actually perform the move yet.
        // The grid is useful for tracking what is blocked so that rat movement is resolved
        // sequentially. But we should wait for animations to complete before placing things at
        // their final positions.
        self.grid = prev_grid;
        // Remove entities from their old positions now that they're tracked as moving entities.
        for m in &self.moving {
            *self.grid.at_mut(m.from) = Cell::Empty;
        }
    }
}

impl GameState {
    pub(crate) fn find_player(&self) -> Option<(Position, Dir4)> {
        let (pos, player) = self
            .grid
            .find_entities(|cell| matches!(cell, Cell::Player(_)))
            .next()?;
        let Cell::Player(dir) = player else {
            unreachable!();
        };
        Some((pos, dir))
    }
}

impl Game {
    pub(crate) fn enter_portal(&self) -> Option<&str> {
        self.state.standing_on_portal()
    }
}
