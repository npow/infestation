use std::collections::HashMap;

use enum_map::Enum;

use crate::direction::{Dir4, Dir8};
use crate::position::Position;

mod parse;
pub(crate) use parse::{LevelMetadata, NoteText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Enum)]
pub(crate) enum Player {
    Player1,
    Player2,
}

pub(crate) struct FoundPlayer {
    pub(crate) pos: Position,
    pub(crate) dir: Dir4,
    pub(crate) player: Player,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Cell {
    Empty,
    Wall,
    Player(Player, Dir4),
    Rat(Dir8),
    CyborgRat(Dir8),
    Plank,
    Spiderweb,
    BlackHole,
    Explosive,
    Trigger(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    Empty,
    Wall,
    Player,
    Rat,
    CyborgRat,
    Plank,
    Spiderweb,
    BlackHole,
    Explosive,
    Trigger(u8),
}

impl CellKind {
    fn from_cell(cell: Cell) -> Self {
        match cell {
            Cell::Empty => Self::Empty,
            Cell::Wall => Self::Wall,
            Cell::Player(..) => Self::Player,
            Cell::Rat(_) => Self::Rat,
            Cell::CyborgRat(_) => Self::CyborgRat,
            Cell::Plank => Self::Plank,
            Cell::Spiderweb => Self::Spiderweb,
            Cell::BlackHole => Self::BlackHole,
            Cell::Explosive => Self::Explosive,
            Cell::Trigger(n) => Self::Trigger(n),
        }
    }
}

impl Cell {
    pub(crate) fn blocks_player(&self) -> bool {
        matches!(self, Cell::Wall | Cell::Plank)
    }

    pub(crate) fn blocks_rat(&self) -> bool {
        matches!(
            self,
            Cell::Wall | Cell::Rat(_) | Cell::CyborgRat(_) | Cell::Spiderweb
        )
    }

    // Not blocked by rats, only other cyborg rats
    pub(crate) fn blocks_cyborg_rat(&self) -> bool {
        matches!(self, Cell::Wall | Cell::CyborgRat(_) | Cell::Spiderweb)
    }

    /// Returns the player identity and direction if this is a player cell.
    pub(crate) fn as_player(&self) -> Option<(Player, Dir4)> {
        match self {
            Cell::Player(player, dir) => Some((*player, *dir)),
            _ => None,
        }
    }

    /// Creates a player cell for the given player and direction.
    pub(crate) fn player(player: Player, dir: Dir4) -> Cell {
        Cell::Player(player, dir)
    }
}

#[derive(Clone)]
pub struct Grid {
    cells: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    portals: HashMap<Position, String>,
    notes: HashMap<Position, NoteText>,
}

impl Grid {
    pub(crate) fn new(
        cells: Vec<Vec<Cell>>,
        portals: HashMap<Position, String>,
        notes: HashMap<Position, NoteText>,
    ) -> Self {
        let height = cells.len();
        let width = cells.first().map(|r| r.len()).unwrap();
        for row in &cells {
            assert_eq!(row.len(), width);
        }
        Self {
            cells,
            width,
            height,
            portals,
            notes,
        }
    }

    pub(crate) fn create_empty(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::Empty; width]; height];
        Self {
            cells,
            width,
            height,
            portals: HashMap::new(),
            notes: HashMap::new(),
        }
    }

    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();
        for y in 0..self.height {
            let mut row = Vec::new();
            for x in 0..self.width {
                let cell_str = match self.cells[y][x] {
                    Cell::Player(Player::Player1, Dir4::North) => "▲".to_string(),
                    Cell::Player(Player::Player1, Dir4::South) => "▼".to_string(),
                    Cell::Player(Player::Player1, Dir4::East) => "►".to_string(),
                    Cell::Player(Player::Player1, Dir4::West) => "◄".to_string(),
                    Cell::Player(Player::Player2, Dir4::North) => "△".to_string(),
                    Cell::Player(Player::Player2, Dir4::South) => "▽".to_string(),
                    Cell::Player(Player::Player2, Dir4::East) => "▷".to_string(),
                    Cell::Player(Player::Player2, Dir4::West) => "◁".to_string(),
                    Cell::Wall => "#".to_string(),
                    Cell::Rat(_) => "R".to_string(),
                    Cell::CyborgRat(_) => "C".to_string(),
                    Cell::Plank => "=".to_string(),
                    Cell::Spiderweb => "w".to_string(),
                    Cell::BlackHole => "O".to_string(),
                    Cell::Explosive => "X".to_string(),
                    Cell::Trigger(n) => n.to_string(),
                    Cell::Empty => ".".to_string(),
                };
                row.push(cell_str);
            }
            lines.push(row.join(",") + "\n");
        }
        lines.join("")
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell_kind_at(&self, x: usize, y: usize) -> CellKind {
        if x >= self.width || y >= self.height {
            return CellKind::Wall;
        }
        CellKind::from_cell(self.cells[y][x])
    }

