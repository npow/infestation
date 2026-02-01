#![cfg(test)]
use infestation::testing::{Action, Dir4, game_from_csv, grid_to_csv};

macro_rules! scenario_test {
    ($name:ident, $before:expr, $after:expr, $action:expr) => {
        #[test]
        fn $name() {
            let mut game = game_from_csv($before);
            game.apply_action($action);
            let result = grid_to_csv(&game);
            assert_eq!(result.trim(), $after.trim());
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/scenario_tests.rs"));
