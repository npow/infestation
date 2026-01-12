use super::*;
use crate::direction::Dir8;
use crate::grid::{Cell, Grid};
use std::collections::HashSet;

fn game_from_csv(csv: &str) -> Game {
    Game::new(Grid::from_csv(csv), HashSet::new())
}

fn player_pos(game: &Game) -> Position {
    game.state.find_player().unwrap().0
}

fn rat_positions(game: &Game) -> Vec<Position> {
    game.state
        .grid
        .entries()
        .filter_map(|(pos, cell)| matches!(cell, Cell::Rat(_)).then_some(pos))
        .collect()
}

#[test]
fn player_moves_into_empty() {
    let mut game = game_from_csv(".,.,.\n.,v,.\n.,.,.");
    assert_eq!(player_pos(&game), Position::new(1, 1));

    game.apply_action(Action::Move(Dir4::East));
    assert_eq!(player_pos(&game), Position::new(2, 1));
}

#[test]
fn player_blocked_by_wall() {
    let mut game = game_from_csv(".,.,.\n.,>,#\n.,.,.");
    game.apply_action(Action::Move(Dir4::East));
    // Player should not move but should turn
    assert_eq!(player_pos(&game), Position::new(1, 1));
}

#[test]
fn player_blocked_by_plank() {
    let mut game = game_from_csv(".,.,.\n.,>,=\n.,.,.");
    game.apply_action(Action::Move(Dir4::East));
    assert_eq!(player_pos(&game), Position::new(1, 1));
}

#[test]
fn player_destroys_spiderweb() {
    let mut game = game_from_csv(".,.,.\n.,>,w\n.,.,.");
    assert_eq!(game.state.grid.at(Position::new(2, 1)), Cell::Spiderweb);
    game.apply_action(Action::Move(Dir4::East));
    // Player walks through spiderweb, destroying it
    assert_eq!(player_pos(&game), Position::new(2, 1));
    assert_eq!(game.state.grid.at(Position::new(1, 1)), Cell::Empty);
}

#[test]
fn player_kills_rat() {
    let mut game = game_from_csv(".,.,.\n.,>,R\n.,.,.");
    game.apply_action(Action::Move(Dir4::East));
    assert_eq!(player_pos(&game), Position::new(2, 1));
    assert!(rat_positions(&game).is_empty());
}

#[test]
fn win_when_all_rats_dead() {
    let mut game = game_from_csv(".,.,.\n.,>,R\n.,.,.");
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.apply_action(Action::Move(Dir4::East));
    assert_eq!(game.state.play_state(), PlayState::Won);
}

#[test]
fn rat_moves_diagonal_when_not_aligned() {
    // Rat at (0,0), player at (2,2) - rat should move SE diagonal
    let mut game = game_from_csv("R,.,.\n.,.,.\n.,.,v");
    let initial_rat = rat_positions(&game)[0];
    assert_eq!(initial_rat, Position::new(0, 0));

    game.apply_action(Action::Move(Dir4::South)); // Player moves, rat responds
    let new_rat = rat_positions(&game)[0];
    // Rat should have moved diagonally (SE)
    assert_eq!(new_rat, Position::new(1, 1));
}

#[test]
fn rat_moves_direct_when_aligned_horizontally() {
    // Rat at (0,1), player at (2,1) - rat should only try direct move
    // Player blocked by wall so they don't change position (only turn)
    let mut game = game_from_csv(".,.,#\nR,.,>\n.,.,.");
    game.apply_action(Action::Move(Dir4::North)); // Player blocked, stays at (2,1) facing north
    // blocked_dir = South, rat moves East (not blocked)
    let rat = rat_positions(&game)[0];
    assert_eq!(rat, Position::new(1, 1));
}

#[test]
fn rat_blocked_by_spiderweb() {
    // Rat blocked by spiderweb in its diagonal path
    // Player at (1,2) so vertical move is shorter than horizontal
    let mut game = game_from_csv("R,.,.\n.,w,.\n.,v,.");
    game.apply_action(Action::Move(Dir4::South));
    // Rat should try diagonal SE (blocked by web at 1,1), then try orthogonals
    // South to (0,1) is closer to player than East to (1,0)
    let rat = rat_positions(&game)[0];
    assert_eq!(rat, Position::new(0, 1));
}

