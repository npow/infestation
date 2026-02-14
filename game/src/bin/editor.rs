use std::env;

use macroquad::prelude::next_frame;

use infestation::editor_app::App;
use infestation::sprites::Sprites;

#[macroquad::main("Level Editor")]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let sprites = Sprites::load().await;

    let mut app = if args.get(1).is_some_and(|a| a == "--solution") {
        let encoded = args
            .get(2)
            .expect("Usage: editor --solution <base64_string>");
        let decoded = infestation::solution::decode_solution(encoded);
        App::new_solution(sprites, decoded)
    } else {
        assert_eq!(args.len(), 2, "Usage: editor <level_name>");
        App::new_level(sprites, &args[1])
    };

    while app.tick() {
        next_frame().await;
    }
}
