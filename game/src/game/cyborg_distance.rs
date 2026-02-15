use std::{cmp::Ordering, ops::Sub};

use crate::{direction::Dir8, grid::Player};

/// Distance metric for cyborg rat pathfinding: A + B*sqrt(2)
/// Stored as (orthogonal_count, diagonal_count)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CyborgDistance(i32, i32);

impl CyborgDistance {
    pub(crate) const ZERO: CyborgDistance = CyborgDistance(0, 0);
    pub(crate) const ONE_ORTHO: CyborgDistance = CyborgDistance(1, 0);

    pub(crate) fn add_step(self, dir: Dir8) -> CyborgDistance {
        let CyborgDistance(ortho, diag) = self;
        if dir.is_diagonal() {
            CyborgDistance(ortho, diag + 1)
        } else {
            CyborgDistance(ortho + 1, diag)
        }
    }

    fn compare_with_zero(&self) -> Ordering {
        let CyborgDistance(ortho, diag) = *self;
        (ortho.pow(2) * ortho.signum() + 2 * diag.pow(2) * diag.signum()).cmp(&(0i32))
    }
}

impl Ord for CyborgDistance {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self - *other).compare_with_zero()
    }
}

impl Sub for CyborgDistance {
    type Output = CyborgDistance;

    fn sub(self, other: Self) -> Self::Output {
        let CyborgDistance(ortho1, diag1) = self;
        let CyborgDistance(ortho2, diag2) = other;
        CyborgDistance(ortho1 - ortho2, diag1 - diag2)
    }
}

impl PartialOrd for CyborgDistance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Entry for the priority queue in Dijkstra (min-heap via Reverse ordering)
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub(crate) struct CyborgEntry {
    pub(crate) dist: CyborgDistance,
    pub(crate) still: bool,
    pub(crate) player: Player,
}