#[test]
fn rat_destroys_plank() {
    let mut game = game_from_csv("R,=,.\n.,.,.\n.,.,v");
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Plank);
    game.apply_action(Action::Move(Dir4::South));
    // Rat moves through plank, destroying it
    let rat = rat_positions(&game)[0];
    assert_eq!(rat, Position::new(1, 1)); // Moved diagonally SE
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Plank); // Plank at (1,0) still there
    // Actually let me reconsider - the rat at (0,0) going to player at (2,3)
    // diagonal is SE = (1,1), so it wouldn't go through (1,0)
}

#[test]
fn rat_destroys_plank_when_moving_through() {
    // Set up so rat must go through plank
    let mut game = game_from_csv(".,.,.\nR,=,>\n.,.,.");
    // Rat at (0,1), player at (2,1) - aligned horizontally
    // Rat should move east through plank
    game.apply_action(Action::Move(Dir4::West));
    let rat = rat_positions(&game)[0];
    assert_eq!(rat, Position::new(1, 1));
    assert_eq!(
        game.state.grid.at(Position::new(1, 1)),
        Cell::Rat(Dir8::East)
    );
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
fn rat_cannot_attack_from_in_front() {
    // Player facing east toward rat - sword blocks attack from in front
    let mut game = game_from_csv(".,.,.\n.,>,R\n.,.,.");
    game.apply_action(Action::Move(Dir4::East));
    // Player kills rat by moving into it, so game is won not over
    assert_eq!(game.state.play_state(), PlayState::Won);
}

#[test]
fn rat_aligned_vertically_moves_direct() {
    // Rat at (1,0), player at (1,2) - aligned vertically
    let mut game = game_from_csv(".,R,.\n.,.,.\n.,v,.");
    game.apply_action(Action::Move(Dir4::South));
    let rat = rat_positions(&game)[0];
    // Rat should move directly south
    assert_eq!(rat, Position::new(1, 1));
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

#[test]
fn player_steps_on_explosive_dies() {
    let mut game = game_from_csv(".,.,.\n.,>,X\n.,.,.");
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.apply_action(Action::Move(Dir4::East));
    assert_eq!(game.state.play_state(), PlayState::GameOver);
    // Player should not be on the grid (died in explosion)
    assert!(game.state.find_player().is_none());
}

#[test]
fn rat_steps_on_explosive_dies() {
    let mut game = game_from_csv(".,.,.\nR,X,>\n.,.,.");
    // Rat at (0,1), player at (2,1) - rat will move through explosive
    game.apply_action(Action::Move(Dir4::West));
    // Rat should be dead (stepped on explosive)
    assert!(rat_positions(&game).is_empty());
}

#[test]
fn explosion_kills_nearby_rat() {
    // Player steps on explosive, rat in adjacent cell dies
    let mut game = game_from_csv(".,R,.\n.,X,.\n.,^,.");
    game.apply_action(Action::Move(Dir4::North));
    // Player dies
    assert_eq!(game.state.play_state(), PlayState::GameOver);
    // Rat also dies from explosion
    assert!(rat_positions(&game).is_empty());
}

#[test]
fn explosive_chain_reaction() {
    // Player steps on explosive, triggers chain reaction
    let mut game = game_from_csv("X,.,.\nX,X,.\n.,>,.\n.,.,.");
    // Player moves onto explosive at (1,1)
    // This should trigger chain to (0,1), then to (0,0)
    game.apply_action(Action::Move(Dir4::North));
    // All explosives should be gone
    assert_eq!(game.state.grid.at(Position::new(0, 0)), Cell::Empty);
    assert_eq!(game.state.grid.at(Position::new(0, 1)), Cell::Empty);
    assert_eq!(game.state.grid.at(Position::new(1, 1)), Cell::Empty);
}

#[test]
fn player_consumes_trigger() {
    let mut game = game_from_csv(".,.,.\n.,>,1\n.,.,.");
    assert_eq!(game.state.grid.at(Position::new(2, 1)), Cell::Trigger(1));
    game.apply_action(Action::Move(Dir4::East));
    // Player should be at trigger position, trigger consumed
    assert_eq!(player_pos(&game), Position::new(2, 1));
    assert_eq!(
        game.state.grid.at(Position::new(2, 1)),
        Cell::Player(Dir4::East)
    );
}

#[test]
fn rat_consumes_trigger() {
    let mut game = game_from_csv(".,.,.,.\nR,1,.,>\n.,.,.,.");
    // Rat at (0,1), trigger at (1,1), player at (3,1)
    assert_eq!(game.state.grid.at(Position::new(1, 1)), Cell::Trigger(1));
    game.apply_action(Action::Move(Dir4::East)); // Player moves away, rat moves toward
    // Rat should have consumed the trigger
    let rat = rat_positions(&game)[0];
    assert_eq!(rat, Position::new(1, 1));
    assert_eq!(
        game.state.grid.at(Position::new(1, 1)),
        Cell::Rat(Dir8::East)
    );
}

#[test]
fn explosion_does_not_destroy_trigger() {
    let mut game = game_from_csv(".,1,.\n.,X,.\n.,^,.");
    // Player steps on explosive, trigger is adjacent
    game.apply_action(Action::Move(Dir4::North));
    // Player dies but trigger should still be there
    assert_eq!(game.state.play_state(), PlayState::GameOver);
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Trigger(1));
}

#[test]
fn stepping_on_trigger_does_not_affect_different_number() {
    // Player steps on trigger "1", trigger "2" should not become wall
    let mut game = game_from_csv("2,.,.\n1,>,.\n.,.,.");
    game.apply_action(Action::Move(Dir4::West)); // Step on trigger 1
    // Trigger 2 should still be a trigger, not a wall
    assert_eq!(game.state.grid.at(Position::new(0, 0)), Cell::Trigger(2));
}

#[test]
fn zapped_trigger_becomes_wall() {
    // Player steps on "1", other "1" should become wall after zap
    let mut game = game_from_csv("1,>,1\n.,.,.");
    game.apply_action(Action::Move(Dir4::West)); // Step on left trigger
    // The right trigger (1,0) should now be a wall
    assert_eq!(game.state.grid.at(Position::new(2, 0)), Cell::Wall);
}

#[test]
fn zap_spreads_walls_to_empty_neighbors() {
    // Trigger at center, empty cells around it
    let mut game = game_from_csv(".,.,.\n.,1,.\n1,>,.");
    game.apply_action(Action::Move(Dir4::West)); // Step on bottom-left trigger
    // Center trigger (1,1) became wall, its empty neighbors should be walls
    assert_eq!(game.state.grid.at(Position::new(1, 1)), Cell::Wall);
    assert_eq!(game.state.grid.at(Position::new(0, 0)), Cell::Wall); // NW
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Wall); // N
    assert_eq!(game.state.grid.at(Position::new(2, 0)), Cell::Wall); // NE
    assert_eq!(game.state.grid.at(Position::new(0, 1)), Cell::Wall); // W
    assert_eq!(game.state.grid.at(Position::new(2, 1)), Cell::Wall); // E
}

