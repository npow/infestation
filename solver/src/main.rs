//! Brute-force solver for Infestation levels, using the real game logic as an oracle.
//!
//! Usage:
//!   solver verify   <csv_file> <action_string>     -- replay actions, print final state
//!   solver solve    <csv_file> [--depth N] [--secs S] [--strategy gbfs|astar|bfs] [--weight W]
//!
//! Action string uses arrows: ^ v < > for N/S/E/W and . for stall (single player).
//! For two players, use "a1|a2 a1|a2 ..." space-separated turns (each turn pipe-separated).

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::time::Instant;

use infestation::testing::{Action, CellKind, Dir4, Grid, PlayState, grid_from_csv, step_grid};

fn ch_to_action(c: char) -> Option<Action> {
    match c {
        '^' | '↑' => Some(Action::Move(Dir4::North)),
        'v' | '↓' => Some(Action::Move(Dir4::South)),
        '<' | '←' => Some(Action::Move(Dir4::West)),
        '>' | '→' => Some(Action::Move(Dir4::East)),
        '.' | 's' => Some(Action::Stall),
        _ => None,
    }
}

fn action_to_ch(a: Action) -> char {
    match a {
        Action::Move(Dir4::North) => '^',
        Action::Move(Dir4::South) => 'v',
        Action::Move(Dir4::West) => '<',
        Action::Move(Dir4::East) => '>',
        Action::Stall => '.',
    }
}

fn arrow(a: Action) -> char {
    match a {
        Action::Move(Dir4::North) => '↑',
        Action::Move(Dir4::South) => '↓',
        Action::Move(Dir4::West) => '←',
        Action::Move(Dir4::East) => '→',
        Action::Stall => '.',
    }
}

fn count_players(grid: &Grid) -> usize {
    let mut players = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Player {
                players += 1;
            }
        }
    }
    players
}

fn count_rats(grid: &Grid) -> usize {
    let mut rats = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Rat | CellKind::CyborgRat) {
                rats += 1;
            }
        }
    }
    rats
}

/// Player walk-distance BFS: players are blocked by walls, planks,
/// black holes, and explosives. Webs, empties, triggers, and rat cells
/// are walkable for heuristic purposes.
/// Returns distance map from all player positions.
fn player_dist_map(grid: &Grid) -> Vec<Vec<i32>> {
    let h = grid.height();
    let w = grid.width();
    let mut dist = vec![vec![i32::MAX; w]; h];
    let mut q = VecDeque::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Player {
                dist[y][x] = 0;
                q.push_back((x, y));
            }
        }
    }
    let dirs = [(0i32, -1i32), (0, 1), (1, 0), (-1, 0)];
    while let Some((x, y)) = q.pop_front() {
        let d = dist[y][x];
        for (dx, dy) in dirs {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || ny as usize >= h || nx as usize >= w {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let cell = grid.cell_kind_at(nx, ny);
            if matches!(
                cell,
                CellKind::Wall | CellKind::Plank | CellKind::BlackHole | CellKind::Explosive
            ) {
                continue;
            }
            if dist[ny][nx] > d + 1 {
                dist[ny][nx] = d + 1;
                q.push_back((nx, ny));
            }
        }
    }
    dist
}

fn player_reachable_cell_count(grid: &Grid) -> usize {
    player_dist_map(grid)
        .iter()
        .flatten()
        .filter(|&&dist| dist != i32::MAX)
        .count()
}

fn rat_component_map(grid: &Grid) -> (Vec<Vec<i32>>, Vec<usize>) {
    let h = grid.height();
    let w = grid.width();
    let mut comp = vec![vec![-1; w]; h];
    let mut sizes = Vec::new();
    let dirs = [
        (-1i32, -1i32),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    for y in 0..h {
        for x in 0..w {
            if comp[y][x] >= 0 || !rat_walkable_static(grid.cell_kind_at(x, y)) {
                continue;
            }
            let id = sizes.len() as i32;
            let mut q = VecDeque::new();
            q.push_back((x, y));
            comp[y][x] = id;
            let mut size = 0usize;
            while let Some((cx, cy)) = q.pop_front() {
                size += 1;
                for (dx, dy) in dirs {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    let (nx, ny) = (nx as usize, ny as usize);
                    if comp[ny][nx] >= 0 || !rat_walkable_static(grid.cell_kind_at(nx, ny)) {
                        continue;
                    }
                    comp[ny][nx] = id;
                    q.push_back((nx, ny));
                }
            }
            sizes.push(size);
        }
    }

    (comp, sizes)
}

fn rat_walkable_static(cell: CellKind) -> bool {
    !matches!(cell, CellKind::Wall | CellKind::Spiderweb)
}

fn adjacent_count(grid: &Grid, x: usize, y: usize, kind: CellKind) -> usize {
    let mut count = 0;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            if grid.cell_kind_at(nx as usize, ny as usize) == kind {
                count += 1;
            }
        }
    }
    count
}

fn print_diagnostics(grid: &Grid) {
    let features = Features::from_grid(grid);
    let player_dist = player_dist_map(grid);
    let (rat_components, component_sizes) = rat_component_map(grid);
    let players = positions_for(grid, CellKind::Player);
    let rats = positions_for_any(grid, &[CellKind::Rat, CellKind::CyborgRat]);
    let triggers = positions_matching(grid, |cell| matches!(cell, CellKind::Trigger(_)));
    let explosives = positions_for(grid, CellKind::Explosive);
    let blackholes = positions_for(grid, CellKind::BlackHole);

    let player_reachable_cells = player_dist
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&dist| dist != i32::MAX)
        .count();
    let reachable_rats = rats
        .iter()
        .filter(|&&(x, y)| player_dist[y][x] != i32::MAX)
        .count();
    let reachable_triggers = triggers
        .iter()
        .filter(|&&(x, y)| player_dist[y as usize][x as usize] != i32::MAX)
        .count();
    let trapped_unreachable_rats = trapped_unreachable_rat_count(grid);

    println!(
        "features rats={} explosives={} webs={} triggers={} planks={} walls={}",
        features.rats,
        features.explosives,
        features.webs,
        features.triggers,
        features.planks,
        features.walls
    );
    println!(
        "players=[{}] rats=[{}]",
        format_positions(&players),
        format_positions(&rats)
    );
    println!(
        "player_reachable cells={} rats={}/{} triggers={}/{}",
        player_reachable_cells,
        reachable_rats,
        rats.len(),
        reachable_triggers,
        triggers.len()
    );
    println!("trapped_unreachable_rats={trapped_unreachable_rats}");
    println!("explosives=[{}]", format_positions(&explosives));
    println!("blackholes=[{}]", format_positions(&blackholes));
    for &(x, y) in &triggers {
        let dist = player_dist[y as usize][x as usize];
        let dist_text = if dist == i32::MAX {
            "unreachable".to_string()
        } else {
            dist.to_string()
        };
        println!(
            "trigger {:?} at ({x},{y}) player_dist={dist_text}",
            grid.cell_kind_at(x as usize, y as usize)
        );
    }

    for &(x, y) in &rats {
        let dist = player_dist[y][x];
        let dist_text = if dist == i32::MAX {
            "unreachable".to_string()
        } else {
            dist.to_string()
        };
        let component = rat_components[y][x];
        let component_size = if component >= 0 {
            component_sizes[component as usize]
        } else {
            0
        };
        println!(
            "rat ({x},{y}) player_dist={} rat_component={} component_size={} adj_x={} adj_o={} adj_trigger={} adj_plank={} adj_web={}",
            dist_text,
            component,
            component_size,
            adjacent_count(grid, x, y, CellKind::Explosive),
            adjacent_count(grid, x, y, CellKind::BlackHole),
            adjacent_trigger_count(grid, x, y),
            adjacent_count(grid, x, y, CellKind::Plank),
            adjacent_count(grid, x, y, CellKind::Spiderweb)
        );
    }
}

fn csv_tokens(grid: &Grid) -> Vec<Vec<String>> {
    grid.to_csv()
        .lines()
        .map(|line| line.split(',').map(str::to_string).collect())
        .collect()
}

fn tokens_to_csv(tokens: &[Vec<String>]) -> String {
    let mut csv = tokens
        .iter()
        .map(|row| row.join(","))
        .collect::<Vec<_>>()
        .join("\n");
    csv.push('\n');
    csv
}

fn is_player_token(token: &str) -> bool {
    matches!(token, "▲" | "▼" | "►" | "◄" | "△" | "▽" | "▷" | "◁")
}

fn is_mutation_floor(token: &str) -> bool {
    matches!(token, "." | "w")
}

