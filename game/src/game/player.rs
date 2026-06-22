use std::borrow::BorrowMut;

use crate::grid::{Cell, Player};
use crate::position::Position;
use crate::{direction::Dir4, grid::Grid};

use super::{Action, Game, MoveHandler, Moving, PlayerInfo};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    pub(crate) fn find_players(&self) -> Vec<crate::grid::FoundPlayer> {
        self.grid.borrow().find_players()
    }

    /// Execute player moves. Takes a slice of Action, one per player found in the grid.
    pub(crate) fn do_player_moves(&mut self, actions: &[Action]) {
        let players = self.find_players();
        assert!(!players.is_empty());

        let grid = self.grid.borrow();
        let prev_grid = grid.clone();

        // Phase 1: Determine each player's intended destination (wall/plank blocks only)
        let mut dests: Vec<Position> = Vec::new();
        let mut move_dirs: Vec<Option<Dir4>> = Vec::new();
        let mut facing_dirs: Vec<Dir4> = Vec::new();

        for (i, player) in players.iter().enumerate() {
            let (player_pos, current_dir) = (player.pos, player.dir);
            let action = actions.get(i).copied().unwrap_or(Action::Stall);
            match action {
                Action::Move(dir) => {
                    let candidate = player_pos + dir.delta();
                    let wall_blocked = self.grid.borrow().at(candidate).blocks_player();
                    dests.push(if wall_blocked { player_pos } else { candidate });
                    move_dirs.push(Some(dir));
                    facing_dirs.push(dir);
                }
                Action::Stall => {
                    dests.push(player_pos);
                    move_dirs.push(None);
                    facing_dirs.push(current_dir);
                }
            }
        }

        // Phase 2: Resolve player-player conflicts
        if players.len() == 2 {
            let moving = [dests[0] != players[0].pos, dests[1] != players[1].pos];

            if moving[0] && moving[1] && dests[0] == dests[1] {
                self.contested_cell = Some(dests[0]);
                *self.grid.borrow_mut().at_mut(dests[0]) = Cell::Empty;
                // Both target same cell → both blocked
                dests[0] = players[0].pos;
                dests[1] = players[1].pos;
            } else if moving[0]
                && moving[1]
                && dests[0] == players[1].pos
                && dests[1] == players[0].pos
            {
                // Swap → both blocked
                dests[0] = players[0].pos;
                dests[1] = players[1].pos;
            } else {
                // Check each direction: player i moving into player j's position
                for (i, j) in [(0usize, 1usize), (1, 0)] {
                    if moving[i] && dests[i] == players[j].pos && dests[j] == players[j].pos {
                        // Player i moves into player j's cell while j stays
                        if facing_dirs[j].is_opposite(move_dirs[i].unwrap()) {
                            // j faces i → i blocked
                            dests[i] = players[i].pos;
                        }
                        // else: i kills j (handled naturally by overwrite in finish_moving)
                    }
                }
            }
        }

        // Phase 3: Create Moving entities and build PlayerInfos
        let mut player_infos: Vec<PlayerInfo> = Vec::new();
        for (i, found) in players.iter().enumerate() {
            let dest = dests[i];
            let blocked = dest == found.pos;

            if move_dirs[i].is_some() {
                let dir = facing_dirs[i];
                self.begin_move(Moving {
                    cell: Cell::player(found.player, dir),
                    from: found.pos,
                    progress: if blocked { 1.0 } else { 0.0 },
                    to: dest,
                });
            }

            player_infos.push(PlayerInfo {
                pos: dest,
                dir: facing_dirs[i],
                moved: move_dirs[i].is_some(),
                player: found.player,
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

impl Game {
    pub(crate) fn enter_portal(&self, player: Player) -> Option<&str> {
        self.state.player_standing_on_portal(player)
    }
}
