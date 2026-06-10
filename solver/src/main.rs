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

use infestation::testing::{Action, Dir4, PlayState, apply_actions, game_from_csv, grid_to_csv, play_state};

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

/// Count players in a csv grid (number of arrow glyphs).
fn count_players(csv: &str) -> usize {
    let mut n = 0;
    for c in csv.chars() {
        if matches!(c, '▲' | '▼' | '►' | '◄' | '△' | '▽' | '▷' | '◁') {
            n += 1;
        }
    }
    n
}

/// Parse grid into a 2D vector of chars (first token of each comma cell).
fn parse_grid(csv: &str) -> Vec<Vec<char>> {
    csv.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            line.split(',')
                .map(|tok| tok.trim().chars().next().unwrap_or('.'))
                .collect()
        })
        .collect()
}

const RATS: &[char] = &['R', 'C'];
const PLAYER_GLYPHS: &[char] = &['▲', '▼', '►', '◄', '△', '▽', '▷', '◁'];

fn count_rats(grid: &[Vec<char>]) -> usize {
    grid.iter()
        .flat_map(|r| r.iter())
        .filter(|c| RATS.contains(c))
        .count()
}

/// Player walk-distance BFS: players are blocked by Wall(#), Plank(=), BlackHole(O).
/// Everything else is walkable (webs, empties, triggers, explosives, rats).
/// Returns distance map from all player positions.
fn player_dist_map(grid: &[Vec<char>]) -> Vec<Vec<i32>> {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };
    let mut dist = vec![vec![i32::MAX; w]; h];
    let mut q = VecDeque::new();
    for y in 0..h {
        for x in 0..grid[y].len() {
            if PLAYER_GLYPHS.contains(&grid[y][x]) {
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
            if nx < 0 || ny < 0 || ny as usize >= h || nx as usize >= grid[ny as usize].len() {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let cell = grid[ny][nx];
            if matches!(cell, '#' | '=' | 'O') {
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
fn heuristic(csv: &str) -> i64 {
    let grid = parse_grid(csv);
    let rats = count_rats(&grid) as i64;
    if rats == 0 {
        return 0;
    }
    let dist = player_dist_map(&grid);
    let h = grid.len();
    let mut nearest_rat = i32::MAX;
    let mut nearest_trigger = i32::MAX;
    for y in 0..h {
        for x in 0..grid[y].len() {
            let c = grid[y][x];
            let d = dist[y][x];
            if d == i32::MAX {
                continue;
            }
            if RATS.contains(&c) {
                // player can step onto a rat to kill it (players walk through webs)
                nearest_rat = nearest_rat.min(d);
            } else if c.is_ascii_digit() && c != '0' {
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
    for row in &grid {
        for &c in row {
            if c.is_ascii_digit() && c != '0' {
                triggers_left += 1;
            } else if c == 'X' {
                explosives_left += 1;
            } else if c == 'w' {
                webs_left += 1;
            }
        }
    }
    let secondary = nearest_rat.min(nearest_trigger);
    let secondary = if secondary == i32::MAX { 1000 } else { secondary as i64 };
    // weights chosen so rats dominate, then structural progress, then positioning.
    let use_progress = std::env::var("PROGRESS_H").is_ok();
    if use_progress {
        rats * 1_000_000
            + explosives_left * 300
            + webs_left * 100
            + triggers_left * 2_000
            + secondary
    } else {
        rats * 1_000_000 + triggers_left * 2_000 + secondary
    }
}

/// Transition: from a csv state, apply one set of actions, return (next_csv, play_state).
fn step(csv: &str, actions: &[Action]) -> (String, PlayState) {
    let mut g = game_from_csv(csv);
    let applied = apply_actions(&mut g, actions);
    let st = play_state(&g);
    if !applied {
        // No-op move (blocked entirely / invalid). Return same state, still playing.
        return (csv.to_string(), st);
    }
    (grid_to_csv(&g), st)
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
    csv: String,
    parent: usize,        // usize::MAX for root
    action: Vec<Action>,  // action taken from parent to reach this node
    depth: u32,
}

fn hash_csv(s: &str) -> u64 {
    // FNV-1a
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
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

fn solve(csv: &str, max_depth: usize, time_limit_secs: f64, strategy: &str, weight: i64) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(csv);
    let tuples = all_action_tuples(nplayers);
    let start = Instant::now();

    {
        let g = game_from_csv(csv);
        if play_state(&g) == PlayState::Won {
            return Some(vec![]);
        }
    }

    let mut nodes: Vec<Node> = vec![Node {
        csv: csv.to_string(),
        parent: usize::MAX,
        action: vec![],
        depth: 0,
    }];
    // visited: state-hash -> best depth seen
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(hash_csv(csv), 0);

    let mut expansions: u64 = 0;

    if strategy == "bfs" {
        let mut q: VecDeque<usize> = VecDeque::new();
        q.push_back(0);
        while let Some(idx) = q.pop_front() {
            expansions += 1;
            if expansions % 200_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
                return None;
            }
            let cur_csv = nodes[idx].csv.clone();
            let cur_depth = nodes[idx].depth;
            if cur_depth as usize >= max_depth {
                continue;
            }
            for t in &tuples {
                let (ns, st) = step(&cur_csv, t);
                if st == PlayState::GameOver {
                    continue;
                }
                if st == PlayState::Won {
                    let leaf = nodes.len();
                    nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: cur_depth + 1 });
                    return Some(reconstruct(&nodes, leaf));
                }
                let hh = hash_csv(&ns);
                if !visited.contains_key(&hh) {
                    visited.insert(hh, cur_depth + 1);
                    nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: cur_depth + 1 });
                    q.push_back(nodes.len() - 1);
                }
            }
        }
        return None;
    }

    // Priority-queue search (astar or gbfs)
    let mut pq: BinaryHeap<PQItem> = BinaryHeap::new();
    let h0 = heuristic(csv);
    pq.push(PQItem { f: h0, g: 0, idx: 0 });

    let mut best_h_seen = i64::MAX;
    while let Some(item) = pq.pop() {
        expansions += 1;
        if expansions % 100_000 == 0 && start.elapsed().as_secs_f64() > time_limit_secs {
            eprintln!("  [timeout after {} expansions, best_h={}, nodes={}]", expansions, best_h_seen, nodes.len());
            return None;
        }
        let idx = item.idx;
        let cur_csv = nodes[idx].csv.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g {
            continue; // stale
        }
        if cur_g as usize >= max_depth {
            continue;
        }
        for t in &tuples {
            let (ns, st) = step(&cur_csv, t);
            if st == PlayState::GameOver {
                continue;
            }
            if st == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: (cur_g + 1) as u32 });
                return Some(reconstruct(&nodes, leaf));
            }
            let ng = cur_g + 1;
            let hh = hash_csv(&ns);
            let better = match visited.get(&hh) {
                None => true,
                Some(&pg) => (ng as u32) < pg,
            };
            if better {
                visited.insert(hh, ng as u32);
                let h = heuristic(&ns);
                if h < best_h_seen {
                    best_h_seen = h;
                }
                let f = match strategy {
                    "gbfs" => h,                 // greedy: ignore g
                    _ => ng + weight * h,        // weighted A*
                };
                nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: ng as u32 });
                pq.push(PQItem { f, g: ng, idx: nodes.len() - 1 });
            }
        }
    }
    None
}

