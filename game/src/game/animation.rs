use crate::grid::Cell;

use super::{Game, MOVE_SPEED, MoveHandler};

impl MoveHandler {
    /// Resolve all pending animations immediately.
    ///
    /// This uses the same `animate()` function as visual playback, but with
    /// `f32::INFINITY` as the time delta. This causes all progress values to
    /// instantly reach 1.0, completing each animation phase in a single call.
    ///
    /// This approach ensures instant resolution and animated playback use
    /// identical logic - there's only one code path for game state transitions,
    /// eliminating bugs where the two could diverge.
    pub(crate) fn resolve_all(&mut self) {
        let mut dt = f32::INFINITY;
        let completed = self.advance_animation(&mut dt);
        assert!(completed, "resolve_all should complete all animations");
    }

    /// Advance animation by dt seconds. Returns true if animation is complete.
    pub(crate) fn advance_animation(&mut self, dt: &mut f32) -> bool {
        // First, handle movement animations
        if !self.moving.is_empty() {
            let mut all_done = true;
            let mut max_advancement: f32 = 0.0;
            for m in &mut self.moving {
                max_advancement = max_advancement.max(1.0 - m.progress);
                m.progress = (m.progress + *dt * MOVE_SPEED).min(1.0);
                if m.progress < 1.0 {
                    all_done = false;
                }
            }

            if all_done {
                self.finish_moving();
                *dt -= max_advancement / MOVE_SPEED;
            } else {
                return false;
            }
        }

        // If we have pending zaps but no moves are animating, start them
        if !self.triggered_numbers.is_empty() {
            self.start_zap_wave();
        }

        // Then, handle zap animations
        if !self.zapping.is_empty() {
            let mut all_done = true;
            let mut max_advancement: f32 = 0.0;
            for z in &mut self.zapping {
                max_advancement = max_advancement.max(1.0 - z.progress);
                z.progress = (z.progress + *dt * MOVE_SPEED).min(1.0);
                if z.progress < 1.0 {
                    all_done = false;
                }
            }

            if all_done {
                self.finish_zap_wave();
                *dt -= max_advancement / MOVE_SPEED;
            } else {
                return false;
            }
        }

        while !self.pending_explosions.is_empty() || !self.exploding.is_empty() {
            // If we have pending explosions but nothing is animating, start the wave
            if self.exploding.is_empty() {
                self.start_explosion_wave();
            }

            // Then, handle explosion animations
            let mut all_done = true;
            let mut max_advancement: f32 = 0.0;
            for e in &mut self.exploding {
                max_advancement = max_advancement.max(1.0 - e.progress);
                e.progress = (e.progress + *dt * MOVE_SPEED).min(1.0);
                if e.progress < 1.0 {
                    all_done = false;
                }
            }

            if all_done {
                self.finish_explosion_wave();
                *dt -= max_advancement / MOVE_SPEED;
            } else {
                return false;
            }
        }

        // Nothing left to animate
        true
    }

    fn finish_moving(&mut self) {
        // Update grid: place entities at destination (if they survived)
        for m in self.moving.drain(..) {
            match self.grid.at(m.to) {
                Cell::BlackHole => {
                    // Black holes swallow entities - don't modify the cell
                    continue;
                }
                Cell::Explosive => {
                    // Moving onto an explosive triggers it
                    if !self.pending_explosions.contains(&m.to) {
                        self.pending_explosions.push(m.to);
                    }
                }
                Cell::Trigger(n) => {
                    // Moving onto a trigger activates it
                    if !self.triggered_numbers.contains(&n) {
                        self.triggered_numbers.push(n);
                    }
                }
                _ => {}
            }
            // Place entity (overwrites whatever was there)
            *self.grid.at_mut(m.to) = m.cell;
        }
    }
}

impl Game {
    pub(crate) fn animate(&mut self, mut dt: f32) {
        while let Some(ref mut handler) = self.animation {
            let done = handler.advance_animation(&mut dt);
            if !done {
                break;
            }
            // Animation complete - check portal before processing queued move
            self.animation = None;
            if self.state.portal_destination().is_some() {
                // Player just stepped on uncompleted portal - drop queued move
                // to allow portal transition to happen
                self.state.queued_move = None;
            } else if let Some(dir) = self.state.queued_move.take() {
                self.begin_action(dir);
            }
        }
    }
}
