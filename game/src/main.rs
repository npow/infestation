use macroquad::window::next_frame;
use quad_url::{easy_parse, get_program_parameters};

use infestation::{game_app::App, sprites::Sprites};

/// Accepts a level name as a bare CLI argument (native) or a `level=<name>`
/// URL query parameter (web, surfaced by quad-url as `--level=<name>`).
#[macroquad::main("Infestation")]
async fn main() {
    let mut level_name: Option<String> = None;
    for param in get_program_parameters().iter().skip(1) {
        let name = match easy_parse(param) {
            Some(("level", Some(value))) => value,
            None => param.as_str(),
            _ => panic!("Unrecognized argument: {}", param),
        };
        if level_name.replace(name.to_string()).is_some() {
            panic!("Multiple levels specified");
        }
    }
    let mut app = App::new(Sprites::load().await, level_name.as_deref());
    while app.tick() {
        next_frame().await;
    }
}
