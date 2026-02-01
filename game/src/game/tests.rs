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
fn undo_restores_state() {
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    let initial = player_pos(&game);
    game.apply_action(Action::Move(Dir4::East));
    assert_ne!(player_pos(&game), initial);
    game.undo();
    assert_eq!(player_pos(&game), initial);
}

#[test]
fn restart_resets_game() {
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    let initial = player_pos(&game);
    game.apply_action(Action::Move(Dir4::East));
    game.apply_action(Action::Move(Dir4::South));
    game.restart();
    assert_eq!(player_pos(&game), initial);
    assert_eq!(game.state.history.len(), 1);
}
