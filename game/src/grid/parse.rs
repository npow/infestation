use std::collections::HashMap;

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

use crate::direction::Dir4;
use crate::position::Position;

use crate::render::InputHints;

use super::{Cell, Grid, Player};

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum NoteText {
    Simple(String),
    ByPlatform {
        controller: String,
        keyboard: String,
        mobile: String,
    },
    TwoPlayer {
        keyboard_only: String,
        one_controller: String,
        two_controllers: String,
        mobile: String,
    },
}

impl NoteText {
    pub(crate) fn resolve(&self, hints: InputHints, gamepad_count: usize) -> &str {
        match self {
            NoteText::Simple(s) => s,
            NoteText::ByPlatform {
                controller,
                keyboard,
                mobile,
            } => match hints {
                InputHints::Controller(_) => controller,
                InputHints::Touch => mobile,
                InputHints::Keyboard => keyboard,
            },
            NoteText::TwoPlayer {
                keyboard_only,
                one_controller,
                two_controllers,
                mobile,
            } => {
                if matches!(hints, InputHints::Touch) {
                    mobile
                } else if gamepad_count >= 2 {
                    two_controllers
                } else if gamepad_count == 1 {
                    one_controller
                } else {
                    keyboard_only
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct LevelMetadata {
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    portals: Vec<Portal>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<Note>,
}

#[derive(Serialize, Deserialize)]
struct Portal {
    x: i32,
    y: i32,
    level: String,
}

#[derive(Serialize, Deserialize)]
struct Note {
    x: i32,
    y: i32,
    text: NoteText,
}

impl LevelMetadata {
    pub(crate) fn parse(json_str: &str) -> Self {
        serde_json::from_str(json_str).expect("invalid JSON")
    }

    pub(crate) fn from_grid(
        name: &str,
        portals: &HashMap<Position, String>,
        notes: &HashMap<Position, NoteText>,
    ) -> Self {
        let mut portals: Vec<_> = portals
            .iter()
            .map(|(pos, level)| Portal {
                x: pos.x,
                y: pos.y,
                level: level.clone(),
            })
            .collect();
        portals.sort_by_key(|p| (p.y, p.x));

        let mut notes: Vec<_> = notes
            .iter()
            .map(|(pos, text)| Note {
                x: pos.x,
                y: pos.y,
                text: text.clone(),
            })
            .collect();
        notes.sort_by_key(|n| (n.y, n.x));

        Self {
            name: name.to_string(),
            portals,
            notes,
        }
    }

    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Failed to serialize JSON")
    }

    pub(crate) fn portals(&self) -> HashMap<Position, String> {
        self.portals
            .iter()
            .map(|p| (Position::new(p.x as usize, p.y as usize), p.level.clone()))
            .collect()
    }

    pub(crate) fn notes(&self) -> HashMap<Position, NoteText> {
        self.notes
            .iter()
            .map(|n| (Position::new(n.x as usize, n.y as usize), n.text.clone()))
            .collect()
    }
}

impl Grid {
    pub fn from_csv(csv_str: &str) -> Self {
        Self::parse_csv(csv_str, HashMap::new(), HashMap::new())
    }

    pub(crate) fn from_csv_and_metadata(csv_str: &str, metadata: &LevelMetadata) -> Self {
        Self::parse_csv(csv_str, metadata.portals(), metadata.notes())
    }

    fn parse_csv(
        csv_str: &str,
        portals: HashMap<Position, String>,
        notes: HashMap<Position, NoteText>,
    ) -> Self {
        let mut cells: Vec<Vec<Cell>> = Vec::new();
        let mut player_pos: Option<Position> = None;
        let mut player2_pos: Option<Position> = None;
        let mut rat_positions: Vec<Position> = Vec::new();
        let mut cyborg_rat_positions: Vec<Position> = Vec::new();

        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(false)
            .from_reader(csv_str.as_bytes());

        for (y, result) in reader.records().enumerate() {
            let record = result.expect("invalid CSV");
            let mut row = Vec::new();
            for (x, field) in record.iter().enumerate() {
                let pos = Position::new(x, y);
                let cell = match field.trim() {
                    "▲" => {
                        player_pos = Some(pos);
                        Cell::Player(Player::Player1, Dir4::North)
                    }
                    "▼" => {
                        player_pos = Some(pos);
                        Cell::Player(Player::Player1, Dir4::South)
                    }
                    "►" => {
                        player_pos = Some(pos);
                        Cell::Player(Player::Player1, Dir4::East)
                    }
                    "◄" => {
                        player_pos = Some(pos);
                        Cell::Player(Player::Player1, Dir4::West)
                    }
                    "△" => {
                        player2_pos = Some(pos);
                        Cell::Player(Player::Player2, Dir4::North)
                    }
                    "▽" => {
                        player2_pos = Some(pos);
                        Cell::Player(Player::Player2, Dir4::South)
                    }
                    "▷" => {
                        player2_pos = Some(pos);
                        Cell::Player(Player::Player2, Dir4::East)
                    }
                    "◁" => {
                        player2_pos = Some(pos);
                        Cell::Player(Player::Player2, Dir4::West)
                    }
                    "#" => Cell::Wall,
                    "=" => Cell::Plank,
                    "w" => Cell::Spiderweb,
                    "O" => Cell::BlackHole,
                    "X" => Cell::Explosive,
                    "1" => Cell::Trigger(1),
                    "2" => Cell::Trigger(2),
                    "3" => Cell::Trigger(3),
                    "4" => Cell::Trigger(4),
                    "5" => Cell::Trigger(5),
                    "6" => Cell::Trigger(6),
                    "7" => Cell::Trigger(7),
                    "8" => Cell::Trigger(8),
                    "9" => Cell::Trigger(9),
                    "R" => {
                        rat_positions.push(pos);
                        Cell::Empty
                    }
                    "C" => {
                        cyborg_rat_positions.push(pos);
                        Cell::Empty
                    }
                    _ => Cell::Empty,
                };
                row.push(cell);
            }
            cells.push(row);
        }

        let mut grid = Grid::new(cells, portals, notes);
        // Use player1 position if available, otherwise player2
        let target = player_pos.or(player2_pos).unwrap();
        for rat in rat_positions {
            let dir = rat.direction_to(target);
            *grid.at_mut(rat) = Cell::Rat(dir);
        }
        for cyborg in cyborg_rat_positions {
            let dir = cyborg.direction_to(target);
            *grid.at_mut(cyborg) = Cell::CyborgRat(dir);
        }
        grid
    }
}
