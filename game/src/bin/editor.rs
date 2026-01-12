use std::env;

use macroquad::prelude::next_frame;

use infestation::editor_app::App;
use infestation::sprites::Sprites;

#[macroquad::main("Level Editor")]
async fn main() {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 2, "Usage: editor <level_name>");
    let level_name = args.get(1).unwrap();

    let mut app = App::new(Sprites::load().await, level_name);
    while app.tick() {
        next_frame().await;
    }
}