/// Find the (single) player position in a csv grid.
fn find_player(grid: &[Vec<char>]) -> Option<(i32, i32)> {
    for y in 0..grid.len() {
        for x in 0..grid[y].len() {
            if PLAYER_GLYPHS.contains(&grid[y][x]) {
                return Some((x as i32, y as i32));
            }
        }
    }
    None
}

fn manhattan(a: (i32, i32), b: (i32, i32)) -> i64 {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as i64
}

enum SegResult {
    Won(Vec<Vec<Action>>),
    Reached(String, Vec<Vec<Action>>),
    Failed,
}

/// Segment A*: drive the player to `target` cell. Heuristic = Manhattan distance.
/// Returns Won if the level is won en route, or Reached(end_csv, moves) on arrival.
fn solve_segment(start: &str, target: (i32, i32), tuples: &[Vec<Action>], per_secs: f64) -> SegResult {
    let t0 = Instant::now();
    let mut nodes: Vec<Node> = vec![Node { csv: start.to_string(), parent: usize::MAX, action: vec![], depth: 0 }];
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(hash_csv(start), 0);
    let mut pq: BinaryHeap<PQItem> = BinaryHeap::new();
    let g0 = parse_grid(start);
    let p0 = find_player(&g0).unwrap();
    pq.push(PQItem { f: manhattan(p0, target), g: 0, idx: 0 });
    let mut exp: u64 = 0;
    while let Some(item) = pq.pop() {
        exp += 1;
        if exp % 50_000 == 0 && t0.elapsed().as_secs_f64() > per_secs {
            return SegResult::Failed;
        }
        let idx = item.idx;
        let cur_csv = nodes[idx].csv.clone();
        let cur_g = nodes[idx].depth as i64;
        if cur_g > item.g {
            continue;
        }
        for t in tuples {
            let (ns, st) = step(&cur_csv, t);
            if st == PlayState::GameOver {
                continue;
            }
            if st == PlayState::Won {
                let leaf = nodes.len();
                nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: (cur_g + 1) as u32 });
                return SegResult::Won(reconstruct(&nodes, leaf));
            }
            let cg = parse_grid(&ns);
            let pp = match find_player(&cg) { Some(p) => p, None => continue };
            if pp == target {
                let leaf = nodes.len();
                nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: (cur_g + 1) as u32 });
                return SegResult::Reached(nodes[leaf].csv.clone(), reconstruct(&nodes, leaf));
            }
            let ng = cur_g + 1;
            let hh = hash_csv(&ns);
            let better = match visited.get(&hh) { None => true, Some(&pg) => (ng as u32) < pg };
            if better {
                visited.insert(hh, ng as u32);
                let h = manhattan(pp, target);
                let leaf = nodes.len();
                nodes.push(Node { csv: ns, parent: idx, action: t.clone(), depth: ng as u32 });
                pq.push(PQItem { f: ng + h, g: ng, idx: leaf });
            }
        }
    }
    SegResult::Failed
}

