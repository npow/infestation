use std::borrow::BorrowMut;

use crate::direction::Dir8;
use crate::grid::{Cell, Grid, Player};

use super::{MoveHandler, Moving, PlayerInfo};

mod never_compared;

use self::never_compared::NeverCompared;

/// Sort key for choosing the best rat move option.
/// Fields are ordered for derived Ord: lower values are preferred.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RatMoveKey {
    /// Euclidean distance² to target player after moving
    score: i32,
    /// Euclidean distance² the rat has to move
    move_d2: i32,
    /// Did the target player stall this turn? (false < true)
    player_still: bool,
    /// Which player this option targets (Player1 < Player2).
    player: Player,
    /// true if the move direction is vertical; horizontal is preferred (false < true).
    is_vertical: bool,
    /// None for stay, Some(dir) for a move. All ties are already broken at this point.
    dir: NeverCompared<Option<Dir8>>,
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
                is_vertical: false,
                dir: NeverCompared(None),
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
                        is_vertical: dir.delta().dx == 0,
                        dir: NeverCompared(Some(dir)),
                    });
                }
            }

            let best = candidates.into_iter().min().unwrap();

            if let Some(dir) = best.dir.0 {
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
