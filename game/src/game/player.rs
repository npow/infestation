use std::borrow::BorrowMut;

use crate::grid::Cell;
use crate::position::Position;
use crate::{direction::Dir4, grid::Grid};

use super::{Action, Game, GameState, MoveHandler, Moving, PlayerInfo};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    /// Find all players in the grid, sorted by player index.
    pub(crate) fn find_players(&self) -> Vec<(Position, Dir4, usize)> {
        let mut players: Vec<_> = self
            .grid
            .borrow()
            .entries()
            .filter_map(|(pos, cell)| cell.as_player().map(|(idx, dir)| (pos, dir, idx)))
            .collect();
        players.sort_by_key(|&(_, _, idx)| idx);
        players
    }

    /// Execute player moves. Takes a slice of Option<Action>, one per player found in the grid.
    /// None means the player didn't act this turn. Some(action) means they did.
    pub(crate) fn do_player_moves(&mut self, actions: &[Option<Action>]) {
        let players = self.find_players();
        if players.is_empty() {
            return;
        }

        let grid = self.grid.borrow();
        let prev_grid = grid.clone();

        // Process each player's action and build PlayerInfo vec
        let mut player_infos: Vec<PlayerInfo> = Vec::new();
        for (i, &(player_pos, current_dir, player_index)) in players.iter().enumerate() {
            let action = actions.get(i).copied().flatten();

            let (new_pos, new_dir, acted) = match action {
                Some(Action::Move(dir)) => {
                    let candidate = player_pos + dir.delta();
                    let target_cell = self.grid.borrow().at(candidate);
                    let blocked = target_cell.blocks_player();
                    let new_pos = if blocked { player_pos } else { candidate };

                    self.begin_move(Moving {
                        cell: Cell::player(player_index, dir),
                        from: player_pos,
                        progress: if blocked { 1.0 } else { 0.0 },
                        to: new_pos,
                    });
                    (new_pos, dir, true)
                }
                Some(Action::Stall) => (player_pos, current_dir, true),
                None => (player_pos, current_dir, false),
            };

            player_infos.push(PlayerInfo {
                pos: new_pos,
                dir: new_dir,
                acted,
                player_index,
            });
        }

        self.move_cyborg_rats(&player_infos);
        self.move_rats(&player_infos);

        // Don't actually perform the move yet.
        // The grid is useful for tracking what is blocked so that rat movement is resolved
        // sequentially. But we should wait for animations to complete before placing things at
        // their final positions.
        let curr_grid = self.grid.borrow_mut();
        *curr_grid = prev_grid;
        // Remove entities from their old positions now that they're tracked as moving entities.
        for m in &self.moving {
            *curr_grid.at_mut(m.from) = Cell::Empty;
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
