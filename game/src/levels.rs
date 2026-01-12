use std::sync::LazyLock;

use crate::grid::{Grid, LevelMetadata};

include!(concat!(env!("OUT_DIR"), "/levels.rs"));

struct TextLevel {
    name: &'static str,
    csv: &'static str,
    json: &'static str,
}

pub(crate) struct Level {
    pub(crate) name: &'static str,
    pub(crate) display_name: String,
    pub(crate) grid: Grid,
}

impl Level {
    fn parse(text: &TextLevel) -> Self {
        let metadata = LevelMetadata::parse(text.json);
        let display_name = metadata.name.clone();
        Self {
            name: text.name,
            display_name,
            grid: Grid::from_csv_and_metadata(text.csv, &metadata),
        }
    }
}

static LEVELS: LazyLock<Vec<Level>> = LazyLock::new(|| {
    LEVEL_DATA
        .iter()
        .map(|(name, csv, json)| Level::parse(&TextLevel { name, csv, json }))
        .collect()
});

pub(crate) fn get_level(name: &str) -> Option<&'static Level> {
    LEVELS.iter().find(|l| l.name == name)
}
