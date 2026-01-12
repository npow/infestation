use crate::direction::{Dir4, Dir8};
use crate::grid::Cell;
use crate::position::Position;

use super::{MoveHandler, Moving};

impl MoveHandler {
    pub(crate) fn move_rats(&mut self, player: Position, player_facing: Dir4) {
        let blocked_dir = player_facing.opposite();
        let mut rats: Vec<_> = self
            .grid
            .find_entities(|cell| matches!(cell, Cell::Rat(_)))
            .map(|(pos, _)| pos)
            .collect();

        rats.sort_by_key(|&pos| (pos.dist_sq(player), pos));

        for rat_pos in rats {
            let face_dir = Dir8::from_delta(player - rat_pos);

            // Build list of moves to try in order
            let moves_to_try: Vec<Dir8> = match face_dir {
                Some(dir) => {
                    if dir.is_diagonal() {
                        // Rat doesn't share row or column - try diagonal first, then orthogonals
                        let h_move = dir.x_only().unwrap();
                        let v_move = dir.y_only().unwrap();

                        // Sort orthogonals by distance, tie-break horizontal first
                        let h_pos = rat_pos + h_move.delta();
                        let v_pos = rat_pos + v_move.delta();
                        let h_dist = h_pos.dist_sq(player);
                        let v_dist = v_pos.dist_sq(player);

                        if h_dist <= v_dist {
                            vec![dir, h_move, v_move]
                        } else {
                            vec![dir, v_move, h_move]
                        }
                    } else {
                        // Rat shares row or column with player - only try direct move
                        vec![dir]
                    }
                }
                None => vec![], // On top of player, no move
            };

            // Try each move in order
            let mut chosen_dir: Option<Dir8> = None;
            for dir in moves_to_try {
                let new_pos = rat_pos + dir.delta();

                let target_cell = self.grid.at(new_pos);
                // If rat is moving to player's destination, skip block check
                // (player is clearing whatever was there, e.g. spiderweb)
                if target_cell.blocks_rat() {
                    continue;
                }

                // Can't attack player from in front (sword blocks)
                if new_pos == player && dir == blocked_dir {
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
            } else if let Some(face_dir) = face_dir {
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
