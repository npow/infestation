#![cfg(test)]
use infestation::testing::{ScenarioInput, apply_actions, game_from_csv, grid_to_csv, play_state};

fn run_scenario_test(before: &str, after: &str, after_path: &str, json_path: &str) {
    let json_content = std::fs::read_to_string(json_path).expect("Failed to read JSON");
    let input: ScenarioInput =
        serde_json::from_str(&json_content).expect("Failed to parse scenario JSON");

    let mut game = game_from_csv(before);

    let expected_state = match &input {
        ScenarioInput::TwoPlayer { p1, p2, state } => {
            apply_actions(&mut game, &[*p1, *p2]);
            *state
        }
        ScenarioInput::SinglePlayer { action, state } => {
            game.apply_action(*action);
            *state
        }
    };

    let result = grid_to_csv(&game);
    let actual_state = play_state(&game);

    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        std::fs::write(after_path, &result).expect("Failed to update snapshot");
        let updated = match input {
            ScenarioInput::TwoPlayer { p1, p2, .. } => ScenarioInput::TwoPlayer {
                p1,
                p2,
                state: actual_state,
            },
            ScenarioInput::SinglePlayer { action, .. } => ScenarioInput::SinglePlayer {
                action,
                state: actual_state,
            },
        };
        let formatted =
            serde_json::to_string_pretty(&updated).expect("Failed to serialize scenario");
        std::fs::write(json_path, formatted + "\n").expect("Failed to write JSON");
    } else {
        assert_eq!(result.trim(), after.trim());
        assert_eq!(actual_state, expected_state, "play_state mismatch");
    }
}

include!(concat!(env!("OUT_DIR"), "/scenario_tests.rs"));
