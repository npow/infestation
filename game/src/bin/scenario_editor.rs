use std::env;

use macroquad::prelude::next_frame;

use infestation::editor_app::App;
use infestation::sprites::Sprites;

#[macroquad::main("Scenario Editor")]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let scenario_filter: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();

    let mut app = App::new_scenario(Sprites::load().await, &scenario_filter);
    while app.tick() {
        next_frame().await;
    }
}
