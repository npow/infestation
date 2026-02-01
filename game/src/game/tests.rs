use super::*;
use crate::grid::Grid;
use std::collections::HashSet;

fn game_from_csv(csv: &str) -> Game {
    Game::new(Grid::from_csv(csv), HashSet::new())
}

fn player_pos(game: &Game) -> Position {
    game.state.find_player().unwrap().0
}
#[test]
fn win_when_all_rats_dead() {
    let mut game = game_from_csv(".,.,.\n.,>,R\n.,.,.");
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.apply_action(Action::Move(Dir4::East));
    assert_eq!(game.state.play_state(), PlayState::Won);
}

#[test]
fn game_over_when_rat_reaches_player() {
    // Player blocked by wall, faces north after move attempt
    // Rat can attack from east (blocked_dir = South)
    let mut game = game_from_csv(".,#,.\n.,^,R\n.,.,.");
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.apply_action(Action::Move(Dir4::North)); // Player blocked, stays at (1,1) facing north
    // Rat at (2,1) moves west to (1,1), attacking player
    assert_eq!(game.state.play_state(), PlayState::GameOver);
}

#[test]
fn undo_restores_state() {
    let mut game = game_from_csv(".,.,.\n.,v,.\n.,.,.");
    let initial = player_pos(&game);
    game.apply_action(Action::Move(Dir4::East));
    assert_ne!(player_pos(&game), initial);
    game.undo();
    assert_eq!(player_pos(&game), initial);
}

#[test]
fn restart_resets_game() {
    let mut game = game_from_csv(".,.,.\n.,v,.\n.,.,.");
    let initial = player_pos(&game);
    game.apply_action(Action::Move(Dir4::East));
    game.apply_action(Action::Move(Dir4::South));
    game.restart();
    assert_eq!(player_pos(&game), initial);
    assert_eq!(game.state.history.len(), 1);
}
// Cyborg rat tests

fn cyborg_positions(game: &Game) -> Vec<Position> {
    game.state
        .grid
        .entries()
        .filter_map(|(pos, cell)| matches!(cell, Cell::CyborgRat(_)).then_some(pos))
        .collect()
}

#[test]
fn win_requires_all_cyborg_rats_dead() {
    // Level with only a cyborg rat
    let mut game = game_from_csv(".,.,.\n.,>,C\n.,.,.");
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.apply_action(Action::Move(Dir4::East)); // Player kills cyborg
    assert_eq!(game.state.play_state(), PlayState::Won);
}

#[test]
fn win_requires_both_rats_and_cyborgs_dead() {
    // Use walls to completely isolate cyborg, and put rat directly in front of player
    let mut game = game_from_csv(".,R,.,.,#,#,#\n.,^,.,.,#,C,#\n.,.,.,.,#,#,#");
    // Rat at (1,0), player at (1,1) facing north (toward rat), cyborg at (5,1) trapped
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.apply_action(Action::Move(Dir4::North)); // Kill the rat (player moves to 1,0)
    // Still playing because cyborg is alive (though trapped)
    assert_eq!(game.state.play_state(), PlayState::Playing);
    assert!(!cyborg_positions(&game).is_empty());
}
