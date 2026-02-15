use std::borrow::BorrowMut;
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap};

use crate::direction::Dir8;
use crate::grid::{Cell, Grid, Player};
use crate::position::Position;

use super::cyborg_distance::{CyborgDistance, DijkstraEntry};
use super::{MoveHandler, Moving, PlayerInfo};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    /// Compute shortest path distances from all players using multi-source Dijkstra.
    /// Returns each position's distance to the nearest player and which player that is.
    /// Ties broken by: prefer moved players, then P1.
    fn compute_cyborg_distances(
        &self,
        players: &[PlayerInfo],
    ) -> HashMap<Position, (CyborgDistance, Player)> {
        let mut distances: HashMap<Position, (CyborgDistance, Player)> = HashMap::new();
        let mut heap: BinaryHeap<DijkstraEntry> = BinaryHeap::new();

        for p in players {
            heap.push(DijkstraEntry {
                dist: CyborgDistance::ZERO,
                pos: p.pos,
                player: p.player,
                moved: p.moved,
            });
        }

        let grid = self.grid.borrow();
        let bounds = grid.bounds();

        while let Some(DijkstraEntry {
            dist,
            pos,
            player,
            moved,
        }) = heap.pop()
        {
            match distances.entry(pos) {
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert((dist, player));
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

                heap.push(DijkstraEntry {
                    dist: dist.add_step(dir),
                    pos: neighbor,
                    player,
                    moved,
                });
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
            let nearest = players
                .iter()
                .min_by_key(|p| cyborg_pos.dist_sq(p.pos))
                .unwrap();
            let face_dir = Dir8::from_delta(nearest.pos - cyborg_pos).unwrap();
            self.begin_move(Moving {
                cell: Cell::CyborgRat(face_dir),
                from: cyborg_pos,
                progress: 1.0,
                to: cyborg_pos,
            });
        }

        // Sort reachable cyborg rats by distance to their nearest player
        let mut movable_cyborgs: Vec<_> = reachable
            .into_iter()
            .map(|pos| {
                let &(dist, target_player) = distances.get(&pos).unwrap();
                (dist, pos, target_player)
            })
            .collect();
        movable_cyborgs.sort_by_key(|&(dist, pos, _)| (dist, pos));

        for (current_dist, cyborg_pos, target_player) in movable_cyborgs {
            let player = players.iter().find(|p| p.player == target_player).unwrap();
            let blocked_dir = player.dir.opposite();

            // Find best adjacent cell
            let mut best_move: Option<(Dir8, CyborgDistance)> = None;

            for dir in Dir8::all() {
                let new_pos = cyborg_pos + dir.delta();

                let Some(&(target_dist, assigned)) = distances.get(&new_pos) else {
                    continue;
                };

                // Only follow gradient toward our target player
                if assigned != target_player {
                    continue;
                }

                // Must be an improvement (to allow moving toward goal)
                // unless the player is about to kill us
                if target_dist >= current_dist && current_dist != CyborgDistance::ONE_ORTHO {
                    continue;
                }

                // Don't move to orthogonally adjacent to player where they can reach us
                // (distance (1, 0))
                if target_dist == CyborgDistance::ONE_ORTHO {
                    continue;
                }

                // Can't attack from in front of player (sword blocks)
                if new_pos == player.pos && dir == blocked_dir {
                    continue;
                }

                // Check if cell is available
                let target_cell = self.grid.borrow().at(new_pos);

                // Can't move into walls, other cyborg rats, spiderwebs, black holes
                if target_cell.blocks_cyborg_rat() {
                    continue;
                }

                // This is a valid move - check if it's the best
                if best_move.is_none_or(|(_, best_dist)| target_dist < best_dist) {
                    best_move = Some((dir, target_dist));
                }
            }

            if let Some((dir, _)) = best_move {
                self.begin_move(Moving {
                    cell: Cell::CyborgRat(dir),
                    from: cyborg_pos,
                    progress: 0.0,
                    to: cyborg_pos + dir.delta(),
                });
            } else {
                let face_dir = Dir8::from_delta(player.pos - cyborg_pos).unwrap();
                // Cyborg rat can't move - turn to face the player
                self.begin_move(Moving {
                    cell: Cell::CyborgRat(face_dir),
                    from: cyborg_pos,
                    progress: 1.0,
                    to: cyborg_pos,
                });
            }
        }
    }
}