fn print_ignition_geometries(grid: &Grid, limit: usize) {
    let base_features = Features::from_grid(grid);
    let base_tokens = csv_tokens(grid);
    let rats = positions_for_any(grid, &[CellKind::Rat, CellKind::CyborgRat]);
    let players = positions_for(grid, CellKind::Player);
    let player_symbols = ["▲", "▼", "►", "◄"];
    let tuples = all_action_tuples(1);
    let mut printed = 0usize;
    let mut seen = HashSet::new();

    for &(source_rat_x, source_rat_y) in &rats {
        if source_rat_y < 3 {
            continue;
        }
        for rat_y in 0..grid.height() {
            for rat_x in 0..grid.width() {
                let rat_base = &base_tokens[rat_y][rat_x];
                if !is_mutation_floor(rat_base) && (rat_x, rat_y) != (source_rat_x, source_rat_y) {
                    continue;
                }
                for player_y in 0..grid.height() {
                    for player_x in 0..grid.width() {
                        if (player_x, player_y) == (rat_x, rat_y) {
                            continue;
                        }
                        let player_base = &base_tokens[player_y][player_x];
                        if !is_mutation_floor(player_base)
                            && !is_player_token(player_base)
                            && (player_x, player_y)
                                != players.first().copied().unwrap_or((usize::MAX, usize::MAX))
                        {
                            continue;
                        }
                        for player_symbol in player_symbols {
                            let mut tokens = base_tokens.clone();
                            for row in &mut tokens {
                                for token in row {
                                    if is_player_token(token) {
                                        *token = ".".to_string();
                                    }
                                }
                            }
                            tokens[source_rat_y][source_rat_x] = ".".to_string();
                            tokens[rat_y][rat_x] = "R".to_string();
                            tokens[player_y][player_x] = player_symbol.to_string();
                            let candidate_csv = tokens_to_csv(&tokens);
                            let candidate = grid_from_csv(&candidate_csv);
                            for actions in &tuples {
                                let (next, play_state) = step(&candidate, actions);
                                let next_features = Features::from_grid(&next);
                                if play_state != PlayState::GameOver
                                    && (play_state == PlayState::Won
                                        || next_features.explosives < base_features.explosives)
                                {
                                    let key = (
                                        rat_x,
                                        rat_y,
                                        player_x,
                                        player_y,
                                        player_symbol.to_string(),
                                        action_to_ch(actions[0]),
                                    );
                                    if !seen.insert(key) {
                                        continue;
                                    }
                                    println!(
                                        "rat=({rat_x},{rat_y}) player=({player_x},{player_y},{player_symbol}) action={} result={play_state:?} next_rats={} next_explosives={} next_webs={}",
                                        action_to_ch(actions[0]),
                                        next_features.rats,
                                        next_features.explosives,
                                        next_features.webs
                                    );
                                    printed += 1;
                                    if printed >= limit {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn player_symbols_for_index(index: usize) -> [&'static str; 4] {
    match index {
        0 => ["▲", "▼", "►", "◄"],
        1 => ["△", "▽", "▷", "◁"],
        _ => panic!("unsupported player index {index}"),
    }
}

fn print_rat_step_geometries(
    grid: &Grid,
    source_rat: (usize, usize),
    target: (i32, i32),
    limit: usize,
) {
    let nplayers = count_players(grid);
    assert!(nplayers <= 2, "rat geometry supports at most two players");
    let base_tokens = csv_tokens(grid);
    let player_positions = positions_for(grid, CellKind::Player);
    let mut candidate_cells = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let token = &base_tokens[y][x];
            if is_mutation_floor(token)
                || is_player_token(token)
                || player_positions.contains(&(x, y))
            {
                candidate_cells.push((x, y));
            }
        }
    }

    let mut printed = 0usize;
    let stall_actions = vec![Action::Stall; nplayers];
    let mut seen = HashSet::new();
    for &p0 in &candidate_cells {
        for &p1 in if nplayers == 2 {
            candidate_cells.as_slice()
        } else {
            &[(usize::MAX, usize::MAX)]
        } {
            let player_cells = if nplayers == 2 {
                vec![p0, p1]
            } else {
                vec![p0]
            };
            if player_cells.iter().any(|&pos| pos == source_rat) || (nplayers == 2 && p0 == p1) {
                continue;
            }

            for s0 in player_symbols_for_index(0) {
                for s1 in if nplayers == 2 {
                    player_symbols_for_index(1)
                } else {
                    ["", "", "", ""]
                } {
                    let mut tokens = base_tokens.clone();
                    for row in &mut tokens {
                        for token in row {
                            if is_player_token(token) {
                                *token = ".".to_string();
                            }
                        }
                    }
                    tokens[source_rat.1][source_rat.0] = "R".to_string();
                    tokens[p0.1][p0.0] = s0.to_string();
                    if nplayers == 2 {
                        tokens[p1.1][p1.0] = s1.to_string();
                    }
                    let candidate = grid_from_csv(&tokens_to_csv(&tokens));
                    let (next, play_state) = step(&candidate, &stall_actions);
                    if play_state == PlayState::GameOver || !rat_at(&next, target) {
                        continue;
                    }
                    let key = (
                        p0,
                        p1,
                        s0.to_string(),
                        s1.to_string(),
                        positions_key(&next, rat_or_cyborg),
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    if nplayers == 2 {
                        println!(
                            "p1=({},{},{}) p2=({},{},{}) target=({},{}) result={:?}",
                            p0.0, p0.1, s0, p1.0, p1.1, s1, target.0, target.1, play_state
                        );
                    } else {
                        println!(
                            "player=({},{},{}) target=({},{}) result={:?}",
                            p0.0, p0.1, s0, target.0, target.1, play_state
                        );
                    }
                    printed += 1;
                    if printed >= limit {
                        return;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeathTargetKind {
    Any,
    BlackHole,
    Explosive,
}

impl DeathTargetKind {
    fn parse(input: &str) -> Self {
        match input {
            "any" => Self::Any,
            "blackhole" | "black-hole" | "hole" => Self::BlackHole,
            "explosive" | "bomb" => Self::Explosive,
            other => panic!("unknown death target kind {other}"),
        }
    }

    fn accepts(self, cell: CellKind) -> bool {
        match self {
            Self::Any => matches!(cell, CellKind::BlackHole | CellKind::Explosive),
            Self::BlackHole => cell == CellKind::BlackHole,
            Self::Explosive => cell == CellKind::Explosive,
        }
    }
}

fn print_rat_death_step_geometries(
    grid: &Grid,
    source_rat: (usize, usize),
    target: Option<(i32, i32)>,
    target_kind: DeathTargetKind,
    limit: usize,
) {
    let nplayers = count_players(grid);
    assert!(
        nplayers <= 2,
        "rat death geometry supports at most two players"
    );
    let base_tokens = csv_tokens(grid);
    let player_positions = positions_for(grid, CellKind::Player);
    let mut candidate_cells = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let token = &base_tokens[y][x];
            if is_mutation_floor(token)
                || is_player_token(token)
                || player_positions.contains(&(x, y))
            {
                candidate_cells.push((x, y));
            }
        }
    }

    let mut printed = 0usize;
    let stall_actions = vec![Action::Stall; nplayers];
    let mut seen = HashSet::new();
    for &p0 in &candidate_cells {
        for &p1 in if nplayers == 2 {
            candidate_cells.as_slice()
        } else {
            &[(usize::MAX, usize::MAX)]
        } {
            let player_cells = if nplayers == 2 {
                vec![p0, p1]
            } else {
                vec![p0]
            };
            if player_cells.iter().any(|&pos| pos == source_rat) || (nplayers == 2 && p0 == p1) {
                continue;
            }

            for s0 in player_symbols_for_index(0) {
                for s1 in if nplayers == 2 {
                    player_symbols_for_index(1)
                } else {
                    ["", "", "", ""]
                } {
                    let mut tokens = base_tokens.clone();
                    for row in &mut tokens {
                        for token in row {
                            if is_player_token(token) {
                                *token = ".".to_string();
                            }
                        }
                    }
                    tokens[source_rat.1][source_rat.0] = "R".to_string();
                    tokens[p0.1][p0.0] = s0.to_string();
                    if nplayers == 2 {
                        tokens[p1.1][p1.0] = s1.to_string();
                    }
                    let candidate = grid_from_csv(&tokens_to_csv(&tokens));
                    let before_features = Features::from_grid(&candidate);
                    let target_cell = target.map(|point| {
                        assert!(
                            point.0 >= 0
                                && point.1 >= 0
                                && (point.0 as usize) < candidate.width()
                                && (point.1 as usize) < candidate.height(),
                            "target must be inside the grid"
                        );
                        candidate.cell_kind_at(point.0 as usize, point.1 as usize)
                    });
                    if target_cell.is_some_and(|cell| !target_kind.accepts(cell)) {
                        continue;
                    }

                    let (next, play_state) = step(&candidate, &stall_actions);
                    if play_state == PlayState::GameOver {
                        continue;
                    }
                    let next_features = Features::from_grid(&next);
                    if next_features.rats >= before_features.rats {
                        continue;
                    }
                    let target_text = target
                        .map(|(x, y)| format!("target=({x},{y})"))
                        .unwrap_or_else(|| "target=any".to_string());
                    let key = (
                        p0,
                        p1,
                        s0.to_string(),
                        s1.to_string(),
                        target_text.clone(),
                        positions_key(&next, rat_or_cyborg),
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    if nplayers == 2 {
                        println!(
                            "p1=({},{},{}) p2=({},{},{}) {} kind={:?} result={:?} rats {}->{} explosives {}->{} rats_at=[{}]",
                            p0.0,
                            p0.1,
                            s0,
                            p1.0,
                            p1.1,
                            s1,
                            target_text,
                            target_kind,
                            play_state,
                            before_features.rats,
                            next_features.rats,
                            before_features.explosives,
                            next_features.explosives,
                            format_positions(&positions_for_any(
                                &next,
                                &[CellKind::Rat, CellKind::CyborgRat],
                            ))
                        );
                    } else {
                        println!(
                            "player=({},{},{}) {} kind={:?} result={:?} rats {}->{} explosives {}->{} rats_at=[{}]",
                            p0.0,
                            p0.1,
                            s0,
                            target_text,
                            target_kind,
                            play_state,
                            before_features.rats,
                            next_features.rats,
                            before_features.explosives,
                            next_features.explosives,
                            format_positions(&positions_for_any(
                                &next,
                                &[CellKind::Rat, CellKind::CyborgRat],
                            ))
                        );
                    }
                    printed += 1;
                    if printed >= limit {
                        return;
                    }
                }
            }
        }
    }
}

fn adjacent_trigger_count(grid: &Grid, x: usize, y: usize) -> usize {
    let mut count = 0;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            if matches!(
                grid.cell_kind_at(nx as usize, ny as usize),
                CellKind::Trigger(_)
            ) {
                count += 1;
            }
        }
    }
    count
}

/// Heuristic: rats_remaining is dominant. Secondary: distance from player to the
/// nearest "actionable" cell (a rat reachable to be killed, or an explosive).
/// This provides a gradient even in levels where rats only die at the end.
fn heuristic(grid: &Grid) -> i64 {
    let rats = count_rats(grid) as i64;
    if rats == 0 {
        return 0;
    }
    let dist = player_dist_map(grid);
    let mut nearest_rat = i32::MAX;
    let mut nearest_trigger = i32::MAX;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let cell = grid.cell_kind_at(x, y);
            let d = dist[y][x];
            if d == i32::MAX {
                continue;
            }
            if matches!(cell, CellKind::Rat | CellKind::CyborgRat) {
                // player can step onto a rat to kill it (players walk through webs)
                nearest_rat = nearest_rat.min(d);
            } else if matches!(cell, CellKind::Trigger(_)) {
                // numbered trigger: stepping on it zaps explosives safely
                nearest_trigger = nearest_trigger.min(d);
            }
            // NOTE: explosives are NOT targets — stepping on one kills the player.
        }
    }
    // count remaining numbered triggers and explosives — consuming/detonating them
    // is progress in chain-reaction puzzles (clears walls protecting the rats).
    let mut triggers_left = 0i64;
    let mut explosives_left = 0i64;
    let mut webs_left = 0i64;
    let mut unreachable_rats = 0i64;
    let mut unreachable_triggers = 0i64;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            match grid.cell_kind_at(x, y) {
                CellKind::Rat | CellKind::CyborgRat => {
                    if dist[y][x] == i32::MAX {
                        unreachable_rats += 1;
                    }
                }
                CellKind::Trigger(_) => {
                    triggers_left += 1;
                    if dist[y][x] == i32::MAX {
                        unreachable_triggers += 1;
                    }
                }
                CellKind::Explosive => explosives_left += 1,
                CellKind::Spiderweb => webs_left += 1,
                _ => {}
            }
        }
    }
    let secondary = nearest_rat.min(nearest_trigger);
    let secondary = if secondary == i32::MAX {
        1000
    } else {
        secondary as i64
    };
    let dead_end_penalty = if nearest_rat == i32::MAX && nearest_trigger == i32::MAX {
        rats * 250_000_000
    } else {
        0
    };
    // weights chosen so rats dominate, then structural progress, then positioning.
    let use_progress = std::env::var("PROGRESS_H").is_ok();
    let smart_penalty = if std::env::var("SMART_H").is_ok() {
        let trapped_unreachable_rats = trapped_unreachable_rat_count(grid) as i64;
        unreachable_rats * 25_000_000
            + trapped_unreachable_rats * 100_000_000
            + unreachable_triggers * 500_000
    } else {
        0
    };
    if use_progress {
        rats * 1_000_000
            + dead_end_penalty
            + smart_penalty
            + explosives_left * 300
            + webs_left * 100
            + triggers_left * 2_000
            + secondary
    } else {
        rats * 1_000_000 + dead_end_penalty + smart_penalty + triggers_left * 2_000 + secondary
    }
}

/// Transition: from a grid state, apply one set of actions, return (next_grid, play_state).
fn step(grid: &Grid, actions: &[Action]) -> (Grid, PlayState) {
    step_grid(grid, actions)
}

/// Enumerate all action-tuples for the given number of players.
fn all_action_tuples(nplayers: usize) -> Vec<Vec<Action>> {
    let single = [
        Action::Move(Dir4::North),
        Action::Move(Dir4::South),
        Action::Move(Dir4::East),
        Action::Move(Dir4::West),
        Action::Stall,
    ];
    if nplayers == 1 {
        return single.iter().map(|a| vec![*a]).collect();
    }
    if nplayers == 2 {
        if std::env::var("MIRROR_2P").is_ok() {
            return vec![
                vec![Action::Move(Dir4::North), Action::Move(Dir4::North)],
                vec![Action::Move(Dir4::South), Action::Move(Dir4::South)],
                vec![Action::Move(Dir4::East), Action::Move(Dir4::West)],
                vec![Action::Move(Dir4::West), Action::Move(Dir4::East)],
                vec![Action::Stall, Action::Stall],
            ];
        }
        let mut out = Vec::new();
        for a in single {
            for b in single {
                out.push(vec![a, b]);
            }
        }
        return out;
    }
    panic!("unsupported player count {nplayers}");
}

struct Node {
    grid: Grid,
    parent: usize,       // usize::MAX for root
    action: Vec<Action>, // action taken from parent to reach this node
    depth: u32,
}

fn reconstruct(nodes: &[Node], mut idx: usize) -> Vec<Vec<Action>> {
    let mut acts: Vec<Vec<Action>> = Vec::new();
    while nodes[idx].parent != usize::MAX {
        acts.push(nodes[idx].action.clone());
        idx = nodes[idx].parent;
    }
    acts.reverse();
    acts
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LookupOrder {
    Bfs,
    Gbfs,
    Astar,
}

impl LookupOrder {
    fn parse(input: &str) -> Self {
        match input {
            "bfs" => Self::Bfs,
            "gbfs" => Self::Gbfs,
            "astar" => Self::Astar,
            other => panic!("unknown lookup order {other}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TrapConstraints {
    min_reachable_rats: Option<usize>,
    max_trapped_rats: Option<usize>,
    min_reachable_cells: Option<usize>,
    min_reachable_triggers: Option<usize>,
    require_reachable_trigger: Option<u8>,
    require_reachable_cell: Option<(i32, i32)>,
    min_explosives: Option<usize>,
    min_triggers: Option<usize>,
    max_webs: Option<usize>,
}

impl TrapConstraints {
    fn accepts(self, grid: &Grid, play_state: PlayState) -> bool {
        if play_state == PlayState::Won {
            return true;
        }
        let features = Features::from_grid(grid);
        self.min_reachable_rats
            .is_none_or(|minimum| reachable_rat_count(grid) >= minimum)
            && self
                .max_trapped_rats
                .is_none_or(|maximum| trapped_unreachable_rat_count(grid) <= maximum)
            && self
                .min_reachable_cells
                .is_none_or(|minimum| player_reachable_cell_count(grid) >= minimum)
            && self
                .min_reachable_triggers
                .is_none_or(|minimum| reachable_trigger_count(grid) >= minimum)
            && self
                .require_reachable_trigger
                .is_none_or(|number| nearest_reachable_trigger_distance(grid, number).is_some())
            && self
                .require_reachable_cell
                .is_none_or(|point| distance_from_player(grid, point) < 1_000)
            && self
                .min_explosives
                .is_none_or(|minimum| features.explosives >= minimum)
            && self
                .min_triggers
                .is_none_or(|minimum| features.triggers >= minimum)
            && self.max_webs.is_none_or(|maximum| features.webs <= maximum)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LookupGoal {
    Win,
    WinReady,
    RectangleReady,
    RectangleLowerReady,
    RectangleLowerSeparated,
    RectangleLowerIgnition,
    TriggerNumber(u8),
    TriggerNumberOnly(u8),
    TriggerNumberOpen(u8, usize),
    CellChanged(i32, i32),
    CellIs(i32, i32, CellKind),
    CellIsWithPlayerAt(i32, i32, CellKind, i32, i32),
    CellNot(i32, i32, CellKind),
    CellNotWithCellIs(i32, i32, CellKind, i32, i32, CellKind),
    CellNotAndRatAt(i32, i32, CellKind, i32, i32),
    CellNotWithPlayerAt(i32, i32, CellKind, i32, i32),
    CellNotWithPlayerInRect(i32, i32, CellKind, i32, i32, i32, i32),
    CellNotWithRatInRect(i32, i32, CellKind, i32, i32, i32, i32),
    CellNotWithNoRatsInRect(i32, i32, CellKind, i32, i32, i32, i32),
    CellReachable(i32, i32),
    PlayerAt(i32, i32),
    PlayerFacing(i32, i32, Dir4),
    RatAt(i32, i32),
    RatAtFarFromPlayer(i32, i32, i64),
    RatAtWithPlayer(i32, i32, i32, i32),
    RatInRectWithPlayerInRect(i32, i32, i32, i32, i32, i32, i32, i32),
    RatInRectWithPlayerInRectAndCellIs(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, CellKind),
    RatInRectWithCellIs(i32, i32, i32, i32, i32, i32, CellKind),
    RatAtWithPlayerFacing(i32, i32, i32, i32, Dir4),
    RatAtWithCell(i32, i32, i32, i32, CellKind),
    RatGone(i32, i32),
    NoRatsAt2(i32, i32, i32, i32),
    NoRatsInRect(i32, i32, i32, i32),
    RatsAtMost(usize),
    ReachableRatsAtLeast(usize),
    AllRatsReachable,
    TriggersAtMost(usize),
    ExplosivesAtMost(usize),
    WebsAtMost(usize),
    RatDrop,
    RatsAtMostWithCellIs(usize, i32, i32, CellKind),
    RatsAtMostWithCellNot(usize, i32, i32, CellKind),
    RatsAtMostWithRatInRect(usize, i32, i32, i32, i32),
    RatsAtMostWithPlayerAt(usize, i32, i32),
    RatsAtMostWithPlayerFacing(usize, i32, i32, Dir4),
    TriggerNumberOnlyWithCellIs(u8, i32, i32, CellKind),
    TriggerNumberOnlyWithCellNot(u8, i32, i32, CellKind),
}

impl LookupGoal {
    fn parse(input: &str) -> Self {
        let Some((kind, arg)) = input.split_once(':') else {
            return match input {
                "win" => Self::Win,
                "readywin" | "winready" => Self::WinReady,
                "rectready" | "rectangle-ready" => Self::RectangleReady,
                "rectlower" | "rectangle-lower" => Self::RectangleLowerReady,
                "rectsep" | "rectangle-lower-separated" => Self::RectangleLowerSeparated,
                "rectignite" | "rectangle-lower-ignition" => Self::RectangleLowerIgnition,
                "allreachable" | "all-rats-reachable" => Self::AllRatsReachable,
                "ratdrop" => Self::RatDrop,
                other => panic!("unknown lookup goal {other}"),
            };
        };
        match kind {
            "trigger" => Self::TriggerNumber(arg.parse().expect("trigger number")),
            "triggeronly" | "triggerstrict" | "triggerexact" => {
                Self::TriggerNumberOnly(arg.parse().expect("trigger number"))
            }
            "triggeropen" | "triggerreachable" => {
                let mut parts = arg.split(',');
                let number = parts
                    .next()
                    .expect("trigger number")
                    .trim()
                    .parse()
                    .expect("trigger number");
                let reachable_cells = parts
                    .next()
                    .expect("reachable cell count")
                    .trim()
                    .parse()
                    .expect("reachable cell count");
                assert!(parts.next().is_none(), "expected trigger,reachable_cells");
                Self::TriggerNumberOpen(number, reachable_cells)
            }
            "cell" | "cellchanged" => {
                let (x, y) = parse_required_point(arg);
                Self::CellChanged(x, y)
            }
            "cellis" | "cellkind" => {
                let (point, kind) = parse_point_and_cell_kind(arg);
                Self::CellIs(point.0, point.1, kind)
            }
            "cellisplayer" | "cellisplayerat" | "cellis-and-playerat" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let player_x = parts
                    .next()
                    .expect("player x")
                    .trim()
                    .parse()
                    .expect("player x");
                let player_y = parts
                    .next()
                    .expect("player y")
                    .trim()
                    .parse()
                    .expect("player y");
                assert!(
                    parts.next().is_none(),
                    "expected cellx,celly,kind,playerx,playery"
                );
                Self::CellIsWithPlayerAt(x, y, kind, player_x, player_y)
            }
            "cellnot" => {
                let (point, kind) = parse_point_and_cell_kind(arg);
                Self::CellNot(point.0, point.1, kind)
            }
            "cellnotcellis" | "cellnot-and-cellis" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let not_kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let other_x = parts
                    .next()
                    .expect("other cell x")
                    .trim()
                    .parse()
                    .expect("other cell x");
                let other_y = parts
                    .next()
                    .expect("other cell y")
                    .trim()
                    .parse()
                    .expect("other cell y");
                let other_kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("other cell kind");
                assert!(
                    parts.next().is_none(),
                    "expected cellx,celly,notkind,otherx,othery,otherkind"
                );
                Self::CellNotWithCellIs(x, y, not_kind, other_x, other_y, other_kind)
            }
            "cellnotratat" | "cellnot-and-ratat" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let rat_x = parts.next().expect("rat x").trim().parse().expect("rat x");
                let rat_y = parts.next().expect("rat y").trim().parse().expect("rat y");
                assert!(
                    parts.next().is_none(),
                    "expected cellx,celly,kind,ratx,raty"
                );
                Self::CellNotAndRatAt(x, y, kind, rat_x, rat_y)
            }
            "cellnotplayer" | "cellnotplayerat" | "cellnot-and-playerat" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let player_x = parts
                    .next()
                    .expect("player x")
                    .trim()
                    .parse()
                    .expect("player x");
                let player_y = parts
                    .next()
                    .expect("player y")
                    .trim()
                    .parse()
                    .expect("player y");
                assert!(
                    parts.next().is_none(),
                    "expected cellx,celly,kind,playerx,playery"
                );
                Self::CellNotWithPlayerAt(x, y, kind, player_x, player_y)
            }
            "cellnotplayerrect" | "cellnot-and-playerrect" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let values: Vec<i32> = parts
                    .map(|part| part.trim().parse().expect("player rect coordinate"))
                    .collect();
                assert_eq!(
                    values.len(),
                    4,
                    "expected cellx,celly,kind,playerx1,playery1,playerx2,playery2"
                );
                Self::CellNotWithPlayerInRect(
                    x, y, kind, values[0], values[1], values[2], values[3],
                )
            }
            "cellnotratrect" | "cellnot-and-ratrect" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let values: Vec<i32> = parts
                    .map(|part| part.trim().parse().expect("rat rect coordinate"))
                    .collect();
                assert_eq!(
                    values.len(),
                    4,
                    "expected cellx,celly,kind,ratx1,raty1,ratx2,raty2"
                );
                Self::CellNotWithRatInRect(x, y, kind, values[0], values[1], values[2], values[3])
            }
            "cellnotnoratsrect" | "cellnot-and-noratsrect" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                let values: Vec<i32> = parts
                    .map(|part| part.trim().parse().expect("rat rect coordinate"))
                    .collect();
                assert_eq!(
                    values.len(),
                    4,
                    "expected cellx,celly,kind,ratx1,raty1,ratx2,raty2"
                );
                Self::CellNotWithNoRatsInRect(
                    x, y, kind, values[0], values[1], values[2], values[3],
                )
            }
            "reachable" | "cellreachable" => {
                let (x, y) = parse_required_point(arg);
                Self::CellReachable(x, y)
            }
            "playerat" => {
                let (x, y) = parse_required_point(arg);
                Self::PlayerAt(x, y)
            }
            "playerfacing" | "playerdir" => {
                let mut parts = arg.split(',');
                let x = parts
                    .next()
                    .expect("player x")
                    .trim()
                    .parse()
                    .expect("player x");
                let y = parts
                    .next()
                    .expect("player y")
                    .trim()
                    .parse()
                    .expect("player y");
                let dir = parts
                    .next()
                    .map(str::trim)
                    .map(parse_dir4_name)
                    .expect("player direction");
                assert!(parts.next().is_none(), "expected playerx,playery,dir");
                Self::PlayerFacing(x, y, dir)
            }
            "ratat" => {
                let (x, y) = parse_required_point(arg);
                Self::RatAt(x, y)
            }
            "ratfar" | "ratatfar" => {
                let ((x, y), distance) = parse_point_and_distance(arg);
                Self::RatAtFarFromPlayer(x, y, distance)
            }
            "ratplayer" => {
                let mut parts = arg.split(',');
                let rat_x = parts.next().expect("rat x").trim().parse().expect("rat x");
                let rat_y = parts.next().expect("rat y").trim().parse().expect("rat y");
                let player_x = parts
                    .next()
                    .expect("player x")
                    .trim()
                    .parse()
                    .expect("player x");
                let player_y = parts
                    .next()
                    .expect("player y")
                    .trim()
                    .parse()
                    .expect("player y");
                assert!(parts.next().is_none(), "expected ratx,raty,playerx,playery");
                Self::RatAtWithPlayer(rat_x, rat_y, player_x, player_y)
            }
            "ratrectplayerrect" | "ratinrectplayerinrect" => {
                let values: Vec<i32> = arg
                    .split(',')
                    .map(|part| part.trim().parse().expect("coordinate"))
                    .collect();
                assert_eq!(
                    values.len(),
                    8,
                    "expected ratx1,raty1,ratx2,raty2,playerx1,playery1,playerx2,playery2"
                );
                Self::RatInRectWithPlayerInRect(
                    values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                    values[7],
                )
            }
            "ratrectplayerrectcellis" | "ratinrectplayerinrectcellis" => {
                let mut parts = arg.split(',');
                let values: Vec<i32> = parts
                    .by_ref()
                    .take(10)
                    .map(|part| part.trim().parse().expect("coordinate"))
                    .collect();
                assert_eq!(
                    values.len(),
                    10,
                    "expected ratx1,raty1,ratx2,raty2,playerx1,playery1,playerx2,playery2,cellx,celly,kind"
                );
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                assert!(
                    parts.next().is_none(),
                    "expected ratx1,raty1,ratx2,raty2,playerx1,playery1,playerx2,playery2,cellx,celly,kind"
                );
                Self::RatInRectWithPlayerInRectAndCellIs(
                    values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                    values[7], values[8], values[9], kind,
                )
            }
            "ratrectcellis" | "ratinrectcellis" => {
                let mut parts = arg.split(',');
                let values: Vec<i32> = parts
                    .by_ref()
                    .take(6)
                    .map(|part| part.trim().parse().expect("coordinate"))
                    .collect();
                assert_eq!(
                    values.len(),
                    6,
                    "expected ratx1,raty1,ratx2,raty2,cellx,celly,kind"
                );
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                assert!(
                    parts.next().is_none(),
                    "expected ratx1,raty1,ratx2,raty2,cellx,celly,kind"
                );
                Self::RatInRectWithCellIs(
                    values[0], values[1], values[2], values[3], values[4], values[5], kind,
                )
            }
            "ratplayerfacing" | "ratplayerdir" => {
                let mut parts = arg.split(',');
                let rat_x = parts.next().expect("rat x").trim().parse().expect("rat x");
                let rat_y = parts.next().expect("rat y").trim().parse().expect("rat y");
                let player_x = parts
                    .next()
                    .expect("player x")
                    .trim()
                    .parse()
                    .expect("player x");
                let player_y = parts
                    .next()
                    .expect("player y")
                    .trim()
                    .parse()
                    .expect("player y");
                let dir = parts
                    .next()
                    .map(str::trim)
                    .map(parse_dir4_name)
                    .expect("player direction");
                assert!(
                    parts.next().is_none(),
                    "expected ratx,raty,playerx,playery,dir"
                );
                Self::RatAtWithPlayerFacing(rat_x, rat_y, player_x, player_y, dir)
            }
            "ratcell" => {
                let mut parts = arg.split(',');
                let rat_x = parts.next().expect("rat x").trim().parse().expect("rat x");
                let rat_y = parts.next().expect("rat y").trim().parse().expect("rat y");
                let cell_x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let cell_y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                assert!(
                    parts.next().is_none(),
                    "expected ratx,raty,cellx,celly,kind"
                );
                Self::RatAtWithCell(rat_x, rat_y, cell_x, cell_y, kind)
            }
            "ratgone" => {
                let (x, y) = parse_required_point(arg);
                Self::RatGone(x, y)
            }
            "norats2" | "noratsat2" => {
                let mut parts = arg.split(',');
                let x1 = parts
                    .next()
                    .expect("first x")
                    .trim()
                    .parse()
                    .expect("first x");
                let y1 = parts
                    .next()
                    .expect("first y")
                    .trim()
                    .parse()
                    .expect("first y");
                let x2 = parts
                    .next()
                    .expect("second x")
                    .trim()
                    .parse()
                    .expect("second x");
                let y2 = parts
                    .next()
                    .expect("second y")
                    .trim()
                    .parse()
                    .expect("second y");
                assert!(parts.next().is_none(), "expected x1,y1,x2,y2");
                Self::NoRatsAt2(x1, y1, x2, y2)
            }
            "noratsrect" | "noratsinrect" => {
                let values: Vec<i32> = arg
                    .split(',')
                    .map(|part| part.trim().parse().expect("coordinate"))
                    .collect();
                assert_eq!(values.len(), 4, "expected x1,y1,x2,y2");
                Self::NoRatsInRect(values[0], values[1], values[2], values[3])
            }
            "ratsle" | "ratsatmost" => Self::RatsAtMost(arg.parse().expect("rat count")),
            "reachablege" | "reachableratsge" => {
                Self::ReachableRatsAtLeast(arg.parse().expect("reachable rat count"))
            }
            "triggersle" | "triggersatmost" => {
                Self::TriggersAtMost(arg.parse().expect("trigger count"))
            }
            "explosivesle" | "explosivesatmost" => {
                Self::ExplosivesAtMost(arg.parse().expect("explosive count"))
            }
            "websle" | "websatmost" => Self::WebsAtMost(arg.parse().expect("web count")),
            "ratslecellis" | "ratsatmostcellis" => {
                let (count, point, kind) = parse_count_point_and_cell_kind(arg);
                Self::RatsAtMostWithCellIs(count, point.0, point.1, kind)
            }
            "ratslecellnot" | "ratsatmostcellnot" => {
                let (count, point, kind) = parse_count_point_and_cell_kind(arg);
                Self::RatsAtMostWithCellNot(count, point.0, point.1, kind)
            }
            "ratsleratrect" | "ratsatmostratrect" => {
                let values: Vec<i32> = arg
                    .split(',')
                    .map(|part| part.trim().parse().expect("coordinate"))
                    .collect();
                assert_eq!(values.len(), 5, "expected count,x1,y1,x2,y2");
                Self::RatsAtMostWithRatInRect(
                    values[0] as usize,
                    values[1],
                    values[2],
                    values[3],
                    values[4],
                )
            }
            "ratsleplayer" | "ratsatmostplayer" | "ratsleplayerat" => {
                let (count, (x, y)) = parse_count_and_point(arg);
                Self::RatsAtMostWithPlayerAt(count, x, y)
            }
            "ratsleplayerfacing" | "ratsatmostplayerfacing" | "ratsleplayerdir" => {
                let (count, (x, y), dir) = parse_count_point_and_dir(arg);
                Self::RatsAtMostWithPlayerFacing(count, x, y, dir)
            }
            "triggeronlycellis" | "triggerstrictcellis" => {
                let mut parts = arg.split(',');
                let number = parts
                    .next()
                    .expect("trigger number")
                    .trim()
                    .parse()
                    .expect("trigger number");
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                assert!(parts.next().is_none(), "expected trigger,cellx,celly,kind");
                Self::TriggerNumberOnlyWithCellIs(number, x, y, kind)
            }
            "triggeronlycellnot" | "triggerstrictcellnot" => {
                let mut parts = arg.split(',');
                let number = parts
                    .next()
                    .expect("trigger number")
                    .trim()
                    .parse()
                    .expect("trigger number");
                let x = parts
                    .next()
                    .expect("cell x")
                    .trim()
                    .parse()
                    .expect("cell x");
                let y = parts
                    .next()
                    .expect("cell y")
                    .trim()
                    .parse()
                    .expect("cell y");
                let kind = parts
                    .next()
                    .map(str::trim)
                    .map(parse_cell_kind_name)
                    .expect("cell kind");
                assert!(parts.next().is_none(), "expected trigger,cellx,celly,kind");
                Self::TriggerNumberOnlyWithCellNot(number, x, y, kind)
            }
            other => panic!("unknown lookup goal {other}"),
        }
    }
}

fn parse_dir4_name(input: &str) -> Dir4 {
    match input {
        "n" | "north" | "^" | "up" => Dir4::North,
        "s" | "south" | "v" | "down" => Dir4::South,
        "e" | "east" | ">" | "right" => Dir4::East,
        "w" | "west" | "<" | "left" => Dir4::West,
        other => panic!("unknown direction {other}"),
    }
}

fn parse_required_point(s: &str) -> (i32, i32) {
    parse_optional_point(s).expect("x,y point")
}

fn parse_point_and_cell_kind(s: &str) -> ((i32, i32), CellKind) {
    let mut parts = s.split(',');
    let x = parts.next().expect("x").trim().parse().expect("x");
    let y = parts.next().expect("y").trim().parse().expect("y");
    let kind = parts
        .next()
        .map(str::trim)
        .map(parse_cell_kind_name)
        .expect("cell kind");
    assert!(parts.next().is_none(), "expected x,y,kind");
    ((x, y), kind)
}

fn parse_point_and_distance(s: &str) -> ((i32, i32), i64) {
    let mut parts = s.split(',');
    let x = parts.next().expect("x").trim().parse().expect("x");
    let y = parts.next().expect("y").trim().parse().expect("y");
    let distance = parts
        .next()
        .expect("distance")
        .trim()
        .parse()
        .expect("distance");
    assert!(distance >= 0, "distance must be non-negative");
    assert!(parts.next().is_none(), "expected x,y,distance");
    ((x, y), distance)
}

fn parse_count_point_and_cell_kind(s: &str) -> (usize, (i32, i32), CellKind) {
    let mut parts = s.split(',');
    let count = parts
        .next()
        .expect("rat count")
        .trim()
        .parse()
        .expect("rat count");
    let x = parts.next().expect("x").trim().parse().expect("x");
    let y = parts.next().expect("y").trim().parse().expect("y");
    let kind = parts
        .next()
        .map(str::trim)
        .map(parse_cell_kind_name)
        .expect("cell kind");
    assert!(parts.next().is_none(), "expected count,x,y,kind");
    (count, (x, y), kind)
}

fn parse_count_and_point(s: &str) -> (usize, (i32, i32)) {
    let mut parts = s.split(',');
    let count = parts
        .next()
        .expect("rat count")
        .trim()
        .parse()
        .expect("rat count");
    let x = parts.next().expect("x").trim().parse().expect("x");
    let y = parts.next().expect("y").trim().parse().expect("y");
    assert!(parts.next().is_none(), "expected count,x,y");
    (count, (x, y))
}

fn parse_count_point_and_dir(s: &str) -> (usize, (i32, i32), Dir4) {
    let mut parts = s.split(',');
    let count = parts
        .next()
        .expect("rat count")
        .trim()
        .parse()
        .expect("rat count");
    let x = parts.next().expect("x").trim().parse().expect("x");
    let y = parts.next().expect("y").trim().parse().expect("y");
    let dir = parts
        .next()
        .map(str::trim)
        .map(parse_dir4_name)
        .expect("player direction");
    assert!(parts.next().is_none(), "expected count,x,y,dir");
    (count, (x, y), dir)
}

fn parse_cell_kind_name(s: &str) -> CellKind {
    match s.to_ascii_lowercase().as_str() {
        "empty" | "." => CellKind::Empty,
        "wall" | "#" => CellKind::Wall,
        "player" | "p" => CellKind::Player,
        "rat" | "r" => CellKind::Rat,
        "cyborg" | "cyborgrat" | "c" => CellKind::CyborgRat,
        "plank" | "=" => CellKind::Plank,
        "web" | "spiderweb" | "w" => CellKind::Spiderweb,
        "blackhole" | "hole" | "o" => CellKind::BlackHole,
        "explosive" | "x" => CellKind::Explosive,
        other => {
            if let Some(number) = other.strip_prefix("trigger") {
                return CellKind::Trigger(number.parse().expect("trigger number"));
            }
            if let Ok(number) = other.parse() {
                return CellKind::Trigger(number);
            }
            panic!("unknown cell kind {s}");
        }
    }
}

fn trigger_count(grid: &Grid, number: u8) -> usize {
    let mut count = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Trigger(number) {
                count += 1;
            }
        }
    }
    count
}

fn trigger_counts(grid: &Grid) -> HashMap<u8, usize> {
    let mut counts = HashMap::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if let CellKind::Trigger(number) = grid.cell_kind_at(x, y) {
                *counts.entry(number).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn only_trigger_changed(initial: &Grid, current: &Grid, number: u8) -> bool {
    let initial_counts = trigger_counts(initial);
    let current_counts = trigger_counts(current);
    if current_counts.get(&number).copied().unwrap_or(0)
        >= initial_counts.get(&number).copied().unwrap_or(0)
    {
        return false;
    }

    initial_counts
        .keys()
        .chain(current_counts.keys())
        .copied()
        .filter(|&candidate| candidate != number)
        .all(|candidate| {
            initial_counts.get(&candidate).copied().unwrap_or(0)
                == current_counts.get(&candidate).copied().unwrap_or(0)
        })
}

fn win_ready(grid: &Grid) -> bool {
    let nplayers = count_players(grid);
    let Ok(tuples) = std::panic::catch_unwind(|| all_action_tuples(nplayers)) else {
        return false;
    };
    winning_action(grid, &tuples).is_some()
}

fn lookup_goal_reached(
    goal: LookupGoal,
    initial: &Grid,
    current: &Grid,
    play_state: PlayState,
) -> bool {
    match goal {
        LookupGoal::Win => play_state == PlayState::Won,
        LookupGoal::WinReady => play_state == PlayState::Won || win_ready(current),
        LookupGoal::RectangleReady => play_state == PlayState::Won || win_ready(current),
        LookupGoal::RectangleLowerReady => {
            play_state == PlayState::Won || rectangle_lower_ready(current)
        }
        LookupGoal::RectangleLowerSeparated => {
            play_state == PlayState::Won || rectangle_lower_separated(current)
        }
        LookupGoal::RectangleLowerIgnition => {
            play_state == PlayState::Won || rectangle_lower_ignition_ready(current)
        }
        LookupGoal::TriggerNumber(number) => {
            trigger_count(current, number) < trigger_count(initial, number)
        }
        LookupGoal::TriggerNumberOnly(number) => only_trigger_changed(initial, current, number),
        LookupGoal::TriggerNumberOpen(number, reachable_cells) => {
            trigger_count(current, number) < trigger_count(initial, number)
                && player_reachable_cell_count(current) >= reachable_cells
        }
        LookupGoal::CellChanged(x, y) => {
            current.cell_kind_at(x as usize, y as usize)
                != initial.cell_kind_at(x as usize, y as usize)
        }
        LookupGoal::CellIs(x, y, kind) => current.cell_kind_at(x as usize, y as usize) == kind,
        LookupGoal::CellIsWithPlayerAt(x, y, kind, player_x, player_y) => {
            current.cell_kind_at(x as usize, y as usize) == kind
                && positions_matching(current, |cell| cell == CellKind::Player)
                    .contains(&(player_x, player_y))
        }
        LookupGoal::CellNot(x, y, kind) => current.cell_kind_at(x as usize, y as usize) != kind,
        LookupGoal::CellNotWithCellIs(x, y, not_kind, other_x, other_y, other_kind) => {
            current.cell_kind_at(x as usize, y as usize) != not_kind
                && current.cell_kind_at(other_x as usize, other_y as usize) == other_kind
        }
        LookupGoal::CellNotAndRatAt(x, y, kind, rat_x, rat_y) => {
            current.cell_kind_at(x as usize, y as usize) != kind && rat_at(current, (rat_x, rat_y))
        }
        LookupGoal::CellNotWithPlayerAt(x, y, kind, player_x, player_y) => {
            current.cell_kind_at(x as usize, y as usize) != kind
                && positions_matching(current, |cell| cell == CellKind::Player)
                    .contains(&(player_x, player_y))
        }
        LookupGoal::CellNotWithPlayerInRect(x, y, kind, px1, py1, px2, py2) => {
            current.cell_kind_at(x as usize, y as usize) != kind
                && positions_matching(current, |cell| cell == CellKind::Player)
                    .iter()
                    .any(|&point| point_in_rect(point, px1, py1, px2, py2))
        }
        LookupGoal::CellNotWithRatInRect(x, y, kind, rx1, ry1, rx2, ry2) => {
            current.cell_kind_at(x as usize, y as usize) != kind
                && rat_positions(current)
                    .iter()
                    .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
        }
        LookupGoal::CellNotWithNoRatsInRect(x, y, kind, rx1, ry1, rx2, ry2) => {
            current.cell_kind_at(x as usize, y as usize) != kind
                && rat_positions(current)
                    .iter()
                    .all(|&point| !point_in_rect(point, rx1, ry1, rx2, ry2))
        }
        LookupGoal::CellReachable(x, y) => distance_from_player(current, (x, y)) < 1_000,
        LookupGoal::PlayerAt(x, y) => {
            positions_matching(current, |cell| cell == CellKind::Player).contains(&(x, y))
        }
        LookupGoal::PlayerFacing(x, y, dir) => player_facing(current, (x, y), dir),
        LookupGoal::RatAt(x, y) => rat_at(current, (x, y)),
        LookupGoal::RatAtFarFromPlayer(x, y, min_distance) => {
            rat_at(current, (x, y))
                && nearest_player_distance_from(current, (x, y))
                    .is_some_and(|distance| distance >= min_distance)
        }
        LookupGoal::RatAtWithPlayer(rat_x, rat_y, player_x, player_y) => {
            rat_at(current, (rat_x, rat_y))
                && positions_matching(current, |cell| cell == CellKind::Player)
                    .contains(&(player_x, player_y))
        }
        LookupGoal::RatInRectWithPlayerInRect(rx1, ry1, rx2, ry2, px1, py1, px2, py2) => {
            rat_positions(current)
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                && positions_matching(current, |cell| cell == CellKind::Player)
                    .iter()
                    .any(|&point| point_in_rect(point, px1, py1, px2, py2))
        }
        LookupGoal::RatInRectWithPlayerInRectAndCellIs(
            rx1,
            ry1,
            rx2,
            ry2,
            px1,
            py1,
            px2,
            py2,
            cell_x,
            cell_y,
            kind,
        ) => {
            rat_positions(current)
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                && positions_matching(current, |cell| cell == CellKind::Player)
                    .iter()
                    .any(|&point| point_in_rect(point, px1, py1, px2, py2))
                && current.cell_kind_at(cell_x as usize, cell_y as usize) == kind
        }
        LookupGoal::RatInRectWithCellIs(rx1, ry1, rx2, ry2, cell_x, cell_y, kind) => {
            rat_positions(current)
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                && current.cell_kind_at(cell_x as usize, cell_y as usize) == kind
        }
        LookupGoal::RatAtWithPlayerFacing(rat_x, rat_y, player_x, player_y, dir) => {
            rat_at(current, (rat_x, rat_y)) && player_facing(current, (player_x, player_y), dir)
        }
        LookupGoal::RatAtWithCell(rat_x, rat_y, cell_x, cell_y, kind) => {
            rat_at(current, (rat_x, rat_y))
                && current.cell_kind_at(cell_x as usize, cell_y as usize) == kind
        }
        LookupGoal::RatGone(x, y) => !rat_at(current, (x, y)),
        LookupGoal::NoRatsAt2(x1, y1, x2, y2) => {
            !rat_at(current, (x1, y1)) && !rat_at(current, (x2, y2))
        }
        LookupGoal::NoRatsInRect(x1, y1, x2, y2) => rat_positions(current)
            .iter()
            .all(|&point| !point_in_rect(point, x1, y1, x2, y2)),
        LookupGoal::RatsAtMost(count) => count_rats(current) <= count,
        LookupGoal::ReachableRatsAtLeast(count) => reachable_rat_count(current) >= count,
        LookupGoal::AllRatsReachable => reachable_rat_count(current) == count_rats(current),
        LookupGoal::TriggersAtMost(count) => Features::from_grid(current).triggers <= count,
        LookupGoal::ExplosivesAtMost(count) => Features::from_grid(current).explosives <= count,
        LookupGoal::WebsAtMost(count) => Features::from_grid(current).webs <= count,
        LookupGoal::RatDrop => count_rats(current) < count_rats(initial),
        LookupGoal::RatsAtMostWithCellIs(count, x, y, kind) => {
            count_rats(current) <= count && current.cell_kind_at(x as usize, y as usize) == kind
        }
        LookupGoal::RatsAtMostWithCellNot(count, x, y, kind) => {
            count_rats(current) <= count && current.cell_kind_at(x as usize, y as usize) != kind
        }
        LookupGoal::RatsAtMostWithRatInRect(count, x1, y1, x2, y2) => {
            count_rats(current) <= count
                && rat_positions(current)
                    .into_iter()
                    .any(|point| point_in_rect(point, x1, y1, x2, y2))
        }
        LookupGoal::RatsAtMostWithPlayerAt(count, x, y) => {
            count_rats(current) <= count
                && positions_matching(current, |cell| cell == CellKind::Player).contains(&(x, y))
        }
        LookupGoal::RatsAtMostWithPlayerFacing(count, x, y, dir) => {
            count_rats(current) <= count && player_facing(current, (x, y), dir)
        }
        LookupGoal::TriggerNumberOnlyWithCellIs(number, x, y, kind) => {
            only_trigger_changed(initial, current, number)
                && current.cell_kind_at(x as usize, y as usize) == kind
        }
        LookupGoal::TriggerNumberOnlyWithCellNot(number, x, y, kind) => {
            only_trigger_changed(initial, current, number)
                && current.cell_kind_at(x as usize, y as usize) != kind
        }
    }
}

fn lookup_goal_heuristic(goal: LookupGoal, initial: &Grid, current: &Grid) -> i64 {
    if lookup_goal_reached(goal, initial, current, PlayState::Playing) {
        return 0;
    }

    match goal {
        LookupGoal::Win => heuristic(current),
        LookupGoal::WinReady => {
            if std::env::var("TRAP_H").is_ok() && current.width() >= 16 {
                rectangle_trap_heuristic(current, count_rats(initial), count_explosives(initial))
            } else {
                heuristic(current)
            }
        }
        LookupGoal::RectangleReady => {
            rectangle_winready_heuristic(current, count_rats(initial), count_explosives(initial))
        }
        LookupGoal::RectangleLowerReady => {
            rectangle_lower_ready_heuristic(current, count_rats(initial), count_explosives(initial))
        }
        LookupGoal::RectangleLowerSeparated => rectangle_lower_separated_heuristic(
            current,
            count_rats(initial),
            count_explosives(initial),
        ),
        LookupGoal::RectangleLowerIgnition => rectangle_lower_ignition_heuristic(
            current,
            count_rats(initial),
            count_explosives(initial),
        ),
        LookupGoal::TriggerNumber(number) | LookupGoal::TriggerNumberOnly(number) => {
            let dist = player_dist_map(current);
            let nearest = trigger_positions(current, number)
                .iter()
                .filter_map(|&(x, y)| {
                    let d = dist[y as usize][x as usize];
                    (d != i32::MAX).then_some(d as i64)
                })
                .min()
                .unwrap_or(1_000);
            nearest + heuristic(current) / 1_000
        }
        LookupGoal::TriggerNumberOpen(number, reachable_cells) => {
            let dist = player_dist_map(current);
            let nearest = trigger_positions(current, number)
                .iter()
                .filter_map(|&(x, y)| {
                    let d = dist[y as usize][x as usize];
                    (d != i32::MAX).then_some(d as i64)
                })
                .min()
                .unwrap_or(1_000);
            let reachable_penalty =
                reachable_cells.saturating_sub(player_reachable_cell_count(current)) as i64;
            nearest + reachable_penalty * 1_000 + heuristic(current) / 1_000
        }
        LookupGoal::CellChanged(x, y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::CellIs(x, y, _) | LookupGoal::CellNot(x, y, _) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::CellNotWithCellIs(x, y, not_kind, other_x, other_y, other_kind) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != not_kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let other_penalty =
                if current.cell_kind_at(other_x as usize, other_y as usize) == other_kind {
                    0
                } else {
                    500_000
                };
            cell_penalty + other_penalty + heuristic(current) / 1_000
        }
        LookupGoal::CellIsWithPlayerAt(x, y, kind, player_x, player_y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) == kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let player_penalty = nearest_target_distance(&players, &[(player_x, player_y)]) * 1_000;
            cell_penalty + player_penalty + heuristic(current) / 1_000
        }
        LookupGoal::CellNotAndRatAt(x, y, kind, rat_x, rat_y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let rats = rat_positions(current);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let rat_penalty = nearest_target_distance(&rats, &[(rat_x, rat_y)]) * 1_000;
            cell_penalty + rat_penalty + heuristic(current) / 1_000
        }
        LookupGoal::CellNotWithPlayerAt(x, y, kind, player_x, player_y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let player_penalty = nearest_target_distance(&players, &[(player_x, player_y)]) * 1_000;
            cell_penalty + player_penalty + heuristic(current) / 1_000
        }
        LookupGoal::CellNotWithPlayerInRect(x, y, kind, px1, py1, px2, py2) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let player_penalty = nearest_rect_distance(&players, px1, py1, px2, py2) * 1_000;
            cell_penalty + player_penalty + heuristic(current) / 1_000
        }
        LookupGoal::CellNotWithRatInRect(x, y, kind, rx1, ry1, rx2, ry2) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let rats = rat_positions(current);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let rat_penalty = nearest_rect_distance(&rats, rx1, ry1, rx2, ry2) * 1_000;
            cell_penalty + rat_penalty + heuristic(current) / 1_000
        }
        LookupGoal::CellNotWithNoRatsInRect(x, y, kind, rx1, ry1, rx2, ry2) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                nearest_target_distance(&players, &[(x, y)]) * 50 + 100_000
            };
            let rats_in_rect = rat_positions(current)
                .into_iter()
                .filter(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                .count() as i64;
            cell_penalty + rats_in_rect * 250_000 + heuristic(current) / 1_000
        }
        LookupGoal::CellReachable(x, y) => {
            distance_from_player(current, (x, y)) * 10_000 + heuristic(current) / 1_000
        }
        LookupGoal::PlayerAt(x, y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::PlayerFacing(x, y, _) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::RatAt(x, y) => {
            let rats = rat_positions(current);
            nearest_target_distance(&rats, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::RatAtFarFromPlayer(x, y, min_distance) => {
            let target = (x, y);
            let rats = rat_positions(current);
            let nearest_player = nearest_player_distance_from(current, target).unwrap_or(0);
            let separation_penalty = min_distance.saturating_sub(nearest_player) * 100_000;
            nearest_target_distance(&rats, &[target])
                + separation_penalty
                + heuristic(current) / 1_000
        }
        LookupGoal::RatAtWithPlayer(rat_x, rat_y, player_x, player_y) => {
            let rats = rat_positions(current);
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&rats, &[(rat_x, rat_y)])
                + nearest_target_distance(&players, &[(player_x, player_y)])
                + heuristic(current) / 1_000
        }
        LookupGoal::RatInRectWithPlayerInRect(rx1, ry1, rx2, ry2, px1, py1, px2, py2) => {
            let rats = rat_positions(current);
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_rect_distance(&rats, rx1, ry1, rx2, ry2) * 1_000
                + nearest_rect_distance(&players, px1, py1, px2, py2)
                + heuristic(current) / 1_000
        }
        LookupGoal::RatInRectWithPlayerInRectAndCellIs(
            rx1,
            ry1,
            rx2,
            ry2,
            px1,
            py1,
            px2,
            py2,
            cell_x,
            cell_y,
            kind,
        ) => {
            let rats = rat_positions(current);
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let cell_penalty = if current.cell_kind_at(cell_x as usize, cell_y as usize) == kind {
                0
            } else {
                500_000
            };
            nearest_rect_distance(&rats, rx1, ry1, rx2, ry2) * 1_000
                + nearest_rect_distance(&players, px1, py1, px2, py2)
                + cell_penalty
                + heuristic(current) / 1_000
        }
        LookupGoal::RatInRectWithCellIs(rx1, ry1, rx2, ry2, cell_x, cell_y, kind) => {
            let rats = rat_positions(current);
            let cell_penalty = if current.cell_kind_at(cell_x as usize, cell_y as usize) == kind {
                0
            } else {
                500_000
            };
            nearest_rect_distance(&rats, rx1, ry1, rx2, ry2) * 1_000
                + cell_penalty
                + heuristic(current) / 1_000
        }
        LookupGoal::RatAtWithPlayerFacing(rat_x, rat_y, player_x, player_y, _) => {
            let rats = rat_positions(current);
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&rats, &[(rat_x, rat_y)])
                + nearest_target_distance(&players, &[(player_x, player_y)])
                + heuristic(current) / 1_000
        }
        LookupGoal::RatAtWithCell(rat_x, rat_y, cell_x, cell_y, kind) => {
            let rats = rat_positions(current);
            let cell_penalty = if current.cell_kind_at(cell_x as usize, cell_y as usize) == kind {
                0
            } else {
                200_000
            };
            nearest_target_distance(&rats, &[(rat_x, rat_y)])
                + cell_penalty
                + heuristic(current) / 1_000
        }
        LookupGoal::RatGone(x, y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::NoRatsAt2(x1, y1, x2, y2) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let targets = [(x1, y1), (x2, y2)];
            let remaining = targets
                .iter()
                .filter(|&&(x, y)| rat_at(current, (x, y)))
                .count() as i64;
            remaining * 500_000 + nearest_target_distance(&players, &targets)
        }
        LookupGoal::NoRatsInRect(x1, y1, x2, y2) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let rats_in_rect = rat_positions(current)
                .into_iter()
                .filter(|&point| point_in_rect(point, x1, y1, x2, y2))
                .count() as i64;
            rats_in_rect * 500_000 + nearest_rect_distance(&players, x1, y1, x2, y2)
        }
        LookupGoal::RatsAtMost(count) => {
            count_rats(current).saturating_sub(count) as i64 * 1_000_000
                + heuristic(current) / 1_000
        }
        LookupGoal::ReachableRatsAtLeast(count) => {
            count.saturating_sub(reachable_rat_count(current)) as i64 * 1_000_000
                + count_rats(current) as i64 * 1_000
        }
        LookupGoal::AllRatsReachable => {
            count_rats(current).saturating_sub(reachable_rat_count(current)) as i64 * 1_000_000
                + count_rats(current) as i64 * 1_000
        }
        LookupGoal::TriggersAtMost(count) => {
            Features::from_grid(current).triggers.saturating_sub(count) as i64 * 1_000_000
                + heuristic(current) / 1_000
        }
        LookupGoal::ExplosivesAtMost(count) => {
            Features::from_grid(current)
                .explosives
                .saturating_sub(count) as i64
                * 1_000_000
                + heuristic(current) / 1_000
        }
        LookupGoal::WebsAtMost(count) => {
            Features::from_grid(current).webs.saturating_sub(count) as i64 * 100_000
                + heuristic(current) / 1_000
        }
        LookupGoal::RatDrop => heuristic(current),
        LookupGoal::RatsAtMostWithCellIs(count, x, y, kind) => {
            let rats_penalty = count_rats(current).saturating_sub(count) as i64 * 1_000_000;
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) == kind {
                0
            } else {
                500_000
            };
            rats_penalty + cell_penalty + heuristic(current) / 1_000
        }
        LookupGoal::RatsAtMostWithCellNot(count, x, y, kind) => {
            let rats_penalty = count_rats(current).saturating_sub(count) as i64 * 1_000_000;
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                500_000
            };
            rats_penalty + cell_penalty + heuristic(current) / 1_000
        }
        LookupGoal::RatsAtMostWithRatInRect(count, x1, y1, x2, y2) => {
            let rats_penalty = count_rats(current).saturating_sub(count) as i64 * 1_000_000;
            let rats = rat_positions(current);
            let rect_penalty = if rats
                .iter()
                .any(|&point| point_in_rect(point, x1, y1, x2, y2))
            {
                0
            } else {
                nearest_rect_distance(&rats, x1, y1, x2, y2)
            };
            rats_penalty + rect_penalty + heuristic(current) / 1_000
        }
        LookupGoal::RatsAtMostWithPlayerAt(count, x, y) => {
            let rats_penalty = count_rats(current).saturating_sub(count) as i64 * 1_000_000;
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            rats_penalty + nearest_target_distance(&players, &[(x, y)]) + heuristic(current) / 1_000
        }
        LookupGoal::RatsAtMostWithPlayerFacing(count, x, y, dir) => {
            let rats_penalty = count_rats(current).saturating_sub(count) as i64 * 1_000_000;
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let facing_penalty = if player_facing(current, (x, y), dir) {
                0
            } else {
                1_000
            };
            rats_penalty
                + nearest_target_distance(&players, &[(x, y)])
                + facing_penalty
                + heuristic(current) / 1_000
        }
        LookupGoal::TriggerNumberOnlyWithCellIs(number, x, y, kind) => {
            let trigger_h =
                lookup_goal_heuristic(LookupGoal::TriggerNumberOnly(number), initial, current);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) == kind {
                0
            } else {
                500_000
            };
            trigger_h + cell_penalty
        }
        LookupGoal::TriggerNumberOnlyWithCellNot(number, x, y, kind) => {
            let trigger_h =
                lookup_goal_heuristic(LookupGoal::TriggerNumberOnly(number), initial, current);
            let cell_penalty = if current.cell_kind_at(x as usize, y as usize) != kind {
                0
            } else {
                500_000
            };
            trigger_h + cell_penalty
        }
    }
}