/// Waypoint-guided solve: visit each target cell in order, then mop up remaining rats.
fn solve_waypoints(
    csv: &str,
    waypoints: &[(i32, i32)],
    per_secs: f64,
    mop_secs: f64,
    mop_strategy: &str,
    mop_weight: i64,
    depth: usize,
) -> Option<Vec<Vec<Action>>> {
    let nplayers = count_players(csv);
    let tuples = all_action_tuples(nplayers);
    let mut all: Vec<Vec<Action>> = Vec::new();
    let mut cur = csv.to_string();

    // already won?
    if play_state(&game_from_csv(&cur)) == PlayState::Won {
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
                eprintln!("  waypoint {} ({},{}) reached in {} moves", i, wp.0, wp.1, mv.len());
                let nplayers_inner = count_players(&end);
                let partial_ascii: String = mv.iter().map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>()).collect::<Vec<_>>().join(" ");
                eprintln!("  partial path: {}", partial_ascii);
                eprintln!("  state after wp{}:\n{}", i, end);
                all.extend(mv);
                cur = end;
                if play_state(&game_from_csv(&cur)) == PlayState::Won {
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
    if let Some(mv) = solve(&cur, depth, mop_secs, mop_strategy, mop_weight) {
        all.extend(mv);
        return Some(all);
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

    if mode == "verify" {
        let action_str = &args[3];
        let nplayers = count_players(&csv);
        let mut state = csv.clone();
        let mut result = PlayState::Playing;
        let mut applied_count = 0;
        if nplayers == 1 {
            for c in action_str.chars() {
                if let Some(a) = ch_to_action(c) {
                    let (ns, st) = step(&state, &[a]);
                    state = ns;
                    result = st;
                    applied_count += 1;
                    if st != PlayState::Playing {
                        break;
                    }
                }
            }
        } else {
            // turns separated by spaces, players by '|'
            for turn in action_str.split_whitespace() {
                let acts: Vec<Action> = turn
                    .chars()
                    .filter(|c| *c != '|')
                    .filter_map(ch_to_action)
                    .collect();
                if acts.len() != nplayers {
                    continue;
                }
                let (ns, st) = step(&state, &acts);
                state = ns;
                result = st;
                applied_count += 1;
                if st != PlayState::Playing {
                    break;
                }
            }
        }
        println!("result={:?} turns_applied={}", result, applied_count);
        return;
    }

    if mode == "trace" {
        // solver trace <csv> "<actions>" — print grid + state after each turn (single player).
        // Lets an agent observe state transitions to reason about play.
        let action_str = &args[3];
        let nplayers = count_players(&csv);
        let mut state = csv.clone();
        println!("turn 0 (initial):\n{}\n", state);
        let mut turn = 0;
        if nplayers == 1 {
            for c in action_str.chars() {
                if let Some(a) = ch_to_action(c) {
                    let (ns, st) = step(&state, &[a]);
                    state = ns;
                    turn += 1;
                    println!("turn {} action={:?} state={:?}:\n{}\n", turn, a, st, state);
                    if st != PlayState::Playing { break; }
                }
            }
        } else {
            for grp in action_str.split_whitespace() {
                let acts: Vec<Action> = grp.chars().filter(|c| *c != '|').filter_map(ch_to_action).collect();
                if acts.len() != nplayers { continue; }
                let (ns, st) = step(&state, &acts);
                state = ns;
                turn += 1;
                println!("turn {} actions={:?} state={:?}:\n{}\n", turn, acts, st, state);
                if st != PlayState::Playing { break; }
            }
        }
        return;
    }

    if mode == "wp" {
        // solver wp <csv> --waypoints "x,y;x,y;..." [--persecs S] [--mopsecs S] [--mopstrat astar] [--mopweight 2] [--depth N]
        let mut waypoints: Vec<(i32, i32)> = Vec::new();
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
                        if p.is_empty() { continue; }
                        let mut it = p.split(',');
                        let x: i32 = it.next().unwrap().trim().parse().unwrap();
                        let y: i32 = it.next().unwrap().trim().parse().unwrap();
                        waypoints.push((x, y));
                    }
                    i += 2;
                }
                "--persecs" => { per_secs = args[i + 1].parse().unwrap(); i += 2; }
                "--mopsecs" => { mop_secs = args[i + 1].parse().unwrap(); i += 2; }
                "--mopstrat" => { mop_strategy = args[i + 1].clone(); i += 2; }
                "--mopweight" => { mop_weight = args[i + 1].parse().unwrap(); i += 2; }
                "--depth" => { depth = args[i + 1].parse().unwrap(); i += 2; }
                _ => { i += 1; }
            }
        }
        let nplayers = count_players(&csv);
        eprintln!("wp solve: {} waypoints, players={}", waypoints.len(), nplayers);
        let t0 = Instant::now();
        match solve_waypoints(&csv, &waypoints, per_secs, mop_secs, &mop_strategy, mop_weight, depth) {
            Some(path) => {
                println!("SOLVED moves={} time={:.1}s", path.len(), t0.elapsed().as_secs_f64());
                println!("ARROWS {}", format_path(&path));
                let ascii: String = if nplayers == 1 {
                    path.iter().map(|a| action_to_ch(a[0])).collect()
                } else {
                    path.iter().map(|a| a.iter().map(|x| action_to_ch(*x)).collect::<String>()).collect::<Vec<_>>().join(" ")
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
        let nplayers = count_players(&csv);
        eprintln!(
            "solving: {}x{} players={} strategy={} depth={} secs={} weight={}",
            csv.lines().next().map(|l| l.split(',').count()).unwrap_or(0),
            csv.lines().filter(|l| !l.trim().is_empty()).count(),
            nplayers,
            strategy,
            depth,
            secs,
            weight
        );
        let t0 = Instant::now();
        match solve(&csv, depth, secs, &strategy, weight) {
            Some(path) => {
                println!("SOLVED moves={} time={:.1}s", path.len(), t0.elapsed().as_secs_f64());
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
