use crate::position::PositionDelta;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Dir4 {
    North,
    South,
    East,
    West,
}

impl Dir4 {
    pub(crate) fn delta(self) -> PositionDelta {
        match self {
            Dir4::North => PositionDelta::new(0, -1),
            Dir4::South => PositionDelta::new(0, 1),
            Dir4::East => PositionDelta::new(1, 0),
            Dir4::West => PositionDelta::new(-1, 0),
        }
    }

    pub(crate) fn opposite(self) -> Dir8 {
        match self {
            Dir4::North => Dir8::South,
            Dir4::South => Dir8::North,
            Dir4::East => Dir8::West,
            Dir4::West => Dir8::East,
        }
    }

    pub(crate) fn rotate_cw(self) -> Dir4 {
        match self {
            Dir4::North => Dir4::East,
            Dir4::East => Dir4::South,
            Dir4::South => Dir4::West,
            Dir4::West => Dir4::North,
        }
    }

    pub(crate) fn rotate_ccw(self) -> Dir4 {
        match self {
            Dir4::North => Dir4::West,
            Dir4::West => Dir4::South,
            Dir4::South => Dir4::East,
            Dir4::East => Dir4::North,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Dir8 {
    North,
    South,
    East,
    West,
    Northeast,
    Northwest,
    Southeast,
    Southwest,
}

impl Dir8 {
    pub(crate) fn delta(self) -> PositionDelta {
        match self {
            Self::Northwest => PositionDelta::new(-1, -1),
            Self::North => PositionDelta::new(0, -1),
            Self::Northeast => PositionDelta::new(1, -1),
            Self::West => PositionDelta::new(-1, 0),
            Self::East => PositionDelta::new(1, 0),
            Self::Southwest => PositionDelta::new(-1, 1),
            Self::South => PositionDelta::new(0, 1),
            Self::Southeast => PositionDelta::new(1, 1),
        }
    }

    pub(crate) fn is_diagonal(self) -> bool {
        match self {
            Self::Northeast | Self::Northwest | Self::Southeast | Self::Southwest => true,
            Self::North | Self::South | Self::East | Self::West => false,
        }
    }

    pub(crate) fn all() -> [Self; 8] {
        [
            Self::Northwest,
            Self::North,
            Self::Northeast,
            Self::West,
            Self::East,
            Self::Southwest,
            Self::South,
            Self::Southeast,
        ]
    }

    /// Convert delta values to a Dir8.
    /// Returns None if both dx and dy are 0.
    pub(crate) fn from_delta(delta: PositionDelta) -> Option<Self> {
        use std::cmp::Ordering::*;

        let PositionDelta { dx, dy } = delta;
        match (dx.cmp(&0), dy.cmp(&0)) {
            (Less, Less) => Some(Self::Northwest),
            (Equal, Less) => Some(Self::North),
            (Greater, Less) => Some(Self::Northeast),
            (Less, Equal) => Some(Self::West),
            (Equal, Equal) => None,
            (Greater, Equal) => Some(Self::East),
            (Less, Greater) => Some(Self::Southwest),
            (Equal, Greater) => Some(Self::South),
            (Greater, Greater) => Some(Self::Southeast),
        }
    }

    pub(crate) fn x_only(self) -> Option<Self> {
        let mut delta = self.delta();
        delta.dy = 0;
        Self::from_delta(delta)
    }

    pub(crate) fn y_only(self) -> Option<Self> {
        let mut delta = self.delta();
        delta.dx = 0;
        Self::from_delta(delta)
    }
}