fn lookup_bfs_progress_score(goal: LookupGoal, initial: &Grid, current: &Grid) -> i64 {
    if lookup_goal_reached(goal, initial, current, PlayState::Playing) {
        return 0;
    }

    match goal {
        LookupGoal::Win => {
            count_rats(current) as i64 * 1_000_000
                + Features::from_grid(current).triggers as i64 * 1_000
                + Features::from_grid(current).explosives as i64 * 100
        }
        LookupGoal::WinReady => {
            count_rats(current) as i64 * 1_000_000
                + Features::from_grid(current).explosives as i64 * 100
                + Features::from_grid(current).webs as i64
        }
        LookupGoal::RectangleReady => {
            rectangle_winready_heuristic(current, count_rats(initial), count_explosives(initial))
        }
        LookupGoal::RectangleLowerReady => {
            rectangle_lower_ready_heuristic(current, count_rats(initial), count_explosives(initial))
        }
        LookupGoal::RectangleLowerSeparated => rectangle_lower_separated_heuristic(
            current,
            count_rats(initial),
            count_explosives(initial),
        ),
        LookupGoal::RectangleLowerIgnition => rectangle_lower_ignition_heuristic(
            current,
            count_rats(initial),
            count_explosives(initial),
        ),
        LookupGoal::TriggerNumber(number) | LookupGoal::TriggerNumberOnly(number) => {
            trigger_count(current, number) as i64 * 1_000_000 + count_rats(current) as i64 * 1_000
        }
        LookupGoal::TriggerNumberOpen(number, reachable_cells) => {
            trigger_count(current, number) as i64 * 1_000_000
                + reachable_cells.saturating_sub(player_reachable_cell_count(current)) as i64
                    * 1_000
                + count_rats(current) as i64 * 1_000
        }
        LookupGoal::CellChanged(x, y) => {
            (current.cell_kind_at(x as usize, y as usize)
                == initial.cell_kind_at(x as usize, y as usize)) as i64
        }
        LookupGoal::CellIs(x, y, kind) => {
            (current.cell_kind_at(x as usize, y as usize) != kind) as i64
        }
        LookupGoal::CellIsWithPlayerAt(x, y, kind, player_x, player_y) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) != kind) as i64;
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let player_distance = nearest_target_distance(&players, &[(player_x, player_y)]);
            cell_missing * 1_000 + player_distance
        }
        LookupGoal::CellNot(x, y, kind) => {
            (current.cell_kind_at(x as usize, y as usize) == kind) as i64
        }
        LookupGoal::CellNotWithCellIs(x, y, not_kind, other_x, other_y, other_kind) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) == not_kind) as i64;
            let other_missing =
                (current.cell_kind_at(other_x as usize, other_y as usize) != other_kind) as i64;
            cell_missing * 1_000 + other_missing * 1_000_000
        }
        LookupGoal::CellNotAndRatAt(x, y, kind, rat_x, rat_y) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) == kind) as i64;
            let rat_missing = !rat_at(current, (rat_x, rat_y)) as i64;
            cell_missing + rat_missing
        }
        LookupGoal::CellNotWithPlayerAt(x, y, kind, player_x, player_y) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) == kind) as i64;
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let player_distance = nearest_target_distance(&players, &[(player_x, player_y)]);
            cell_missing * 1_000 + player_distance
        }
        LookupGoal::CellNotWithPlayerInRect(x, y, kind, px1, py1, px2, py2) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) == kind) as i64;
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            cell_missing * 1_000 + nearest_rect_distance(&players, px1, py1, px2, py2)
        }
        LookupGoal::CellNotWithRatInRect(x, y, kind, rx1, ry1, rx2, ry2) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) == kind) as i64;
            let rats = rat_positions(current);
            let rat_missing = !rats
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                as i64;
            cell_missing * 1_000 + rat_missing
        }
        LookupGoal::CellNotWithNoRatsInRect(x, y, kind, rx1, ry1, rx2, ry2) => {
            let cell_missing = (current.cell_kind_at(x as usize, y as usize) == kind) as i64;
            let rats_in_rect = rat_positions(current)
                .into_iter()
                .filter(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                .count() as i64;
            cell_missing * 1_000 + rats_in_rect
        }
        LookupGoal::CellReachable(x, y) => distance_from_player(current, (x, y)),
        LookupGoal::PlayerAt(x, y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)])
        }
        LookupGoal::PlayerFacing(x, y, dir) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            nearest_target_distance(&players, &[(x, y)])
                + (!player_facing(current, (x, y), dir) as i64)
        }
        LookupGoal::RatAt(x, y) => !rat_at(current, (x, y)) as i64,
        LookupGoal::RatAtFarFromPlayer(x, y, min_distance) => {
            let target = (x, y);
            let rats = rat_positions(current);
            let rat_penalty = if rat_at(current, target) {
                0
            } else {
                1_000_000 + nearest_target_distance(&rats, &[target])
            };
            let nearest_player = nearest_player_distance_from(current, target).unwrap_or(0);
            rat_penalty + min_distance.saturating_sub(nearest_player)
        }
        LookupGoal::RatAtWithPlayer(rat_x, rat_y, player_x, player_y) => {
            let player_missing = !positions_matching(current, |cell| cell == CellKind::Player)
                .contains(&(player_x, player_y)) as i64;
            (!rat_at(current, (rat_x, rat_y)) as i64) + player_missing
        }
        LookupGoal::RatInRectWithPlayerInRect(rx1, ry1, rx2, ry2, px1, py1, px2, py2) => {
            let rats = rat_positions(current);
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let rat_missing = !rats
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                as i64;
            rat_missing * 1_000 + nearest_rect_distance(&players, px1, py1, px2, py2)
        }
        LookupGoal::RatInRectWithPlayerInRectAndCellIs(
            rx1,
            ry1,
            rx2,
            ry2,
            px1,
            py1,
            px2,
            py2,
            cell_x,
            cell_y,
            kind,
        ) => {
            let rats = rat_positions(current);
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            let rat_missing = !rats
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                as i64;
            let cell_missing =
                (current.cell_kind_at(cell_x as usize, cell_y as usize) != kind) as i64;
            rat_missing * 1_000
                + nearest_rect_distance(&players, px1, py1, px2, py2)
                + cell_missing * 1_000_000
        }
        LookupGoal::RatInRectWithCellIs(rx1, ry1, rx2, ry2, cell_x, cell_y, kind) => {
            let rats = rat_positions(current);
            let rat_missing = !rats
                .iter()
                .any(|&point| point_in_rect(point, rx1, ry1, rx2, ry2))
                as i64;
            let cell_missing =
                (current.cell_kind_at(cell_x as usize, cell_y as usize) != kind) as i64;
            rat_missing * 1_000 + cell_missing * 1_000_000
        }
        LookupGoal::RatAtWithPlayerFacing(rat_x, rat_y, player_x, player_y, dir) => {
            let facing_missing = !player_facing(current, (player_x, player_y), dir) as i64;
            (!rat_at(current, (rat_x, rat_y)) as i64) + facing_missing
        }
        LookupGoal::RatAtWithCell(rat_x, rat_y, cell_x, cell_y, kind) => {
            let cell_missing =
                (current.cell_kind_at(cell_x as usize, cell_y as usize) != kind) as i64;
            (!rat_at(current, (rat_x, rat_y)) as i64) + cell_missing
        }
        LookupGoal::RatGone(x, y) => rat_at(current, (x, y)) as i64,
        LookupGoal::NoRatsAt2(x1, y1, x2, y2) => {
            rat_at(current, (x1, y1)) as i64 + rat_at(current, (x2, y2)) as i64
        }
        LookupGoal::NoRatsInRect(x1, y1, x2, y2) => rat_positions(current)
            .into_iter()
            .filter(|&point| point_in_rect(point, x1, y1, x2, y2))
            .count() as i64,
        LookupGoal::RatsAtMost(count) => count_rats(current).saturating_sub(count) as i64,
        LookupGoal::ReachableRatsAtLeast(count) => {
            count.saturating_sub(reachable_rat_count(current)) as i64
        }
        LookupGoal::AllRatsReachable => {
            count_rats(current).saturating_sub(reachable_rat_count(current)) as i64
        }
        LookupGoal::TriggersAtMost(count) => {
            Features::from_grid(current).triggers.saturating_sub(count) as i64
        }
        LookupGoal::ExplosivesAtMost(count) => Features::from_grid(current)
            .explosives
            .saturating_sub(count) as i64,
        LookupGoal::WebsAtMost(count) => {
            Features::from_grid(current).webs.saturating_sub(count) as i64
        }
        LookupGoal::RatDrop => count_rats(current) as i64,
        LookupGoal::RatsAtMostWithCellIs(count, x, y, kind) => {
            count_rats(current).saturating_sub(count) as i64
                + (current.cell_kind_at(x as usize, y as usize) != kind) as i64
        }
        LookupGoal::RatsAtMostWithCellNot(count, x, y, kind) => {
            count_rats(current).saturating_sub(count) as i64
                + (current.cell_kind_at(x as usize, y as usize) == kind) as i64
        }
        LookupGoal::RatsAtMostWithRatInRect(count, x1, y1, x2, y2) => {
            let rats = rat_positions(current);
            let rect_penalty = if rats
                .iter()
                .any(|&point| point_in_rect(point, x1, y1, x2, y2))
            {
                0
            } else {
                nearest_rect_distance(&rats, x1, y1, x2, y2)
            };
            count_rats(current).saturating_sub(count) as i64 + rect_penalty
        }
        LookupGoal::RatsAtMostWithPlayerAt(count, x, y) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            count_rats(current).saturating_sub(count) as i64
                + nearest_target_distance(&players, &[(x, y)])
        }
        LookupGoal::RatsAtMostWithPlayerFacing(count, x, y, dir) => {
            let players = positions_matching(current, |cell| cell == CellKind::Player);
            count_rats(current).saturating_sub(count) as i64
                + nearest_target_distance(&players, &[(x, y)])
                + (!player_facing(current, (x, y), dir) as i64)
        }
        LookupGoal::TriggerNumberOnlyWithCellIs(number, x, y, kind) => {
            trigger_count(current, number) as i64 * 1_000_000
                + (current.cell_kind_at(x as usize, y as usize) != kind) as i64
        }
        LookupGoal::TriggerNumberOnlyWithCellNot(number, x, y, kind) => {
            trigger_count(current, number) as i64 * 1_000_000
                + (current.cell_kind_at(x as usize, y as usize) == kind) as i64
        }
    }
}

fn lookup_dead_state(grid: &Grid) -> bool {
    let features = Features::from_grid(grid);
    features.rats > 0
        && features.explosives == 0
        && reachable_rat_count(grid) == 0
        && reachable_trigger_count(grid) == 0
}

struct PQItem {
    f: i64,
    g: i64,
    idx: usize,
}
impl PartialEq for PQItem {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f && self.g == o.g
    }
}
impl Eq for PQItem {}
impl Ord for PQItem {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        // min-heap on f, tiebreak: prefer larger g (deeper, closer to goal) then smaller
        o.f.cmp(&self.f).then(self.g.cmp(&o.g))
    }
}
impl PartialOrd for PQItem {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

struct NoveltyItem {
    novelty: u8,
    h: i64,
    depth: u32,
    idx: usize,
}
impl PartialEq for NoveltyItem {
    fn eq(&self, other: &Self) -> bool {
        self.novelty == other.novelty && self.h == other.h && self.depth == other.depth
    }
}
impl Eq for NoveltyItem {}
impl Ord for NoveltyItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .novelty
            .cmp(&self.novelty)
            .then(other.h.cmp(&self.h))
            .then(other.depth.cmp(&self.depth))
    }
}
impl PartialOrd for NoveltyItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn format_path(path: &[Vec<Action>]) -> String {
    if path.is_empty() {
        return String::new();
    }
    let nplayers = path[0].len();
    if nplayers == 1 {
        path.iter().map(|a| arrow(a[0])).collect()
    } else {
        path.iter()
            .map(|a| a.iter().map(|x| arrow(*x)).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn format_path_ascii(path: &[Vec<Action>]) -> String {
    if path.is_empty() {
        return String::new();
    }
    let nplayers = path[0].len();
    if nplayers == 1 {
        path.iter().map(|a| action_to_ch(a[0])).collect()
    } else {
        path.iter()
            .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn parse_action_string(action_str: &str, nplayers: usize) -> Vec<Vec<Action>> {
    if nplayers == 1 {
        action_str
            .chars()
            .filter_map(ch_to_action)
            .map(|action| vec![action])
            .collect()
    } else {
        action_str
            .split_whitespace()
            .filter_map(|turn| {
                let acts: Vec<Action> = turn
                    .chars()
                    .filter(|c| *c != '|')
                    .filter_map(ch_to_action)
                    .collect();
                (acts.len() == nplayers).then_some(acts)
            })
            .collect()
    }
}

fn player_facing(grid: &Grid, point: (i32, i32), dir: Dir4) -> bool {
    let (x, y) = point;
    if x < 0 || y < 0 {
        return false;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= grid.width() || y >= grid.height() {
        return false;
    }
    let csv = grid.to_csv();
    let Some(row) = csv.lines().nth(y) else {
        return false;
    };
    let Some(cell) = row.split(',').nth(x) else {
        return false;
    };
    matches!(
        (cell, dir),
        ("▲" | "△", Dir4::North)
            | ("▼" | "▽", Dir4::South)
            | ("►" | "▷", Dir4::East)
            | ("◄" | "◁", Dir4::West)
    )
}

fn replay_path(grid: &Grid, path: &[Vec<Action>]) -> (Grid, PlayState, usize) {
    let mut state = grid.clone();
    let mut result = PlayState::Playing;
    let mut applied = 0;
    for actions in path {
        let (next, play_state) = step(&state, actions);
        state = next;
        result = play_state;
        applied += 1;
        if play_state != PlayState::Playing {
            break;
        }
    }
    (state, result, applied)
}

fn positions_for(grid: &Grid, kind: CellKind) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == kind {
                positions.push((x, y));
            }
        }
    }
    positions
}

fn positions_for_any(grid: &Grid, kinds: &[CellKind]) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if kinds.contains(&grid.cell_kind_at(x, y)) {
                positions.push((x, y));
            }
        }
    }
    positions
}

fn format_positions(positions: &[(usize, usize)]) -> String {
    positions
        .iter()
        .map(|(x, y)| format!("({x},{y})"))
        .collect::<Vec<_>>()
        .join(",")
}

fn print_state_summary(turn: usize, actions: Option<&[Action]>, state: PlayState, grid: &Grid) {
    let features = Features::from_grid(grid);
    let action_text = actions
        .map(|turn_actions| turn_actions.iter().map(|a| action_to_ch(*a)).collect())
        .unwrap_or_else(|| "-".to_string());
    let rats = positions_for_any(grid, &[CellKind::Rat, CellKind::CyborgRat]);
    let players = positions_for(grid, CellKind::Player);
    println!(
        "turn={turn} actions={action_text} state={state:?} rats={} explosives={} webs={} triggers={} planks={} players=[{}] rats_at=[{}]",
        features.rats,
        features.explosives,
        features.webs,
        features.triggers,
        features.planks,
        format_positions(&players),
        format_positions(&rats)
    );
}

fn cell_kind_code(kind: CellKind) -> u32 {
    match kind {
        CellKind::Empty => 0,
        CellKind::Wall => 1,
        CellKind::Player => 2,
        CellKind::Rat => 3,
        CellKind::CyborgRat => 4,
        CellKind::Plank => 5,
        CellKind::Spiderweb => 6,
        CellKind::BlackHole => 7,
        CellKind::Explosive => 8,
        CellKind::Trigger(n) => 9 + n as u32,
    }
}

fn base_cell_kinds(grid: &Grid) -> Vec<CellKind> {
    let mut kinds = Vec::with_capacity(grid.width() * grid.height());
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            kinds.push(grid.cell_kind_at(x, y));
        }
    }
    kinds
}

fn state_atoms(grid: &Grid, base: &[CellKind]) -> Vec<u32> {
    let mut atoms = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let idx = y * grid.width() + x;
            let kind = grid.cell_kind_at(x, y);
            let dynamic_entity = matches!(
                kind,
                CellKind::Player
                    | CellKind::Rat
                    | CellKind::CyborgRat
                    | CellKind::Trigger(_)
                    | CellKind::Explosive
                    | CellKind::Spiderweb
                    | CellKind::Plank
            );
            if dynamic_entity || kind != base[idx] {
                atoms.push((idx as u32) * 32 + cell_kind_code(kind));
            }
        }
    }
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn pair_atom(left: u32, right: u32) -> u64 {
    ((left as u64) << 32) | right as u64
}

struct NoveltyTable {
    max_novelty: u8,
    atoms: HashSet<u32>,
    pairs: HashSet<u64>,
}

impl NoveltyTable {
    fn new(max_novelty: u8) -> Self {
        Self {
            max_novelty,
            atoms: HashSet::new(),
            pairs: HashSet::new(),
        }
    }

    fn novelty(&mut self, atoms: &[u32]) -> Option<u8> {
        let has_new_atom = atoms.iter().any(|atom| !self.atoms.contains(atom));
        if has_new_atom {
            self.record(atoms);
            return Some(1);
        }

        if self.max_novelty >= 2 {
            for i in 0..atoms.len() {
                for j in i + 1..atoms.len() {
                    if !self.pairs.contains(&pair_atom(atoms[i], atoms[j])) {
                        self.record(atoms);
                        return Some(2);
                    }
                }
            }
        }

        None
    }

    fn record(&mut self, atoms: &[u32]) {
        for &atom in atoms {
            self.atoms.insert(atom);
        }
        if self.max_novelty >= 2 {
            for i in 0..atoms.len() {
                for j in i + 1..atoms.len() {
                    self.pairs.insert(pair_atom(atoms[i], atoms[j]));
                }
            }
        }
    }
}

#[must_use]
fn solve(
    grid: &Grid,
    max_depth: usize,
    time_limit_secs: f64,
    strategy: &str,
    weight: i64,
) -> Option<Vec<Vec<Action>>> {
    solve_with_context(grid, max_depth, time_limit_secs, strategy, weight, &[])
}

#[must_use]
fn solve_with_context(
    grid: &Grid,
    max_depth: usize,
    time_limit_secs: f64,
    strategy: &str,
    weight: i64,
    context_prefix: &[Vec<Action>],
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();

    let mut nodes: Vec<Node> = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: vec![],
        depth: 0,
    }];
    // visited: state-hash -> best depth seen
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(grid.state_hash(), 0);

    let mut expansions: u64 = 0;

    if strategy == "bfs" {
        let mut q: VecDeque<usize> = VecDeque::new();
        q.push_back(0);
        while let Some(idx) = q.pop_front() {
            expansions += 1;
            if expansions % 10_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
                return None;
            }
            let cur_grid = nodes[idx].grid.clone();
            let cur_depth = nodes[idx].depth;
            if cur_depth as usize >= max_depth {
                continue;
            }
            for t in &tuples {
                let (ns, st) = step(&cur_grid, t);
                if st == PlayState::GameOver {
                    continue;
                }
                if st == PlayState::Won {
                    let leaf = nodes.len();
                    nodes.push(Node {
                        grid: ns,
                        parent: idx,
                        action: t.clone(),
                        depth: cur_depth + 1,
                    });
                    return Some(reconstruct(&nodes, leaf));
                }
                let hh = ns.state_hash();
                if !visited.contains_key(&hh) {
                    visited.insert(hh, cur_depth + 1);
                    nodes.push(Node {
                        grid: ns,
                        parent: idx,
                        action: t.clone(),
                        depth: cur_depth + 1,
                    });
                    q.push_back(nodes.len() - 1);
                }
            }
        }
        return None;
    }

    // Priority-queue search (astar or gbfs)
    let mut pq: BinaryHeap<PQItem> = BinaryHeap::new();
    let h0 = heuristic(grid);
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });

    let mut best_h_seen = h0;
    let mut best_idx_seen = 0usize;
    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 10_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            let best_path = reconstruct(&nodes, best_idx_seen);
            eprintln!(
                "  [timeout after {expansions} expansions, best_h={best_h_seen}, nodes={}]",
                nodes.len()
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best_path));
            let best_ascii = format_path_ascii(&best_path);
            eprintln!("  BEST_ASCII {}", best_ascii);
            if !context_prefix.is_empty() {
                let mut full_path = context_prefix.to_vec();
                full_path.extend(best_path);
                eprintln!("  BEST_FULL_ASCII {}", format_path_ascii(&full_path));
            }
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx_seen].grid.to_csv());
            return None;
        }
        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g {
            continue; // stale
        }
        if cur_g as usize >= max_depth {
            continue;
        }
        for t in &tuples {
            let (ns, st) = step(&cur_grid, t);
            if st == PlayState::GameOver {
                continue;
            }
            if st == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns,
                    parent: idx,
                    action: t.clone(),
                    depth: (cur_g + 1) as u32,
                });
                return Some(reconstruct(&nodes, leaf));
            }
            let ng = cur_g + 1;
            let hh = ns.state_hash();
            let better = match visited.get(&hh) {
                None => true,
                Some(&pg) => (ng as u32) < pg,
            };
            if better {
                visited.insert(hh, ng as u32);
                let h = heuristic(&ns);
                if h < best_h_seen {
                    best_h_seen = h;
                    best_idx_seen = nodes.len();
                }
                let f = match strategy {
                    "gbfs" => h,          // greedy: ignore g
                    _ => ng + weight * h, // weighted A*
                };
                nodes.push(Node {
                    grid: ns,
                    parent: idx,
                    action: t.clone(),
                    depth: ng as u32,
                });
                pq.push(PQItem {
                    f,
                    g: ng,
                    idx: nodes.len() - 1,
                });
            }
        }
    }
    None
}

