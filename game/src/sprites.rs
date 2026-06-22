use enum_map::{EnumMap, enum_map};

use crate::direction::{Dir4, Dir8};
use macroquad::prelude::*;

/// An enum map of direction → texture loaded from `assets/<folder>/<dir>.png`.
macro_rules! dir_sprite {
    ($folder:literal, $file:literal) => {{
        let bytes: &[u8] = include_bytes!(concat!("../../assets/", $folder, "/", $file, ".png"));
        load_png(bytes)
    }};
}

macro_rules! dir4_sprites {
    ($folder:literal) => {
        enum_map! {
            Dir4::North => dir_sprite!($folder, "north"),
            Dir4::South => dir_sprite!($folder, "south"),
            Dir4::East => dir_sprite!($folder, "east"),
            Dir4::West => dir_sprite!($folder, "west"),
        }
    };
}

macro_rules! dir8_sprites {
    ($folder:literal) => {
        enum_map! {
            Dir8::North => dir_sprite!($folder, "north"),
            Dir8::South => dir_sprite!($folder, "south"),
            Dir8::East => dir_sprite!($folder, "east"),
            Dir8::West => dir_sprite!($folder, "west"),
            Dir8::Northeast => dir_sprite!($folder, "northeast"),
            Dir8::Northwest => dir_sprite!($folder, "northwest"),
            Dir8::Southeast => dir_sprite!($folder, "southeast"),
            Dir8::Southwest => dir_sprite!($folder, "southwest"),
        }
    };
}

#[derive(Clone)]
pub struct Sprites {
    pub(crate) player: EnumMap<Dir4, Texture2D>,
    pub(crate) player2: EnumMap<Dir4, Texture2D>,
    pub(crate) rat: EnumMap<Dir8, Texture2D>,
    pub(crate) cyborg_rat: EnumMap<Dir8, Texture2D>,
    pub(crate) wall: Texture2D,
    portal_unvisited: Texture2D,
    portal_visited: Texture2D,
    pub(crate) note: Texture2D,
    pub(crate) planks: Texture2D,
    pub(crate) spiderweb: Texture2D,
    pub(crate) blackhole: Texture2D,
    pub(crate) explosive: Texture2D,
    pub(crate) explosion: Texture2D,
    pub(crate) zap: Texture2D,
    font: Font,
}

fn load_png(data: &[u8]) -> Texture2D {
    Texture2D::from_file_with_format(data, Some(ImageFormat::Png))
}

async fn load_font() -> Font {
    let path = "assets/DejaVuSans.ttf";
    load_ttf_font(path)
        .await
        .unwrap_or_else(|e| panic!("Failed to load {path}: {e:?}"))
}

impl Sprites {
    pub async fn load() -> Self {
        Self {
            player: dir4_sprites!("player"),
            player2: dir4_sprites!("player2"),
            rat: dir8_sprites!("rat"),
            cyborg_rat: dir8_sprites!("cyborgrat"),
            wall: load_png(include_bytes!("../../assets/wall.png")),
            portal_unvisited: load_png(include_bytes!("../../assets/portal/unvisited.png")),
            portal_visited: load_png(include_bytes!("../../assets/portal/visited.png")),
            note: load_png(include_bytes!("../../assets/note.png")),
            planks: load_png(include_bytes!("../../assets/planks.png")),
            spiderweb: load_png(include_bytes!("../../assets/spiderweb.png")),
            blackhole: load_png(include_bytes!("../../assets/blackhole.png")),
            explosive: load_png(include_bytes!("../../assets/explosive.png")),
            explosion: load_png(include_bytes!("../../assets/explosion.png")),
            zap: load_png(include_bytes!("../../assets/zap.png")),
            font: load_font().await,
        }
    }

    pub(crate) fn portal(&self, visited: bool) -> &Texture2D {
        if visited {
            &self.portal_visited
        } else {
            &self.portal_unvisited
        }
    }

    pub(crate) fn font(&self) -> &Font {
        &self.font
    }
}
