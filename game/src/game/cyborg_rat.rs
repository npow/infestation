use std::borrow::BorrowMut;
use std::cmp::{Ordering, Reverse};
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap};

use crate::direction::Dir8;
use crate::game::rat::RatMoveKey;
use crate::grid::{Cell, Grid};
use crate::position::Position;

use super::cyborg_distance::{CyborgDistance, CyborgEntry};
use super::{MoveHandler, Moving, PlayerInfo};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    /// Compute shortest path distances from all players using multi-source Dijkstra.
    /// Returns each position's distance to the nearest player and which player that is.
    /// Ties broken by: prefer moved players, then P1.
    fn compute_cyborg_distances(&self, players: &[PlayerInfo]) -> HashMap<Position, CyborgEntry> {
        let mut distances: HashMap<Position, CyborgEntry> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(CyborgEntry, Position)>> = BinaryHeap::new();

        for p in players {
            heap.push(Reverse((
                CyborgEntry {
                    dist: CyborgDistance::ZERO,
                    player: p.player,
                    still: !p.moved,
                },
                p.pos,
            )));
        }

        let grid = self.grid.borrow();
        let bounds = grid.bounds();

        while let Some(Reverse((
            entry @ CyborgEntry {
                dist,
                player,
                still,
            },
            pos,
        ))) = heap.pop()
        {
            match distances.entry(pos) {
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(entry);
                }
                Entry::Occupied(_) => continue,
            };

            // Check all 8 neighbors
            for dir in Dir8::all() {
                let neighbor = pos + dir.delta();

                if !neighbor.in_bounds(bounds) {
                    continue;
                }

                // Skip if already finalized
                if distances.contains_key(&neighbor) {
                    continue;
                }

                // Check if traversable for cyborg pathfinding
                let cell = grid.at(neighbor);
                let traversable = match cell {
                    Cell::Wall | Cell::BlackHole | Cell::Spiderweb | Cell::Explosive => false,
                    Cell::Empty
                    | Cell::Plank
                    | Cell::Rat(_)
                    | Cell::CyborgRat(_)
                    | Cell::Trigger(_)
                    | Cell::Player(..) => true,
                };

                if !traversable {
                    continue;
                }

                heap.push(Reverse((
                    CyborgEntry {
                        dist: dist.add_step(dir),
                        player,
                        still,
                    },
                    neighbor,
                )));
            }
        }

        distances
    }

    pub(crate) fn move_cyborg_rats(&mut self, players: &[PlayerInfo]) {
        // Single multi-source Dijkstra from all players
        let distances = self.compute_cyborg_distances(players);

        let cyborg_positions: Vec<_> = self
            .grid
            .borrow()
            .find_entities(|cell| matches!(cell, Cell::CyborgRat(_)))
            .map(|(pos, _)| pos)
            .collect();

        // Partition into reachable and unreachable
        let (reachable, unreachable): (Vec<_>, Vec<_>) = cyborg_positions
            .into_iter()
            .partition(|pos| distances.contains_key(pos));

        // Unreachable cyborg rats just turn to face the nearest player (by Euclidean)
        for cyborg_pos in unreachable {
            self.turn_to_face_nearest(cyborg_pos, players, Cell::CyborgRat);
        }

        // Sort reachable cyborg rats
        let mut movable_cyborgs: Vec<_> = reachable
            .into_iter()
            .map(|pos| {
                let &entry = distances.get(&pos).unwrap();
                (entry, pos)
            })
            .collect();
        movable_cyborgs.sort_by_key(|&(entry, pos)| (entry, pos));

        for (entry, cyborg_pos) in movable_cyborgs {
            let CyborgEntry {
                dist: current_dist,
                still,
                player,
            } = entry;
            let mut best_move: RatMoveKey<Dist> = RatMoveKey {
                score: Dist(current_dist),
                move_d2: 0,
                player_still: still,
                player,
                dir: None,
            };
            'outer: for dir in Dir8::all() {
                let new_pos = cyborg_pos + dir.delta();

                let Some(&CyborgEntry {
                    dist,
                    still,
                    player,
                }) = distances.get(&new_pos)
                else {
                    continue;
                };

                // Can't attack from in front of player (sword blocks)
                for player_info in players {
                    if new_pos == player_info.pos && dir == player_info.dir.opposite() {
                        continue 'outer;
                    }
                }

                // Check if cell is available
                let target_cell = self.grid.borrow().at(new_pos);

                // Can't move into walls, other cyborg rats, spiderwebs, black holes
                if target_cell.blocks_cyborg_rat() {
                    continue;
                }

                let target_move = RatMoveKey {
                    score: Dist(dist),
                    move_d2: dir.dist_sq(),
                    player_still: still,
                    player,
                    dir: Some(dir),
                };

                // This is a valid move - check if it's the best
                if target_move < best_move {
                    best_move = target_move;
                }
            }

            if let Some(dir) = best_move.dir {
                self.begin_move(Moving {
                    cell: Cell::CyborgRat(dir),
                    from: cyborg_pos,
                    progress: 0.0,
                    to: cyborg_pos + dir.delta(),
                });
            } else {
                // Cyborg rat can't move - turn to face the player
                self.turn_to_face_nearest(cyborg_pos, players, Cell::CyborgRat);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Dist(CyborgDistance);

impl PartialOrd for Dist {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Dist {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0 == CyborgDistance::ONE_ORTHO)
            .cmp(&(other.0 == CyborgDistance::ONE_ORTHO))
            .then_with(|| self.0.cmp(&other.0))
    }
}
