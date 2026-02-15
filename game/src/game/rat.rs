use std::borrow::BorrowMut;

use crate::direction::Dir8;
use crate::grid::{Cell, Grid};
use crate::position::Position;

use super::{MoveHandler, Moving, PlayerInfo};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    /// Find the nearest player to a position using Euclidean distance.
    /// Returns the PlayerInfo for the nearest player.
    /// If tied, prefers moving players, then lower index.
    fn nearest_player_euclidean<'a>(
        &self,
        pos: Position,
        players: &'a [PlayerInfo],
    ) -> &'a PlayerInfo {
        players
            .iter()
            .min_by_key(|p| {
                let dist = pos.dist_sq(p.pos);
                // Tie-break: prefer moving players, then lower player (P1 < P2)
                (dist, !p.moved, p.player)
            })
            .unwrap()
    }

    pub(crate) fn move_rats(&mut self, players: &[PlayerInfo]) {
        if players.is_empty() {
            return;
        }

        let mut rats: Vec<_> = self
            .grid
            .borrow()
            .find_entities(|cell| matches!(cell, Cell::Rat(_)))
            .map(|(pos, _)| pos)
            .collect();

        // Sort by distance to nearest player
        rats.sort_by_key(|&pos| {
            let nearest = self.nearest_player_euclidean(pos, players);
            (pos.dist_sq(nearest.pos), pos)
        });

        for rat_pos in rats {
            let nearest = self.nearest_player_euclidean(rat_pos, players);
            let target = nearest.pos;
            let blocked_dir = nearest.dir.opposite();
            let face_dir = Dir8::from_delta(target - rat_pos).unwrap();

            // Build list of moves to try in order
            let moves_to_try: Vec<Dir8> = if face_dir.is_diagonal() {
                // Rat doesn't share row or column - try diagonal first, then orthogonals
                let h_move = face_dir.x_only().unwrap();
                let v_move = face_dir.y_only().unwrap();

                // Sort orthogonals by distance, tie-break horizontal first
                let h_pos = rat_pos + h_move.delta();
                let v_pos = rat_pos + v_move.delta();
                let h_dist = h_pos.dist_sq(target);
                let v_dist = v_pos.dist_sq(target);

                if h_dist <= v_dist {
                    vec![face_dir, h_move, v_move]
                } else {
                    vec![face_dir, v_move, h_move]
                }
            } else {
                // Rat shares row or column with player - only try direct move
                vec![face_dir]
            };

            // Try each move in order
            let mut chosen_dir: Option<Dir8> = None;
            for dir in moves_to_try {
                let new_pos = rat_pos + dir.delta();

                let target_cell = self.grid.borrow().at(new_pos);
                if target_cell.blocks_rat() {
                    continue;
                }

                // Can't attack player from in front (sword blocks)
                if new_pos == target && dir == blocked_dir {
                    continue;
                }

                chosen_dir = Some(dir);
                break;
            }

            if let Some(dir) = chosen_dir {
                self.begin_move(Moving {
                    cell: Cell::Rat(dir),
                    from: rat_pos,
                    progress: 0.0,
                    to: rat_pos + dir.delta(),
                });
            } else {
                // Rat can't move - turn to face the player
                self.begin_move(Moving {
                    cell: Cell::Rat(face_dir),
                    from: rat_pos,
                    progress: 1.0,
                    to: rat_pos,
                });
            }
        }
    }
}
