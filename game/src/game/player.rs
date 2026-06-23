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

        // Phase 1: Determine each player's intended destination (wall/plank blocks only)
        let mut dests = [Position { x: 0, y: 0 }; 2];
        let mut move_dirs = [None; 2];
        let mut facing_dirs = [Dir4::North; 2];

        for (i, player) in players.iter().enumerate() {
            let (player_pos, current_dir) = (player.pos, player.dir);
            let action = actions.get(i).copied().unwrap_or(Action::Stall);
            match action {
                Action::Move(dir) => {
                    let candidate = player_pos + dir.delta();
                    let wall_blocked = self.grid.borrow().at(candidate).blocks_player();
                    dests[i] = if wall_blocked { player_pos } else { candidate };
                    move_dirs[i] = Some(dir);
                    facing_dirs[i] = dir;
                }
                Action::Stall => {
                    dests[i] = player_pos;
                    move_dirs[i] = None;
                    facing_dirs[i] = current_dir;
                }
            }
        }

        // Phase 2: Resolve player-player conflicts
        if players.len() == 2 {
            let moving = [dests[0] != players[0].pos, dests[1] != players[1].pos];

            if moving[0] && moving[1] && dests[0] == dests[1] {
                self.contested_cell = Some(dests[0]);
                self.set_cell_for_movement(dests[0], Cell::Empty);
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
        let mut player_infos = [PlayerInfo {
            pos: Position { x: 0, y: 0 },
            dir: Dir4::North,
            moved: false,
            player: Player::Player1,
        }; 2];
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

            player_infos[i] = PlayerInfo {
                pos: dest,
                dir: facing_dirs[i],
                moved: move_dirs[i].is_some(),
                player: found.player,
            };
        }

        let player_infos = &player_infos[..players.len()];
        self.move_cyborg_rats(player_infos);
        self.move_rats(player_infos);

        // Don't actually perform the move yet.
        // The grid is useful for tracking what is blocked so that rat movement is resolved
        // sequentially. But we should wait for animations to complete before placing things at
        // their final positions.
        self.restore_movement_grid();
    }
}

impl Game {
    pub(crate) fn enter_portal(&self, player: Player) -> Option<&str> {
        self.state.player_standing_on_portal(player)
    }
}