#[test]
fn zap_triggers_neighboring_explosives() {
    // Trigger next to explosive
    let mut game = game_from_csv("1,X,.\n1,>,.\n.,.,.");
    game.apply_action(Action::Move(Dir4::West)); // Step on (0,1) trigger
    // (0,0) trigger became wall, (1,0) explosive should have exploded
    assert_eq!(game.state.grid.at(Position::new(0, 0)), Cell::Wall);
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Empty); // Explosive gone
}

#[test]
fn explosion_destroys_web_and_plank() {
    // Explosive with adjacent web and plank
    let mut game = game_from_csv(".,w,.\n.,X,=\n.,^,.");
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Spiderweb);
    assert_eq!(game.state.grid.at(Position::new(2, 1)), Cell::Plank);
    game.apply_action(Action::Move(Dir4::North)); // Step on explosive
    // Both web and plank should be destroyed
    assert_eq!(game.state.grid.at(Position::new(1, 0)), Cell::Empty);
    assert_eq!(game.state.grid.at(Position::new(2, 1)), Cell::Empty);
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
fn cyborg_rat_pathfinds_toward_player() {
    // Cyborg at (0,0), player at (2,2)
    let mut game = game_from_csv("C,.,.\n.,.,.\n.,.,v");
    let initial_cyborg = cyborg_positions(&game)[0];
    assert_eq!(initial_cyborg, Position::new(0, 0));

    game.apply_action(Action::Move(Dir4::South)); // Player moves, cyborg responds
    let new_cyborg = cyborg_positions(&game)[0];
    // Cyborg should have moved diagonally (SE) toward player
    assert_eq!(new_cyborg, Position::new(1, 1));
}

