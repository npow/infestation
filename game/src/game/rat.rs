use std::borrow::BorrowMut;

use crate::direction::Dir8;
use crate::grid::{Cell, Grid, Player};

use super::{MoveHandler, Moving, PlayerInfo};

/// Sort key for choosing the best rat move option.
/// Fields are ordered for derived Ord: lower values are preferred.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RatMoveKey<T = i32> {
    pub(super) score: T,
    /// Euclidean distance² the rat has to move
    pub(super) move_d2: i32,
    /// Did the target player stall this turn? (false < true)
    pub(super) player_still: bool,
    /// Which player this option targets (Player1 < Player2).
    pub(super) player: Player,
    /// None for stay, Some(dir) for a move.
    pub(super) dir: Option<Dir8>,
}

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    pub(crate) fn move_rats(&mut self, players: &[PlayerInfo]) {
        let mut rats: Vec<_> = self
            .grid
            .borrow()
            .find_entities(|cell| matches!(cell, Cell::Rat(_)))
            .map(|(pos, _)| pos)
            .collect();

        // Sort by distance to nearest player (either one), tiebreak by position
        rats.sort_by_key(|&pos| {
            let min_dist = players.iter().map(|p| pos.dist_sq(p.pos)).min().unwrap();
            (min_dist, pos)
        });

        for rat_pos in rats {
            // Stay option: scored by min distance to any player
            let score = players
                .iter()
                .map(|p| rat_pos.dist_sq(p.pos))
                .min()
                .unwrap();
            let mut candidates = vec![RatMoveKey {
                score,
                move_d2: 0,
                player_still: true,
                player: Player::Player1,
                dir: None,
            }];

            // Generate move options for each player
            for player_info in players {
                let face_dir = Dir8::from_delta(player_info.pos - rat_pos).unwrap();
                let dirs: Vec<Dir8> = if face_dir.is_diagonal() {
                    vec![
                        face_dir,
                        face_dir.x_only().unwrap(),
                        face_dir.y_only().unwrap(),
                    ]
                } else {
                    vec![face_dir]
                };

                for dir in dirs {
                    let new_pos = rat_pos + dir.delta();

                    if self.grid.borrow().at(new_pos).blocks_rat() {
                        continue;
                    }

                    // Check sword blocking against all players
                    let sword_blocked = players
                        .iter()
                        .any(|p| new_pos == p.pos && dir == p.dir.opposite());
                    if sword_blocked {
                        continue;
                    }

                    candidates.push(RatMoveKey {
                        score: new_pos.dist_sq(player_info.pos),
                        move_d2: dir.dist_sq(),
                        player_still: !player_info.moved,
                        player: player_info.player,
                        dir: Some(dir),
                    });
                }
            }

            let best = candidates.into_iter().min().unwrap();

            if let Some(dir) = best.dir {
                self.begin_move(Moving {
                    cell: Cell::Rat(dir),
                    from: rat_pos,
                    progress: 0.0,
                    to: rat_pos + dir.delta(),
                });
            } else {
                // Rat stays: face nearest player
                let nearest = players
                    .iter()
                    .min_by_key(|p| (rat_pos.dist_sq(p.pos), !p.moved, p.player))
                    .unwrap();
                let face_dir = Dir8::from_delta(nearest.pos - rat_pos).unwrap_or(Dir8::South);
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
