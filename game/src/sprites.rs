use crate::direction::{Dir4, Dir8};
use macroquad::prelude::*;

pub struct Sprites {
    player: [Texture2D; 4],
    rat: [Texture2D; 8],
    cyborg_rat: [Texture2D; 8],
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
            player: [
                load_png(include_bytes!("../../assets/player/north.png")),
                load_png(include_bytes!("../../assets/player/south.png")),
                load_png(include_bytes!("../../assets/player/east.png")),
                load_png(include_bytes!("../../assets/player/west.png")),
            ],
            rat: [
                load_png(include_bytes!("../../assets/rat/north.png")),
                load_png(include_bytes!("../../assets/rat/south.png")),
                load_png(include_bytes!("../../assets/rat/east.png")),
                load_png(include_bytes!("../../assets/rat/west.png")),
                load_png(include_bytes!("../../assets/rat/northeast.png")),
                load_png(include_bytes!("../../assets/rat/northwest.png")),
                load_png(include_bytes!("../../assets/rat/southeast.png")),
                load_png(include_bytes!("../../assets/rat/southwest.png")),
            ],
            cyborg_rat: [
                load_png(include_bytes!("../../assets/cyborgrat/north.png")),
                load_png(include_bytes!("../../assets/cyborgrat/south.png")),
                load_png(include_bytes!("../../assets/cyborgrat/east.png")),
                load_png(include_bytes!("../../assets/cyborgrat/west.png")),
                load_png(include_bytes!("../../assets/cyborgrat/northeast.png")),
                load_png(include_bytes!("../../assets/cyborgrat/northwest.png")),
                load_png(include_bytes!("../../assets/cyborgrat/southeast.png")),
                load_png(include_bytes!("../../assets/cyborgrat/southwest.png")),
            ],
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
        &self.player[dir as usize]
    }

    pub(crate) fn rat(&self, dir: Dir8) -> &Texture2D {
        &self.rat[dir as usize]
    }

    pub(crate) fn cyborg_rat(&self, dir: Dir8) -> &Texture2D {
        &self.cyborg_rat[dir as usize]
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
