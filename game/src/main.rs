use macroquad::window::next_frame;

use infestation::{game_app::App, sprites::Sprites};

#[macroquad::main("Infestation")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let level_name = args.next();
    if let Some(arg) = args.next() {
        panic!("Unrecognized argument: {}", arg);
    }
    let mut app = App::new(Sprites::load().await, level_name.as_deref());
    while app.tick() {
        next_frame().await;
    }
}
