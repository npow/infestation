use enum_map::{EnumMap, enum_map};

use crate::direction::{Dir4, Dir8};
use macroquad::prelude::*;

#[derive(Clone)]
pub struct Sprites {
    player: EnumMap<Dir4, Texture2D>,
    player2: EnumMap<Dir4, Texture2D>,
    rat: EnumMap<Dir8, Texture2D>,
    cyborg_rat: EnumMap<Dir8, Texture2D>,
    wall: Texture2D,
    portal_unvisited: Texture2D,
    portal_visited: Texture2D,
    note: Texture2D,
    planks: Texture2D,
    spiderweb: Texture2D,
    blackhole: Texture2D,
    explosive: Texture2D,
    explosion: Texture2D,
    zap: Texture2D,
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
            player: enum_map! {
                Dir4::North => load_png(include_bytes!("../../assets/player/north.png")),
                Dir4::South => load_png(include_bytes!("../../assets/player/south.png")),
                Dir4::East => load_png(include_bytes!("../../assets/player/east.png")),
                Dir4::West => load_png(include_bytes!("../../assets/player/west.png")),
            },
            player2: enum_map! {
                Dir4::North => load_png(include_bytes!("../../assets/player2/north.png")),
                Dir4::South => load_png(include_bytes!("../../assets/player2/south.png")),
                Dir4::East => load_png(include_bytes!("../../assets/player2/east.png")),
                Dir4::West => load_png(include_bytes!("../../assets/player2/west.png")),
            },
            rat: enum_map! {
                Dir8::North => load_png(include_bytes!("../../assets/rat/north.png")),
                Dir8::South => load_png(include_bytes!("../../assets/rat/south.png")),
                Dir8::East => load_png(include_bytes!("../../assets/rat/east.png")),
                Dir8::West => load_png(include_bytes!("../../assets/rat/west.png")),
                Dir8::Northeast => load_png(include_bytes!("../../assets/rat/northeast.png")),
                Dir8::Northwest => load_png(include_bytes!("../../assets/rat/northwest.png")),
                Dir8::Southeast => load_png(include_bytes!("../../assets/rat/southeast.png")),
                Dir8::Southwest => load_png(include_bytes!("../../assets/rat/southwest.png")),
            },
            cyborg_rat: enum_map! {
                Dir8::North => load_png(include_bytes!("../../assets/cyborgrat/north.png")),
                Dir8::South => load_png(include_bytes!("../../assets/cyborgrat/south.png")),
                Dir8::East => load_png(include_bytes!("../../assets/cyborgrat/east.png")),
                Dir8::West => load_png(include_bytes!("../../assets/cyborgrat/west.png")),
                Dir8::Northeast => load_png(include_bytes!("../../assets/cyborgrat/northeast.png")),
                Dir8::Northwest => load_png(include_bytes!("../../assets/cyborgrat/northwest.png")),
                Dir8::Southeast => load_png(include_bytes!("../../assets/cyborgrat/southeast.png")),
                Dir8::Southwest => load_png(include_bytes!("../../assets/cyborgrat/southwest.png")),
            },
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

    pub(crate) fn player(&self, dir: Dir4) -> &Texture2D {
        &self.player[dir]
    }

    pub(crate) fn player2(&self, dir: Dir4) -> &Texture2D {
        &self.player2[dir]
    }

    pub(crate) fn rat(&self, dir: Dir8) -> &Texture2D {
        &self.rat[dir]
    }

    pub(crate) fn cyborg_rat(&self, dir: Dir8) -> &Texture2D {
        &self.cyborg_rat[dir]
    }

    pub(crate) fn wall(&self) -> &Texture2D {
        &self.wall
    }

    pub(crate) fn portal(&self, visited: bool) -> &Texture2D {
        if visited {
            &self.portal_visited
        } else {
            &self.portal_unvisited
        }
    }

    pub(crate) fn note(&self) -> &Texture2D {
        &self.note
    }

    pub(crate) fn planks(&self) -> &Texture2D {
        &self.planks
    }

    pub(crate) fn spiderweb(&self) -> &Texture2D {
        &self.spiderweb
    }

    pub(crate) fn blackhole(&self) -> &Texture2D {
        &self.blackhole
    }

    pub(crate) fn explosive(&self) -> &Texture2D {
        &self.explosive
    }

    pub(crate) fn explosion(&self) -> &Texture2D {
        &self.explosion
    }

    pub(crate) fn zap(&self) -> &Texture2D {
        &self.zap
    }

    pub(crate) fn font(&self) -> &Font {
        &self.font
    }
}
