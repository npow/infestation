use std::collections::HashMap;

use crate::direction::{Dir4, Dir8};
use crate::game::PlayState;
use crate::position::Position;

mod parse;
pub(crate) use parse::LevelMetadata;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Cell {
    Empty,
    Wall,
    Player(Dir4),
    Rat(Dir8),
    CyborgRat(Dir8),
    Plank,
    Spiderweb,
    BlackHole,
    Explosive,
    Trigger(u8),
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
}

#[derive(Clone)]
pub(crate) struct Grid {
    cells: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    portals: HashMap<Position, String>,
    notes: HashMap<Position, String>,
}

impl Grid {
    pub(crate) fn new(
        cells: Vec<Vec<Cell>>,
        portals: HashMap<Position, String>,
        notes: HashMap<Position, String>,
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

    pub(crate) fn to_csv(&self) -> String {
        let mut lines = Vec::new();
        for y in 0..self.height {
            let mut row = Vec::new();
            for x in 0..self.width {
                let cell_str = match self.cells[y][x] {
                    Cell::Player(Dir4::North) => "^".to_string(),
                    Cell::Player(Dir4::South) => "v".to_string(),
                    Cell::Player(Dir4::East) => ">".to_string(),
                    Cell::Player(Dir4::West) => "<".to_string(),
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

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
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

    pub(crate) fn get_note(&self, pos: Position) -> Option<&str> {
        self.notes.get(&pos).map(String::as_str)
    }

    pub(crate) fn notes(&self) -> impl Iterator<Item = (Position, &str)> {
        self.notes.iter().map(|(&pos, text)| (pos, text.as_str()))
    }

    pub(crate) fn insert_note(&mut self, pos: Position, text: String) {
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

    pub(crate) fn play_state(&self) -> PlayState {
        let has_player = self
            .find_entities(|cell| matches!(cell, Cell::Player(_)))
            .next()
            .is_some();
        if !has_player {
            return PlayState::GameOver;
        }
        let has_rats = self
            .find_entities(|cell| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)))
            .next()
            .is_some();
        if has_rats {
            PlayState::Playing
        } else {
            PlayState::Won
        }
    }

    pub(crate) fn to_json(&self, level_name: &str) -> String {
        LevelMetadata::from_grid(level_name, &self.portals, &self.notes).to_json()
    }
}
