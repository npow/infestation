use std::ops::{Add, Sub};

use crate::direction::Dir8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Position {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl Position {
    pub(crate) fn new(x: usize, y: usize) -> Self {
        Self {
            x: x.try_into().unwrap(),
            y: y.try_into().unwrap(),
        }
    }

    pub(crate) fn dist_sq(self, to: Position) -> i32 {
        (to - self).magnitude_sq()
    }

    pub(crate) fn in_bounds(self, bounds: (usize, usize)) -> bool {
        let (width, height) = bounds;
        let x_in_bounds = self.x >= 0 && (self.x as usize) < width;
        let y_in_bounds = self.y >= 0 && (self.y as usize) < height;
        x_in_bounds && y_in_bounds
    }

    pub(crate) fn direction_to(self, to: Position) -> Dir8 {
        let mut best_dir = Dir8::South;
        let mut best_dist = i32::MAX;
        for dir in Dir8::all() {
            let new_pos = self + dir.delta();
            let d = new_pos.dist_sq(to);
            if d < best_dist {
                best_dist = d;
                best_dir = dir;
            }
        }
        best_dir
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct PositionDelta {
    pub(crate) dx: i32,
    pub(crate) dy: i32,
}

impl PositionDelta {
    pub(crate) fn new(dx: i32, dy: i32) -> Self {
        Self { dx, dy }
    }

    pub(crate) fn magnitude_sq(self) -> i32 {
        self.dx * self.dx + self.dy * self.dy
    }
}

impl Add<PositionDelta> for Position {
    type Output = Position;

    fn add(self, delta: PositionDelta) -> Position {
        let x = self.x + delta.dx;
        let y = self.y + delta.dy;
        Position { x, y }
    }
}

impl Sub for Position {
    type Output = PositionDelta;

    fn sub(self, other: Position) -> PositionDelta {
        PositionDelta {
            dx: self.x - other.x,
            dy: self.y - other.y,
        }
    }
}