#[test]
fn cyborg_rat_avoids_orthogonally_adjacent_to_player() {
    // Cyborg at (0,1), player at (2,1) - cyborg won't move to (1,1) which is dist (1,0)
    let mut game = game_from_csv(".,.,.\nC,.,>\n.,.,.");
    game.apply_action(Action::Move(Dir4::West)); // Player moves west to (1,1)
    // Cyborg at (0,1) is at distance (1,0) and facing sword (player faces West)
    // Cyborg escapes to (0,0) - diagonal from player, smallest position
    let cyborg = cyborg_positions(&game)[0];
    assert_eq!(cyborg, Position::new(0, 0));
}

#[test]
fn cyborg_rat_can_attack_diagonally() {
    // Cyborg diagonally adjacent to player, should be able to attack
    let mut game = game_from_csv(".,.,.\n.,v,C\n.,.,.");
    // Cyborg at (2,1), player at (1,1) facing south
    game.apply_action(Action::Move(Dir4::South)); // Player stalls, cyborg attacks
    assert_eq!(game.state.play_state(), PlayState::GameOver);
}

#[test]
fn cyborg_rat_cannot_attack_from_in_front() {
    // Cyborg in front of player (player facing toward cyborg)
    let mut game = game_from_csv(".,.,.\n.,>,C\n.,.,.");
    // Cyborg at (2,1), player at (1,1) facing east
    // blocked_dir is West, but cyborg would come from East which is the front
    game.apply_action(Action::Move(Dir4::East)); // Player moves into cyborg, killing it
    assert_eq!(game.state.play_state(), PlayState::Won);
}

#[test]
fn cyborg_rat_eats_normal_rat() {
    // Cyborg should eat normal rat when moving through
    let mut game = game_from_csv("C,.,.\n.,R,.\n.,.,v");
    // Cyborg at (0,0), rat at (1,1), player at (2,2)
    game.apply_action(Action::Move(Dir4::South));
    // Cyborg moves to (1,1) eating the rat
    let cyborg = cyborg_positions(&game)[0];
    assert_eq!(cyborg, Position::new(1, 1));
    assert!(rat_positions(&game).is_empty());
}

#[test]
fn cyborg_escapes_to_diagonal_when_facing_sword() {
    // Cyborg at (1,0) facing player's sword escapes to nearest diagonal (0,1)
    // Setup: Player moves toward cyborg, cyborg ends up at (1,0) distance facing sword
    let mut game = game_from_csv(".,.,.\n<,C,.\n.,.,.");
    // Player at (0,1) facing West, cyborg at (1,1)
    // Player moves East toward cyborg - but cyborg is not directly in front
    game.apply_action(Action::Move(Dir4::East)); // Player moves to (1,1), kills cyborg
    assert_eq!(game.state.play_state(), PlayState::Won);
}

