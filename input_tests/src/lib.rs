#![cfg(test)]

use std::collections::HashSet;

use enum_map::EnumMap;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use infestation::game::PlayState;
use infestation::input::{FrameInput, FrameOutput, GameContext, InputProcessor};

// --- Test-side serde types ---

#[derive(Deserialize)]
struct TestInput {
    context: GameContext,
    #[serde(default)]
    context_changes: Vec<ContextChange>,
    #[serde(default = "default_dt")]
    dt: f32,
    events: Vec<KeyEvent>,
}

fn default_dt() -> f32 {
    1.0 / 60.0
}

/// Partial context override applied at a specific time.
#[derive(Deserialize)]
struct ContextChange {
    t: f32,
    #[serde(default)]
    player_count: Option<usize>,
    #[serde(default)]
    is_animating: Option<bool>,
    #[serde(default)]
    play_state: Option<PlayState>,
    #[serde(default)]
    can_undo: Option<bool>,
    #[serde(default)]
    can_exit: Option<bool>,
    #[serde(default)]
    dialog_active: Option<bool>,
    #[serde(default)]
    overlay_active: Option<bool>,
}

fn apply_context_change(ctx: &mut GameContext, change: &ContextChange) {
    if let Some(v) = change.player_count {
        ctx.player_count = v;
    }
    if let Some(v) = change.is_animating {
        ctx.is_animating = v;
    }
    if let Some(v) = change.play_state {
        ctx.play_state = v;
    }
    if let Some(v) = change.can_undo {
        ctx.can_undo = v;
    }
    if let Some(v) = change.can_exit {
        ctx.can_exit = v;
    }
    if let Some(v) = change.dialog_active {
        ctx.dialog_active = v;
    }
    if let Some(v) = change.overlay_active {
        ctx.overlay_active = v;
    }
}

#[derive(Deserialize)]
struct KeyEvent {
    t: f32,
    key: String,
    action: KeyAction,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum KeyAction {
    Press,
    Release,
}

// --- Golden output types ---

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TimedOutput {
    frame: usize,
    output: FrameOutput,
}

// --- KeyCode parsing ---

fn parse_keycode(s: &str) -> KeyCode {
    match s {
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "W" => KeyCode::W,
        "A" => KeyCode::A,
        "S" => KeyCode::S,
        "D" => KeyCode::D,
        "Space" => KeyCode::Space,
        "Escape" => KeyCode::Escape,
        "Backspace" => KeyCode::Backspace,
        "Enter" => KeyCode::Enter,
        "Tab" => KeyCode::Tab,
        "LeftControl" => KeyCode::LeftControl,
        "RightControl" => KeyCode::RightControl,
        "LeftShift" => KeyCode::LeftShift,
        "RightShift" => KeyCode::RightShift,
        "R" => KeyCode::R,
        "U" => KeyCode::U,
        "Z" => KeyCode::Z,
        _ => panic!("Unknown key name in test: {s:?}"),
    }
}

// --- Frame synthesis + test runner ---

fn run_input_test(input_json: &str, expected_json: &str, output_path: &str) {
    let test_input: TestInput =
        serde_json::from_str(input_json).expect("Failed to parse input JSON");
    let expected: Vec<TimedOutput> =
        serde_json::from_str(expected_json).expect("Failed to parse expected output JSON");

    let mut ctx = test_input.context;
    let dt = test_input.dt;

    // Determine frame count from the latest timestamp across events and context changes
    let last_event_t = test_input
        .events
        .iter()
        .map(|e| e.t)
        .fold(0.0_f32, f32::max);
    let last_ctx_t = test_input
        .context_changes
        .iter()
        .map(|c| c.t)
        .fold(0.0_f32, f32::max);
    let last_t = last_event_t.max(last_ctx_t);
    let frame_count = ((last_t / dt).ceil() as usize) + 2;

    // Sort events and context changes by time
    let mut events = test_input.events;
    events.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());

    let mut context_changes = test_input.context_changes;
    context_changes.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());

    let mut processor = InputProcessor::default();
    let mut keys_down: HashSet<KeyCode> = HashSet::new();
    let mut actual_outputs: Vec<TimedOutput> = Vec::new();
    let mut event_idx = 0;
    let mut ctx_change_idx = 0;

    for frame in 0..frame_count {
        let frame_end = (frame + 1) as f32 * dt;

        // Apply context changes that fall before this frame's end
        while ctx_change_idx < context_changes.len()
            && context_changes[ctx_change_idx].t < frame_end
        {
            apply_context_change(&mut ctx, &context_changes[ctx_change_idx]);
            ctx_change_idx += 1;
        }

        let mut keys_pressed = HashSet::new();
        let mut keys_released = Vec::new();

        // Process all key events that fall before this frame's end
        while event_idx < events.len() && events[event_idx].t < frame_end {
            let event = &events[event_idx];
            let keycode = parse_keycode(&event.key);
            match event.action {
                KeyAction::Press => {
                    keys_pressed.insert(keycode);
                    keys_down.insert(keycode);
                }
                KeyAction::Release => {
                    keys_released.push(keycode);
                }
            }
            event_idx += 1;
        }

        let frame_input = FrameInput {
            keys_pressed,
            keys_down: keys_down.clone(),
            gamepads: EnumMap::default(),
            touches: vec![],
            mouse_click: None,
            dt,
        };

        let output = processor.process(&frame_input, &ctx);
        if output != FrameOutput::None {
            actual_outputs.push(TimedOutput { frame, output });
        }

        // Apply releases after the frame so keys held during this frame
        // remain in keys_down for the FrameInput above
        for key in keys_released {
            keys_down.remove(&key);
        }
    }

    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        let formatted =
            serde_json::to_string_pretty(&actual_outputs).expect("Failed to serialize outputs");
        std::fs::write(output_path, formatted + "\n").expect("Failed to write output snapshot");
    } else {
        assert_eq!(
            actual_outputs,
            expected,
            "Output mismatch.\nActual:\n{}\nExpected:\n{}",
            serde_json::to_string_pretty(&actual_outputs).unwrap(),
            serde_json::to_string_pretty(&expected).unwrap(),
        );
    }
}

include!(concat!(env!("OUT_DIR"), "/input_tests.rs"));