fn solve_novelty(
    grid: &Grid,
    max_depth: usize,
    time_limit_secs: f64,
    max_novelty: u8,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();
    let base = base_cell_kinds(grid);
    let mut novelty = NoveltyTable::new(max_novelty);
    let root_atoms = state_atoms(grid, &base);
    assert!(novelty.novelty(&root_atoms).is_some());

    let mut nodes = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(grid.state_hash(), 0);
    let mut pq = BinaryHeap::new();
    pq.push(NoveltyItem {
        novelty: 1,
        h: heuristic(grid),
        depth: 0,
        idx: 0,
    });

    let mut expansions = 0u64;
    let mut best_h = heuristic(grid);
    let mut best_idx = 0usize;
    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 10_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            let best_path = reconstruct(&nodes, best_idx);
            eprintln!(
                "  [novelty timeout after {} expansions, best_h={}, nodes={}]",
                expansions,
                best_h,
                nodes.len()
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best_path));
            let best_ascii: String = if nplayers == 1 {
                best_path.iter().map(|a| action_to_ch(a[0])).collect()
            } else {
                best_path
                    .iter()
                    .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            eprintln!("  BEST_ASCII {}", best_ascii);
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx].grid.to_csv());
            return None;
        }

        let idx = item.idx;
        let cur_depth = nodes[idx].depth;
        if cur_depth as usize >= max_depth {
            continue;
        }
        let cur_grid = nodes[idx].grid.clone();
        for actions in &tuples {
            let (next_grid, play_state) = step(&cur_grid, actions);
            if play_state == PlayState::GameOver {
                continue;
            }
            if play_state == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: next_grid,
                    parent: idx,
                    action: actions.clone(),
                    depth: cur_depth + 1,
                });
                return Some(reconstruct(&nodes, leaf));
            }

            let hash = next_grid.state_hash();
            if let Some(&previous_depth) = visited.get(&hash)
                && previous_depth <= cur_depth + 1
            {
                continue;
            }

            let atoms = state_atoms(&next_grid, &base);
            let Some(novelty_score) = novelty.novelty(&atoms) else {
                continue;
            };

            visited.insert(hash, cur_depth + 1);
            let h = heuristic(&next_grid);
            let node_idx = nodes.len();
            if h < best_h {
                best_h = h;
                best_idx = node_idx;
            }
            nodes.push(Node {
                grid: next_grid,
                parent: idx,
                action: actions.clone(),
                depth: cur_depth + 1,
            });
            pq.push(NoveltyItem {
                novelty: novelty_score,
                h,
                depth: cur_depth + 1,
                idx: node_idx,
            });
        }
    }

    None
}

#[must_use]
fn solve_lookup(
    grid: &Grid,
    max_depth: usize,
    time_limit_secs: f64,
    max_nodes: usize,
    order: LookupOrder,
    weight: i64,
    canonical: bool,
    goal: LookupGoal,
    min_rats: Option<usize>,
    trap_constraints: TrapConstraints,
    progress_every: u64,
) -> Option<(Vec<Vec<Action>>, PlayState)> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();
    let prune_dead = std::env::var("PRUNE_DEAD").is_ok();
    let state_key = |state: &Grid| {
        if canonical {
            state.search_hash()
        } else {
            state.state_hash()
        }
    };

    if min_rats.is_some_and(|minimum| count_rats(grid) < minimum) {
        return None;
    }
    if lookup_goal_reached(goal, grid, grid, PlayState::Playing)
        && trap_constraints.accepts(grid, PlayState::Playing)
    {
        return Some((Vec::new(), PlayState::Playing));
    }

    let mut nodes = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(state_key(grid), 0);
    let mut expansions = 0u64;
    let mut best_h = if order == LookupOrder::Bfs {
        lookup_bfs_progress_score(goal, grid, grid)
    } else {
        lookup_goal_heuristic(goal, grid, grid)
    };
    let mut best_idx = 0usize;

    if order == LookupOrder::Bfs {
        let mut q = VecDeque::new();
        q.push_back(0usize);
        while let Some(idx) = q.pop_front() {
            expansions += 1;
            if progress_every > 0 && expansions % progress_every == 0 {
                eprintln!(
                    "  [lookup expansions={} depth={} queue={} nodes={} visited={} best_h={} elapsed={:.1}s]",
                    expansions,
                    nodes[idx].depth,
                    q.len(),
                    nodes.len(),
                    visited.len(),
                    best_h,
                    start.elapsed().as_secs_f64()
                );
            }
            if start.elapsed().as_secs_f64() > time_limit_secs || nodes.len() >= max_nodes {
                let best_path = reconstruct(&nodes, best_idx);
                eprintln!(
                    "  [lookup stop expansions={} queue={} nodes={} visited={} best_h={} elapsed={:.1}s]",
                    expansions,
                    q.len(),
                    nodes.len(),
                    visited.len(),
                    best_h,
                    start.elapsed().as_secs_f64()
                );
                eprintln!("  BEST_ARROWS {}", format_path(&best_path));
                eprintln!("  BEST_ASCII {}", format_path_ascii(&best_path));
                eprintln!("  BEST_STATE:\n{}", nodes[best_idx].grid.to_csv());
                return None;
            }

            let cur_depth = nodes[idx].depth;
            if cur_depth as usize >= max_depth {
                continue;
            }
            let cur_grid = nodes[idx].grid.clone();
            for actions in &tuples {
                let (next_grid, play_state) = step(&cur_grid, actions);
                if play_state == PlayState::GameOver {
                    continue;
                }
                if play_state != PlayState::Won
                    && min_rats.is_some_and(|minimum| count_rats(&next_grid) < minimum)
                {
                    continue;
                }
                if prune_dead
                    && play_state == PlayState::Playing
                    && !lookup_goal_reached(goal, grid, &next_grid, play_state)
                    && lookup_dead_state(&next_grid)
                {
                    continue;
                }
                let hash = state_key(&next_grid);
                let accepted_goal = play_state == PlayState::Won
                    || (lookup_goal_reached(goal, grid, &next_grid, play_state)
                        && trap_constraints.accepts(&next_grid, play_state));
                if !accepted_goal && visited.contains_key(&hash) {
                    continue;
                }
                let node_idx = nodes.len();
                nodes.push(Node {
                    grid: next_grid.clone(),
                    parent: idx,
                    action: actions.clone(),
                    depth: cur_depth + 1,
                });
                if accepted_goal {
                    return Some((reconstruct(&nodes, node_idx), play_state));
                }

                visited.insert(hash, cur_depth + 1);
                let h = lookup_bfs_progress_score(goal, grid, &next_grid);
                if h < best_h {
                    best_h = h;
                    best_idx = node_idx;
                }
                q.push_back(node_idx);
            }
        }
        return None;
    }

    let mut pq = BinaryHeap::new();
    pq.push(PQItem {
        f: best_h,
        g: 0,
        idx: 0,
    });
    while let Some(item) = pq.pop() {
        expansions += 1;
        if progress_every > 0 && expansions % progress_every == 0 {
            eprintln!(
                "  [lookup expansions={} depth={} open={} nodes={} visited={} best_h={} elapsed={:.1}s]",
                expansions,
                nodes[item.idx].depth,
                pq.len(),
                nodes.len(),
                visited.len(),
                best_h,
                start.elapsed().as_secs_f64()
            );
        }
        if start.elapsed().as_secs_f64() > time_limit_secs || nodes.len() >= max_nodes {
            let best_path = reconstruct(&nodes, best_idx);
            eprintln!(
                "  [lookup stop expansions={} open={} nodes={} visited={} best_h={} elapsed={:.1}s]",
                expansions,
                pq.len(),
                nodes.len(),
                visited.len(),
                best_h,
                start.elapsed().as_secs_f64()
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best_path));
            eprintln!("  BEST_ASCII {}", format_path_ascii(&best_path));
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx].grid.to_csv());
            return None;
        }

        let idx = item.idx;
        let cur_depth = nodes[idx].depth;
        if cur_depth as i64 > item.g || cur_depth as usize >= max_depth {
            continue;
        }
        let cur_grid = nodes[idx].grid.clone();
        for actions in &tuples {
            let (next_grid, play_state) = step(&cur_grid, actions);
            if play_state == PlayState::GameOver {
                continue;
            }
            if play_state != PlayState::Won
                && min_rats.is_some_and(|minimum| count_rats(&next_grid) < minimum)
            {
                continue;
            }
            if prune_dead
                && play_state == PlayState::Playing
                && !lookup_goal_reached(goal, grid, &next_grid, play_state)
                && lookup_dead_state(&next_grid)
            {
                continue;
            }
            let next_depth = cur_depth + 1;
            let hash = state_key(&next_grid);
            if let Some(&previous_depth) = visited.get(&hash)
                && previous_depth <= next_depth
            {
                continue;
            }

            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next_grid.clone(),
                parent: idx,
                action: actions.clone(),
                depth: next_depth,
            });
            if play_state == PlayState::Won
                || (lookup_goal_reached(goal, grid, &next_grid, play_state)
                    && trap_constraints.accepts(&next_grid, play_state))
            {
                return Some((reconstruct(&nodes, node_idx), play_state));
            }

            visited.insert(hash, next_depth);
            let h = lookup_goal_heuristic(goal, grid, &next_grid);
            if h < best_h {
                best_h = h;
                best_idx = node_idx;
            }
            let f = match order {
                LookupOrder::Gbfs => h,
                LookupOrder::Astar => next_depth as i64 + weight * h,
                LookupOrder::Bfs => unreachable!(),
            };
            pq.push(PQItem {
                f,
                g: next_depth as i64,
                idx: node_idx,
            });
        }
    }

    None
}

fn lookup_branch_score(grid: &Grid, path_len: usize) -> i64 {
    let features = Features::from_grid(grid);
    let reachable_rats = reachable_rat_count(grid);
    let unreachable_rats = features.rats.saturating_sub(reachable_rats);
    let reachable_triggers = reachable_trigger_count(grid);
    let trapped_unreachable_rats = trapped_unreachable_rat_count(grid);
    let stranded_remote_penalty = if unreachable_rats > 0 && reachable_triggers == 0 {
        unreachable_rats as i64 * 100_000_000_000
    } else {
        0
    };
    features.rats as i64 * 1_000_000_000
        + unreachable_rats as i64 * 20_000_000_000
        + trapped_unreachable_rats as i64 * 80_000_000_000
        + stranded_remote_penalty
        + resource_exhaustion_penalty(features)
        + features.webs as i64 * 10_000
        + features.planks as i64 * 5_000
        + features.walls as i64 * 500
        + path_len as i64
        - features.explosives as i64 * 50_000
        - features.triggers as i64 * 20_000
}

fn rat_can_step_on(cell: CellKind) -> bool {
    !matches!(
        cell,
        CellKind::Wall | CellKind::Rat | CellKind::CyborgRat | CellKind::Spiderweb
    )
}

fn component_has_local_rat_death(grid: &Grid, start: (usize, usize)) -> bool {
    let mut q = VecDeque::new();
    let mut seen = HashSet::new();
    q.push_back(start);
    seen.insert(start);

    while let Some((x, y)) = q.pop_front() {
        let cell = grid.cell_kind_at(x, y);
        if matches!(cell, CellKind::Explosive | CellKind::BlackHole) {
            return true;
        }

        let dirs = [(0i32, -1i32), (0, 1), (1, 0), (-1, 0)];
        for (dx, dy) in dirs {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx as usize >= grid.width() || ny as usize >= grid.height() {
                continue;
            }
            let next = (nx as usize, ny as usize);
            let next_cell = grid.cell_kind_at(next.0, next.1);
            if next_cell == CellKind::Explosive {
                return true;
            }
            if rat_can_step_on(next_cell) && seen.insert(next) {
                q.push_back(next);
            }
        }
    }

    false
}

fn trapped_unreachable_rat_count(grid: &Grid) -> usize {
    let dist = player_dist_map(grid);
    let mut count = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Rat | CellKind::CyborgRat)
                && dist[y][x] == i32::MAX
                && !component_has_local_rat_death(grid, (x, y))
            {
                count += 1;
            }
        }
    }
    count
}

fn positions_key(grid: &Grid, predicate: fn(CellKind) -> bool) -> String {
    let mut positions = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if predicate(grid.cell_kind_at(x, y)) {
                positions.push(format!("{x},{y}"));
            }
        }
    }
    positions.join(";")
}

fn rat_or_cyborg(cell: CellKind) -> bool {
    matches!(cell, CellKind::Rat | CellKind::CyborgRat)
}

fn player_cell(cell: CellKind) -> bool {
    cell == CellKind::Player
}

fn explosive_cell(cell: CellKind) -> bool {
    cell == CellKind::Explosive
}

fn trigger_cell(cell: CellKind) -> bool {
    matches!(cell, CellKind::Trigger(_))
}

fn web_cell(cell: CellKind) -> bool {
    cell == CellKind::Spiderweb
}

fn plank_cell(cell: CellKind) -> bool {
    cell == CellKind::Plank
}

fn dropchain_diversity_key(grid: &Grid) -> String {
    let features = Features::from_grid(grid);
    format!(
        "r{}:x{}:w{}:t{}:p{}:{}:{}:{}:{}:{}:{}",
        features.rats,
        features.explosives,
        features.webs,
        features.triggers,
        features.planks,
        positions_key(grid, rat_or_cyborg),
        positions_key(grid, player_cell),
        positions_key(grid, explosive_cell),
        positions_key(grid, trigger_cell),
        positions_key(grid, web_cell),
        positions_key(grid, plank_cell)
    )
}

fn solve_lookup_goal_branches(
    grid: &Grid,
    max_depth: usize,
    time_limit_secs: f64,
    max_nodes: usize,
    goal: LookupGoal,
    max_results: usize,
    min_rats: Option<usize>,
    trap_constraints: TrapConstraints,
    canonical: bool,
) -> Vec<Branch> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();
    let prune_dead = std::env::var("PRUNE_DEAD").is_ok();
    let state_key = |state: &Grid| {
        if canonical {
            state.search_hash()
        } else {
            state.state_hash()
        }
    };
    let mut nodes = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited = HashSet::new();
    visited.insert(state_key(grid));
    let mut reached = HashSet::new();
    let mut q = VecDeque::new();
    q.push_back(0usize);
    let mut results = Vec::new();
    let mut expansions = 0u64;
    let raw_result_limit = max_results.saturating_mul(8).max(max_results);

    while let Some(idx) = q.pop_front() {
        expansions += 1;
        if expansions % 256 == 0
            && (start.elapsed().as_secs_f64() > time_limit_secs || nodes.len() >= max_nodes)
        {
            break;
        }
        if results.len() >= raw_result_limit {
            break;
        }

        let cur_depth = nodes[idx].depth;
        if cur_depth as usize >= max_depth {
            continue;
        }
        let cur_grid = nodes[idx].grid.clone();
        for actions in &tuples {
            let (next_grid, play_state) = step(&cur_grid, actions);
            if play_state == PlayState::GameOver {
                continue;
            }
            if prune_dead
                && play_state == PlayState::Playing
                && !lookup_goal_reached(goal, grid, &next_grid, play_state)
                && lookup_dead_state(&next_grid)
            {
                continue;
            }
            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next_grid.clone(),
                parent: idx,
                action: actions.clone(),
                depth: cur_depth + 1,
            });

            let goal_reached = play_state == PlayState::Won
                || lookup_goal_reached(goal, grid, &next_grid, play_state);
            if goal_reached && trap_constraints.accepts(&next_grid, play_state) {
                if play_state != PlayState::Won
                    && min_rats.is_some_and(|min_rats| count_rats(&next_grid) < min_rats)
                {
                    // Keep exploring this branch: the irreversible event may be useful only
                    // after a short stabilization sequence.
                } else {
                    let hash = state_key(&next_grid);
                    if reached.insert(hash) {
                        let path = reconstruct(&nodes, node_idx);
                        results.push(Branch {
                            grid: next_grid,
                            score: lookup_branch_score(&nodes[node_idx].grid, path.len()),
                            path,
                        });
                        results.sort_by_key(|branch| branch.score);
                        results.truncate(raw_result_limit);
                    }
                    continue;
                }
            }

            let hash = state_key(&next_grid);
            if visited.insert(hash) {
                q.push_back(node_idx);
            }
        }
    }

    results.sort_by_key(|branch| branch.score);
    let mut diversity_seen = HashSet::new();
    let mut diverse = Vec::new();
    let mut deferred = Vec::new();
    for branch in results {
        let diversity_key = dropchain_diversity_key(&branch.grid);
        if diversity_seen.insert(diversity_key) {
            diverse.push(branch);
        } else {
            deferred.push(branch);
        }
        if diverse.len() >= max_results {
            break;
        }
    }
    if diverse.len() < max_results {
        for branch in deferred {
            diverse.push(branch);
            if diverse.len() >= max_results {
                break;
            }
        }
    }
    diverse
}

#[must_use]
fn solve_ratdrop_chain(
    grid: &Grid,
    steps: usize,
    segment_depth: usize,
    segment_secs: f64,
    segment_nodes: usize,
    segment_results: usize,
    beam: usize,
    mop_depth: usize,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
) -> Option<Vec<Vec<Action>>> {
    let verbose_candidates = std::env::var("DROPCHAIN_VERBOSE").is_ok();
    let mut frontier = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: lookup_branch_score(grid, 0),
    }];

    for step_idx in 0..steps {
        frontier.sort_by_key(|branch| branch.score);
        if mop_secs > 0.0 {
            for branch in frontier.iter().take(beam.min(frontier.len())) {
                if let Some(mop) = solve_with_context(
                    &branch.grid,
                    mop_depth,
                    mop_secs,
                    mop_strategy,
                    mop_weight,
                    &branch.path,
                ) {
                    let mut path = branch.path.clone();
                    path.extend(mop);
                    return Some(path);
                }
            }
        }

        let mut next_frontier = Vec::new();
        for branch in frontier.iter().take(beam.min(frontier.len())) {
            let before = Features::from_grid(&branch.grid);
            let results = solve_lookup_goal_branches(
                &branch.grid,
                segment_depth,
                segment_secs,
                segment_nodes,
                LookupGoal::RatDrop,
                segment_results,
                None,
                TrapConstraints::default(),
                true,
            );
            eprintln!(
                "  drop step {} branch path={} features={:?} produced {} result(s)",
                step_idx + 1,
                branch.path.len(),
                before,
                results.len()
            );
            for result in results {
                let mut path = branch.path.clone();
                path.extend(result.path);
                let after = Features::from_grid(&result.grid);
                if after.rats == 0 {
                    return Some(path);
                }
                let score = lookup_branch_score(&result.grid, path.len());
                if verbose_candidates {
                    eprintln!(
                        "    candidate path={} features={:?} reachable_rats={} score={} ascii={}",
                        path.len(),
                        after,
                        reachable_rat_count(&result.grid),
                        score,
                        format_path_ascii(&path)
                    );
                }
                next_frontier.push(Branch {
                    grid: result.grid,
                    path,
                    score,
                });
            }
        }

        if next_frontier.is_empty() {
            eprintln!("  drop step {}: no rat-drop branches", step_idx + 1);
            return None;
        }

        next_frontier.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut diversity_seen = HashSet::new();
        let mut deduped = Vec::new();
        let mut deferred = Vec::new();
        for branch in next_frontier {
            if !seen.insert(branch.grid.search_hash()) {
                continue;
            }
            let diversity_key = dropchain_diversity_key(&branch.grid);
            if diversity_seen.insert(diversity_key) {
                deduped.push(branch);
            } else {
                deferred.push(branch);
            }
            if deduped.len() >= beam {
                break;
            }
        }
        if deduped.len() < beam {
            for branch in deferred {
                deduped.push(branch);
                if deduped.len() >= beam {
                    break;
                }
            }
        }
        eprintln!(
            "  drop step {}: kept {} branch(es), best_score={}",
            step_idx + 1,
            deduped.len(),
            deduped[0].score
        );
        eprintln!(
            "  drop step {} best: path={} features={:?} reachable_rats={} ascii={}",
            step_idx + 1,
            deduped[0].path.len(),
            Features::from_grid(&deduped[0].grid),
            reachable_rat_count(&deduped[0].grid),
            format_path_ascii(&deduped[0].path)
        );
        frontier = deduped;
    }

    frontier.sort_by_key(|branch| branch.score);
    if mop_secs > 0.0 {
        for branch in frontier {
            if let Some(mop) = solve_with_context(
                &branch.grid,
                mop_depth,
                mop_secs,
                mop_strategy,
                mop_weight,
                &branch.path,
            ) {
                let mut path = branch.path;
                path.extend(mop);
                return Some(path);
            }
        }
    }

    None
}

/// Find the first player position in a grid.
fn find_player(grid: &Grid) -> Option<(i32, i32)> {
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Player {
                return Some((x as i32, y as i32));
            }
        }
    }
    None
}

fn find_player_positions(grid: &Grid) -> Vec<(i32, i32)> {
    grid.player_positions()
        .into_iter()
        .map(|(x, y)| (x as i32, y as i32))
        .collect()
}

fn manhattan(a: (i32, i32), b: (i32, i32)) -> i64 {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as i64
}

fn target_sum_distance(players: &[(i32, i32)], targets: &[Option<(i32, i32)>]) -> i64 {
    players
        .iter()
        .zip(targets.iter())
        .filter_map(|(&player, &target)| target.map(|target| manhattan(player, target)))
        .sum()
}

fn reached_player_targets(players: &[(i32, i32)], targets: &[Option<(i32, i32)>]) -> bool {
    players.len() >= targets.len()
        && players
            .iter()
            .zip(targets.iter())
            .all(|(&player, &target)| target.is_none_or(|target| player == target))
}

enum SegResult {
    Won(Vec<Vec<Action>>),
    Reached(Grid, Vec<Vec<Action>>),
    Failed,
}

#[derive(Clone)]
struct SegmentBranch {
    grid: Grid,
    path: Vec<Vec<Action>>,
    won: bool,
}

#[derive(Clone)]
struct Branch {
    grid: Grid,
    path: Vec<Vec<Action>>,
    score: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Features {
    rats: usize,
    explosives: usize,
    webs: usize,
    triggers: usize,
    planks: usize,
    walls: usize,
}

impl Features {
    fn from_grid(grid: &Grid) -> Self {
        let mut features = Self {
            rats: 0,
            explosives: 0,
            webs: 0,
            triggers: 0,
            planks: 0,
            walls: 0,
        };
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                match grid.cell_kind_at(x, y) {
                    CellKind::Rat | CellKind::CyborgRat => features.rats += 1,
                    CellKind::Explosive => features.explosives += 1,
                    CellKind::Spiderweb => features.webs += 1,
                    CellKind::Trigger(_) => features.triggers += 1,
                    CellKind::Plank => features.planks += 1,
                    CellKind::Wall => features.walls += 1,
                    _ => {}
                }
            }
        }
        features
    }
}

fn resource_exhaustion_penalty(features: Features) -> i64 {
    if features.rats > 0 && features.triggers == 0 && features.explosives == 0 {
        features.rats as i64 * 20_000_000
    } else {
        0
    }
}

fn reachable_rat_count(grid: &Grid) -> usize {
    let dist = player_dist_map(grid);
    let mut reachable = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Rat | CellKind::CyborgRat)
                && dist[y][x] != i32::MAX
            {
                reachable += 1;
            }
        }
    }
    reachable
}

fn reachable_trigger_count(grid: &Grid) -> usize {
    let dist = player_dist_map(grid);
    let mut reachable = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Trigger(_)) && dist[y][x] != i32::MAX {
                reachable += 1;
            }
        }
    }
    reachable
}

fn nearest_reachable_trigger_distance(grid: &Grid, number: u8) -> Option<i64> {
    let dist = player_dist_map(grid);
    trigger_positions(grid, number)
        .into_iter()
        .filter_map(|(x, y)| {
            let distance = dist[y as usize][x as usize];
            (distance != i32::MAX).then_some(distance as i64)
        })
        .min()
}

fn nearest_reachable_any_trigger_distance(grid: &Grid) -> Option<i64> {
    let dist = player_dist_map(grid);
    let mut best: Option<i64> = None;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Trigger(_)) {
                let distance = dist[y][x];
                if distance != i32::MAX {
                    best =
                        Some(best.map_or(distance as i64, |current| current.min(distance as i64)));
                }
            }
        }
    }
    best
}

fn trigger_continuation_penalty(grid: &Grid, next_trigger: Option<u8>) -> i64 {
    match next_trigger {
        Some(number) => match nearest_reachable_trigger_distance(grid, number) {
            Some(distance) => distance * 500_000,
            None => 100_000_000_000,
        },
        None => {
            if Features::from_grid(grid).triggers == 0 {
                0
            } else {
                match nearest_reachable_any_trigger_distance(grid) {
                    Some(distance) => distance * 100_000,
                    None => 50_000_000_000,
                }
            }
        }
    }
}

fn trigger_branch_score(grid: &Grid, path_len: usize, before: Features, after: Features) -> i64 {
    let useful_gain = before.rats.saturating_sub(after.rats) as i64 * 50_000_000
        + before.webs.saturating_sub(after.webs) as i64 * 200_000
        + before.planks.saturating_sub(after.planks) as i64 * 200_000
        + before.triggers.saturating_sub(after.triggers) as i64 * 50_000;
    lookup_branch_score(grid, path_len) - useful_gain
}

#[derive(Clone)]
struct EventSuccessor {
    grid: Grid,
    path: Vec<Vec<Action>>,
    features: Features,
    score: i64,
}

#[derive(Clone)]
struct MacroNode {
    grid: Grid,
    path: Vec<Vec<Action>>,
}

#[derive(Clone)]
struct BeamState {
    grid: Grid,
    path: Vec<Vec<Action>>,
    score: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetKind {
    Explosion,
    IgnitionReady,
    RatNearExplosive,
    Trigger,
    RatDrop,
    Structural,
}

impl TargetKind {
    fn parse(s: &str) -> Self {
        match s {
            "explosion" | "ignite" => Self::Explosion,
            "ready" | "ignition-ready" => Self::IgnitionReady,
            "ratnearx" | "lure" => Self::RatNearExplosive,
            "trigger" => Self::Trigger,
            "ratdrop" | "rat" => Self::RatDrop,
            "structural" | "any" => Self::Structural,
            other => panic!("unknown event kind {other}"),
        }
    }
}

/// Segment A*: drive the player to `target` cell. Heuristic = Manhattan distance.
/// Returns Won if the level is won en route, or Reached(end_grid, moves) on arrival.
fn solve_segment(
    start: &Grid,
    target: (i32, i32),
    tuples: &[Vec<Action>],
    per_secs: f64,
) -> SegResult {
    let t0 = Instant::now();
    let mut nodes: Vec<Node> = vec![Node {
        grid: start.clone(),
        parent: usize::MAX,
        action: vec![],
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(start.search_hash(), 0);
    let mut pq: BinaryHeap<PQItem> = BinaryHeap::new();
    let p0 = find_player(start).unwrap();
    pq.push(PQItem {
        f: manhattan(p0, target),
        g: 0,
        idx: 0,
    });
    let mut exp: u64 = 0;
    while let Some(item) = pq.pop() {
        exp += 1;
        if exp % 5_000 == 0 && t0.elapsed().as_secs_f64() > per_secs {
            return SegResult::Failed;
        }
        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g {
            continue;
        }
        for t in tuples {
            let (ns, st) = step(&cur_grid, t);
            if st == PlayState::GameOver {
                continue;
            }
            if st == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns,
                    parent: idx,
                    action: t.clone(),
                    depth: (cur_g + 1) as u32,
                });
                return SegResult::Won(reconstruct(&nodes, leaf));
            }
            let pp = match find_player(&ns) {
                Some(p) => p,
                None => continue,
            };
            if pp == target {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns.clone(),
                    parent: idx,
                    action: t.clone(),
                    depth: (cur_g + 1) as u32,
                });
                return SegResult::Reached(ns, reconstruct(&nodes, leaf));
            }
            let ng = cur_g + 1;
            let hh = ns.search_hash();
            let better = match visited.get(&hh) {
                None => true,
                Some(&pg) => (ng as u32) < pg,
            };
            if better {
                visited.insert(hh, ng as u32);
                let h = manhattan(pp, target);
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns,
                    parent: idx,
                    action: t.clone(),
                    depth: ng as u32,
                });
                pq.push(PQItem {
                    f: ng + h,
                    g: ng,
                    idx: leaf,
                });
            }
        }
    }
    SegResult::Failed
}

