use std::borrow::BorrowMut;
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap};

use crate::direction::{Dir4, Dir8};
use crate::grid::{Cell, Grid};
use crate::position::Position;

use super::cyborg_distance::{CyborgDistance, DijkstraEntry};
use super::{MoveHandler, Moving};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    /// Compute shortest path distances from target using Dijkstra with A + B*sqrt(2) metric
    fn compute_cyborg_distances(&self, target: Position) -> HashMap<Position, CyborgDistance> {
        let mut distances: HashMap<Position, CyborgDistance> = HashMap::new();
        let mut heap: BinaryHeap<DijkstraEntry> = BinaryHeap::new();

        heap.push(DijkstraEntry {
            dist: CyborgDistance::ZERO,
            pos: target,
        });

        let grid = self.grid.borrow();
        let bounds = grid.bounds();

        while let Some(DijkstraEntry { dist, pos }) = heap.pop() {
            match distances.entry(pos) {
                Entry::Vacant(vacant_entry) => vacant_entry.insert(dist), // Finalize this position
                Entry::Occupied(_) => continue,                           // Already finalized
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
                    | Cell::Player(_) => true,
                };

                if !traversable {
                    continue;
                }

                heap.push(DijkstraEntry {
                    dist: dist.add_step(dir),
                    pos: neighbor,
                });
            }
        }

        distances
    }

    pub(crate) fn move_cyborg_rats(&mut self, player: Position, player_facing: Dir4) {
        let distances = self.compute_cyborg_distances(player);
        let blocked_dir = player_facing.opposite();

        // Partition cyborg rats into reachable and unreachable
        let (reachable, unreachable): (Vec<_>, Vec<_>) = self
            .grid
            .borrow()
            .find_entities(|cell| matches!(cell, Cell::CyborgRat(_)))
            .map(|(pos, _)| pos)
            .partition(|pos| distances.contains_key(pos));

        // Unreachable cyborg rats just turn to face the player
        for cyborg_pos in unreachable {
            if let Some(face_dir) = Dir8::from_delta(player - cyborg_pos) {
                self.begin_move(Moving {
                    cell: Cell::CyborgRat(face_dir),
                    from: cyborg_pos,
                    progress: 1.0,
                    to: cyborg_pos,
                });
            }
        }

        // Sort reachable cyborg rats by distance (closest first), tiebreak by position
        let mut movable_cyborgs: Vec<_> = reachable
            .into_iter()
            .map(|pos| (distances[&pos], pos))
            .collect();
        movable_cyborgs.sort();

        for (current_dist, cyborg_pos) in movable_cyborgs {
            // Find best adjacent cell
            let mut best_move: Option<(Dir8, CyborgDistance)> = None;

            for dir in Dir8::all() {
                let new_pos = cyborg_pos + dir.delta();

                let Some(&target_dist) = distances.get(&new_pos) else {
                    continue;
                };

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
                if new_pos == player && dir == blocked_dir {
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
            } else if let Some(face_dir) = Dir8::from_delta(player - cyborg_pos) {
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
