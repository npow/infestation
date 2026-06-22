use std::collections::VecDeque;

use crate::direction::Dir4;
use crate::grid::Player;
use crate::position::{Position, PositionDelta};

/// A path being dragged out on the grid, starting at a player's cell.
pub(crate) struct PathDrag {
    pub(crate) player: Player,
    /// Cells the player will visit, starting with the cell the drag began on.
    pub(crate) cells: Vec<Position>,
}

impl PathDrag {
    pub(crate) fn new(player: Player, start: Position) -> Self {
        Self {
            player,
            cells: vec![start],
        }
    }

    /// Extend the path one unit step at a time toward `target`, never
    /// entering a blocked cell: a step prefers the dominant axis, sidesteps
    /// along the other axis when that's blocked, and the path stops growing
    /// when both are. Stepping back onto any already-drawn cell erases the
    /// path back to that cell, so retracing the path (or finger jitter on a
    /// cell boundary) self-corrects.
    pub(crate) fn extend_to(&mut self, target: Position, blocked: impl Fn(Position) -> bool) {
        loop {
            let last = *self.cells.last().unwrap();
            let delta = target - last;
            let x_step = PositionDelta::new(delta.dx.signum(), 0);
            let y_step = PositionDelta::new(0, delta.dy.signum());
            let steps = if delta.dx.abs() >= delta.dy.abs() {
                [x_step, y_step]
            } else {
                [y_step, x_step]
            };
            let Some(next) = steps
                .into_iter()
                .filter(|&step| step != PositionDelta::new(0, 0))
                .map(|step| last + step)
                .find(|&next| !blocked(next))
            else {
                break;
            };
            if let Some(index) = self.cells.iter().position(|&cell| cell == next) {
                self.cells.truncate(index + 1);
            } else {
                self.cells.push(next);
            }
        }
    }

    /// Convert into a follower over the waypoints after the start cell.
    /// Returns None if the drag never left its starting cell.
    pub(crate) fn into_follower(self) -> Option<PathFollower> {
        (self.cells.len() > 1).then(|| PathFollower {
            waypoints: self.cells.into_iter().skip(1).collect(),
            attempted: false,
        })
    }
}

/// Follows a dragged-out path, one move per turn.
pub(crate) struct PathFollower {
    waypoints: VecDeque<Position>,
    /// Whether a move toward the front waypoint has already been issued.
    attempted: bool,
}

/// What a path follower wants a player standing at a given position to do.
#[derive(Debug, PartialEq)]
pub(crate) enum NextMove {
    Move(Dir4),
    /// All waypoints have been reached.
    Finished,
    /// The player isn't where the path expects it (a move was blocked),
    /// so the path should be abandoned.
    OffPath,
}

impl PathFollower {
    /// The next move for a player at `pos`. Call once per turn; an issued
    /// move is verified to have landed before the next one is returned.
    pub(crate) fn next_move(&mut self, pos: Position) -> NextMove {
        if self.attempted {
            if self.waypoints.front() == Some(&pos) {
                self.waypoints.pop_front();
                self.attempted = false;
            } else {
                return NextMove::OffPath;
            }
        }
        let Some(&next) = self.waypoints.front() else {
            return NextMove::Finished;
        };
        let Some(dir) = Dir4::from_delta(next - pos) else {
            return NextMove::OffPath;
        };
        self.attempted = true;
        NextMove::Move(dir)
    }

    /// The cells yet to be visited, prefixed with the player's current
    /// position (for rendering).
    pub(crate) fn preview(&self, pos: Position) -> Vec<Position> {
        std::iter::once(pos)
            .chain(self.waypoints.iter().copied())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag_from(x: usize, y: usize) -> PathDrag {
        PathDrag::new(Player::Player1, Position::new(x, y))
    }

    fn positions(coords: &[(usize, usize)]) -> Vec<Position> {
        coords.iter().map(|&(x, y)| Position::new(x, y)).collect()
    }

    fn open(_: Position) -> bool {
        false
    }

    #[test]
    fn extend_interpolates_skipped_cells() {
        let mut drag = drag_from(0, 0);
        drag.extend_to(Position::new(2, 1), open);
        assert_eq!(drag.cells, positions(&[(0, 0), (1, 0), (2, 0), (2, 1)]));
    }

    #[test]
    fn retracing_erases_steps() {
        let mut drag = drag_from(0, 0);
        drag.extend_to(Position::new(2, 0), open);
        drag.extend_to(Position::new(1, 0), open);
        assert_eq!(drag.cells, positions(&[(0, 0), (1, 0)]));
        drag.extend_to(Position::new(0, 0), open);
        assert_eq!(drag.cells, positions(&[(0, 0)]));
        assert!(drag.into_follower().is_none());
    }

    #[test]
    fn loops_around_older_cells_regress_to_that_cell() {
        let mut drag = drag_from(0, 0);
        for &(x, y) in &[(1, 0), (1, 1), (0, 1), (0, 0)] {
            drag.extend_to(Position::new(x, y), open);
        }
        assert_eq!(drag.cells, positions(&[(0, 0)]));
    }

    #[test]
    fn winding_back_to_middle_discards_later_steps() {
        let mut drag = drag_from(0, 0);
        for &(x, y) in &[(1, 0), (2, 0), (2, 1), (1, 1), (1, 0)] {
            drag.extend_to(Position::new(x, y), open);
        }
        assert_eq!(drag.cells, positions(&[(0, 0), (1, 0)]));
    }

    #[test]
    fn blocked_cells_are_never_entered() {
        let wall = Position::new(1, 0);
        let mut drag = drag_from(0, 0);
        // Dragging straight onto (or past) a wall doesn't extend the path.
        drag.extend_to(Position::new(2, 0), |p| p == wall);
        assert_eq!(drag.cells, positions(&[(0, 0)]));
        // A diagonal target sidesteps along the open axis instead.
        drag.extend_to(Position::new(2, 1), |p| p == wall);
        assert!(!drag.cells.contains(&wall));
        assert_eq!(*drag.cells.last().unwrap(), Position::new(2, 1));
    }

    #[test]
    fn fully_blocked_path_stops_growing() {
        let mut drag = drag_from(0, 0);
        // Everything except the start cell is blocked.
        drag.extend_to(Position::new(3, 3), |p| p != Position::new(0, 0));
        assert_eq!(drag.cells, positions(&[(0, 0)]));
    }

    #[test]
    fn follower_walks_the_path() {
        let mut drag = drag_from(0, 0);
        drag.extend_to(Position::new(1, 0), open);
        drag.extend_to(Position::new(1, 1), open);
        let mut follower = drag.into_follower().unwrap();

        assert_eq!(
            follower.next_move(Position::new(0, 0)),
            NextMove::Move(Dir4::East)
        );
        assert_eq!(
            follower.next_move(Position::new(1, 0)),
            NextMove::Move(Dir4::South)
        );
        assert_eq!(follower.next_move(Position::new(1, 1)), NextMove::Finished);
    }

    #[test]
    fn follower_abandons_when_a_move_does_not_land() {
        let mut drag = drag_from(0, 0);
        drag.extend_to(Position::new(1, 0), open);
        let mut follower = drag.into_follower().unwrap();

        assert_eq!(
            follower.next_move(Position::new(0, 0)),
            NextMove::Move(Dir4::East)
        );
        // The move was blocked: the player is still at the start.
        assert_eq!(follower.next_move(Position::new(0, 0)), NextMove::OffPath);
    }
}