fn solve_segment_branches(
    start: &Grid,
    target: (i32, i32),
    tuples: &[Vec<Action>],
    per_secs: f64,
    max_results: usize,
) -> Vec<SegmentBranch> {
    let t0 = Instant::now();
    let mut nodes: Vec<Node> = vec![Node {
        grid: start.clone(),
        parent: usize::MAX,
        action: vec![],
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(start.search_hash(), 0);
    let mut reached_hashes = HashSet::new();
    let mut results = Vec::new();
    let mut pq: BinaryHeap<PQItem> = BinaryHeap::new();
    let p0 = find_player(start).unwrap();
    pq.push(PQItem {
        f: manhattan(p0, target),
        g: 0,
        idx: 0,
    });
    let mut exp: u64 = 0;
    while let Some(item) = pq.pop() {
        exp += 1;
        if exp % 5_000 == 0 && t0.elapsed().as_secs_f64() > per_secs {
            break;
        }
        if results.len() >= max_results {
            break;
        }

        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g {
            continue;
        }
        for t in tuples {
            let (ns, st) = step(&cur_grid, t);
            if st == PlayState::GameOver {
                continue;
            }

            let node_idx = nodes.len();
            nodes.push(Node {
                grid: ns.clone(),
                parent: idx,
                action: t.clone(),
                depth: (cur_g + 1) as u32,
            });

            if st == PlayState::Won {
                results.push(SegmentBranch {
                    grid: ns,
                    path: reconstruct(&nodes, node_idx),
                    won: true,
                });
                break;
            }

            let pp = match find_player(&ns) {
                Some(p) => p,
                None => continue,
            };
            let hash = ns.search_hash();
            if pp == target {
                if reached_hashes.insert(hash) {
                    results.push(SegmentBranch {
                        grid: ns,
                        path: reconstruct(&nodes, node_idx),
                        won: false,
                    });
                }
                continue;
            }

            let ng = cur_g + 1;
            let better = match visited.get(&hash) {
                None => true,
                Some(&pg) => (ng as u32) < pg,
            };
            if better {
                visited.insert(hash, ng as u32);
                let h = manhattan(pp, target);
                pq.push(PQItem {
                    f: ng + h,
                    g: ng,
                    idx: node_idx,
                });
            }
        }
    }

    results.sort_by_key(|branch| heuristic(&branch.grid) + branch.path.len() as i64);
    results
}

fn solve_segment_multi(
    start: &Grid,
    targets: &[Option<(i32, i32)>],
    tuples: &[Vec<Action>],
    per_secs: f64,
) -> SegResult {
    let t0 = Instant::now();
    let mut nodes: Vec<Node> = vec![Node {
        grid: start.clone(),
        parent: usize::MAX,
        action: vec![],
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(start.search_hash(), 0);
    let mut pq: BinaryHeap<PQItem> = BinaryHeap::new();
    let players = find_player_positions(start);
    pq.push(PQItem {
        f: target_sum_distance(&players, targets),
        g: 0,
        idx: 0,
    });
    let mut exp: u64 = 0;
    while let Some(item) = pq.pop() {
        exp += 1;
        if exp % 5_000 == 0 && t0.elapsed().as_secs_f64() > per_secs {
            return SegResult::Failed;
        }
        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g {
            continue;
        }
        for t in tuples {
            let (ns, st) = step(&cur_grid, t);
            if st == PlayState::GameOver {
                continue;
            }
            if st == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns,
                    parent: idx,
                    action: t.clone(),
                    depth: (cur_g + 1) as u32,
                });
                return SegResult::Won(reconstruct(&nodes, leaf));
            }
            let players = find_player_positions(&ns);
            if reached_player_targets(&players, targets) {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns.clone(),
                    parent: idx,
                    action: t.clone(),
                    depth: (cur_g + 1) as u32,
                });
                return SegResult::Reached(ns, reconstruct(&nodes, leaf));
            }
            let ng = cur_g + 1;
            let hh = ns.search_hash();
            let better = match visited.get(&hh) {
                None => true,
                Some(&pg) => (ng as u32) < pg,
            };
            if better {
                visited.insert(hh, ng as u32);
                let h = target_sum_distance(&players, targets);
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: ns,
                    parent: idx,
                    action: t.clone(),
                    depth: ng as u32,
                });
                pq.push(PQItem {
                    f: ng + h,
                    g: ng,
                    idx: leaf,
                });
            }
        }
    }
    SegResult::Failed
}

/// Waypoint-guided solve: visit each target cell in order, then mop up remaining rats.
fn solve_waypoints(
    grid: &Grid,
    prefix: Vec<Vec<Action>>,
    waypoints: &[(i32, i32)],
    per_secs: f64,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let (mut cur, prefix_state, applied) = replay_path(grid, &prefix);
    if applied != prefix.len() || prefix_state != PlayState::Playing {
        return if prefix_state == PlayState::Won {
            Some(prefix)
        } else {
            None
        };
    }
    let mut all = prefix;

    // already won?
    if count_rats(&cur) == 0 {
        return Some(all);
    }

    for (i, &wp) in waypoints.iter().enumerate() {
        match solve_segment(&cur, wp, &tuples, per_secs) {
            SegResult::Won(mv) => {
                all.extend(mv);
                eprintln!("  waypoint {} ({},{}) -> WON", i, wp.0, wp.1);
                return Some(all);
            }
            SegResult::Reached(end, mv) => {
                eprintln!(
                    "  waypoint {} ({},{}) reached in {} moves",
                    i,
                    wp.0,
                    wp.1,
                    mv.len()
                );
                let partial_ascii: String = mv
                    .iter()
                    .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ");
                eprintln!("  partial path: {}", partial_ascii);
                eprintln!("  state after wp{}:\n{}", i, end.to_csv());
                all.extend(mv);
                cur = end;
                if count_rats(&cur) == 0 {
                    return Some(all);
                }
            }
            SegResult::Failed => {
                eprintln!("  waypoint {} ({},{}) UNREACHABLE", i, wp.0, wp.1);
                return None;
            }
        }
    }

    // mop-up: rat-count A* from current state
    eprintln!("  waypoints done, mopping up remaining rats...");
    if let Some(mv) = solve_with_context(&cur, depth, mop_secs, mop_strategy, mop_weight, &all) {
        all.extend(mv);
        return Some(all);
    }
    None
}

fn solve_waypoint_pairs(
    grid: &Grid,
    prefix: Vec<Vec<Action>>,
    waypoints: &[Vec<Option<(i32, i32)>>],
    per_secs: f64,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let (mut cur, prefix_state, applied) = replay_path(grid, &prefix);
    if applied != prefix.len() || prefix_state != PlayState::Playing {
        return if prefix_state == PlayState::Won {
            Some(prefix)
        } else {
            None
        };
    }
    let mut all = prefix;

    if count_rats(&cur) == 0 {
        return Some(all);
    }

    for (i, targets) in waypoints.iter().enumerate() {
        match solve_segment_multi(&cur, targets, &tuples, per_secs) {
            SegResult::Won(mv) => {
                all.extend(mv);
                eprintln!("  waypoint-pair {} -> WON", i);
                return Some(all);
            }
            SegResult::Reached(end, mv) => {
                eprintln!("  waypoint-pair {} reached in {} moves", i, mv.len());
                let partial_ascii: String = mv
                    .iter()
                    .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ");
                eprintln!("  partial path: {}", partial_ascii);
                eprintln!("  state after wp{}:\n{}", i, end.to_csv());
                all.extend(mv);
                cur = end;
                if count_rats(&cur) == 0 {
                    return Some(all);
                }
            }
            SegResult::Failed => {
                eprintln!("  waypoint-pair {} UNREACHABLE", i);
                return None;
            }
        }
    }

    eprintln!("  waypoints done, mopping up remaining rats...");
    if let Some(mv) = solve_with_context(&cur, depth, mop_secs, mop_strategy, mop_weight, &all) {
        all.extend(mv);
        return Some(all);
    }
    None
}

fn trigger_positions(grid: &Grid, number: u8) -> Vec<(i32, i32)> {
    let mut positions = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Trigger(number) {
                positions.push((x as i32, y as i32));
            }
        }
    }
    positions
}

fn trigger_numbers(grid: &Grid) -> Vec<u8> {
    let mut numbers = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if let CellKind::Trigger(n) = grid.cell_kind_at(x, y)
                && !numbers.contains(&n)
            {
                numbers.push(n);
            }
        }
    }
    numbers.sort_unstable();
    numbers
}

fn trigger_cells(grid: &Grid) -> Vec<((i32, i32), u8)> {
    let mut cells = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if let CellKind::Trigger(n) = grid.cell_kind_at(x, y) {
                cells.push(((x as i32, y as i32), n));
            }
        }
    }
    cells
}

fn positions_matching<F>(grid: &Grid, mut predicate: F) -> Vec<(i32, i32)>
where
    F: FnMut(CellKind) -> bool,
{
    let mut positions = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if predicate(grid.cell_kind_at(x, y)) {
                positions.push((x as i32, y as i32));
            }
        }
    }
    positions
}

fn rat_near_explosive(grid: &Grid) -> bool {
    let rats = positions_matching(grid, |cell| {
        matches!(cell, CellKind::Rat | CellKind::CyborgRat)
    });
    let explosives = positions_matching(grid, |cell| cell == CellKind::Explosive);
    rats.iter()
        .any(|&rat| explosives.iter().any(|&x| manhattan(rat, x) <= 2))
}

fn rat_at(grid: &Grid, target: (i32, i32)) -> bool {
    target.0 >= 0
        && target.1 >= 0
        && matches!(
            grid.cell_kind_at(target.0 as usize, target.1 as usize),
            CellKind::Rat | CellKind::CyborgRat
        )
}

fn nearest_player_distance_from(grid: &Grid, target: (i32, i32)) -> Option<i64> {
    positions_matching(grid, |cell| cell == CellKind::Player)
        .into_iter()
        .map(|player| manhattan(player, target))
        .min()
}

fn player_at_any(grid: &Grid, targets: &[(i32, i32)]) -> bool {
    targets.is_empty()
        || positions_matching(grid, |cell| cell == CellKind::Player)
            .iter()
            .any(|player| targets.contains(player))
}

fn lure_reached(grid: &Grid, rat_target: (i32, i32), safe_targets: &[(i32, i32)]) -> bool {
    rat_at(grid, rat_target) && player_at_any(grid, safe_targets)
}

fn rat_at_any(grid: &Grid, targets: &[(i32, i32)]) -> bool {
    targets.iter().any(|&target| rat_at(grid, target))
}

fn point_in_rect(point: (i32, i32), x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    point.0 >= min_x && point.0 <= max_x && point.1 >= min_y && point.1 <= max_y
}

fn nearest_rect_distance(points: &[(i32, i32)], x1: i32, y1: i32, x2: i32, y2: i32) -> i64 {
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    points
        .iter()
        .map(|&(x, y)| {
            let dx = if x < min_x {
                min_x - x
            } else if x > max_x {
                x - max_x
            } else {
                0
            };
            let dy = if y < min_y {
                min_y - y
            } else if y > max_y {
                y - max_y
            } else {
                0
            };
            (dx + dy) as i64
        })
        .min()
        .unwrap_or(1_000)
}

fn nearest_target_distance(points: &[(i32, i32)], targets: &[(i32, i32)]) -> i64 {
    points
        .iter()
        .flat_map(|&point| targets.iter().map(move |&target| manhattan(point, target)))
        .min()
        .unwrap_or(1_000)
}

fn lure_heuristic(
    grid: &Grid,
    rat_target: (i32, i32),
    safe_targets: &[(i32, i32)],
    initial_rats: usize,
) -> i64 {
    if lure_reached(grid, rat_target, safe_targets) {
        return 0;
    }

    let rats = rat_positions(grid);
    let rat_distance = rats
        .iter()
        .map(|&rat| manhattan(rat, rat_target))
        .min()
        .unwrap_or(1_000);
    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    let safe_distance = if safe_targets.is_empty() {
        0
    } else {
        nearest_pair_distance(&players, safe_targets)
    };
    let target_clearance = if rat_target.0 < 0
        || rat_target.1 < 0
        || rat_target.0 as usize >= grid.width()
        || rat_target.1 as usize >= grid.height()
    {
        100_000
    } else {
        match grid.cell_kind_at(rat_target.0 as usize, rat_target.1 as usize) {
            CellKind::Spiderweb => {
                let player_to_target = players
                    .iter()
                    .map(|&player| manhattan(player, rat_target))
                    .min()
                    .unwrap_or(1_000);
                20_000 + player_to_target * 200
            }
            CellKind::Wall | CellKind::BlackHole => 100_000,
            _ => 0,
        }
    };
    let lost_rats = initial_rats.saturating_sub(count_rats(grid)) as i64;

    lost_rats * 10_000_000 + rat_distance * 1_000 + safe_distance * 80 + target_clearance
}

fn rectangle_corridor_cells(grid: &Grid) -> Vec<(i32, i32)> {
    if grid.width() < 16 {
        return Vec::new();
    }

    let mut cells = Vec::new();
    for y in 3..=6 {
        for x in 1..=7 {
            cells.push((x, y));
        }
    }
    for y in 6..=8 {
        for x in 13..=15 {
            cells.push((x, y));
        }
    }
    cells
}

fn geom_lure_heuristic(
    grid: &Grid,
    rat_targets: &[(i32, i32)],
    safe_targets: &[(i32, i32)],
    initial_rats: usize,
    initial_explosives: usize,
    preserve_rats: bool,
) -> i64 {
    let current_explosives = count_explosives(grid);
    if current_explosives < initial_explosives {
        return count_rats(grid) as i64;
    }

    let all_rats = rat_positions(grid);
    let lure_rats = tinder_lure_rat_positions(grid);
    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    let rat_on_target = rat_at_any(grid, rat_targets);
    let player_safe = player_at_any(grid, safe_targets);
    if rat_on_target && player_safe {
        return 1;
    }

    let rat_distance = nearest_target_distance(&lure_rats, rat_targets);
    let safe_distance = nearest_target_distance(&players, safe_targets);
    let target_clearance = rat_targets
        .iter()
        .map(|&(x, y)| {
            if x < 0 || y < 0 || x as usize >= grid.width() || y as usize >= grid.height() {
                return 100_000;
            }
            match grid.cell_kind_at(x as usize, y as usize) {
                CellKind::Spiderweb => distance_from_player(grid, (x, y)) * 80 + 20_000,
                CellKind::Wall | CellKind::BlackHole => 100_000,
                _ => 0,
            }
        })
        .min()
        .unwrap_or(100_000);

    let mut corridor_webs = 0i64;
    let mut nearest_corridor_web = 1_000i64;
    for (x, y) in rectangle_corridor_cells(grid) {
        if grid.cell_kind_at(x as usize, y as usize) == CellKind::Spiderweb {
            corridor_webs += 1;
            nearest_corridor_web = nearest_corridor_web.min(distance_from_player(grid, (x, y)));
        }
    }

    let nearest_rat_to_player = players
        .iter()
        .flat_map(|&player| all_rats.iter().map(move |&rat| manhattan(player, rat)))
        .min()
        .unwrap_or(1_000);
    let contact_penalty = if !player_safe && nearest_rat_to_player <= 1 {
        50_000
    } else if !player_safe && nearest_rat_to_player == 2 {
        5_000
    } else {
        0
    };
    let lost_rats = initial_rats.saturating_sub(count_rats(grid)) as i64;
    let lost_rat_penalty = if preserve_rats {
        lost_rats * 10_000_000
    } else {
        lost_rats * 10_000
    };

    if rat_on_target {
        lost_rat_penalty + contact_penalty + safe_distance * 250
    } else {
        lost_rat_penalty
            + contact_penalty
            + target_clearance
            + corridor_webs * 1_200
            + nearest_corridor_web.min(50) * 35
            + rat_distance * 900
            + safe_distance.min(50) * 30
    }
}

fn print_best_search_state(
    label: &str,
    reason: &str,
    expansions: u64,
    best_h_seen: i64,
    nodes: &[Node],
    best_idx_seen: usize,
) {
    let best_path = reconstruct(nodes, best_idx_seen);
    eprintln!(
        "  [{label} {reason} after {expansions} expansions, best_h={}, nodes={}]",
        best_h_seen,
        nodes.len()
    );
    eprintln!("  BEST_ARROWS {}", format_path(&best_path));
    eprintln!("  BEST_ASCII {}", format_path_ascii(&best_path));
    eprintln!("  BEST_STATE:\n{}", nodes[best_idx_seen].grid.to_csv());
}

#[must_use]
fn solve_lure(
    grid: &Grid,
    rat_target: (i32, i32),
    safe_targets: &[(i32, i32)],
    preserve_rats: bool,
    max_depth: usize,
    time_limit_secs: f64,
    max_nodes: usize,
    strategy: &str,
    weight: i64,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let initial_rats = count_rats(grid);
    let start = Instant::now();
    let mut nodes: Vec<Node> = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(grid.state_hash(), 0);
    let h0 = lure_heuristic(grid, rat_target, safe_targets, initial_rats);
    let mut pq = BinaryHeap::new();
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });
    let mut expansions = 0u64;
    let mut best_h_seen = h0;
    let mut best_idx_seen = 0usize;

    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 20_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            print_best_search_state(
                "lure",
                "timeout",
                expansions,
                best_h_seen,
                &nodes,
                best_idx_seen,
            );
            return None;
        }

        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g || cur_g as usize >= max_depth {
            continue;
        }

        for actions in &tuples {
            let (next_grid, play_state) = step(&cur_grid, actions);
            if play_state == PlayState::GameOver {
                continue;
            }
            if play_state == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node {
                    grid: next_grid,
                    parent: idx,
                    action: actions.clone(),
                    depth: (cur_g + 1) as u32,
                });
                return Some(reconstruct(&nodes, leaf));
            }
            if preserve_rats && count_rats(&next_grid) < initial_rats {
                continue;
            }

            let hash = next_grid.state_hash();
            let next_depth = (cur_g + 1) as u32;
            if let Some(&previous_depth) = visited.get(&hash)
                && previous_depth <= next_depth
            {
                continue;
            }
            if nodes.len() >= max_nodes {
                print_best_search_state(
                    "lure",
                    "node-limit",
                    expansions,
                    best_h_seen,
                    &nodes,
                    best_idx_seen,
                );
                return None;
            }

            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next_grid,
                parent: idx,
                action: actions.clone(),
                depth: next_depth,
            });
            if lure_reached(&nodes[node_idx].grid, rat_target, safe_targets) {
                return Some(reconstruct(&nodes, node_idx));
            }

            visited.insert(hash, next_depth);
            let h = lure_heuristic(
                &nodes[node_idx].grid,
                rat_target,
                safe_targets,
                initial_rats,
            );
            if h < best_h_seen {
                best_h_seen = h;
                best_idx_seen = node_idx;
            }
            let f = match strategy {
                "gbfs" => h,
                _ => cur_g + 1 + weight * h,
            };
            pq.push(PQItem {
                f,
                g: cur_g + 1,
                idx: node_idx,
            });
        }
    }

    None
}

#[must_use]
fn solve_geom_lure(
    grid: &Grid,
    rat_targets: &[(i32, i32)],
    safe_targets: &[(i32, i32)],
    preserve_rats: bool,
    max_depth: usize,
    time_limit_secs: f64,
    max_nodes: usize,
    strategy: &str,
    weight: i64,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let initial_rats = count_rats(grid);
    let initial_explosives = count_explosives(grid);
    let start = Instant::now();
    let mut nodes: Vec<Node> = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(grid.state_hash(), 0);
    let h0 = geom_lure_heuristic(
        grid,
        rat_targets,
        safe_targets,
        initial_rats,
        initial_explosives,
        preserve_rats,
    );
    let mut pq = BinaryHeap::new();
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });
    let mut expansions = 0u64;
    let mut best_h_seen = h0;
    let mut best_idx_seen = 0usize;

    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 20_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            print_best_search_state(
                "geomlure",
                "timeout",
                expansions,
                best_h_seen,
                &nodes,
                best_idx_seen,
            );
            return None;
        }

        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g || cur_g as usize >= max_depth {
            continue;
        }

        if let Some(actions) = winning_action(&cur_grid, &tuples) {
            let mut path = reconstruct(&nodes, idx);
            path.push(actions);
            return Some(path);
        }

        for actions in &tuples {
            let (next_grid, play_state) = step(&cur_grid, actions);
            if play_state == PlayState::GameOver {
                continue;
            }
            if nodes.len() >= max_nodes {
                print_best_search_state(
                    "geomlure",
                    "node-limit",
                    expansions,
                    best_h_seen,
                    &nodes,
                    best_idx_seen,
                );
                return None;
            }
            let node_idx = nodes.len();
            let next_depth = nodes[idx].depth + 1;
            nodes.push(Node {
                grid: next_grid.clone(),
                parent: idx,
                action: actions.clone(),
                depth: next_depth,
            });
            if play_state == PlayState::Won {
                return Some(reconstruct(&nodes, node_idx));
            }
            if preserve_rats && count_rats(&next_grid) < initial_rats {
                continue;
            }

            let hash = next_grid.state_hash();
            let better = match visited.get(&hash) {
                None => true,
                Some(&previous_depth) => next_depth < previous_depth,
            };
            if !better {
                continue;
            }
            visited.insert(hash, next_depth);

            let h = geom_lure_heuristic(
                &next_grid,
                rat_targets,
                safe_targets,
                initial_rats,
                initial_explosives,
                preserve_rats,
            );
            if h < best_h_seen {
                best_h_seen = h;
                best_idx_seen = node_idx;
            }
            let f = match strategy {
                "gbfs" => h,
                _ => cur_g + 1 + weight * h,
            };
            pq.push(PQItem {
                f,
                g: cur_g + 1,
                idx: node_idx,
            });
        }
    }

    None
}

fn ignition_ready(grid: &Grid) -> bool {
    let nplayers = count_players(grid);
    let Ok(tuples) = std::panic::catch_unwind(|| all_action_tuples(nplayers)) else {
        return false;
    };
    let current = Features::from_grid(grid);
    tuples.iter().any(|actions| {
        let (next, play_state) = step(grid, actions);
        play_state != PlayState::GameOver
            && Features::from_grid(&next).explosives < current.explosives
    })
}

fn target_reached(target: TargetKind, initial: Features, current: Features, grid: &Grid) -> bool {
    match target {
        TargetKind::Explosion => current.explosives < initial.explosives,
        TargetKind::IgnitionReady => ignition_ready(grid),
        TargetKind::RatNearExplosive => rat_near_explosive(grid),
        TargetKind::Trigger => current.triggers < initial.triggers,
        TargetKind::RatDrop => current.rats < initial.rats,
        TargetKind::Structural => current != initial,
    }
}

fn nearest_pair_distance(a: &[(i32, i32)], b: &[(i32, i32)]) -> i64 {
    a.iter()
        .flat_map(|&left| b.iter().map(move |&right| manhattan(left, right)))
        .min()
        .unwrap_or(1000)
}

fn event_heuristic(grid: &Grid, target: TargetKind, initial: Features) -> i64 {
    let current = Features::from_grid(grid);
    if target_reached(target, initial, current, grid) {
        return 0;
    }

    match target {
        TargetKind::Explosion | TargetKind::IgnitionReady | TargetKind::RatNearExplosive => {
            if std::env::var("TRAP_H").is_ok()
                && target != TargetKind::RatNearExplosive
                && grid.width() >= 16
            {
                return rectangle_trap_heuristic(grid, initial.rats, initial.explosives);
            }
            let rats = if std::env::var("LURE_H").is_ok() {
                tinder_lure_rat_positions(grid)
            } else {
                positions_matching(grid, |cell| {
                    matches!(cell, CellKind::Rat | CellKind::CyborgRat)
                })
            };
            let explosives = positions_matching(grid, |cell| cell == CellKind::Explosive);
            let distance = nearest_pair_distance(&rats, &explosives);
            if target == TargetKind::RatNearExplosive {
                distance.saturating_sub(1)
            } else {
                distance
            }
        }
        TargetKind::Trigger => {
            let players = positions_matching(grid, |cell| cell == CellKind::Player);
            let triggers = positions_matching(grid, |cell| matches!(cell, CellKind::Trigger(_)));
            nearest_pair_distance(&players, &triggers)
        }
        TargetKind::RatDrop => heuristic(grid),
        TargetKind::Structural => {
            let players = positions_matching(grid, |cell| cell == CellKind::Player);
            let interesting = positions_matching(grid, |cell| {
                matches!(
                    cell,
                    CellKind::Rat
                        | CellKind::CyborgRat
                        | CellKind::Trigger(_)
                        | CellKind::Explosive
                )
            });
            nearest_pair_distance(&players, &interesting)
        }
    }
}

fn parse_trigger_order(s: &str) -> Vec<u8> {
    s.split([',', ';'])
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.parse().expect("trigger number"))
            }
        })
        .collect()
}

#[must_use]
fn parse_trap_constraint_arg(
    args: &[String],
    index: usize,
    trap_constraints: &mut TrapConstraints,
) -> Option<usize> {
    match args[index].as_str() {
        "--min-reachable-rats" | "--min-reachable" => {
            trap_constraints.min_reachable_rats = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--max-trapped-rats" | "--max-trapped" => {
            trap_constraints.max_trapped_rats = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--min-reachable-cells" => {
            trap_constraints.min_reachable_cells = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--min-reachable-triggers" => {
            trap_constraints.min_reachable_triggers = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--require-reachable-trigger" | "--next-trigger" => {
            trap_constraints.require_reachable_trigger = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--require-reachable-cell" | "--cleanup-cell" => {
            trap_constraints.require_reachable_cell = parse_optional_point(&args[index + 1]);
            Some(index + 2)
        }
        "--min-explosives" => {
            trap_constraints.min_explosives = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--min-triggers" => {
            trap_constraints.min_triggers = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        "--max-webs" => {
            trap_constraints.max_webs = Some(args[index + 1].parse().unwrap());
            Some(index + 2)
        }
        _ => None,
    }
}

fn parse_optional_point(s: &str) -> Option<(i32, i32)> {
    let trimmed = s.trim();
    if trimmed == "." || trimmed == "_" || trimmed.eq_ignore_ascii_case("any") {
        return None;
    }
    let mut parts = trimmed.split(',');
    Some((
        parts.next().unwrap().trim().parse().unwrap(),
        parts.next().unwrap().trim().parse().unwrap(),
    ))
}

fn parse_points(s: &str) -> Vec<(i32, i32)> {
    s.split(';').filter_map(parse_optional_point).collect()
}

fn parse_waypoint_pairs(s: &str, nplayers: usize) -> Vec<Vec<Option<(i32, i32)>>> {
    s.split(';')
        .filter_map(|turn| {
            let turn = turn.trim();
            if turn.is_empty() {
                return None;
            }
            let mut targets: Vec<_> = turn.split('|').map(parse_optional_point).collect();
            while targets.len() < nplayers {
                targets.push(None);
            }
            targets.truncate(nplayers);
            Some(targets)
        })
        .collect()
}

fn solve_trigger_order(
    grid: &Grid,
    order: &[u8],
    per_secs: f64,
    beam: usize,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let segment_results = std::env::var("SEG_RESULTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8usize);
    let mut branches = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];

    for &number in order {
        let mut next_branches = Vec::new();
        for branch in &branches {
            for target in trigger_positions(&branch.grid, number) {
                for segment in
                    solve_segment_branches(&branch.grid, target, &tuples, per_secs, segment_results)
                {
                    if segment.won {
                        let mut path = branch.path.clone();
                        path.extend(segment.path);
                        return Some(path);
                    }

                    let before = Features::from_grid(&branch.grid);
                    let after = Features::from_grid(&segment.grid);
                    let mut path = branch.path.clone();
                    path.extend(segment.path);
                    let score = trigger_branch_score(&segment.grid, path.len(), before, after);
                    next_branches.push(Branch {
                        grid: segment.grid,
                        path,
                        score,
                    });
                }
            }
        }

        if next_branches.is_empty() {
            eprintln!("  trigger {}: no reachable branches", number);
            return None;
        }

        next_branches.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut diversity_seen = HashSet::new();
        let mut deduped = Vec::new();
        let mut deferred = Vec::new();
        for branch in next_branches {
            if !seen.insert(branch.grid.search_hash()) {
                continue;
            }
            let diversity_key = dropchain_diversity_key(&branch.grid);
            if diversity_seen.insert(diversity_key) {
                deduped.push(branch);
            } else {
                deferred.push(branch);
            }
            if deduped.len() >= beam {
                break;
            }
        }
        if deduped.len() < beam {
            for branch in deferred {
                deduped.push(branch);
                if deduped.len() >= beam {
                    break;
                }
            }
        }
        eprintln!(
            "  trigger {}: kept {} branch(es), best_score={}",
            number,
            deduped.len(),
            deduped[0].score
        );
        branches = deduped;
    }

    branches.sort_by_key(|branch| branch.score);
    for branch in branches {
        eprintln!("  trigger order done, mopping from score={}", branch.score);
        if let Some(mop) = solve_with_context(
            &branch.grid,
            depth,
            mop_secs,
            mop_strategy,
            mop_weight,
            &branch.path,
        ) {
            let mut path = branch.path;
            path.extend(mop);
            return Some(path);
        }
    }

    None
}

#[must_use]
fn solve_trigger_order_lookup(
    grid: &Grid,
    order: &[u8],
    segment_depth: usize,
    segment_secs: f64,
    segment_nodes: usize,
    segment_results: usize,
    beam: usize,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
    strict_trigger_order: bool,
    trap_constraints: TrapConstraints,
) -> Option<Vec<Vec<Action>>> {
    let mut branches = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];

    for (order_idx, &number) in order.iter().enumerate() {
        branches.sort_by_key(|branch| branch.score);
        if mop_secs > 0.0 {
            for branch in branches.iter().take(beam.min(branches.len())) {
                if let Some(mop) = solve_with_context(
                    &branch.grid,
                    depth,
                    mop_secs,
                    mop_strategy,
                    mop_weight,
                    &branch.path,
                ) {
                    let mut path = branch.path.clone();
                    path.extend(mop);
                    return Some(path);
                }
            }
        }

        let mut next_branches = Vec::new();
        for branch in branches.iter().take(beam.min(branches.len())) {
            let before = Features::from_grid(&branch.grid);
            let goal = if strict_trigger_order {
                LookupGoal::TriggerNumberOnly(number)
            } else {
                LookupGoal::TriggerNumber(number)
            };
            let results = solve_lookup_goal_branches(
                &branch.grid,
                segment_depth,
                segment_secs,
                segment_nodes,
                goal,
                segment_results,
                None,
                trap_constraints,
                true,
            );
            eprintln!(
                "  trigger {} lookup branch path={} features={:?} produced {} result(s)",
                number,
                branch.path.len(),
                before,
                results.len()
            );
            for result in results {
                let mut path = branch.path.clone();
                path.extend(result.path);
                let after = Features::from_grid(&result.grid);
                if after.rats == 0 {
                    return Some(path);
                }
                let remaining_next = order[order_idx + 1..]
                    .iter()
                    .copied()
                    .find(|&candidate| trigger_count(&result.grid, candidate) > 0);
                let score = trigger_branch_score(&result.grid, path.len(), before, after)
                    + trigger_continuation_penalty(&result.grid, remaining_next);
                next_branches.push(Branch {
                    grid: result.grid,
                    path,
                    score,
                });
            }
        }

        if next_branches.is_empty() {
            eprintln!("  trigger {} lookup: no reachable branches", number);
            return None;
        }

        next_branches.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut diversity_seen = HashSet::new();
        let mut deduped = Vec::new();
        let mut deferred = Vec::new();
        for branch in next_branches {
            if !seen.insert(branch.grid.search_hash()) {
                continue;
            }
            let diversity_key = dropchain_diversity_key(&branch.grid);
            if diversity_seen.insert(diversity_key) {
                deduped.push(branch);
            } else {
                deferred.push(branch);
            }
            if deduped.len() >= beam {
                break;
            }
        }
        if deduped.len() < beam {
            for branch in deferred {
                deduped.push(branch);
                if deduped.len() >= beam {
                    break;
                }
            }
        }
        eprintln!(
            "  trigger {} lookup: kept {} branch(es), best_score={}",
            number,
            deduped.len(),
            deduped[0].score
        );
        eprintln!(
            "  trigger {} lookup best: path={} features={:?} reachable_rats={} ascii={}",
            number,
            deduped[0].path.len(),
            Features::from_grid(&deduped[0].grid),
            reachable_rat_count(&deduped[0].grid),
            format_path_ascii(&deduped[0].path)
        );
        branches = deduped;
    }

    branches.sort_by_key(|branch| branch.score);
    for branch in branches {
        eprintln!(
            "  trigger lookup order done, mopping from score={}",
            branch.score
        );
        if let Some(mop) = solve_with_context(
            &branch.grid,
            depth,
            mop_secs,
            mop_strategy,
            mop_weight,
            &branch.path,
        ) {
            let mut path = branch.path;
            path.extend(mop);
            return Some(path);
        }
    }

    None
}