    pub fn state_hash(&self) -> u64 {
        fn mix_byte(hash: &mut u64, byte: u8) {
            *hash ^= byte as u64;
            *hash = hash.wrapping_mul(0x100000001b3);
        }

        fn mix_usize(hash: &mut u64, value: usize) {
            for byte in value.to_le_bytes() {
                mix_byte(hash, byte);
            }
        }

        fn mix_cell(hash: &mut u64, cell: Cell) {
            match cell {
                Cell::Empty => mix_byte(hash, 0),
                Cell::Wall => mix_byte(hash, 1),
                Cell::Player(player, dir) => {
                    mix_byte(hash, 2);
                    mix_byte(hash, player as u8);
                    mix_byte(hash, dir as u8);
                }
                Cell::Rat(dir) => {
                    mix_byte(hash, 3);
                    mix_byte(hash, dir as u8);
                }
                Cell::CyborgRat(dir) => {
                    mix_byte(hash, 4);
                    mix_byte(hash, dir as u8);
                }
                Cell::Plank => mix_byte(hash, 5),
                Cell::Spiderweb => mix_byte(hash, 6),
                Cell::BlackHole => mix_byte(hash, 7),
                Cell::Explosive => mix_byte(hash, 8),
                Cell::Trigger(n) => {
                    mix_byte(hash, 9);
                    mix_byte(hash, n);
                }
            }
        }

        let mut hash: u64 = 0xcbf29ce484222325;
        mix_usize(&mut hash, self.width);
        mix_usize(&mut hash, self.height);
        for row in &self.cells {
            for &cell in row {
                mix_cell(&mut hash, cell);
            }
        }
        hash
    }

    pub fn player_positions(&self) -> Vec<(usize, usize)> {
        self.find_players()
            .into_iter()
            .map(|player| (player.pos.x as usize, player.pos.y as usize))
            .collect()
    }

    pub(crate) fn at(&self, pos: Position) -> Cell {
        if pos.in_bounds(self.bounds()) {
            self.cells[pos.y as usize][pos.x as usize]
        } else {
            Cell::Wall
        }
    }

    pub(crate) fn at_mut(&mut self, pos: Position) -> &mut Cell {
        &mut self.cells[pos.y as usize][pos.x as usize]
    }

    /// Find all players in the grid, sorted by player identity.
    pub(crate) fn find_players(&self) -> Vec<FoundPlayer> {
        let mut players: Vec<_> = self
            .entries()
            .filter_map(|(pos, cell)| {
                cell.as_player()
                    .map(|(player, dir)| FoundPlayer { pos, dir, player })
            })
            .collect();
        players.sort_by_key(|p| p.player);
        players
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (Position, Cell)> {
        self.cells.iter().enumerate().flat_map(move |(y, row)| {
            row.iter()
                .enumerate()
                .map(move |(x, &cell)| (Position::new(x, y), cell))
        })
    }

    pub(crate) fn bounds(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub(crate) fn resize(&mut self, new_width: usize, new_height: usize) {
        // Adjust height
        if new_height > self.height {
            // Add rows at the bottom
            for _ in self.height..new_height {
                self.cells.push(vec![Cell::Empty; self.width]);
            }
        } else if new_height < self.height {
            self.cells.truncate(new_height);
        }
        self.height = new_height;

        // Adjust width
        if new_width > self.width {
            // Add columns to the right
            for row in &mut self.cells {
                row.resize(new_width, Cell::Empty);
            }
        } else if new_width < self.width {
            for row in &mut self.cells {
                row.truncate(new_width);
            }
        }
        self.width = new_width;

        // Remove portals and notes outside new bounds
        self.portals
            .retain(|pos, _| (pos.x as usize) < new_width && (pos.y as usize) < new_height);
        self.notes
            .retain(|pos, _| (pos.x as usize) < new_width && (pos.y as usize) < new_height);
    }

    pub(crate) fn get_portal(&self, player_pos: Position) -> Option<&str> {
        self.portals.get(&player_pos).map(String::as_str)
    }

    pub(crate) fn portals(&self) -> impl Iterator<Item = (Position, &str)> {
        self.portals
            .iter()
            .map(|(&pos, level)| (pos, level.as_str()))
    }

    pub(crate) fn insert_portal(&mut self, pos: Position, level: String) {
        self.portals.insert(pos, level);
    }

    pub(crate) fn remove_portal(&mut self, pos: Position) {
        self.portals.remove(&pos);
    }

    pub(crate) fn get_note(&self, pos: Position) -> Option<&NoteText> {
        self.notes.get(&pos)
    }

    pub(crate) fn notes(&self) -> impl Iterator<Item = (Position, &NoteText)> {
        self.notes.iter().map(|(&pos, text)| (pos, text))
    }

    pub(crate) fn insert_note(&mut self, pos: Position, text: NoteText) {
        self.notes.insert(pos, text);
    }

    pub(crate) fn remove_note(&mut self, pos: Position) {
        self.notes.remove(&pos);
    }

    pub(crate) fn find_entities<F: FnMut(Cell) -> bool>(
        &self,
        mut f: F,
    ) -> impl Iterator<Item = (Position, Cell)> + use<'_, F> {
        self.entries().filter(move |&(_, cell)| f(cell))
    }

    pub(crate) fn to_json(&self, level_name: &str) -> String {
        LevelMetadata::from_grid(level_name, &self.portals, &self.notes).to_json()
    }
}