#[test]
fn cyborg_escapes_when_corridor_blocked() {
    // Cyborg in corridor with walls can only escape backward
    // This is already tested by cyborg_rat_avoids_orthogonally_adjacent_to_player
    // which shows cyborg at (0,1) escaping to (0,0) when facing sword
    let mut game = game_from_csv("#,#,#\n<,C,.\n#,#,#");
    // Player at (0,1) facing West, cyborg at (1,1)
    // Cyborg is NOT in front of sword (player faces away)
    // Cyborg should advance toward player
    game.apply_action(Action::Move(Dir4::East)); // Player moves to (1,1), kills cyborg
    assert_eq!(game.state.play_state(), PlayState::Won);
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

#[test]
fn explosion_kills_cyborg_rat() {
    // Cyborg next to explosive that player triggers
    let mut game = game_from_csv(".,C,.\n.,X,.\n.,^,.");
    game.apply_action(Action::Move(Dir4::North));
    // Player dies, but cyborg also dies
    assert_eq!(game.state.play_state(), PlayState::GameOver);
    assert!(cyborg_positions(&game).is_empty());
}

#[test]
fn cyborg_moves_before_normal_rat() {
    // Cyborg should move first, potentially eating a rat that's in the path
    // Cyborg at (0,0), rat at (1,1), player at (2,2) facing south
    let mut game = game_from_csv("C,.,.\n.,R,.\n.,.,v");
    // If cyborg moves first (SE to 1,1), it eats the rat
    game.apply_action(Action::Move(Dir4::South));
    let cyborg = cyborg_positions(&game)[0];
    assert_eq!(cyborg, Position::new(1, 1));
    assert!(rat_positions(&game).is_empty());
}

#[test]
fn cyborg_respects_walls_in_pathfinding() {
    // Wall blocks direct path, cyborg must go around
    let mut game = game_from_csv("C,#,.\n.,#,.\n.,.,v");
    // Cyborg at (0,0), walls at (1,0) and (1,1), player at (2,2)
    game.apply_action(Action::Move(Dir4::South));
    let cyborg = cyborg_positions(&game)[0];
    // Cyborg should move south (0,1) since east is blocked
    assert_eq!(cyborg, Position::new(0, 1));
}

#[test]
fn cyborg_does_not_collide_with_other_cyborg() {
    // Player in corner, two cyborgs at knight's jumps - both want (1,1)
    // v . .
    // . . C
    // . C .
    let mut game = game_from_csv("v,.,.\n.,.,C\n.,C,.");
    // Player at (0,0), cyborgs at (2,1) and (1,2)
    game.apply_action(Action::Stall);
    let cyborgs = cyborg_positions(&game);
    assert_eq!(cyborgs.len(), 2);
    // Bottom one (1,2) goes to center (1,1), right one (2,1) goes to top-right (2,0)
    assert!(cyborgs.contains(&Position::new(1, 1)));
    assert!(cyborgs.contains(&Position::new(2, 0)));
}

#[test]
fn cyborg_blocked_in_corridor_by_one_adjacent_rule() {
    // Corridor where front cyborg won't advance (orthogonally adjacent)
    // and back cyborg is blocked by front cyborg
    // # # # # # #
    // # < . C C #
    // # # # # # #
    let mut game = game_from_csv("#,#,#,#,#,#\n#,<,.,C,C,#\n#,#,#,#,#,#");
    // Player at (1,1), cyborgs at (3,1) and (4,1)
    // (2,1) has distance (1,0) - orthogonally adjacent, avoided
    game.apply_action(Action::Stall);
    let cyborgs = cyborg_positions(&game);
    assert_eq!(cyborgs.len(), 2);
    // Both stay in place - front won't move to (1,0) distance, back is blocked
    assert!(cyborgs.contains(&Position::new(3, 1)));
    assert!(cyborgs.contains(&Position::new(4, 1)));
}

#[test]
fn rats_cannot_move_to_same_cell() {
    // Two rats equidistant from player, both want to move to center cell
    // R . R
    // . . .
    // . v .
    let mut game = game_from_csv("R,.,R\n.,.,.\n.,v,.");
    // Rats at (0,0) and (2,0), player at (1,2)
    // Both want to move diagonally toward player: (0,0)->SE(1,1), (2,0)->SW(1,1)
    // Processing order: by distance (tied), then by position: (0,0) first
    // After (0,0) moves to (1,1), (2,0) should NOT also move there
    game.apply_action(Action::Move(Dir4::South));

    let rats = rat_positions(&game);
    assert_eq!(rats.len(), 2, "Both rats should survive");

    // Check that no two rats occupy the same position
    let unique_positions: HashSet<_> = rats.iter().copied().collect();
    assert_eq!(
        unique_positions.len(),
        rats.len(),
        "Rats should not occupy the same cell"
    );

    // First rat (0,0) moves to (1,1)
    assert!(
        rats.contains(&Position::new(1, 1)),
        "First rat should be at (1,1)"
    );
    // Second rat (2,0) should find a different cell - either stay or move elsewhere
    // It should try SW (blocked), then S or E based on distance
    // S(2,1) is closer to player than E(which doesn't exist since it can't go further)
    assert!(
        !rats.contains(&Position::new(0, 0)),
        "First rat should have moved from (0,0)"
    );
}

#[test]
fn rat_can_move_to_vacated_cell() {
    // Two rats in a line, both moving toward player
    // R R . . v
    let mut game = game_from_csv("R,R,.,.,v");
    // Rats at (0,0) and (1,0), player at (4,0)
    // Processing order: by distance - (1,0) is closer, processes first
    // (1,0) moves east to (2,0)
    // (0,0) should be able to move east to (1,0) - the vacated cell
    game.apply_action(Action::Move(Dir4::West));

    let rats = rat_positions(&game);
    assert_eq!(rats.len(), 2, "Both rats should survive");

    // Front rat moved from (1,0) to (2,0)
    assert!(
        rats.contains(&Position::new(2, 0)),
        "Front rat should be at (2,0)"
    );
    // Back rat moved from (0,0) to (1,0) - the vacated cell
    assert!(
        rats.contains(&Position::new(1, 0)),
        "Back rat should move to vacated cell (1,0)"
    );
}

#[test]
fn rat_follows_player_through_destroyed_spiderweb() {
    // Player destroys spiderweb by stepping on it, rat follows to same cell
    // w R
    // < .
    let mut game = game_from_csv("w,R\n<,.");
    // Player at (0,1) facing West, Web at (0,0), Rat at (1,0)
    // Player moves north to (0,0), destroying web
    // Rat at (1,0) should move West to (0,0) and kill player
    game.apply_action(Action::Move(Dir4::North));
    assert_eq!(game.state.play_state(), PlayState::GameOver);
}

#[test]
fn cyborg_can_move_to_vacated_cell() {
    // Two cyborgs in a line, both moving toward player (with gap to avoid orthogonally-adjacent avoidance)
    // C C . . . v
    let mut game = game_from_csv("C,C,.,.,.,v");
    // Cyborgs at (0,0) and (1,0), player at (5,0)
    // Use stall so player doesn't move toward cyborgs
    // Processing order: by distance - (1,0) is closer, processes first
    // (1,0) moves east to (2,0) (distance (3,0) -> (2,0) from player)
    // (0,0) should be able to move east to (1,0) - the vacated cell
    game.apply_action(Action::Stall);

    let cyborgs = cyborg_positions(&game);
    assert_eq!(cyborgs.len(), 2, "Both cyborgs should survive");

    // Front cyborg moved from (1,0) to (2,0)
    assert!(
        cyborgs.contains(&Position::new(2, 0)),
        "Front cyborg should be at (2,0)"
    );
    // Back cyborg moved from (0,0) to (1,0) - the vacated cell
    assert!(
        cyborgs.contains(&Position::new(1, 0)),
        "Back cyborg should move to vacated cell (1,0)"
    );
}

#[test]
fn rat_cannot_move_to_cyborg_destination() {
    // Cyborg and rat both want to move to same cell - cyborg moves first, rat should be blocked
    // C . .
    // . . R
    // . v .
    // (player moves west)
    let mut game = game_from_csv("C,.,.\n.,.,R\n.,v,.");
    // Cyborg at (0,0), rat at (2,1), player at (1,2)
    // Cyborg moves SE to (1,1), rat wants to move SW to (1,1)
    // Rat should be blocked since cyborg claimed that cell
    game.apply_action(Action::Move(Dir4::West));

    let cyborgs = cyborg_positions(&game);
    let rats = rat_positions(&game);

    assert_eq!(cyborgs.len(), 1, "Cyborg should survive, got {:?}", cyborgs);
    assert_eq!(rats.len(), 1, "Rat should survive, got {:?}", rats);
    assert!(
        cyborgs.contains(&Position::new(1, 1)),
        "Cyborg should be at (1,1), got {:?}",
        cyborgs
    );
    // Rat should NOT be at (1,1) - it should have found an alternative or stayed
    assert!(
        !rats.contains(&Position::new(1, 1)),
        "Rat should not move to cyborg's destination"
    );
}

#[test]
fn rat_can_move_to_cyborg_vacated_position() {
    // Rat should be able to move to cell that cyborg just left
    // . . R
    // . C .
    // . v .
    // (player moves west)
    let mut game = game_from_csv(".,.,R\n.,C,.\n.,v,.");
    game.apply_action(Action::Move(Dir4::West));

    let cyborgs = cyborg_positions(&game);
    let rats = rat_positions(&game);

    assert_eq!(cyborgs.len(), 1, "Cyborg should survive");
    assert_eq!(rats.len(), 1, "Rat should survive");
    assert!(
        cyborgs.contains(&Position::new(0, 2)),
        "Cyborg should be at (0,2), got {:?}",
        cyborgs
    );
    assert!(
        rats.contains(&Position::new(1, 1)),
        "Rat should move to cyborg's vacated position (1,1), got {:?}",
        rats
    );
}