#[must_use]
fn solve_any_trigger_order_lookup(
    grid: &Grid,
    macro_steps: usize,
    segment_depth: usize,
    segment_secs: f64,
    segment_nodes: usize,
    segment_results: usize,
    beam: usize,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
    trap_constraints: TrapConstraints,
) -> Option<Vec<Vec<Action>>> {
    let mut branches = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];

    for step_idx in 0..macro_steps {
        branches.sort_by_key(|branch| branch.score);
        if mop_secs > 0.0 {
            for branch in branches.iter().take(beam.min(branches.len())) {
                if let Some(mop) = solve_with_context(
                    &branch.grid,
                    depth,
                    mop_secs,
                    mop_strategy,
                    mop_weight,
                    &branch.path,
                ) {
                    let mut path = branch.path.clone();
                    path.extend(mop);
                    return Some(path);
                }
            }
        }

        let mut next_branches = Vec::new();
        for branch in branches.iter().take(beam.min(branches.len())) {
            let before = Features::from_grid(&branch.grid);
            let numbers = trigger_numbers(&branch.grid);
            for number in numbers {
                let results = solve_lookup_goal_branches(
                    &branch.grid,
                    segment_depth,
                    segment_secs,
                    segment_nodes,
                    LookupGoal::TriggerNumber(number),
                    segment_results,
                    None,
                    trap_constraints,
                    true,
                );
                if results.is_empty() {
                    continue;
                }
                eprintln!(
                    "  trigger-any lookup step {} branch path={} trigger={} features={:?} produced {} result(s)",
                    step_idx + 1,
                    branch.path.len(),
                    number,
                    before,
                    results.len()
                );
                for result in results {
                    let mut path = branch.path.clone();
                    path.extend(result.path);
                    let after = Features::from_grid(&result.grid);
                    if after.rats == 0 {
                        return Some(path);
                    }
                    let score = trigger_branch_score(&result.grid, path.len(), before, after)
                        + trigger_continuation_penalty(&result.grid, None);
                    next_branches.push(Branch {
                        grid: result.grid,
                        path,
                        score,
                    });
                }
            }
        }

        if next_branches.is_empty() {
            eprintln!(
                "  trigger-any lookup step {}: no reachable branches",
                step_idx + 1
            );
            return None;
        }

        next_branches.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut diversity_seen = HashSet::new();
        let mut deduped = Vec::new();
        let mut deferred = Vec::new();
        for branch in next_branches {
            if !seen.insert(branch.grid.search_hash()) {
                continue;
            }
            let diversity_key = dropchain_diversity_key(&branch.grid);
            if diversity_seen.insert(diversity_key) {
                deduped.push(branch);
            } else {
                deferred.push(branch);
            }
            if deduped.len() >= beam {
                break;
            }
        }
        if deduped.len() < beam {
            for branch in deferred {
                deduped.push(branch);
                if deduped.len() >= beam {
                    break;
                }
            }
        }
        eprintln!(
            "  trigger-any lookup step {}: kept {} branch(es), best_score={}",
            step_idx + 1,
            deduped.len(),
            deduped[0].score
        );
        eprintln!(
            "  trigger-any lookup step {} best: path={} features={:?} reachable_rats={} ascii={}",
            step_idx + 1,
            deduped[0].path.len(),
            Features::from_grid(&deduped[0].grid),
            reachable_rat_count(&deduped[0].grid),
            format_path_ascii(&deduped[0].path)
        );
        branches = deduped;
    }

    branches.sort_by_key(|branch| branch.score);
    for branch in branches {
        if let Some(mop) = solve_with_context(
            &branch.grid,
            depth,
            mop_secs,
            mop_strategy,
            mop_weight,
            &branch.path,
        ) {
            let mut path = branch.path;
            path.extend(mop);
            return Some(path);
        }
    }

    None
}

fn solve_any_trigger_order(
    grid: &Grid,
    macro_steps: usize,
    per_secs: f64,
    beam: usize,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let verbose_candidates = std::env::var("TRIG_VERBOSE").is_ok();
    let segment_results = std::env::var("SEG_RESULTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8usize);
    let mut branches = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];

    for step_idx in 0..macro_steps {
        branches.sort_by_key(|branch| branch.score);
        if mop_secs > 0.0 {
            for branch in branches.iter().take(beam.min(branches.len())) {
                if let Some(mop) = solve_with_context(
                    &branch.grid,
                    depth,
                    mop_secs,
                    mop_strategy,
                    mop_weight,
                    &branch.path,
                ) {
                    let mut path = branch.path.clone();
                    path.extend(mop);
                    return Some(path);
                }
            }
        }

        let mut next_branches = Vec::new();
        for branch in &branches {
            let cells = trigger_cells(&branch.grid);
            for (target, number) in cells {
                for segment in
                    solve_segment_branches(&branch.grid, target, &tuples, per_secs, segment_results)
                {
                    if segment.won {
                        let mut path = branch.path.clone();
                        path.extend(segment.path);
                        return Some(path);
                    }

                    let before = Features::from_grid(&branch.grid);
                    let after = Features::from_grid(&segment.grid);
                    if after == before && branch.grid.search_hash() == segment.grid.search_hash() {
                        continue;
                    }
                    let mut path = branch.path.clone();
                    path.extend(segment.path);
                    let score = trigger_branch_score(&segment.grid, path.len(), before, after);
                    if verbose_candidates {
                        eprintln!(
                            "  step {} trigger {} at ({},{}): path={} features={:?} score={} path_ascii={}",
                            step_idx + 1,
                            number,
                            target.0,
                            target.1,
                            path.len(),
                            after,
                            score,
                            format_path_ascii(&path)
                        );
                    }
                    next_branches.push(Branch {
                        grid: segment.grid,
                        path,
                        score,
                    });
                }
            }
        }

        if next_branches.is_empty() {
            eprintln!("  trigger step {}: no reachable branches", step_idx + 1);
            return None;
        }

        next_branches.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut diversity_seen = HashSet::new();
        let mut deduped = Vec::new();
        let mut deferred = Vec::new();
        for branch in next_branches {
            if !seen.insert(branch.grid.search_hash()) {
                continue;
            }
            let diversity_key = dropchain_diversity_key(&branch.grid);
            if diversity_seen.insert(diversity_key) {
                deduped.push(branch);
            } else {
                deferred.push(branch);
            }
            if deduped.len() >= beam {
                break;
            }
        }
        if deduped.len() < beam {
            for branch in deferred {
                deduped.push(branch);
                if deduped.len() >= beam {
                    break;
                }
            }
        }
        eprintln!(
            "  trigger step {}: kept {} branch(es), best_score={}",
            step_idx + 1,
            deduped.len(),
            deduped[0].score
        );
        branches = deduped;
    }

    branches.sort_by_key(|branch| branch.score);
    for branch in branches {
        if let Some(mop) = solve_with_context(
            &branch.grid,
            depth,
            mop_secs,
            mop_strategy,
            mop_weight,
            &branch.path,
        ) {
            let mut path = branch.path;
            path.extend(mop);
            return Some(path);
        }
    }

    None
}

fn find_event_successors(
    start_grid: &Grid,
    tuples: &[Vec<Action>],
    max_depth: usize,
    time_limit_secs: f64,
    max_events: usize,
) -> Option<Vec<EventSuccessor>> {
    let start_time = Instant::now();
    let start_features = Features::from_grid(start_grid);
    let mut nodes: Vec<Node> = vec![Node {
        grid: start_grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited = HashSet::new();
    visited.insert(start_grid.search_hash());
    let mut q = VecDeque::new();
    q.push_back(0usize);
    let mut events = Vec::new();
    let mut event_hashes = HashSet::new();
    let mut expansions = 0u64;

    while let Some(idx) = q.pop_front() {
        expansions += 1;
        if expansions % 5_000 == 0 && start_time.elapsed().as_secs_f64() > time_limit_secs {
            break;
        }

        let cur_depth = nodes[idx].depth;
        if cur_depth as usize >= max_depth {
            continue;
        }
        let cur_grid = nodes[idx].grid.clone();
        for t in tuples {
            let (next_grid, play_state) = step(&cur_grid, t);
            if play_state == PlayState::GameOver {
                continue;
            }

            let hash = next_grid.search_hash();
            if !visited.insert(hash) {
                continue;
            }

            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next_grid.clone(),
                parent: idx,
                action: t.clone(),
                depth: cur_depth + 1,
            });

            if play_state == PlayState::Won {
                return Some(vec![EventSuccessor {
                    grid: next_grid,
                    path: reconstruct(&nodes, node_idx),
                    features: Features::from_grid(&nodes[node_idx].grid),
                    score: i64::MIN,
                }]);
            }

            let features = Features::from_grid(&next_grid);
            if features != start_features {
                if event_hashes.insert(hash) {
                    let path = reconstruct(&nodes, node_idx);
                    let score = trigger_branch_score(
                        &nodes[node_idx].grid,
                        path.len(),
                        start_features,
                        features,
                    );
                    events.push(EventSuccessor {
                        grid: next_grid,
                        score,
                        path,
                        features,
                    });
                    events.sort_by_key(|event| event.score);
                    events.truncate(max_events);
                }
            } else {
                q.push_back(node_idx);
            }
        }
    }

    if events.is_empty() {
        None
    } else {
        Some(events)
    }
}

fn solve_macro_events(
    grid: &Grid,
    segment_depth: usize,
    segment_secs: f64,
    event_beam: usize,
    max_events: usize,
    total_secs: f64,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start_time = Instant::now();
    let mut nodes: Vec<MacroNode> = vec![MacroNode {
        grid: grid.clone(),
        path: Vec::new(),
    }];
    let mut pq = BinaryHeap::new();
    let h0 = lookup_branch_score(grid, 0);
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });
    let mut visited = HashSet::new();
    visited.insert(grid.search_hash());
    let mut expansions = 0u64;
    let mut best_idx = 0usize;
    let mut best_score = h0;

    while let Some(item) = pq.pop() {
        expansions += 1;
        if start_time.elapsed().as_secs_f64() > total_secs {
            eprintln!(
                "  [macro timeout after {} event nodes, stored={}, best_score={}]",
                expansions,
                nodes.len(),
                best_score
            );
            eprintln!("  BEST_ARROWS {}", format_path(&nodes[best_idx].path));
            eprintln!("  BEST_ASCII {}", format_path_ascii(&nodes[best_idx].path));
            eprintln!(
                "  BEST_FEATURES {:?}",
                Features::from_grid(&nodes[best_idx].grid)
            );
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx].grid.to_csv());
            return None;
        }
        if nodes.len() >= max_events {
            eprintln!(
                "  [macro event limit reached, stored={}, best_score={}]",
                nodes.len(),
                best_score
            );
            eprintln!("  BEST_ARROWS {}", format_path(&nodes[best_idx].path));
            eprintln!("  BEST_ASCII {}", format_path_ascii(&nodes[best_idx].path));
            eprintln!(
                "  BEST_FEATURES {:?}",
                Features::from_grid(&nodes[best_idx].grid)
            );
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx].grid.to_csv());
            return None;
        }

        let idx = item.idx;
        let prefix = nodes[idx].path.clone();
        let current = nodes[idx].grid.clone();
        let Some(events) =
            find_event_successors(&current, &tuples, segment_depth, segment_secs, event_beam)
        else {
            continue;
        };

        for event in events.into_iter().take(event_beam) {
            let mut path = prefix.clone();
            path.extend(event.path);
            if event.features.rats == 0 {
                return Some(path);
            }

            let hash = event.grid.search_hash();
            if !visited.insert(hash) {
                continue;
            }

            let depth = path.len() as u32;
            let f = event.score + depth as i64;
            let node_idx = nodes.len();
            nodes.push(MacroNode {
                grid: event.grid,
                path,
            });
            if f < best_score {
                best_score = f;
                best_idx = node_idx;
            }
            pq.push(PQItem {
                f,
                g: depth as i64,
                idx: node_idx,
            });
        }
    }

    None
}

fn solve_to_event(
    grid: &Grid,
    target: TargetKind,
    max_depth: usize,
    time_limit_secs: f64,
    strategy: &str,
    weight: i64,
) -> Option<(Vec<Vec<Action>>, Grid)> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let initial_features = Features::from_grid(grid);
    let start = Instant::now();
    let mut nodes: Vec<Node> = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited = HashSet::new();
    visited.insert(grid.search_hash());

    if strategy == "bfs" {
        let mut q = VecDeque::new();
        q.push_back(0usize);
        let mut expansions = 0u64;
        while let Some(idx) = q.pop_front() {
            expansions += 1;
            if expansions % 20_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
                eprintln!("  [event timeout after {} expansions]", expansions);
                return None;
            }

            let cur_depth = nodes[idx].depth;
            if cur_depth as usize >= max_depth {
                continue;
            }
            let cur_grid = nodes[idx].grid.clone();
            for t in &tuples {
                let (next_grid, play_state) = step(&cur_grid, t);
                if play_state == PlayState::GameOver {
                    continue;
                }
                let hash = next_grid.search_hash();
                if !visited.insert(hash) {
                    continue;
                }
                let node_idx = nodes.len();
                nodes.push(Node {
                    grid: next_grid.clone(),
                    parent: idx,
                    action: t.clone(),
                    depth: cur_depth + 1,
                });
                let features = Features::from_grid(&next_grid);
                if play_state == PlayState::Won
                    || target_reached(target, initial_features, features, &next_grid)
                {
                    return Some((reconstruct(&nodes, node_idx), next_grid));
                }
                q.push_back(node_idx);
            }
        }
        return None;
    }

    let mut pq = BinaryHeap::new();
    let h0 = event_heuristic(grid, target, initial_features);
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });
    let mut expansions = 0u64;
    let mut best_h_seen = h0;
    let mut best_idx_seen = 0usize;
    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 20_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            let best_path = reconstruct(&nodes, best_idx_seen);
            eprintln!(
                "  [event timeout after {} expansions, best_h={}, nodes={}]",
                expansions,
                best_h_seen,
                nodes.len()
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best_path));
            eprintln!("  BEST_ASCII {}", format_path_ascii(&best_path));
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx_seen].grid.to_csv());
            return None;
        }
        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g || cur_g as usize >= max_depth {
            continue;
        }
        for t in &tuples {
            let (next_grid, play_state) = step(&cur_grid, t);
            if play_state == PlayState::GameOver {
                continue;
            }
            let hash = next_grid.search_hash();
            if !visited.insert(hash) {
                continue;
            }
            let node_idx = nodes.len();
            let next_depth = (cur_g + 1) as u32;
            nodes.push(Node {
                grid: next_grid.clone(),
                parent: idx,
                action: t.clone(),
                depth: next_depth,
            });
            let features = Features::from_grid(&next_grid);
            if play_state == PlayState::Won
                || target_reached(target, initial_features, features, &next_grid)
            {
                return Some((reconstruct(&nodes, node_idx), next_grid));
            }

            let h = event_heuristic(&next_grid, target, initial_features);
            if h < best_h_seen {
                best_h_seen = h;
                best_idx_seen = node_idx;
            }
            let f = match strategy {
                "gbfs" => h,
                _ => cur_g + 1 + weight * h,
            };
            pq.push(PQItem {
                f,
                g: cur_g + 1,
                idx: node_idx,
            });
        }
    }

    None
}

fn jitter_for(hash: u64, depth: usize, seed: u64, jitter: i64) -> i64 {
    if jitter <= 0 {
        return 0;
    }
    let mut x = hash ^ seed ^ ((depth as u64).wrapping_mul(0x9e3779b97f4a7c15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    (x % jitter as u64) as i64
}

fn solve_beam(
    grid: &Grid,
    width: usize,
    max_depth: usize,
    time_limit_secs: f64,
    seed: u64,
    jitter: i64,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();
    let mut frontier = vec![BeamState {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];
    let mut global_seen = HashSet::new();
    global_seen.insert(grid.state_hash());
    let mut best = frontier[0].clone();

    for depth in 0..max_depth {
        if start.elapsed().as_secs_f64() > time_limit_secs {
            eprintln!(
                "  [beam timeout at depth={}, frontier={}, best_score={}]",
                depth,
                frontier.len(),
                best.score
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best.path));
            let best_ascii: String = if nplayers == 1 {
                best.path.iter().map(|a| action_to_ch(a[0])).collect()
            } else {
                best.path
                    .iter()
                    .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            eprintln!("  BEST_ASCII {}", best_ascii);
            eprintln!("  BEST_STATE:\n{}", best.grid.to_csv());
            return None;
        }

        let mut next = Vec::new();
        let mut layer_seen = HashSet::new();
        for state in &frontier {
            if start.elapsed().as_secs_f64() > time_limit_secs {
                eprintln!(
                    "  [beam timeout during depth={}, frontier={}, next={}, best_score={}]",
                    depth,
                    frontier.len(),
                    next.len(),
                    best.score
                );
                eprintln!("  BEST_ARROWS {}", format_path(&best.path));
                let best_ascii: String = if nplayers == 1 {
                    best.path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    best.path
                        .iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                eprintln!("  BEST_ASCII {}", best_ascii);
                eprintln!("  BEST_STATE:\n{}", best.grid.to_csv());
                return None;
            }
            for actions in &tuples {
                let (next_grid, play_state) = step(&state.grid, actions);
                if play_state == PlayState::GameOver {
                    continue;
                }
                let hash = next_grid.state_hash();
                if !layer_seen.insert(hash) {
                    continue;
                }
                if !global_seen.insert(hash) && jitter == 0 {
                    continue;
                }

                let mut path = state.path.clone();
                path.push(actions.clone());
                if play_state == PlayState::Won {
                    return Some(path);
                }
                let score = heuristic(&next_grid)
                    + path.len() as i64
                    + jitter_for(hash, depth, seed, jitter);
                let candidate = BeamState {
                    grid: next_grid,
                    path,
                    score,
                };
                if candidate.score < best.score {
                    best = candidate.clone();
                }
                next.push(candidate);
            }
        }

        if next.is_empty() {
            return None;
        }
        next.sort_by_key(|state| state.score);
        next.truncate(width);
        frontier = next;

        if depth % 25 == 24 {
            eprintln!(
                "  [beam depth={} frontier={} best_score={}]",
                depth + 1,
                frontier.len(),
                best.score
            );
        }
    }

    None
}

fn count_explosives(grid: &Grid) -> usize {
    let mut explosives = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Explosive {
                explosives += 1;
            }
        }
    }
    explosives
}

fn rat_positions(grid: &Grid) -> Vec<(i32, i32)> {
    let mut rats = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Rat | CellKind::CyborgRat) {
                rats.push((x as i32, y as i32));
            }
        }
    }
    rats
}

fn tinder_lure_rat_positions(grid: &Grid) -> Vec<(i32, i32)> {
    let lure_rats: Vec<_> = rat_positions(grid)
        .into_iter()
        .filter(|&(_, y)| y >= 3)
        .collect();
    if lure_rats.is_empty() {
        rat_positions(grid)
    } else {
        lure_rats
    }
}

fn distance_from_player(grid: &Grid, target: (i32, i32)) -> i64 {
    if target.0 < 0
        || target.1 < 0
        || target.0 as usize >= grid.width()
        || target.1 as usize >= grid.height()
    {
        return 1_000;
    }

    let dist = player_dist_map(grid);
    let d = dist[target.1 as usize][target.0 as usize];
    if d == i32::MAX { 1_000 } else { d as i64 }
}

fn rectangle_trap_heuristic(grid: &Grid, initial_rats: usize, initial_explosives: usize) -> i64 {
    let current_explosives = count_explosives(grid);
    if current_explosives < initial_explosives {
        return count_rats(grid) as i64;
    }

    let rats = tinder_lure_rat_positions(grid);
    let all_rats = rat_positions(grid);
    let strict_rectangle = std::env::var("RECT_STRICT").is_ok();
    let loose_traps = [(2, 6), (3, 6), (4, 6), (5, 6), (6, 6)];
    let strict_traps = [(4, 6), (5, 6)];
    let traps: &[(i32, i32)] = if strict_rectangle {
        &strict_traps
    } else {
        &loose_traps
    };
    let loose_safe_positions = rectangle_lower_safe_targets();
    let strict_safe_positions = rectangle_lower_safe_targets();
    let safe_positions: &[(i32, i32)] = if strict_rectangle {
        &strict_safe_positions
    } else {
        &loose_safe_positions
    };

    let player = find_player(grid);
    let nearest_rat_to_player = player
        .and_then(|player_pos| all_rats.iter().map(|&rat| manhattan(rat, player_pos)).min())
        .unwrap_or(1_000);
    let contact_penalty = if nearest_rat_to_player <= 1 {
        2_000
    } else if nearest_rat_to_player == 2 {
        400
    } else {
        0
    };

    let rats_lost = initial_rats.saturating_sub(count_rats(grid)) as i64;
    let pre_ignition_kill_penalty = rats_lost * 5_000;

    let mut best_plan_score = 50_000i64;
    for &rat in &rats {
        for &trap in traps {
            let rat_to_trap = manhattan(rat, trap);
            let dx = (trap.0 - rat.0).signum();
            let dy = (trap.1 - rat.1).signum();
            let mut cursor = rat;
            let mut corridor_webs = 0i64;
            let mut nearest_corridor_web = 1_000i64;
            while cursor != trap {
                if cursor.0 != trap.0 {
                    cursor.0 += dx;
                }
                if cursor.1 != trap.1 {
                    cursor.1 += dy;
                }
                if cursor.0 < 0
                    || cursor.1 < 0
                    || cursor.0 as usize >= grid.width()
                    || cursor.1 as usize >= grid.height()
                {
                    corridor_webs += 20;
                    continue;
                }
                let kind = grid.cell_kind_at(cursor.0 as usize, cursor.1 as usize);
                if kind == CellKind::Spiderweb {
                    corridor_webs += 1;
                    nearest_corridor_web =
                        nearest_corridor_web.min(distance_from_player(grid, cursor));
                } else if matches!(kind, CellKind::Wall | CellKind::BlackHole) {
                    corridor_webs += 20;
                }
            }

            for &safe in safe_positions {
                let player_to_safe = distance_from_player(grid, safe);
                let safe_side_bonus = if safe.0 > trap.0 && safe.1 > trap.1 {
                    0
                } else {
                    2_000
                };
                let trap_ready_bonus = if rat == trap && player_to_safe == 0 {
                    -5_000
                } else {
                    0
                };
                let score = rat_to_trap * 350
                    + corridor_webs * 900
                    + nearest_corridor_web.min(50) * 25
                    + player_to_safe * 70
                    + safe_side_bonus
                    + trap_ready_bonus;
                best_plan_score = best_plan_score.min(score);
            }
        }
    }

    pre_ignition_kill_penalty + contact_penalty + best_plan_score
}

fn rectangle_winready_heuristic(
    grid: &Grid,
    initial_rats: usize,
    initial_explosives: usize,
) -> i64 {
    if win_ready(grid) {
        return 0;
    }
    if count_explosives(grid) < initial_explosives {
        return 1_000_000_000 + count_rats(grid) as i64 * 1_000_000;
    }

    let all_rats = rat_positions(grid);
    let lure_rats = tinder_lure_rat_positions(grid);
    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    let player_dist = player_dist_map(grid);
    let lost_rats = initial_rats.saturating_sub(count_rats(grid)) as i64;
    let nearest_rat_to_player = players
        .iter()
        .flat_map(|&player| all_rats.iter().map(move |&rat| manhattan(player, rat)))
        .min()
        .unwrap_or(1_000);
    let contact_penalty = if nearest_rat_to_player <= 1 {
        100_000
    } else if nearest_rat_to_player == 2 {
        10_000
    } else {
        0
    };

    let row_three_safe: Vec<_> = (1..=15).map(|x| (x, 3)).collect();
    let lower_safe = rectangle_lower_safe_targets();
    let lower_targets = [(2, 6), (3, 6), (4, 6), (5, 6), (6, 6)];
    let corner_targets = [(0, 0), (16, 0), (13, 8)];

    let lower_overrun = lure_rats
        .iter()
        .filter(|&&(_, y)| y == 6)
        .filter_map(|&(x, _)| (x > 6).then_some((x - 6) as i64))
        .min()
        .unwrap_or(0);

    let lower_score = rectangle_lower_plan_score(
        grid,
        &lure_rats,
        &lower_targets,
        &lower_safe,
        &player_dist,
        80_000 * lower_overrun,
    );
    let corner_score = rectangle_plan_score(
        grid,
        &all_rats,
        &corner_targets,
        &row_three_safe,
        &player_dist,
        0,
    );

    lost_rats * 250_000 + contact_penalty + lower_score.min(corner_score)
}

fn rectangle_lower_rat_targets() -> [(i32, i32); 5] {
    [(2, 6), (3, 6), (4, 6), (5, 6), (6, 6)]
}

fn rectangle_lower_safe_targets() -> [(i32, i32); 6] {
    [(14, 6), (14, 7), (15, 7), (13, 8), (14, 8), (15, 8)]
}

fn rectangle_lower_ready(grid: &Grid) -> bool {
    if win_ready(grid) {
        return true;
    }

    let targets = rectangle_lower_rat_targets();
    let safe_targets = rectangle_lower_safe_targets();
    rat_at_any(grid, &targets) && player_at_any(grid, &safe_targets)
}

fn rectangle_lower_separated(grid: &Grid) -> bool {
    if win_ready(grid) {
        return true;
    }

    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    rat_positions(grid).into_iter().any(|rat| {
        rat.1 == 6
            && (2..=6).contains(&rat.0)
            && players
                .iter()
                .any(|&player| rectangle_lower_has_escape_staging(player))
    })
}

fn rectangle_lower_ignition_ready(grid: &Grid) -> bool {
    if win_ready(grid) {
        return true;
    }

    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    if !players
        .iter()
        .any(|&player| rectangle_lower_has_escape_staging(player))
    {
        return false;
    }

    rat_positions(grid).into_iter().any(|rat| {
        rat.1 == 6
            && (2..=6).contains(&rat.0)
            && rectangle_lower_east_blocked(grid, rat.0, rat.1)
            && rectangle_lower_has_adjacent_ignition(grid, rat.0, rat.1)
    })
}

fn rectangle_lower_has_escape_staging(player: (i32, i32)) -> bool {
    rectangle_lower_safe_targets().contains(&player)
}

fn rectangle_lower_east_blocked(grid: &Grid, x: i32, y: i32) -> bool {
    let east_x = x + 1;
    if east_x < 0 || y < 0 || east_x as usize >= grid.width() || y as usize >= grid.height() {
        return true;
    }

    matches!(
        grid.cell_kind_at(east_x as usize, y as usize),
        CellKind::Wall | CellKind::Spiderweb
    )
}

fn rectangle_lower_has_adjacent_ignition(grid: &Grid, x: i32, y: i32) -> bool {
    [(x - 1, y + 1), (x, y + 1), (x + 1, y + 1)]
        .into_iter()
        .any(|(candidate_x, candidate_y)| {
            candidate_x >= 0
                && candidate_y >= 0
                && (candidate_x as usize) < grid.width()
                && (candidate_y as usize) < grid.height()
                && grid.cell_kind_at(candidate_x as usize, candidate_y as usize)
                    == CellKind::Explosive
        })
}

fn rectangle_lower_separated_heuristic(
    grid: &Grid,
    initial_rats: usize,
    initial_explosives: usize,
) -> i64 {
    if rectangle_lower_separated(grid) {
        return 0;
    }
    if count_explosives(grid) < initial_explosives {
        return 1_000_000_000 + count_rats(grid) as i64 * 1_000_000;
    }

    let rat_targets = rectangle_lower_rat_targets();
    let safe_targets = rectangle_lower_safe_targets();
    let rats = tinder_lure_rat_positions(grid);
    let all_rats = rat_positions(grid);
    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    let lost_rats = initial_rats.saturating_sub(count_rats(grid)) as i64;

    let best_pair_score = rats
        .iter()
        .flat_map(|&rat| {
            players.iter().map(move |&player| {
                let target_cost = if rat.1 == 6 && (2..=6).contains(&rat.0) {
                    0
                } else if rat.1 == 6 && rat.0 > 6 {
                    500_000 + (rat.0 - 6) as i64 * 150_000
                } else {
                    nearest_target_distance(&[rat], &rat_targets) * 45_000
                };
                target_cost + nearest_target_distance(&[player], &safe_targets) * 70_000
            })
        })
        .min()
        .unwrap_or(1_000_000);

    let player_dist = player_dist_map(grid);
    let target_clearance = rat_targets
        .iter()
        .map(|&(x, y)| match grid.cell_kind_at(x as usize, y as usize) {
            CellKind::Wall | CellKind::BlackHole => 1_000_000,
            CellKind::Spiderweb => {
                let distance = player_dist[y as usize][x as usize];
                70_000
                    + if distance == i32::MAX {
                        70_000
                    } else {
                        distance as i64 * 700
                    }
            }
            _ => 0,
        })
        .min()
        .unwrap_or(1_000_000);

    let nearest_rat_to_player = players
        .iter()
        .flat_map(|&player| all_rats.iter().map(move |&rat| manhattan(player, rat)))
        .min()
        .unwrap_or(1_000);
    let contact_penalty = if nearest_rat_to_player <= 1 {
        100_000
    } else if nearest_rat_to_player == 2 {
        20_000
    } else {
        0
    };

    lost_rats * 500_000 + contact_penalty + target_clearance + best_pair_score
}

