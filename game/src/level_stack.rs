use std::mem;

use crate::game::{Game, PlayState};

/// Manages the stack of game states when navigating between levels via portals.
pub(crate) struct LevelStack {
    stack: Vec<(Game, String)>,
    pub(crate) current_level: String,
}

impl LevelStack {
    pub(crate) fn new(initial_level: String) -> Self {
        Self {
            stack: Vec::new(),
            current_level: initial_level,
        }
    }

    pub(crate) fn can_exit(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Push current state onto stack before entering a new level.
    pub(crate) fn enter_level(&mut self, game: &Game, new_level: String) {
        self.stack.push((
            game.clone(),
            mem::replace(&mut self.current_level, new_level),
        ));
    }

    /// Exit current level and return to parent. Returns the restored game state.
    ///
    /// - If current level was won, marks it as completed.
    /// - If current level was not completed and was previously unvisited, undoes the portal step.
    pub(crate) fn exit_level(&mut self, current_game: &Game) -> Option<Game> {
        let (mut saved, prev_level) = self.stack.pop()?;

        // Transfer completed_levels from current game to saved
        saved.state.completed_levels = current_game.state.completed_levels.clone();

        if current_game.state.play_state() == PlayState::Won {
            // Level completed - mark as visited
            saved.state.mark_level_completed(&self.current_level);
        } else if !saved.is_level_completed(&self.current_level) {
            // Level was unvisited and not completed - undo the portal step
            saved.undo();
        }

        self.current_level = prev_level;

        Some(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direction::Dir4;
    use crate::game::Action;
    use crate::grid::{Cell, Grid};
    use crate::position::Position;
    use std::collections::HashSet;

    fn game_from_csv(csv: &str) -> Game {
        Game::new(Grid::from_csv(csv), HashSet::new())
    }

    fn player_pos(game: &Game) -> Position {
        game.state
            .grid
            .entries()
            .find_map(|(pos, cell)| matches!(cell, Cell::Player(_)).then_some(pos))
            .unwrap()
    }

    fn game_with_portal_at(portal_pos: Position, portal_target: &str) -> Game {
        let mut grid = Grid::create_empty(3, 3);
        *grid.at_mut(Position::new(0, 1)) = Cell::Player(Dir4::East);
        grid.insert_portal(portal_pos, portal_target.to_string());
        Game::new(grid, HashSet::new())
    }

    #[test]
    fn exit_uncompleted_unvisited_level_undoes_portal_step() {
        // Set up parent level with player at (0,1), portal at (1,1)
        let mut parent_game = game_with_portal_at(Position::new(1, 1), "sublevel");
        let initial_pos = player_pos(&parent_game);

        // Player moves onto portal
        parent_game.apply_action(Action::Move(Dir4::East));
        assert_eq!(player_pos(&parent_game), Position::new(1, 1));

        // Create level stack and enter sublevel
        let mut stack = LevelStack::new("world".to_string());
        stack.enter_level(&parent_game, "sublevel".to_string());

        // Create sublevel game (not completed - still playing)
        let sublevel_game = game_from_csv(".,.,.\n.,v,R\n.,.,.");
        assert_eq!(sublevel_game.state.play_state(), PlayState::Playing);

        // Exit without completing
        let restored = stack.exit_level(&sublevel_game).unwrap();

        // Player should be back at initial position (portal step undone)
        assert_eq!(player_pos(&restored), initial_pos);
    }

    #[test]
    fn exit_completed_level_does_not_undo() {
        // Set up parent level with player at (0,1), portal at (1,1)
        let mut parent_game = game_with_portal_at(Position::new(1, 1), "sublevel");

        // Player moves onto portal
        parent_game.apply_action(Action::Move(Dir4::East));
        let pos_on_portal = player_pos(&parent_game);

        // Create level stack and enter sublevel
        let mut stack = LevelStack::new("world".to_string());
        stack.enter_level(&parent_game, "sublevel".to_string());

        // Create sublevel game that's been won
        let mut sublevel_game = game_from_csv(".,.,.\n.,>,R\n.,.,.");
        sublevel_game.apply_action(Action::Move(Dir4::East)); // Kill the rat
        assert_eq!(sublevel_game.state.play_state(), PlayState::Won);

        // Exit after completing
        let restored = stack.exit_level(&sublevel_game).unwrap();

        // Player should still be on the portal (no undo)
        assert_eq!(player_pos(&restored), pos_on_portal);
        // Level should be marked completed
        assert!(restored.state.completed_levels.contains("sublevel"));
    }

    #[test]
    fn exit_previously_visited_level_does_not_undo() {
        // Set up parent level with player at (0,1), portal at (1,1)
        let mut parent_game = game_with_portal_at(Position::new(1, 1), "sublevel");
        // Mark sublevel as already completed
        parent_game
            .state
            .completed_levels
            .insert("sublevel".to_string());

        // Player moves onto portal
        parent_game.apply_action(Action::Move(Dir4::East));
        let pos_on_portal = player_pos(&parent_game);

        // Create level stack and enter sublevel
        let mut stack = LevelStack::new("world".to_string());
        stack.enter_level(&parent_game, "sublevel".to_string());

        // Create sublevel game with inherited completed_levels (simulates how main.rs works)
        let sublevel_game = Game::new(
            Grid::from_csv(".,.,.\n.,v,R\n.,.,.\n"),
            parent_game.state.completed_levels.clone(),
        );
        assert_eq!(sublevel_game.state.play_state(), PlayState::Playing);

        // Exit without completing (but level was previously visited)
        let restored = stack.exit_level(&sublevel_game).unwrap();

        // Player should still be on the portal (no undo - level was previously visited)
        assert_eq!(player_pos(&restored), pos_on_portal);
    }
}
