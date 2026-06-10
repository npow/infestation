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

/// Player walk-distance BFS: players are blocked by Wall(#), Plank(=), BlackHole(O).
/// Everything else is walkable (webs, empties, triggers, explosives, rats).
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
            if matches!(cell, CellKind::Wall | CellKind::Plank | CellKind::BlackHole) {
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
        rats * 5_000_000
    } else {
        0
    };
    // weights chosen so rats dominate, then structural progress, then positioning.
    let use_progress = std::env::var("PROGRESS_H").is_ok();
    let smart_penalty = if std::env::var("SMART_H").is_ok() {
        unreachable_rats * 4_000_000 + unreachable_triggers * 500_000
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

    if count_rats(grid) == 0 {
        return Some(vec![]);
    }

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
            if expansions % 200_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
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
        if expansions % 100_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
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
        if expansions % 100_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
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

fn trigger_branch_score(grid: &Grid, path_len: usize, before: Features, after: Features) -> i64 {
    let structural_gain = before.rats.saturating_sub(after.rats) as i64 * 1_000_000
        + before.explosives.saturating_sub(after.explosives) as i64 * 2_000
        + before.webs.saturating_sub(after.webs) as i64 * 500
        + before.planks.saturating_sub(after.planks) as i64 * 500
        + before.triggers.saturating_sub(after.triggers) as i64 * 2_000;
    heuristic(grid) + path_len as i64 + resource_exhaustion_penalty(after) - structural_gain
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
    visited.insert(start.state_hash(), 0);
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
            let hh = ns.state_hash();
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
    visited.insert(start.state_hash(), 0);
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
            let hh = ns.state_hash();
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
            let rats = positions_matching(grid, |cell| {
                matches!(cell, CellKind::Rat | CellKind::CyborgRat)
            });
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
    let mut branches = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];

    for &number in order {
        let mut next_branches = Vec::new();
        for branch in &branches {
            for target in trigger_positions(&branch.grid, number) {
                match solve_segment(&branch.grid, target, &tuples, per_secs) {
                    SegResult::Won(segment) => {
                        let mut path = branch.path.clone();
                        path.extend(segment);
                        return Some(path);
                    }
                    SegResult::Reached(end, segment) => {
                        let before = Features::from_grid(&branch.grid);
                        let after = Features::from_grid(&end);
                        let mut path = branch.path.clone();
                        path.extend(segment);
                        let score = trigger_branch_score(&end, path.len(), before, after);
                        next_branches.push(Branch {
                            grid: end,
                            path,
                            score,
                        });
                    }
                    SegResult::Failed => {}
                }
            }
        }

        if next_branches.is_empty() {
            eprintln!("  trigger {}: no reachable branches", number);
            return None;
        }

        next_branches.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut deduped = Vec::new();
        for branch in next_branches {
            if seen.insert(branch.grid.state_hash()) {
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
    let mut branches = vec![Branch {
        grid: grid.clone(),
        path: Vec::new(),
        score: heuristic(grid),
    }];

    for step_idx in 0..macro_steps {
        branches.sort_by_key(|branch| branch.score);
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

        let mut next_branches = Vec::new();
        for branch in &branches {
            let cells = trigger_cells(&branch.grid);
            for (target, number) in cells {
                match solve_segment(&branch.grid, target, &tuples, per_secs) {
                    SegResult::Won(segment) => {
                        let mut path = branch.path.clone();
                        path.extend(segment);
                        return Some(path);
                    }
                    SegResult::Reached(end, segment) => {
                        let before = Features::from_grid(&branch.grid);
                        let after = Features::from_grid(&end);
                        if after == before && branch.grid.state_hash() == end.state_hash() {
                            continue;
                        }
                        let mut path = branch.path.clone();
                        path.extend(segment);
                        let score = trigger_branch_score(&end, path.len(), before, after);
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
                        next_branches.push(Branch {
                            grid: end,
                            path,
                            score,
                        });
                    }
                    SegResult::Failed => {}
                }
            }
        }

        if next_branches.is_empty() {
            eprintln!("  trigger step {}: no reachable branches", step_idx + 1);
            return None;
        }

        next_branches.sort_by_key(|branch| branch.score);
        let mut seen = HashSet::new();
        let mut deduped = Vec::new();
        for branch in next_branches {
            if seen.insert(branch.grid.state_hash()) {
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
    visited.insert(start_grid.state_hash());
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

            let hash = next_grid.state_hash();
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
                    let score = heuristic(&nodes[node_idx].grid)
                        + path.len() as i64
                        + resource_exhaustion_penalty(features);
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
    let h0 = heuristic(grid);
    pq.push(PQItem {
        f: h0,
        g: 0,
        idx: 0,
    });
    let mut visited = HashSet::new();
    visited.insert(grid.state_hash());
    let mut expansions = 0u64;

    while let Some(item) = pq.pop() {
        expansions += 1;
        if start_time.elapsed().as_secs_f64() > total_secs {
            eprintln!(
                "  [macro timeout after {} event nodes, stored={}]",
                expansions,
                nodes.len()
            );
            return None;
        }
        if nodes.len() >= max_events {
            eprintln!("  [macro event limit reached, stored={}]", nodes.len());
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

            let hash = event.grid.state_hash();
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
    visited.insert(grid.state_hash());

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
                let hash = next_grid.state_hash();
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
            let hash = next_grid.state_hash();
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
    if grid.width() >= 16 {
        return rat_positions(grid);
    }

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

fn tinderbox_heuristic(
    grid: &Grid,
    initial_rats: usize,
    initial_explosives: usize,
    rat_target: (i32, i32),
    safe_target: (i32, i32),
) -> i64 {
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
        vec![
            (10, 2),
            (11, 2),
            (12, 2),
            (13, 2),
            (14, 2),
            (15, 2),
            (12, 3),
            (13, 3),
            (14, 3),
            (15, 3),
            (10, 4),
            (12, 4),
            (14, 4),
            (15, 4),
            (10, 5),
            (12, 5),
            (14, 5),
            (15, 5),
            (10, 6),
            (11, 6),
            (12, 6),
            (14, 6),
            (15, 6),
            (14, 7),
            (15, 7),
            (13, 8),
            (14, 8),
            (15, 8),
        ]
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

    if mode == "tinder" {
        // solver tinder <csv> [--depth N] [--secs S] [--strategy gbfs|astar] [--weight W]
        //                   [--rat-target x,y] [--safe-target x,y]
        let mut depth = 260usize;
        let mut secs = 120.0f64;
        let mut strategy = "gbfs".to_string();
        let mut weight = 1i64;
        let mut rat_target = (6, 5);
        let mut safe_target = (6, 8);
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
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        eprintln!(
            "tinder solve: players={} strategy={} depth={} secs={} weight={} rat_target={:?} safe_target={:?}",
            nplayers, strategy, depth, secs, weight, rat_target, safe_target
        );
        let t0 = Instant::now();
        match solve_tinderbox(
            &grid,
            depth,
            secs,
            &strategy,
            weight,
            rat_target,
            safe_target,
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
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        eprintln!(
            "tinder beam: players={} width={} depth={} secs={} seed={} jitter={} rat_target={:?} safe_target={:?}",
            nplayers, width, depth, secs, seed, jitter, rat_target, safe_target
        );
        let t0 = Instant::now();
        match solve_tinderbox_beam(
            &grid,
            width,
            depth,
            secs,
            seed,
            jitter,
            rat_target,
            safe_target,
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
        eprintln!(
            "trigger solve: order={:?}, players={}, per_secs={}, beam={}",
            order, nplayers, per_secs, beam
        );
        let t0 = Instant::now();
        match solve_trigger_order(
            &grid,
            &order,
            per_secs,
            beam,
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

    if mode == "trigany" {
        // solver trigany <csv> [--steps N] [--persecs S] [--beam N]
        //                      [--mopsecs S] [--mopstrat astar] [--mopweight 2] [--depth N]
        let mut steps = trigger_numbers(&grid).len().max(1);
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
        eprintln!(
            "trigger-any solve: players={} steps={} per_secs={} beam={}",
            nplayers, steps, per_secs, beam
        );
        let t0 = Instant::now();
        match solve_any_trigger_order(
            &grid,
            steps,
            per_secs,
            beam,
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

    if mode == "macro" {
        // solver macro <csv> [--segdepth N] [--segsecs S] [--beam N] [--events N] [--secs S]
        let mut segment_depth = 40usize;
        let mut segment_secs = 5.0;
        let mut event_beam = 16usize;
        let mut max_events = 200usize;
        let mut total_secs = 120.0;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
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
        eprintln!(
            "macro solve: players={} segdepth={} segsecs={} beam={} events={} secs={}",
            nplayers, segment_depth, segment_secs, event_beam, max_events, total_secs
        );
        let t0 = Instant::now();
        match solve_macro_events(
            &grid,
            segment_depth,
            segment_secs,
            event_beam,
            max_events,
            total_secs,
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
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        eprintln!(
            "event solve: kind={:?} players={} strategy={} depth={} secs={} weight={}",
            target, nplayers, strategy, depth, secs, weight
        );
        let t0 = Instant::now();
        match solve_to_event(&grid, target, depth, secs, &strategy, weight) {
            Some((mut path, event_grid)) => {
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
        let mut depth = 80usize;
        let mut secs = 30.0;
        let mut max_events = 20usize;
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
        let tuples = all_action_tuples(nplayers);
        eprintln!(
            "events: players={} depth={} secs={} max={}",
            nplayers, depth, secs, max_events
        );
        match find_event_successors(&grid, &tuples, depth, secs, max_events) {
            Some(events) => {
                for (idx, event) in events.iter().enumerate() {
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
                    println!(
                        "EVENT idx={} moves={} score={} features={:?}",
                        idx,
                        event.path.len(),
                        event.score,
                        event.features
                    );
                    println!("ASCII {}", ascii);
                    println!("STATE\n{}", event.grid.to_csv());
                }
            }
            None => println!("NO_EVENTS"),
        }
        return;
    }

    if mode == "beam" {
        // solver beam <csv> [--width N] [--depth N] [--secs S] [--seed N] [--jitter N]
        let mut width = 1000usize;
        let mut depth = 300usize;
        let mut secs = 120.0;
        let mut seed = 1u64;
        let mut jitter = 10_000i64;
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
                _ => {
                    i += 1;
                }
            }
        }
        let nplayers = count_players(&grid);
        eprintln!(
            "beam solve: players={} width={} depth={} secs={} seed={} jitter={}",
            nplayers, width, depth, secs, seed, jitter
        );
        let t0 = Instant::now();
        match solve_beam(&grid, width, depth, secs, seed, jitter) {
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

    if mode == "novelty" {
        // solver novelty <csv> [--k 2] [--depth N] [--secs S]
        let mut k = 2u8;
        let mut depth = 400usize;
        let mut secs = 120.0;
        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
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
        eprintln!(
            "novelty solve: players={} k={} depth={} secs={}",
            nplayers, k, depth, secs
        );
        let t0 = Instant::now();
        match solve_novelty(&grid, depth, secs, k) {
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
