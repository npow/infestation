use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::env;
use std::fs;
use std::time::Instant;

use infestation::testing::{Action, CellKind, Dir4, Grid, PlayState, grid_from_csv, step_grid};

#[derive(Clone)]
struct Node {
    grid: Grid,
    parent: usize,
    action: Action,
    depth: u32,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Item {
    f: i64,
    g: i64,
    idx: usize,
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then(self.g.cmp(&other.g))
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

fn reconstruct(nodes: &[Node], mut idx: usize) -> String {
    let mut out = Vec::new();
    while nodes[idx].parent != usize::MAX {
        out.push(action_to_ch(nodes[idx].action));
        idx = nodes[idx].parent;
    }
    out.reverse();
    out.into_iter().collect()
}

fn find_player(grid: &Grid) -> Option<(usize, usize)> {
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == CellKind::Player {
                return Some((x, y));
            }
        }
    }
    None
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

fn rat_positions(grid: &Grid) -> Vec<(usize, usize)> {
    let mut rats = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if matches!(grid.cell_kind_at(x, y), CellKind::Rat | CellKind::CyborgRat) {
                rats.push((x, y));
            }
        }
    }
    rats
}

fn count_kind(grid: &Grid, kind: CellKind) -> usize {
    let mut count = 0;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.cell_kind_at(x, y) == kind {
                count += 1;
            }
        }
    }
    count
}

fn left_pocket_rats(grid: &Grid) -> usize {
    rat_positions(grid)
        .into_iter()
        .filter(|&(x, y)| x <= 2 && y >= 10)
        .count()
}

fn nearest_player_rat_dist(grid: &Grid) -> i64 {
    let Some((px, py)) = find_player(grid) else {
        return 10_000;
    };
    rat_positions(grid)
        .into_iter()
        .map(|(x, y)| px.abs_diff(x) as i64 + py.abs_diff(y) as i64)
        .min()
        .unwrap_or(0)
}

fn score(grid: &Grid) -> i64 {
    let rats = count_rats(grid) as i64;
    let pocket = left_pocket_rats(grid) as i64;
    let explosives = count_kind(grid, CellKind::Explosive) as i64;
    let webs = count_kind(grid, CellKind::Spiderweb) as i64;
    rats * 1_000_000 + pocket * 5_000_000 + explosives * 500 + webs * 100 + nearest_player_rat_dist(grid)
}

fn solve(grid: &Grid, max_depth: usize, secs: f64) -> Option<String> {
    let actions = [
        Action::Move(Dir4::North),
        Action::Move(Dir4::South),
        Action::Move(Dir4::East),
        Action::Move(Dir4::West),
        Action::Stall,
    ];
    let start = Instant::now();
    let mut nodes = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Action::Stall,
        depth: 0,
    }];
    let mut best_idx = 0usize;
    let mut best_score = score(grid);
    let mut visited: HashMap<u64, u32> = HashMap::new();
    visited.insert(grid.state_hash(), 0);
    let mut heap = BinaryHeap::new();
    heap.push(Item {
        f: best_score,
        g: 0,
        idx: 0,
    });
    let mut expansions = 0u64;

    while let Some(item) = heap.pop() {
        expansions += 1;
        if expansions % 200_000 == 0 && start.elapsed().as_secs_f64() > secs {
            eprintln!(
                "timeout expansions={expansions} nodes={} best_score={best_score}",
                nodes.len()
            );
            eprintln!("BEST {}", reconstruct(&nodes, best_idx));
            eprintln!("{}", nodes[best_idx].grid.to_csv());
            return None;
        }

        let idx = item.idx;
        let depth = nodes[idx].depth;
        if item.g != depth as i64 || depth as usize >= max_depth {
            continue;
        }

        for &action in &actions {
            let (next, state) = step_grid(&nodes[idx].grid, &[action]);
            if state == PlayState::GameOver {
                continue;
            }
            let child_depth = depth + 1;
            let hash = next.state_hash();
            if visited.get(&hash).is_some_and(|&old| old <= child_depth) {
                continue;
            }
            visited.insert(hash, child_depth);
            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next,
                parent: idx,
                action,
                depth: child_depth,
            });
            if state == PlayState::Won {
                return Some(reconstruct(&nodes, node_idx));
            }
            let s = score(&nodes[node_idx].grid);
            if s < best_score {
                best_score = s;
                best_idx = node_idx;
            }
            heap.push(Item {
                f: s + child_depth as i64,
                g: child_depth as i64,
                idx: node_idx,
            });
        }
    }
    None
}

