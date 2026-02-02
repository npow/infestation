#![cfg(test)]
use infestation::testing::{
    Action, Dir4, PlayState, apply_actions, game_from_csv, grid_to_csv, play_state,
};

fn parse_action(s: &str) -> Action {
    match s {
        "north" => Action::Move(Dir4::North),
        "south" => Action::Move(Dir4::South),
        "east" => Action::Move(Dir4::East),
        "west" => Action::Move(Dir4::West),
        "stall" => Action::Stall,
        _ => panic!("Unknown action: {}", s),
    }
}

fn parse_optional_action(value: &serde_json::Value) -> Option<Action> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(parse_action(s)),
        _ => panic!("Expected string or null for action"),
    }
}

fn state_to_str(state: PlayState) -> &'static str {
    match state {
        PlayState::Playing => "playing",
        PlayState::GameOver => "gameover",
        PlayState::Won => "won",
    }
}

fn run_scenario_test(before: &str, after: &str, after_path: &str, json_path: &str) {
    let json_content = std::fs::read_to_string(json_path).expect("Failed to read JSON");
    let mut json: serde_json::Value =
        serde_json::from_str(&json_content).expect("Failed to parse JSON");

    let mut game = game_from_csv(before);

    // Check for two-player format (p1/p2) vs single-player format (move)
    if json.get("p1").is_some() || json.get("p2").is_some() {
        // Two-player format
        let p1_action = parse_optional_action(json.get("p1").unwrap_or(&serde_json::Value::Null));
        let p2_action = parse_optional_action(json.get("p2").unwrap_or(&serde_json::Value::Null));
        apply_actions(&mut game, &[p1_action, p2_action]);
    } else {
        // Single-player format
        let action_str = json["move"].as_str().expect("Missing 'move' field");
        let action = parse_action(action_str);
        game.apply_action(action);
    }

    let result = grid_to_csv(&game);
    let actual_state = play_state(&game);

    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        std::fs::write(after_path, &result).expect("Failed to update snapshot");
        json["state"] = serde_json::Value::String(state_to_str(actual_state).to_string());
        let formatted = serde_json::to_string_pretty(&json).expect("Failed to serialize");
        std::fs::write(json_path, formatted + "\n").expect("Failed to write JSON");
    } else {
        assert_eq!(result.trim(), after.trim());
        let expected = json["state"].as_str().expect("Missing 'state' field");
        assert_eq!(state_to_str(actual_state), expected, "play_state mismatch");
    }
}

include!(concat!(env!("OUT_DIR"), "/scenario_tests.rs"));
