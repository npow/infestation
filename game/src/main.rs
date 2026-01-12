use macroquad::window::next_frame;

use infestation::{game_app::App, sprites::Sprites};

#[macroquad::main("Infestation")]
async fn main() {
    let mut app = App::new(Sprites::load().await);
    while app.tick() {
        next_frame().await;
    }
}