fn shortest_to_no_left_pocket(grid: &Grid, max_depth: usize, secs: f64) -> Option<String> {
    let actions = [
        Action::Move(Dir4::North),
        Action::Move(Dir4::South),
        Action::Move(Dir4::East),
        Action::Move(Dir4::West),
        Action::Stall,
    ];
    let start_pocket = left_pocket_rats(grid);
    let start = Instant::now();
    let mut nodes = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Action::Stall,
        depth: 0,
    }];
    let mut seen = HashMap::new();
    seen.insert(grid.state_hash(), 0u32);
    let mut queue = VecDeque::from([0usize]);
    let mut expansions = 0u64;
    while let Some(idx) = queue.pop_front() {
        expansions += 1;
        if expansions % 50_000 == 0 && start.elapsed().as_secs_f64() > secs {
            eprintln!("no-left timeout expansions={expansions} nodes={}", nodes.len());
            return None;
        }
        let depth = nodes[idx].depth;
        if depth as usize >= max_depth {
            continue;
        }
        for &action in &actions {
            let (next, state) = step_grid(&nodes[idx].grid, &[action]);
            if state == PlayState::GameOver {
                continue;
            }
            let child_depth = depth + 1;
            let hash = next.state_hash();
            if seen.get(&hash).is_some_and(|&old| old <= child_depth) {
                continue;
            }
            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next,
                parent: idx,
                action,
                depth: child_depth,
            });
            if state == PlayState::Won || left_pocket_rats(&nodes[node_idx].grid) < start_pocket {
                return Some(reconstruct(&nodes, node_idx));
            }
            seen.insert(hash, child_depth);
            queue.push_back(node_idx);
        }
    }
    None
}

fn shortest_until_cell_not(
    grid: &Grid,
    target: (usize, usize),
    blocked_kind: CellKind,
    max_depth: usize,
    secs: f64,
) -> Option<String> {
    let actions = [
        Action::Move(Dir4::North),
        Action::Move(Dir4::South),
        Action::Move(Dir4::East),
        Action::Move(Dir4::West),
        Action::Stall,
    ];
    let start = Instant::now();
    let mut nodes = vec![Node {
        grid: grid.clone(),
        parent: usize::MAX,
        action: Action::Stall,
        depth: 0,
    }];
    let mut seen = HashMap::new();
    seen.insert(grid.state_hash(), 0u32);
    let mut queue = VecDeque::from([0usize]);
    let mut expansions = 0u64;
    while let Some(idx) = queue.pop_front() {
        expansions += 1;
        if expansions % 50_000 == 0 && start.elapsed().as_secs_f64() > secs {
            eprintln!("cell-open timeout expansions={expansions} nodes={}", nodes.len());
            return None;
        }
        let depth = nodes[idx].depth;
        if depth as usize >= max_depth {
            continue;
        }
        for &action in &actions {
            let (next, state) = step_grid(&nodes[idx].grid, &[action]);
            if state == PlayState::GameOver {
                continue;
            }
            let child_depth = depth + 1;
            let hash = next.state_hash();
            if seen.get(&hash).is_some_and(|&old| old <= child_depth) {
                continue;
            }
            let node_idx = nodes.len();
            nodes.push(Node {
                grid: next,
                parent: idx,
                action,
                depth: child_depth,
            });
            if state == PlayState::Won
                || nodes[node_idx].grid.cell_kind_at(target.0, target.1) != blocked_kind
            {
                return Some(reconstruct(&nodes, node_idx));
            }
            seen.insert(hash, child_depth);
            queue.push_back(node_idx);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: no_retreat_helper <solve|no-left|open-pocket> <csv> [depth] [secs]");
        std::process::exit(2);
    }
    let csv = fs::read_to_string(&args[2]).expect("read csv");
    let grid = grid_from_csv(csv.trim_end_matches('\n'));
    let depth = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(500);
    let secs = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(60.0);
    match args[1].as_str() {
        "solve" => {
            if let Some(path) = solve(&grid, depth, secs) {
                println!("SOLVED {path}");
            } else {
                println!("NO_SOLUTION");
            }
        }
        "no-left" => {
            if let Some(path) = shortest_to_no_left_pocket(&grid, depth, secs) {
                println!("NO_LEFT {path}");
            } else {
                println!("NO_LEFT_FAILED");
            }
        }
        "open-pocket" => {
            if let Some(path) = shortest_until_cell_not(&grid, (3, 13), CellKind::Plank, depth, secs)
            {
                println!("OPEN_POCKET {path}");
            } else {
                println!("OPEN_POCKET_FAILED");
            }
        }
        other => panic!("unknown mode {other}"),
    }
}
