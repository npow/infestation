use crate::{direction::Dir8, grid::Cell};

use super::{MoveHandler, Zapping};

impl MoveHandler {
    pub(crate) fn start_zap_wave(&mut self) {
        let numbers: Vec<u8> = self.triggered_numbers.drain(..).collect();

        // Find all remaining triggers in the grid that match the triggered numbers
        let mut zap_positions = Vec::new();
        for (pos, cell) in self.grid.entries() {
            if let Cell::Trigger(n) = cell
                && numbers.contains(&n)
            {
                zap_positions.push(pos);
            }
        }

        // Replace triggers with walls and collect neighbors
        for &pos in &zap_positions {
            // Turn trigger into wall
            *self.grid.at_mut(pos) = Cell::Wall;
        }

        self.zapping = zap_positions
            .into_iter()
            .map(|pos| Zapping { pos, progress: 0.0 })
            .collect();
    }

    /// Complete zap wave immediately (apply pending walls, clear animation state).
    pub(crate) fn finish_zap_wave(&mut self) {
        for Zapping { pos, .. } in self.zapping.drain(..) {
            // Check 8-way neighbors
            for dir in Dir8::all() {
                let neighbor = pos + dir.delta();
                match self.grid.at(neighbor) {
                    Cell::Empty => *self.grid.at_mut(neighbor) = Cell::Wall,
                    Cell::Explosive => {
                        if !self.pending_explosions.contains(&neighbor) {
                            self.pending_explosions.push(neighbor);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
