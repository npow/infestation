use std::borrow::BorrowMut;

use crate::direction::Dir8;
use crate::grid::{Cell, Grid};

use super::{Exploding, MoveHandler};

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    pub(crate) fn start_explosion_wave(&mut self) {
        // Move pending explosions to active exploding
        for explosion in &self.pending_explosions {
            // Clear the center of the explosion immediately
            *self.grid.borrow_mut().at_mut(*explosion) = Cell::Empty;
        }
        self.exploding = self
            .pending_explosions
            .drain(..)
            .map(|pos| Exploding { pos, progress: 0.0 })
            .collect();
    }

    pub(crate) fn finish_explosion_wave(&mut self) {
        let grid = self.grid.borrow_mut();
        // For each explosion, check the 3x3 area for chain reactions and casualties
        let exploding: Vec<_> = self.exploding.drain(..).collect();
        for explosion in exploding {
            let center = explosion.pos;

            // Check all 8 neighbors + center for chain reactions and casualties
            for dir in Dir8::all() {
                let pos = center + dir.delta();
                let cell = grid.at(pos);

                match cell {
                    Cell::Explosive => {
                        // Chain reaction - add to pending if not already
                        if !self.pending_explosions.contains(&pos) {
                            self.pending_explosions.push(pos);
                        }
                    }
                    Cell::Rat(_)
                    | Cell::CyborgRat(_)
                    | Cell::Player(_)
                    | Cell::Spiderweb
                    | Cell::Plank => {
                        // Entity destroyed in explosion
                        *grid.at_mut(pos) = Cell::Empty;
                    }
                    Cell::Empty | Cell::BlackHole | Cell::Wall | Cell::Trigger(_) => {}
                }
            }
        }
        // Play state is computed from the grid (no player = game over, no rats = won)
    }
}