fn rectangle_lower_ignition_heuristic(
    grid: &Grid,
    initial_rats: usize,
    initial_explosives: usize,
) -> i64 {
    if rectangle_lower_ignition_ready(grid) {
        return 0;
    }

    let base = rectangle_lower_separated_heuristic(grid, initial_rats, initial_explosives);
    let blocker_penalty = rat_positions(grid)
        .into_iter()
        .filter(|&(x, y)| y == 6 && (2..=6).contains(&x))
        .map(|(x, y)| {
            if rectangle_lower_east_blocked(grid, x, y) {
                0
            } else {
                400_000
            }
        })
        .min()
        .unwrap_or(200_000);

    base + blocker_penalty
}

fn rectangle_lower_ready_heuristic(
    grid: &Grid,
    initial_rats: usize,
    initial_explosives: usize,
) -> i64 {
    if rectangle_lower_ready(grid) {
        return 0;
    }
    if count_explosives(grid) < initial_explosives {
        return 1_000_000_000 + count_rats(grid) as i64 * 1_000_000;
    }

    let rat_targets = rectangle_lower_rat_targets();
    let safe_targets = rectangle_lower_safe_targets();
    let rats = tinder_lure_rat_positions(grid);
    let all_rats = rat_positions(grid);
    let players = positions_matching(grid, |cell| cell == CellKind::Player);
    let player_dist = player_dist_map(grid);
    let rat_on_target = rat_at_any(grid, &rat_targets);
    let player_safe = player_at_any(grid, &safe_targets);
    let lost_rats = initial_rats.saturating_sub(count_rats(grid)) as i64;

    let rat_score = rats
        .iter()
        .map(|&(x, y)| {
            if y == 6 && (2..=6).contains(&x) {
                return 0;
            }
            if y == 6 && x > 6 {
                return 500_000 + (x - 6) as i64 * 150_000;
            }
            nearest_target_distance(&[(x, y)], &rat_targets) * 45_000
        })
        .min()
        .unwrap_or(1_000_000);

    let target_clearance = rat_targets
        .iter()
        .map(|&(x, y)| match grid.cell_kind_at(x as usize, y as usize) {
            CellKind::Wall | CellKind::BlackHole => 1_000_000,
            CellKind::Spiderweb => {
                let distance = player_dist[y as usize][x as usize];
                70_000
                    + if distance == i32::MAX {
                        70_000
                    } else {
                        distance as i64 * 700
                    }
            }
            _ => 0,
        })
        .min()
        .unwrap_or(1_000_000);

    let safe_distance = safe_targets
        .iter()
        .filter_map(|&(x, y)| {
            let distance = player_dist[y as usize][x as usize];
            (distance != i32::MAX).then_some(distance as i64)
        })
        .min()
        .unwrap_or(1_000);

    let lower_overrun = rats
        .iter()
        .filter(|&&(_, y)| y == 6)
        .filter_map(|&(x, _)| (x > 6).then_some((x - 6) as i64))
        .min()
        .unwrap_or(0);
    let nearest_rat_to_player = players
        .iter()
        .flat_map(|&player| all_rats.iter().map(move |&rat| manhattan(player, rat)))
        .min()
        .unwrap_or(1_000);
    let contact_penalty = if nearest_rat_to_player <= 1 {
        100_000
    } else if nearest_rat_to_player == 2 {
        20_000
    } else {
        0
    };

    let staged_safe_score = if rat_on_target {
        safe_distance * 6_000
    } else if player_safe {
        120_000
    } else {
        safe_distance.min(40) * 250
    };

    lost_rats * 500_000
        + lower_overrun * 200_000
        + contact_penalty
        + rat_score
        + target_clearance
        + staged_safe_score
        + if player_safe && !rat_on_target {
            250_000
        } else {
            0
        }
}

fn rectangle_plan_score(
    grid: &Grid,
    rats: &[(i32, i32)],
    rat_targets: &[(i32, i32)],
    safe_targets: &[(i32, i32)],
    player_dist: &[Vec<i32>],
    extra_penalty: i64,
) -> i64 {
    let rat_distance = nearest_target_distance(rats, rat_targets);
    let safe_distance = safe_targets
        .iter()
        .filter_map(|&(x, y)| {
            if x < 0 || y < 0 || x as usize >= grid.width() || y as usize >= grid.height() {
                return None;
            }
            let distance = player_dist[y as usize][x as usize];
            (distance != i32::MAX).then_some(distance as i64)
        })
        .min()
        .unwrap_or(1_000);
    let target_clearance = rat_targets
        .iter()
        .map(|&(x, y)| {
            if x < 0 || y < 0 || x as usize >= grid.width() || y as usize >= grid.height() {
                return 1_000_000;
            }
            match grid.cell_kind_at(x as usize, y as usize) {
                CellKind::Wall | CellKind::BlackHole => 1_000_000,
                CellKind::Spiderweb => {
                    let distance = player_dist[y as usize][x as usize];
                    50_000
                        + if distance == i32::MAX {
                            50_000
                        } else {
                            distance as i64 * 500
                        }
                }
                _ => 0,
            }
        })
        .min()
        .unwrap_or(1_000_000);

    rat_distance * 20_000 + safe_distance * 1_500 + target_clearance + extra_penalty
}

fn rectangle_lower_plan_score(
    grid: &Grid,
    rats: &[(i32, i32)],
    rat_targets: &[(i32, i32)],
    safe_targets: &[(i32, i32)],
    player_dist: &[Vec<i32>],
    extra_penalty: i64,
) -> i64 {
    let safe_distance = safe_targets
        .iter()
        .filter_map(|&(x, y)| {
            if x < 0 || y < 0 || x as usize >= grid.width() || y as usize >= grid.height() {
                return None;
            }
            let distance = player_dist[y as usize][x as usize];
            (distance != i32::MAX).then_some(distance as i64)
        })
        .min()
        .unwrap_or(1_000);

    let best_target_score = rat_targets
        .iter()
        .map(|&(target_x, target_y)| {
            let rat_distance = rats
                .iter()
                .map(|&rat| manhattan(rat, (target_x, target_y)))
                .min()
                .unwrap_or(1_000);
            let target_clearance = match grid.cell_kind_at(target_x as usize, target_y as usize) {
                CellKind::Wall | CellKind::BlackHole => 1_000_000,
                CellKind::Spiderweb => {
                    let distance = player_dist[target_y as usize][target_x as usize];
                    50_000
                        + if distance == i32::MAX {
                            50_000
                        } else {
                            distance as i64 * 500
                        }
                }
                _ => 0,
            };
            let blocker_x = target_x + 1;
            let blocker_penalty = if blocker_x as usize >= grid.width() {
                1_000_000
            } else {
                match grid.cell_kind_at(blocker_x as usize, target_y as usize) {
                    CellKind::Spiderweb => 0,
                    CellKind::Wall => 10_000,
                    _ => 200_000,
                }
            };
            rat_distance * 20_000 + target_clearance + blocker_penalty
        })
        .min()
        .unwrap_or(1_000_000);

    best_target_score + safe_distance * 1_500 + extra_penalty
}

fn tinderbox_heuristic(
    grid: &Grid,
    initial_rats: usize,
    initial_explosives: usize,
    rat_target: (i32, i32),
    safe_target: (i32, i32),
) -> i64 {
    if std::env::var("RECTLOWER_H").is_ok() && grid.width() >= 16 {
        return rectangle_lower_ready_heuristic(grid, initial_rats, initial_explosives);
    }

    if std::env::var("TRAP_H").is_ok() && grid.width() >= 16 {
        return rectangle_trap_heuristic(grid, initial_rats, initial_explosives);
    }

    let current_explosives = count_explosives(grid);
    if current_explosives < initial_explosives {
        return count_rats(grid) as i64;
    }

    let all_rats = rat_positions(grid);
    let rats = tinder_lure_rat_positions(grid);
    let rat_to_target = rats
        .iter()
        .map(|&pos| manhattan(pos, rat_target))
        .min()
        .unwrap_or(1_000);
    let rat_on_target = rats.contains(&rat_target);
    let player = find_player(grid);
    let player_to_safe = distance_from_player(grid, safe_target);
    let player_at_safe = player == Some(safe_target);
    let nearest_rat_to_player = player
        .and_then(|player_pos| all_rats.iter().map(|&rat| manhattan(rat, player_pos)).min())
        .unwrap_or(1_000);

    let mut uncleared_corridor = 0i64;
    let mut nearest_uncleared = 1_000i64;
    let corridor = if grid.width() >= 16 {
        let mut cells = Vec::new();
        for y in 3..grid.height() {
            for x in 0..grid.width() {
                cells.push((x as i32, y as i32));
            }
        }
        cells
    } else {
        vec![
            (2, 2),
            (3, 2),
            (4, 2),
            (5, 2),
            (6, 2),
            (3, 3),
            (4, 3),
            (5, 3),
            (6, 3),
            (7, 3),
            (2, 4),
            (3, 4),
            (4, 4),
            (5, 4),
            (6, 4),
            (7, 4),
            (2, 5),
            (3, 5),
            (4, 5),
            (5, 5),
            (6, 5),
            (7, 5),
            (7, 6),
            (7, 7),
            (7, 8),
            (6, 8),
        ]
    };
    for &(x, y) in &corridor {
        if grid.cell_kind_at(x as usize, y as usize) == CellKind::Spiderweb {
            uncleared_corridor += 1;
            nearest_uncleared = nearest_uncleared.min(distance_from_player(grid, (x, y)));
        }
    }

    let rats_lost = initial_rats.saturating_sub(count_rats(grid)) as i64;
    let pre_ignition_kill_penalty = rats_lost * 2_000;
    let contact_penalty = if !player_at_safe && nearest_rat_to_player <= 1 {
        2_000
    } else if !player_at_safe && nearest_rat_to_player == 2 {
        400
    } else {
        0
    };
    let clear_work = uncleared_corridor * 200 + nearest_uncleared.min(50) * 4;

    if rat_on_target {
        pre_ignition_kill_penalty + contact_penalty + player_to_safe * 20
    } else {
        pre_ignition_kill_penalty
            + contact_penalty
            + clear_work
            + rat_to_target * 80
            + player_to_safe.min(50)
    }
}

fn winning_action(grid: &Grid, tuples: &[Vec<Action>]) -> Option<Vec<Action>> {
    for actions in tuples {
        let (_, play_state) = step(grid, actions);
        if play_state == PlayState::Won {
            return Some(actions.clone());
        }
    }
    None
}

fn solve_tinderbox(
    grid: &Grid,
    max_depth: usize,
    time_limit_secs: f64,
    strategy: &str,
    weight: i64,
    rat_target: (i32, i32),
    safe_target: (i32, i32),
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();
    let initial_rats = count_rats(grid);
    let initial_explosives = count_explosives(grid);

    let mut nodes: Vec<Node> = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Vec::new(),
        depth: 0,
    }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(grid.state_hash(), 0);

    let h0 = tinderbox_heuristic(
        grid,
        initial_rats,
        initial_explosives,
        rat_target,
        safe_target,
    );
    let mut pq = BinaryHeap::new();
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });
    let mut expansions = 0u64;
    let mut best_h_seen = h0;
    let mut best_idx_seen = 0usize;

    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 100_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            let best_path = reconstruct(&nodes, best_idx_seen);
            eprintln!(
                "  [tinder timeout after {expansions} expansions, best_h={best_h_seen}, nodes={}]",
                nodes.len()
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best_path));
            let best_ascii: String = best_path.iter().map(|a| action_to_ch(a[0])).collect();
            eprintln!("  BEST_ASCII {}", best_ascii);
            eprintln!("  BEST_STATE:\n{}", nodes[best_idx_seen].grid.to_csv());
            return None;
        }

        let idx = item.idx;
        let cur_grid = nodes[idx].grid.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g || cur_g as usize >= max_depth {
            continue;
        }

        if let Some(actions) = winning_action(&cur_grid, &tuples) {
            let mut path = reconstruct(&nodes, idx);
            path.push(actions);
            return Some(path);
        }

        for actions in &tuples {
            let (next_grid, play_state) = step(&cur_grid, actions);
            if play_state == PlayState::GameOver {
                continue;
            }

            let node_idx = nodes.len();
            let next_depth = nodes[idx].depth + 1;
            nodes.push(Node {
                grid: next_grid.clone(),
                parent: idx,
                action: actions.clone(),
                depth: next_depth,
            });
            if play_state == PlayState::Won {
                return Some(reconstruct(&nodes, node_idx));
            }

            let hash = next_grid.state_hash();
            let better = match visited.get(&hash) {
                None => true,
                Some(&previous_depth) => next_depth < previous_depth,
            };
            if !better {
                continue;
            }
            visited.insert(hash, next_depth);

            let h = tinderbox_heuristic(
                &next_grid,
                initial_rats,
                initial_explosives,
                rat_target,
                safe_target,
            );
            if h < best_h_seen {
                best_h_seen = h;
                best_idx_seen = node_idx;
            }
            let f = match strategy {
                "gbfs" => h,
                _ => cur_g + 1 + weight * h,
            };
            pq.push(PQItem {
                f,
                g: cur_g + 1,
                idx: node_idx,
            });
        }
    }

    None
}

fn solve_tinderbox_beam(
    grid: &Grid,
    width: usize,
    max_depth: usize,
    time_limit_secs: f64,
    seed: u64,
    jitter: i64,
    rat_target: (i32, i32),
    safe_target: (i32, i32),
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(grid);
    let tuples = all_action_tuples(nplayers);
    let initial_rats = count_rats(grid);
    let initial_explosives = count_explosives(grid);
    let start = Instant::now();
    let mut frontier = vec![BeamState {
        grid: grid.clone(),
        path: Vec::new(),
        score: tinderbox_heuristic(
            grid,
            initial_rats,
            initial_explosives,
            rat_target,
            safe_target,
        ),
    }];
    let mut global_seen = HashSet::new();
    global_seen.insert(grid.state_hash());
    let mut best = frontier[0].clone();

    for depth in 0..max_depth {
        if start.elapsed().as_secs_f64() > time_limit_secs {
            eprintln!(
                "  [tinder beam timeout at depth={}, frontier={}, best_score={}]",
                depth,
                frontier.len(),
                best.score
            );
            eprintln!("  BEST_ARROWS {}", format_path(&best.path));
            let best_ascii: String = best.path.iter().map(|a| action_to_ch(a[0])).collect();
            eprintln!("  BEST_ASCII {}", best_ascii);
            eprintln!("  BEST_STATE:\n{}", best.grid.to_csv());
            return None;
        }

        let mut next = Vec::new();
        let mut layer_seen = HashSet::new();
        for state in &frontier {
            if let Some(actions) = winning_action(&state.grid, &tuples) {
                let mut path = state.path.clone();
                path.push(actions);
                return Some(path);
            }

            for actions in &tuples {
                let (next_grid, play_state) = step(&state.grid, actions);
                if play_state == PlayState::GameOver {
                    continue;
                }

                let mut path = state.path.clone();
                path.push(actions.clone());
                if play_state == PlayState::Won {
                    return Some(path);
                }

                let hash = next_grid.state_hash();
                if !layer_seen.insert(hash) {
                    continue;
                }
                if !global_seen.insert(hash) && jitter == 0 {
                    continue;
                }

                let h = tinderbox_heuristic(
                    &next_grid,
                    initial_rats,
                    initial_explosives,
                    rat_target,
                    safe_target,
                );
                let score = h + path.len() as i64 + jitter_for(hash, depth, seed, jitter);
                let candidate = BeamState {
                    grid: next_grid,
                    path,
                    score,
                };
                if candidate.score < best.score {
                    best = candidate.clone();
                }
                next.push(candidate);
            }
        }

        if next.is_empty() {
            return None;
        }
        next.sort_by_key(|state| state.score);
        next.truncate(width);
        frontier = next;

        if depth % 25 == 24 {
            eprintln!(
                "  [tinder beam depth={} frontier={} best_score={}]",
                depth + 1,
                frontier.len(),
                best.score
            );
        }
    }

    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: solver <verify|solve> <csv_file> [...]");
        std::process::exit(2);
    }
    let mode = &args[1];
    let csv = std::fs::read_to_string(&args[2]).expect("read csv");
    let csv = csv.trim_end_matches('\n').to_string();
    let grid = grid_from_csv(&csv);

    if mode == "verify" {
        let action_str = &args[3];
        let nplayers = count_players(&grid);
        let path = parse_action_string(action_str, nplayers);
        let (_, result, applied_count) = replay_path(&grid, &path);
        println!("result={:?} turns_applied={}", result, applied_count);
        return;
    }

    if mode == "trace" {
        // solver trace <csv> "<actions>" — print grid + state after each turn (single player).
        // Lets an agent observe state transitions to reason about play.
        let action_str = &args[3];
        let nplayers = count_players(&grid);
        let mut state = grid.clone();
        println!("turn 0 (initial):\n{}\n", state.to_csv());
        let mut turn = 0;
        if nplayers == 1 {
            for c in action_str.chars() {
                if let Some(a) = ch_to_action(c) {
                    let (ns, st) = step(&state, &[a]);
                    state = ns;
                    turn += 1;
                    println!(
                        "turn {} action={:?} state={:?}:\n{}\n",
                        turn,
                        a,
                        st,
                        state.to_csv()
                    );
                    if st != PlayState::Playing {
                        break;
                    }
                }
            }
        } else {
            for grp in action_str.split_whitespace() {
                let acts: Vec<Action> = grp
                    .chars()
                    .filter(|c| *c != '|')
                    .filter_map(ch_to_action)
                    .collect();
                if acts.len() != nplayers {
                    continue;
                }
                let (ns, st) = step(&state, &acts);
                state = ns;
                turn += 1;
                println!(
                    "turn {} actions={:?} state={:?}:\n{}\n",
                    turn,
                    acts,
                    st,
                    state.to_csv()
                );
                if st != PlayState::Playing {
                    break;
                }
            }
        }
        return;
    }

    if mode == "timeline" {
        // solver timeline <csv> "<actions>" — compact per-turn feature summary.
        let action_str = &args[3];
        let nplayers = count_players(&grid);
        let path = parse_action_string(action_str, nplayers);
        let mut state = grid.clone();
        let print_row = |turn: usize, action_text: &str, play_state: PlayState, grid: &Grid| {
            let features = Features::from_grid(grid);
            println!(
                "turn={} action={} state={:?} features={:?} reachable_rats={} reachable_triggers={} players=[{}] rats=[{}]",
                turn,
                action_text,
                play_state,
                features,
                reachable_rat_count(grid),
                reachable_trigger_count(grid),
                positions_key(grid, player_cell),
                positions_key(grid, rat_or_cyborg)
            );
        };
        print_row(0, ".", PlayState::Playing, &state);
        for (idx, actions) in path.iter().enumerate() {
            let (next_state, play_state) = step(&state, actions);
            state = next_state;
            let action_text = if nplayers == 1 {
                actions
                    .first()
                    .map(|action| action_to_ch(*action).to_string())
                    .unwrap_or_else(|| ".".to_string())
            } else {
                actions
                    .iter()
                    .map(|action| action_to_ch(*action))
                    .collect::<String>()
            };
            print_row(idx + 1, &action_text, play_state, &state);
            if play_state != PlayState::Playing {
                break;
            }
        }
        return;
    }

    if mode == "stats" {
        // solver stats <csv> "<actions>" — print feature/position summary after each turn.
        let action_str = &args[3];
        let nplayers = count_players(&grid);
        let path = parse_action_string(action_str, nplayers);
        let mut state = grid.clone();
        print_state_summary(0, None, PlayState::Playing, &state);
        for (turn, actions) in path.iter().enumerate() {
            let (next, play_state) = step(&state, actions);
            state = next;
            print_state_summary(turn + 1, Some(actions), play_state, &state);
            if play_state != PlayState::Playing {
                break;
            }
        }
        return;
    }

    if mode == "diag" {
        // solver diag <csv> [prefix] — replay an optional prefix, then print reachability diagnostics.
        let action_str = args.get(3).map(String::as_str).unwrap_or("");
        let nplayers = count_players(&grid);
        let path = parse_action_string(action_str, nplayers);
        let (state, play_state, applied) = replay_path(&grid, &path);
        println!("state={play_state:?} turns_applied={applied}");
        println!("grid:\n{}", state.to_csv());
        print_diagnostics(&state);
        return;
    }

    if mode == "ignitions" {
        // solver ignitions <csv> [prefix] [--limit N] — list one-step detonation geometries.
        let mut prefix_str = String::new();
        let mut limit = 80usize;
        let mut i = 3;
        if i < args.len() && !args[i].starts_with("--") {
            prefix_str = args[i].clone();
            i += 1;
        }
        while i < args.len() {
            match args[i].as_str() {
                "--limit" => {
                    limit = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => i += 1,
            }
        }
        let nplayers = count_players(&grid);
        let path = parse_action_string(&prefix_str, nplayers);
        let (state, play_state, applied) = replay_path(&grid, &path);
        println!("state={play_state:?} turns_applied={applied}");
        if play_state == PlayState::Playing {
            print_ignition_geometries(&state, limit);
        }
        return;
    }

    if mode == "ratgeom" {
        // solver ratgeom <csv> [prefix] --source x,y --target x,y [--limit N]
        // Enumerate synthetic player placements where stalling makes a specific rat move
        // to a target. This is a local mechanism diagnostic, not a solver.
        let mut prefix_str = String::new();
        let mut source = None;
        let mut target = None;
        let mut limit = 80usize;
        let mut i = 3;
        if i < args.len() && !args[i].starts_with("--") {
            prefix_str = args[i].clone();
            i += 1;
        }
        while i < args.len() {
            match args[i].as_str() {
                "--source" => {
                    let point = parse_required_point(&args[i + 1]);
                    source = Some((point.0 as usize, point.1 as usize));
                    i += 2;
                }
                "--target" => {
                    target = Some(parse_required_point(&args[i + 1]));
                    i += 2;
                }
                "--limit" => {
                    limit = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => i += 1,
            }
        }
        let nplayers = count_players(&grid);
        let path = parse_action_string(&prefix_str, nplayers);
        let (state, play_state, applied) = replay_path(&grid, &path);
        println!("state={play_state:?} turns_applied={applied}");
        if play_state == PlayState::Playing {
            print_rat_step_geometries(
                &state,
                source.expect("--source x,y is required"),
                target.expect("--target x,y is required"),
                limit,
            );
        }
        return;
    }

    if mode == "ratdeathgeom" {
        // solver ratdeathgeom <csv> [prefix] --source x,y [--target x,y]
        //                     [--kind any|blackhole|explosive] [--limit N]
        //
        // Enumerate synthetic player placements where stalling removes at least
        // one rat after placing a rat at source. This is meant for local human
        // mechanism checks such as "can this sealed rat be lured into a black hole?".
        let mut prefix_str = String::new();
        let mut source = None;
        let mut target = None;
        let mut target_kind = DeathTargetKind::Any;
        let mut limit = 80usize;
        let mut i = 3;
        if i < args.len() && !args[i].starts_with("--") {
            prefix_str = args[i].clone();
            i += 1;
        }
        while i < args.len() {
            match args[i].as_str() {
                "--source" => {
                    let point = parse_required_point(&args[i + 1]);
                    source = Some((point.0 as usize, point.1 as usize));
                    i += 2;
                }
                "--target" => {
                    target = parse_optional_point(&args[i + 1]);
                    i += 2;
                }
                "--kind" => {
                    target_kind = DeathTargetKind::parse(&args[i + 1]);
                    i += 2;
                }
                "--limit" => {
                    limit = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => i += 1,
            }
        }
        let nplayers = count_players(&grid);
        let path = parse_action_string(&prefix_str, nplayers);
        let (state, play_state, applied) = replay_path(&grid, &path);
        println!("state={play_state:?} turns_applied={applied}");
        if play_state == PlayState::Playing {
            print_rat_death_step_geometries(
                &state,
                source.expect("--source x,y is required"),
                target,
                target_kind,
                limit,
            );
        }
        return;
    }

    if mode == "lure" {
        // solver lure <csv> --rat x,y [--safe x,y;x,y] [--preserve-rats]
        //                  [--prefix MOVES] [--depth N] [--secs S] [--maxnodes N]
        //                  [--strategy gbfs|astar] [--weight W]
        let mut rat_target = None;
        let mut safe_targets = Vec::new();
        let mut preserve_rats = false;
        let mut prefix_str = String::new();
        let mut depth = 200usize;
        let mut secs = 60.0;
        let mut max_nodes = 500_000usize;
        let mut strategy = "astar".to_string();
        let mut weight = 2i64;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--rat" | "--rat-target" => {
                    rat_target = parse_optional_point(&args[i + 1]);
                    i += 2;
                }
                "--safe" | "--safe-targets" => {
                    safe_targets.clear();
                    for part in args[i + 1].split(';') {
                        if let Some(point) = parse_optional_point(part) {
                            safe_targets.push(point);
                        }
                    }
                    i += 2;
                }
                "--preserve-rats" => {
                    preserve_rats = true;
                    i += 1;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--maxnodes" => {
                    max_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--strategy" => {
                    strategy = args[i + 1].clone();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let rat_target = rat_target.expect("--rat x,y is required");
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "lure solve: players={} prefix={} rat_target={:?} safe_targets={:?} preserve_rats={} strategy={} depth={} secs={} maxnodes={} weight={}",
            nplayers,
            prefix.len(),
            rat_target,
            safe_targets,
            preserve_rats,
            strategy,
            depth,
            secs,
            max_nodes,
            weight
        );
        let t0 = Instant::now();
        match solve_lure(
            &start_grid,
            rat_target,
            &safe_targets,
            preserve_rats,
            depth,
            secs,
            max_nodes,
            &strategy,
            weight,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                println!("ASCII {}", format_path_ascii(&path));
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "geomlure" {
        // solver geomlure <csv> --rats x,y;... --safe x,y;... [--preserve-rats]
        //                      [--prefix MOVES] [--depth N] [--secs S] [--maxnodes N]
        //                      [--strategy gbfs|astar] [--weight W]
        let mut rat_targets = Vec::new();
        let mut safe_targets = Vec::new();
        let mut preserve_rats = false;
        let mut prefix_str = String::new();
        let mut depth = 200usize;
        let mut secs = 60.0;
        let mut max_nodes = 500_000usize;
        let mut strategy = "astar".to_string();
        let mut weight = 2i64;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--rats" | "--rat-targets" => {
                    rat_targets = parse_points(&args[i + 1]);
                    i += 2;
                }
                "--safe" | "--safe-targets" => {
                    safe_targets = parse_points(&args[i + 1]);
                    i += 2;
                }
                "--preserve-rats" => {
                    preserve_rats = true;
                    i += 1;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--maxnodes" => {
                    max_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--strategy" => {
                    strategy = args[i + 1].clone();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        assert!(!rat_targets.is_empty(), "--rats x,y;... is required");
        assert!(!safe_targets.is_empty(), "--safe x,y;... is required");
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "geomlure solve: players={} prefix={} rat_targets={:?} safe_targets={:?} preserve_rats={} strategy={} depth={} secs={} maxnodes={} weight={}",
            nplayers,
            prefix.len(),
            rat_targets,
            safe_targets,
            preserve_rats,
            strategy,
            depth,
            secs,
            max_nodes,
            weight
        );
        let t0 = Instant::now();
        match solve_geom_lure(
            &start_grid,
            &rat_targets,
            &safe_targets,
            preserve_rats,
            depth,
            secs,
            max_nodes,
            &strategy,
            weight,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                println!("ASCII {}", format_path_ascii(&path));
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "tinder" {
        // solver tinder <csv> [--depth N] [--secs S] [--strategy gbfs|astar] [--weight W]
        //                   [--rat-target x,y] [--safe-target x,y]
        let mut depth = 260usize;
        let mut secs = 120.0f64;
        let mut strategy = "gbfs".to_string();
        let mut weight = 1i64;
        let mut rat_target = (6, 5);
        let mut safe_target = (6, 8);
        let mut prefix_str = String::new();
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--strategy" => {
                    strategy = args[i + 1].clone();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--rat-target" => {
                    let mut parts = args[i + 1].split(',');
                    rat_target = (
                        parts.next().unwrap().parse().unwrap(),
                        parts.next().unwrap().parse().unwrap(),
                    );
                    i += 2;
                }
                "--safe-target" => {
                    let mut parts = args[i + 1].split(',');
                    safe_target = (
                        parts.next().unwrap().parse().unwrap(),
                        parts.next().unwrap().parse().unwrap(),
                    );
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "tinder solve: players={} prefix={} strategy={} depth={} secs={} weight={} rat_target={:?} safe_target={:?}",
            nplayers,
            prefix.len(),
            strategy,
            depth,
            secs,
            weight,
            rat_target,
            safe_target
        );
        let t0 = Instant::now();
        match solve_tinderbox(
            &start_grid,
            depth,
            secs,
            &strategy,
            weight,
            rat_target,
            safe_target,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "tinderbeam" {
        // solver tinderbeam <csv> [--width N] [--depth N] [--secs S] [--seed N] [--jitter N]
        //                       [--rat-target x,y] [--safe-target x,y]
        let mut width = 20_000usize;
        let mut depth = 160usize;
        let mut secs = 120.0f64;
        let mut seed = 1u64;
        let mut jitter = 1_000i64;
        let mut rat_target = (6, 5);
        let mut safe_target = (6, 8);
        let mut prefix_str = String::new();
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--width" => {
                    width = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--seed" => {
                    seed = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--jitter" => {
                    jitter = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--rat-target" => {
                    let mut parts = args[i + 1].split(',');
                    rat_target = (
                        parts.next().unwrap().parse().unwrap(),
                        parts.next().unwrap().parse().unwrap(),
                    );
                    i += 2;
                }
                "--safe-target" => {
                    let mut parts = args[i + 1].split(',');
                    safe_target = (
                        parts.next().unwrap().parse().unwrap(),
                        parts.next().unwrap().parse().unwrap(),
                    );
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "tinder beam: players={} prefix={} width={} depth={} secs={} seed={} jitter={} rat_target={:?} safe_target={:?}",
            nplayers,
            prefix.len(),
            width,
            depth,
            secs,
            seed,
            jitter,
            rat_target,
            safe_target
        );
        let t0 = Instant::now();
        match solve_tinderbox_beam(
            &start_grid,
            width,
            depth,
            secs,
            seed,
            jitter,
            rat_target,
            safe_target,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "cont" {
        // solver cont <csv> <prefix_moves> [--depth N] [--secs S] [--strategy gbfs] [--weight W]
        let prefix_str = &args[3];
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }

        let mut depth = 200usize;
        let mut secs = 120.0f64;
        let mut strategy = "gbfs".to_string();
        let mut weight = 5i64;
        let mut i = 4;
        while i < args.len() {
            match args[i].as_str() {
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--strategy" => {
                    strategy = args[i + 1].clone();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }

        eprintln!(
            "continuing: prefix={} strategy={} depth={} secs={} weight={}",
            prefix.len(),
            strategy,
            depth,
            secs,
            weight
        );
        let t0 = Instant::now();
        match solve_with_context(&start_grid, depth, secs, &strategy, weight, &prefix) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "wp" {
        // solver wp <csv> --waypoints "x,y;x,y;..." [--persecs S] [--mopsecs S] [--mopstrat astar] [--mopweight 2] [--depth N]
        let mut waypoints: Vec<(i32, i32)> = Vec::new();
        let mut prefix_str = String::new();
        let mut per_secs = 20.0;
        let mut mop_secs = 60.0;
        let mut mop_strategy = "astar".to_string();
        let mut mop_weight = 2i64;
        let mut depth = 400usize;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--waypoints" => {
                    for pair in args[i + 1].split(';') {
                        let p = pair.trim();
                        if p.is_empty() {
                            continue;
                        }
                        let mut it = p.split(',');
                        let x: i32 = it.next().unwrap().trim().parse().unwrap();
                        let y: i32 = it.next().unwrap().trim().parse().unwrap();
                        waypoints.push((x, y));
                    }
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--persecs" => {
                    per_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        eprintln!(
            "wp solve: {} waypoints, players={}, prefix={}",
            waypoints.len(),
            nplayers,
            prefix_str.chars().filter_map(|c| ch_to_action(c)).count()
        );
        let t0 = Instant::now();
        let prefix = parse_action_string(&prefix_str, nplayers);
        match solve_waypoints(
            &grid,
            prefix,
            &waypoints,
            per_secs,
            mop_secs,
            &mop_strategy,
            mop_weight,
            depth,
        ) {
            Some(path) => {
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "wp2" {
        // solver wp2 <csv> --waypoints "p1x,p1y|p2x,p2y;.|p2x,p2y;..."
        //                  [--persecs S] [--mopsecs S] [--mopstrat astar] [--mopweight 2] [--depth N]
        let nplayers = count_players(&grid);
        let mut waypoints: Vec<Vec<Option<(i32, i32)>>> = Vec::new();
        let mut prefix_str = String::new();
        let mut per_secs = 20.0;
        let mut mop_secs = 60.0;
        let mut mop_strategy = "astar".to_string();
        let mut mop_weight = 2i64;
        let mut depth = 400usize;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--waypoints" => {
                    waypoints = parse_waypoint_pairs(&args[i + 1], nplayers);
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--persecs" => {
                    per_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        eprintln!(
            "wp2 solve: {} waypoint-pairs, players={}, prefix={}",
            waypoints.len(),
            nplayers,
            parse_action_string(&prefix_str, nplayers).len()
        );
        let t0 = Instant::now();
        let prefix = parse_action_string(&prefix_str, nplayers);
        match solve_waypoint_pairs(
            &grid,
            prefix,
            &waypoints,
            per_secs,
            mop_secs,
            &mop_strategy,
            mop_weight,
            depth,
        ) {
            Some(path) => {
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "trig" {
        // solver trig <csv> [--order "1,2,3"] [--persecs S] [--beam N]
        //                   [--mopsecs S] [--mopstrat astar] [--mopweight 2] [--depth N]
        let mut order: Option<Vec<u8>> = None;
        let mut prefix_str = String::new();
        let mut per_secs = 10.0;
        let mut beam = 8usize;
        let mut mop_secs = 60.0;
        let mut mop_strategy = "astar".to_string();
        let mut mop_weight = 2i64;
        let mut depth = 400usize;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--order" => {
                    order = Some(parse_trigger_order(&args[i + 1]));
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--persecs" => {
                    per_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--beam" => {
                    beam = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let order = order.unwrap_or_else(|| trigger_numbers(&grid));
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "trigger solve: order={:?}, players={}, prefix={}, per_secs={}, beam={}",
            order,
            nplayers,
            prefix.len(),
            per_secs,
            beam
        );
        let t0 = Instant::now();
        match solve_trigger_order(
            &start_grid,
            &order,
            per_secs,
            beam,
            mop_secs,
            &mop_strategy,
            mop_weight,
            depth,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "trigany" {
        // solver trigany <csv> [--steps N] [--persecs S] [--beam N]
        //                      [--mopsecs S] [--mopstrat astar] [--mopweight 2] [--depth N]
        let mut steps = trigger_numbers(&grid).len().max(1);
        let mut prefix_str = String::new();
        let mut per_secs = 5.0;
        let mut beam = 16usize;
        let mut mop_secs = 10.0;
        let mut mop_strategy = "astar".to_string();
        let mut mop_weight = 2i64;
        let mut depth = 350usize;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--steps" => {
                    steps = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--persecs" => {
                    per_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--beam" => {
                    beam = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "trigger-any solve: players={} prefix={} steps={} per_secs={} beam={}",
            nplayers,
            prefix.len(),
            steps,
            per_secs,
            beam
        );
        let t0 = Instant::now();
        match solve_any_trigger_order(
            &start_grid,
            steps,
            per_secs,
            beam,
            mop_secs,
            &mop_strategy,
            mop_weight,
            depth,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "triglookup" {
        // solver triglookup <csv> [--order "1,2,3"] [--segdepth N]
        //                         [--segsecs S] [--segnodes N] [--results N]
        //                         [--beam N] [--mopsecs S] [--strict]
        let mut prefix_str = String::new();
        let mut order: Option<Vec<u8>> = None;
        let mut segment_depth = 120usize;
        let mut segment_secs = 10.0;
        let mut segment_nodes = 1_000_000usize;
        let mut segment_results = 32usize;
        let mut beam = 32usize;
        let mut mop_secs = 30.0;
        let mut mop_strategy = "gbfs".to_string();
        let mut mop_weight = 5i64;
        let mut depth = 400usize;
        let mut strict_trigger_order = false;
        let mut trap_constraints = TrapConstraints::default();
        let mut i = 3;
        while i < args.len() {
            if let Some(next_i) = parse_trap_constraint_arg(&args, i, &mut trap_constraints) {
                i = next_i;
                continue;
            }
            match args[i].as_str() {
                "--order" => {
                    order = Some(parse_trigger_order(&args[i + 1]));
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--segdepth" => {
                    segment_depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segsecs" => {
                    segment_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segnodes" => {
                    segment_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--results" => {
                    segment_results = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--beam" => {
                    beam = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--min-reachable-rats" | "--min-reachable" => {
                    trap_constraints.min_reachable_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--max-trapped-rats" | "--max-trapped" => {
                    trap_constraints.max_trapped_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-cells" => {
                    trap_constraints.min_reachable_cells = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-triggers" => {
                    trap_constraints.min_reachable_triggers = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--require-reachable-trigger" | "--next-trigger" => {
                    trap_constraints.require_reachable_trigger = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--strict" => {
                    strict_trigger_order = true;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        let order = order.unwrap_or_else(|| trigger_numbers(&grid));
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "trigger-lookup solve: order={:?}, players={}, prefix={}, segdepth={}, segsecs={}, segnodes={}, results={}, beam={}, strict={}, trap={:?}",
            order,
            nplayers,
            prefix.len(),
            segment_depth,
            segment_secs,
            segment_nodes,
            segment_results,
            beam,
            strict_trigger_order,
            trap_constraints
        );
        let t0 = Instant::now();
        match solve_trigger_order_lookup(
            &start_grid,
            &order,
            segment_depth,
            segment_secs,
            segment_nodes,
            segment_results,
            beam,
            mop_secs,
            &mop_strategy,
            mop_weight,
            depth,
            strict_trigger_order,
            trap_constraints,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                println!("ASCII {}", format_path_ascii(&path));
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "triganylookup" {
        // solver triganylookup <csv> [--steps N] [--segdepth N]
        //                            [--segsecs S] [--segnodes N] [--results N]
        //                            [--beam N] [--mopsecs S]
        let mut prefix_str = String::new();
        let mut steps = trigger_numbers(&grid).len().max(1);
        let mut segment_depth = 120usize;
        let mut segment_secs = 10.0;
        let mut segment_nodes = 1_000_000usize;
        let mut segment_results = 16usize;
        let mut beam = 32usize;
        let mut mop_secs = 30.0;
        let mut mop_strategy = "gbfs".to_string();
        let mut mop_weight = 5i64;
        let mut depth = 400usize;
        let mut trap_constraints = TrapConstraints::default();
        let mut i = 3;
        while i < args.len() {
            if let Some(next_i) = parse_trap_constraint_arg(&args, i, &mut trap_constraints) {
                i = next_i;
                continue;
            }
            match args[i].as_str() {
                "--steps" => {
                    steps = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--segdepth" => {
                    segment_depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segsecs" => {
                    segment_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segnodes" => {
                    segment_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--results" => {
                    segment_results = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--beam" => {
                    beam = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--min-reachable-rats" | "--min-reachable" => {
                    trap_constraints.min_reachable_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--max-trapped-rats" | "--max-trapped" => {
                    trap_constraints.max_trapped_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-cells" => {
                    trap_constraints.min_reachable_cells = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-triggers" => {
                    trap_constraints.min_reachable_triggers = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--require-reachable-trigger" | "--next-trigger" => {
                    trap_constraints.require_reachable_trigger = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                _ => i += 1,
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "trigger-any-lookup solve: players={} prefix={} steps={} segdepth={} segsecs={} segnodes={} results={} beam={} trap={:?}",
            nplayers,
            prefix.len(),
            steps,
            segment_depth,
            segment_secs,
            segment_nodes,
            segment_results,
            beam,
            trap_constraints
        );
        let t0 = Instant::now();
        match solve_any_trigger_order_lookup(
            &start_grid,
            steps,
            segment_depth,
            segment_secs,
            segment_nodes,
            segment_results,
            beam,
            mop_secs,
            &mop_strategy,
            mop_weight,
            depth,
            trap_constraints,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                println!("ASCII {}", format_path_ascii(&path));
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "macro" {
        // solver macro <csv> [--segdepth N] [--segsecs S] [--beam N] [--events N] [--secs S]
        let mut prefix_str = String::new();
        let mut segment_depth = 40usize;
        let mut segment_secs = 5.0;
        let mut event_beam = 16usize;
        let mut max_events = 200usize;
        let mut total_secs = 120.0;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--segdepth" => {
                    segment_depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segsecs" => {
                    segment_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--beam" => {
                    event_beam = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--events" => {
                    max_events = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    total_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "macro solve: players={} prefix={} segdepth={} segsecs={} beam={} events={} secs={}",
            nplayers,
            prefix.len(),
            segment_depth,
            segment_secs,
            event_beam,
            max_events,
            total_secs
        );
        let t0 = Instant::now();
        match solve_macro_events(
            &start_grid,
            segment_depth,
            segment_secs,
            event_beam,
            max_events,
            total_secs,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "event" {
        // solver event <csv> --kind explosion [--depth N] [--secs S] [--strategy gbfs]
        //                    [--weight W] [--mopsecs S] [--mopdepth N]
        let mut target = TargetKind::Explosion;
        let mut depth = 200usize;
        let mut secs = 60.0;
        let mut strategy = "gbfs".to_string();
        let mut weight = 2i64;
        let mut mop_secs = 0.0;
        let mut mop_depth = 400usize;
        let mut mop_strategy = "gbfs".to_string();
        let mut mop_weight = 5i64;
        let mut prefix_str = String::new();
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--kind" => {
                    target = TargetKind::parse(&args[i + 1]);
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--strategy" => {
                    strategy = args[i + 1].clone();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopdepth" => {
                    mop_depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "event solve: kind={:?} players={} prefix={} strategy={} depth={} secs={} weight={}",
            target,
            nplayers,
            prefix.len(),
            strategy,
            depth,
            secs,
            weight
        );
        let t0 = Instant::now();
        match solve_to_event(&start_grid, target, depth, secs, &strategy, weight) {
            Some((suffix, event_grid)) => {
                let mut path = prefix;
                path.extend(suffix);
                let mut solved = Features::from_grid(&event_grid).rats == 0;
                eprintln!(
                    "  event reached in {} moves, features={:?}",
                    path.len(),
                    Features::from_grid(&event_grid)
                );
                eprintln!("  state after event:\n{}", event_grid.to_csv());
                if mop_secs > 0.0
                    && let Some(mop) = solve_with_context(
                        &event_grid,
                        mop_depth,
                        mop_secs,
                        &mop_strategy,
                        mop_weight,
                        &path,
                    )
                {
                    path.extend(mop);
                    solved = true;
                }
                println!(
                    "{} moves={} time={:.1}s",
                    if solved { "SOLVED" } else { "EVENT" },
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "events" {
        // solver events <csv> [--depth N] [--secs S] [--max N]
        // List structural event successors with paths for manual midgame analysis.
        let mut prefix_str = String::new();
        let mut depth = 80usize;
        let mut secs = 30.0;
        let mut max_events = 20usize;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--max" => {
                    max_events = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        let tuples = all_action_tuples(nplayers);
        eprintln!(
            "events: players={} prefix={} depth={} secs={} max={}",
            nplayers,
            prefix.len(),
            depth,
            secs,
            max_events
        );
        match find_event_successors(&start_grid, &tuples, depth, secs, max_events) {
            Some(events) => {
                for (idx, event) in events.iter().enumerate() {
                    let mut full_path = prefix.clone();
                    full_path.extend(event.path.clone());
                    let ascii: String = if nplayers == 1 {
                        event.path.iter().map(|a| action_to_ch(a[0])).collect()
                    } else {
                        event
                            .path
                            .iter()
                            .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                            .collect::<Vec<_>>()
                            .join(" ")
                    };
                    let full_ascii = format_path_ascii(&full_path);
                    println!(
                        "EVENT idx={} moves={} score={} features={:?}",
                        idx,
                        event.path.len(),
                        event.score,
                        event.features
                    );
                    println!("ASCII {}", ascii);
                    println!("FULL_ASCII {}", full_ascii);
                    println!("STATE\n{}", event.grid.to_csv());
                }
            }
            None => println!("NO_EVENTS"),
        }
        return;
    }

    if mode == "beam" {
        // solver beam <csv> [--prefix MOVES] [--width N] [--depth N] [--secs S] [--seed N] [--jitter N]
        let mut prefix_str = String::new();
        let mut width = 1000usize;
        let mut depth = 300usize;
        let mut secs = 120.0;
        let mut seed = 1u64;
        let mut jitter = 10_000i64;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--width" => {
                    width = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--seed" => {
                    seed = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--jitter" => {
                    jitter = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "beam solve: players={} prefix={} width={} depth={} secs={} seed={} jitter={}",
            nplayers,
            prefix.len(),
            width,
            depth,
            secs,
            seed,
            jitter
        );
        let t0 = Instant::now();
        match solve_beam(&start_grid, width, depth, secs, seed, jitter) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "novelty" {
        // solver novelty <csv> [--prefix MOVES] [--k 2] [--depth N] [--secs S]
        let mut prefix_str = String::new();
        let mut k = 2u8;
        let mut depth = 400usize;
        let mut secs = 120.0;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--k" => {
                    k = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "novelty solve: players={} prefix={} k={} depth={} secs={}",
            nplayers,
            prefix.len(),
            k,
            depth,
            secs
        );
        let t0 = Instant::now();
        match solve_novelty(&start_grid, depth, secs, k) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "lookup" {
        // solver lookup <csv> [--prefix MOVES] [--order bfs|gbfs|astar]
        //                     [--depth N] [--secs S] [--maxnodes N]
        //                     [--weight W] [--min-rats N] [--progress N] [--no-canonical]
        let mut prefix_str = String::new();
        let mut order = LookupOrder::Bfs;
        let mut depth = 400usize;
        let mut secs = 120.0;
        let mut max_nodes = 5_000_000usize;
        let mut weight = 5i64;
        let mut canonical = true;
        let mut goal = LookupGoal::Win;
        let mut min_rats: Option<usize> = None;
        let mut trap_constraints = TrapConstraints::default();
        let mut progress_every = 100_000u64;
        let mut i = 3;
        while i < args.len() {
            if let Some(next_i) = parse_trap_constraint_arg(&args, i, &mut trap_constraints) {
                i = next_i;
                continue;
            }
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--order" => {
                    order = LookupOrder::parse(&args[i + 1]);
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--maxnodes" => {
                    max_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--goal" => {
                    goal = LookupGoal::parse(&args[i + 1]);
                    i += 2;
                }
                "--min-rats" => {
                    min_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-rats" | "--min-reachable" => {
                    trap_constraints.min_reachable_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--max-trapped-rats" | "--max-trapped" => {
                    trap_constraints.max_trapped_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-cells" => {
                    trap_constraints.min_reachable_cells = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-triggers" => {
                    trap_constraints.min_reachable_triggers = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--require-reachable-trigger" | "--next-trigger" => {
                    trap_constraints.require_reachable_trigger = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--progress" => {
                    progress_every = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--no-canonical" => {
                    canonical = false;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "lookup solve: players={} prefix={} order={} depth={} secs={} maxnodes={} canonical={} weight={} goal={:?} min_rats={:?} trap={:?}",
            nplayers,
            prefix.len(),
            match order {
                LookupOrder::Bfs => "bfs",
                LookupOrder::Gbfs => "gbfs",
                LookupOrder::Astar => "astar",
            },
            depth,
            secs,
            max_nodes,
            canonical,
            weight,
            goal,
            min_rats,
            trap_constraints
        );
        let t0 = Instant::now();
        match solve_lookup(
            &start_grid,
            depth,
            secs,
            max_nodes,
            order,
            weight,
            canonical,
            goal,
            min_rats,
            trap_constraints,
            progress_every,
        ) {
            Some((suffix, play_state)) => {
                let mut path = prefix;
                path.extend(suffix);
                let solved = play_state == PlayState::Won;
                println!(
                    "{} moves={} time={:.1}s",
                    if solved { "SOLVED" } else { "GOAL" },
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                if !solved {
                    println!("GOAL_REACHED state={:?}", play_state);
                }
                println!("ARROWS {}", format_path(&path));
                println!("ASCII {}", format_path_ascii(&path));
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "branchdump" {
        // solver branchdump <csv> [--prefix MOVES] [--goal win|trigger:n|ratat:x,y|ratgone:x,y]
        //                         [--depth N] [--secs S] [--maxnodes N] [--results N]
        //                         [--min-rats N] [--eval x,y] [--states]
        let mut prefix_str = String::new();
        let mut goal = LookupGoal::Win;
        let mut depth = 60usize;
        let mut secs = 30.0f64;
        let mut max_nodes = 500_000usize;
        let mut results = 20usize;
        let mut min_rats: Option<usize> = None;
        let mut trap_constraints = TrapConstraints::default();
        let mut eval_point: Option<(i32, i32)> = None;
        let mut print_states = false;
        let mut canonical = true;
        let mut i = 3;
        while i < args.len() {
            if let Some(next_i) = parse_trap_constraint_arg(&args, i, &mut trap_constraints) {
                i = next_i;
                continue;
            }
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--goal" => {
                    goal = LookupGoal::parse(&args[i + 1]);
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--maxnodes" => {
                    max_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--results" => {
                    results = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--min-rats" => {
                    min_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-rats" | "--min-reachable" => {
                    trap_constraints.min_reachable_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--max-trapped-rats" | "--max-trapped" => {
                    trap_constraints.max_trapped_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-cells" => {
                    trap_constraints.min_reachable_cells = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--min-reachable-triggers" => {
                    trap_constraints.min_reachable_triggers = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--require-reachable-trigger" | "--next-trigger" => {
                    trap_constraints.require_reachable_trigger = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--eval" => {
                    eval_point = parse_optional_point(&args[i + 1]);
                    i += 2;
                }
                "--states" => {
                    print_states = true;
                    i += 1;
                }
                "--no-canonical" => {
                    canonical = false;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "branchdump: players={} prefix={} goal={:?} depth={} secs={} maxnodes={} results={} min_rats={:?} trap={:?} canonical={}",
            nplayers,
            prefix.len(),
            goal,
            depth,
            secs,
            max_nodes,
            results,
            min_rats,
            trap_constraints,
            canonical
        );
        let branches = solve_lookup_goal_branches(
            &start_grid,
            depth,
            secs,
            max_nodes,
            goal,
            results,
            min_rats,
            trap_constraints,
            canonical,
        );
        for (idx, branch) in branches.iter().enumerate() {
            let mut full_path = prefix.clone();
            full_path.extend(branch.path.clone());
            let features = Features::from_grid(&branch.grid);
            let eval_text = eval_point
                .map(|point| {
                    let dist = distance_from_player(&branch.grid, point);
                    let kind = branch.grid.cell_kind_at(point.0 as usize, point.1 as usize);
                    format!(
                        " eval=({},{}) dist={} kind={:?}",
                        point.0, point.1, dist, kind
                    )
                })
                .unwrap_or_default();
            println!(
                "BRANCH idx={} suffix={} total={} score={} features={:?} reachable_rats={}{}",
                idx,
                branch.path.len(),
                full_path.len(),
                branch.score,
                features,
                reachable_rat_count(&branch.grid),
                eval_text
            );
            println!("ASCII {}", format_path_ascii(&full_path));
            if print_states {
                println!("STATE\n{}", branch.grid.to_csv());
            }
        }
        return;
    }

    if mode == "frontier" {
        // solver frontier <csv> [--prefix MOVES] [--depth N] [--secs S]
        //                       [--maxnodes N] [--limit N] [--min-rats N]
        //                       [--states] [--no-canonical]
        //
        // Enumerate first paths to distinct local configurations. This is a
        // bounded diagnostic for hand-solving, not a global solver.
        let mut prefix_str = String::new();
        let mut depth = 16usize;
        let mut secs = 10.0f64;
        let mut max_nodes = 50_000usize;
        let mut limit = 80usize;
        let mut min_rats: Option<usize> = None;
        let mut print_states = false;
        let mut canonical = true;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--maxnodes" => {
                    max_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--limit" => {
                    limit = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--min-rats" => {
                    min_rats = Some(args[i + 1].parse().unwrap());
                    i += 2;
                }
                "--states" => {
                    print_states = true;
                    i += 1;
                }
                "--no-canonical" => {
                    canonical = false;
                    i += 1;
                }
                _ => i += 1,
            }
        }

        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        if min_rats.is_some_and(|minimum| count_rats(&start_grid) < minimum) {
            println!("NO_FRONTIER prefix has fewer rats than --min-rats");
            return;
        }

        let state_key = |state: &Grid| {
            if canonical {
                state.search_hash()
            } else {
                state.state_hash()
            }
        };
        let signature = |state: &Grid| {
            let features = Features::from_grid(state);
            format!(
                "r{} x{} w{} t{} p{} players=[{}] rats=[{}] reachable_rats={} reachable_triggers={}",
                features.rats,
                features.explosives,
                features.webs,
                features.triggers,
                features.planks,
                positions_key(state, player_cell),
                positions_key(state, rat_or_cyborg),
                reachable_rat_count(state),
                reachable_trigger_count(state)
            )
        };

        eprintln!(
            "frontier: players={} prefix={} depth={} secs={} maxnodes={} limit={} min_rats={:?} canonical={}",
            nplayers,
            prefix.len(),
            depth,
            secs,
            max_nodes,
            limit,
            min_rats,
            canonical
        );

        let start = Instant::now();
        let tuples = all_action_tuples(nplayers);
        let mut nodes = vec![Node {
            grid: start_grid.clone(),
            parent: usize::MAX,
            action: Vec::new(),
            depth: 0,
        }];
        let mut visited = HashSet::new();
        visited.insert(state_key(&start_grid));
        let mut printed = HashSet::new();
        let mut q = VecDeque::new();
        q.push_back(0usize);

        while let Some(idx) = q.pop_front() {
            if start.elapsed().as_secs_f64() > secs || nodes.len() >= max_nodes {
                break;
            }
            let sig = signature(&nodes[idx].grid);
            if printed.insert(sig.clone()) {
                let mut full_path = prefix.clone();
                let suffix = reconstruct(&nodes, idx);
                full_path.extend(suffix.clone());
                let features = Features::from_grid(&nodes[idx].grid);
                println!(
                    "FRONTIER idx={} depth={} total={} features={:?} trapped={} {}",
                    printed.len() - 1,
                    nodes[idx].depth,
                    full_path.len(),
                    features,
                    trapped_unreachable_rat_count(&nodes[idx].grid),
                    sig
                );
                println!("ASCII {}", format_path_ascii(&full_path));
                println!("SUFFIX {}", format_path_ascii(&suffix));
                if print_states {
                    println!("STATE\n{}", nodes[idx].grid.to_csv());
                }
                if printed.len() >= limit {
                    break;
                }
            }

            if nodes[idx].depth as usize >= depth {
                continue;
            }
            let cur_grid = nodes[idx].grid.clone();
            for actions in &tuples {
                let (next_grid, play_state) = step(&cur_grid, actions);
                if play_state == PlayState::GameOver {
                    continue;
                }
                if play_state != PlayState::Won
                    && min_rats.is_some_and(|minimum| count_rats(&next_grid) < minimum)
                {
                    continue;
                }
                let hash = state_key(&next_grid);
                if !visited.insert(hash) {
                    continue;
                }
                let node_idx = nodes.len();
                nodes.push(Node {
                    grid: next_grid,
                    parent: idx,
                    action: actions.clone(),
                    depth: nodes[idx].depth + 1,
                });
                q.push_back(node_idx);
            }
        }
        eprintln!(
            "frontier done: printed={} nodes={} visited={} elapsed={:.1}s",
            printed.len(),
            nodes.len(),
            visited.len(),
            start.elapsed().as_secs_f64()
        );
        return;
    }

    if mode == "dropchain" {
        // solver dropchain <csv> [--prefix MOVES] [--steps N] [--segdepth N]
        //                        [--segsecs S] [--segnodes N] [--results N]
        //                        [--beam N] [--mopdepth N] [--mopsecs S]
        let mut prefix_str = String::new();
        let mut steps = count_rats(&grid);
        let mut segment_depth = 120usize;
        let mut segment_secs = 20.0;
        let mut segment_nodes = 1_000_000usize;
        let mut segment_results = 32usize;
        let mut beam = 32usize;
        let mut mop_depth = 400usize;
        let mut mop_secs = 20.0;
        let mut mop_strategy = "gbfs".to_string();
        let mut mop_weight = 5i64;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--prefix" => {
                    prefix_str = args[i + 1].clone();
                    i += 2;
                }
                "--steps" => {
                    steps = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segdepth" => {
                    segment_depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segsecs" => {
                    segment_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--segnodes" => {
                    segment_nodes = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--results" => {
                    segment_results = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--beam" => {
                    beam = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopdepth" => {
                    mop_depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopsecs" => {
                    mop_secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--mopstrat" => {
                    mop_strategy = args[i + 1].clone();
                    i += 2;
                }
                "--mopweight" => {
                    mop_weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => i += 1,
            }
        }
        let nplayers = count_players(&grid);
        let prefix = parse_action_string(&prefix_str, nplayers);
        let (start_grid, prefix_state, applied) = replay_path(&grid, &prefix);
        if applied != prefix.len() || prefix_state != PlayState::Playing {
            println!(
                "PREFIX_STOP state={:?} turns_applied={}",
                prefix_state, applied
            );
            return;
        }
        eprintln!(
            "dropchain solve: players={} prefix={} steps={} segdepth={} segsecs={} segnodes={} results={} beam={} mopdepth={} mopsecs={} mopstrat={} mopweight={}",
            nplayers,
            prefix.len(),
            steps,
            segment_depth,
            segment_secs,
            segment_nodes,
            segment_results,
            beam,
            mop_depth,
            mop_secs,
            mop_strategy,
            mop_weight
        );
        let t0 = Instant::now();
        match solve_ratdrop_chain(
            &start_grid,
            steps,
            segment_depth,
            segment_secs,
            segment_nodes,
            segment_results,
            beam,
            mop_depth,
            mop_secs,
            &mop_strategy,
            mop_weight,
        ) {
            Some(suffix) => {
                let mut path = prefix;
                path.extend(suffix);
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                println!("ASCII {}", format_path_ascii(&path));
            }
            None => println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64()),
        }
        return;
    }

    if mode == "solve" {
        let mut depth = 200usize;
        let mut secs = 120.0f64;
        let mut strategy = "gbfs".to_string();
        let mut weight = 5i64;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--depth" => {
                    depth = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--secs" => {
                    secs = args[i + 1].parse().unwrap();
                    i += 2;
                }
                "--strategy" => {
                    strategy = args[i + 1].clone();
                    i += 2;
                }
                "--weight" => {
                    weight = args[i + 1].parse().unwrap();
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        eprintln!(
            "solving: {}x{} players={} strategy={} depth={} secs={} weight={}",
            grid.width(),
            grid.height(),
            nplayers,
            strategy,
            depth,
            secs,
            weight
        );
        let t0 = Instant::now();
        match solve(&grid, depth, secs, &strategy, weight) {
            Some(path) => {
                println!(
                    "SOLVED moves={} time={:.1}s",
                    path.len(),
                    t0.elapsed().as_secs_f64()
                );
                println!("ARROWS {}", format_path(&path));
                // also ascii for portability
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter()
                        .map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                println!("ASCII {}", ascii);
            }
            None => {
                println!("NO_SOLUTION time={:.1}s", t0.elapsed().as_secs_f64());
            }
        }
        return;
    }

    eprintln!("unknown mode {mode}");
    std::process::exit(2);
}
