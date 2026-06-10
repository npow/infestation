# Infestation solving campaign — HANDOFF

Resume doc for continuing the effort on another machine. **Goal: solve the 11
remaining hard levels.** 22/33 originals + 5 new puzzles are already solved & shipped.

---

## 0. Resume in 60 seconds

```bash
# this fork, this branch:
git clone https://github.com/npow/infestation.git && cd infestation
git checkout claude/new-puzzles

# build the oracle (the whole effort depends on it).
# build.rs downloads a font on first build; if offline:
printf dummy > assets/DejaVuSans.ttf      # font is only used at runtime by render, NOT by the solver
cargo build --release -p solver           # ~25s first time (compiles macroquad/miniquad natively)

# sanity:
target/release/solver verify levels/rats.csv "v<<^^^>>>><>^"   # -> result=Won
```

Everything I built lives in `solver/` (the oracle) and `solver/solutions/` (results, tooling, docs).

---

## 1. The oracle = ground truth

`solver/` is a Rust crate that **links the game's real logic** (`infestation::testing`),
so every result is exactly what the shipped game does. Binary: `target/release/solver`.

| Mode | Usage | Notes |
|---|---|---|
| **solve** | `solver solve <csv> [--strategy gbfs\|astar\|bfs] [--weight W] [--depth N] [--secs S]` | Heuristic search. `PROGRESS_H=1` env enables the progress heuristic (rewards detonated explosives / cleared webs / consumed triggers — gives a gradient on chain puzzles). |
| **verify** | `solver verify <csv> "<moves>"` | Replays, prints `result=Won/GameOver/Playing turns_applied=N`. |
| **trace** | `solver trace <csv> "<moves>"` | Prints the **board after every turn** — watch rat reactions. Essential for hand-solving. |
| **wp** | `solver wp <csv> --waypoints "x,y;x,y;..." [--persecs S] [--mopsecs S] [--mopstrat astar] [--mopweight W]` | **Waypoint-guided.** Drives the player to each cell in order (gradient = Manhattan dist), then mops up remaining rats. The key tool for gradient-less puzzles: *you supply the plan, it fills in the moves.* |

**Move encoding:** `^`=N `v`=S `<`=W `>`=E `.`=stall. Single-player = a string
(`v<<^>`). Two-player = space-separated 2-char turns, P1 then P2 (`^^ <> v.`).

---

## 2. Mechanics cheat-sheet (verified from source + oracle — trust these)

- **Turn order:** players move → cyborg rats move (Dijkstra path, nearest first) →
  normal rats move (minimize Euclidean dist to nearest player, nearest first) →
  trigger zap waves → explosion waves.
- **Win:** no rats left (level started with ≥1). **Lose:** any player dies.
- **⚠ Stepping onto an explosive ALWAYS kills you** (the blast clears the cell you're
  standing on). Explosives only fire when a **rat** steps on one, or via a **trigger zap**.
  This is why every "rush the rats" solution dies — the trick is always *don't touch X*.
- **Trigger `n`:** stepping on it turns **all other** trigger-`n` cells into walls and
  **zaps** their 8 neighbors (empty→wall, explosive→detonate→chain). The trigger you're
  standing on becomes you (not zapped). → enables **twin-trigger remote detonation**.
- **Sword:** you face your last move direction; a rat/cyborg entering from *directly in
  front* of the sword is blocked, from any other side it kills you. You kill a rat by
  moving onto its cell (you advance onto it; rats move simultaneously, so flanking is real).
- **Webs** block rats & cyborgs, NOT players. **Planks** block players; rats walk through
  destroying them; cyborgs pathfind through. **Black holes** swallow any entity that enters.
- **Pivot-against-wall** (verified): moving into a wall *re-aims your sword without moving*
  — the only way to turn in place (stalling keeps your facing).
- **GOTCHA:** an earlier hand-ported Python engine (`tools/scratch/python_engine_BUGGY.py`)
  had subtle bugs in explosion timing / cyborg AI / 2-player resolution — **4 of its
  "solutions" were actually losses.** Never trust a re-implementation; verify with the Rust oracle.

---

## 3. Status

### Solved — 22/33 originals + 5 new (all oracle-verified `result=Won`)
Move strings: **`solver/solutions/SOLUTIONS.md`** (machine-readable: `final_solutions.json`).
Browser auto-player: `solver/solutions/autoplay.js`. New puzzles: `levels/claude/`.

### UNSOLVED — the 11 (this is the job)

| # | Level | Players | Name-hint / trick | Best lead / recommended attack |
|---|---|---|---|---|
| 1 | `tinderbox` | 1 | "tinderbox" = it **is** ignitable | **Strongest lead:** `(6,6)=X` with `(6,5)=.` open beside it. If a rat reaches `(6,5)` with the player south, it steps onto the X → chain kills all 11. Need to route rat `(1,5)`→`(6,5)`; webs block rats, and clearing webs needs explosions. Hand-trace the ignition. |
| 2 | `tinderrectangle` | 1 | same family as tinderbox | Same ignition idea; map its explosive frame. |
| 3 | `no_retreat` | 1 | forward-only; lure rats onto X | `PROGRESS_H=1 gbfs` once reached **2 rats remaining** then stuck in a local min. Try `astar PROGRESS_H` longer, or find the per-unit lure pattern (`R,w\|X` repeats) and tile via `wp`. |
| 4 | `cyborg_rats/ai_takeover` | 1 | cyborgs + triggers 1-8 | Exploit Dijkstra: trap on the cyborg's shortest path (it commits where a dumb rat wouldn't). |
| 5 | `release` | 1 | **RELEASE** the caged rats (they MOVE) | Not a static-rat puzzle → PDDL failed. `wp` over trigger cells + lure the released rats. |
| 6 | `lock_in` | 1 | **seal** rats in via trigger-walls | Most tractable trigger puzzle. `wp` with trigger-order; or auto-enumerate trigger orderings. |
| 7 | `reload_v3` | 1 | fire→reload cycles; triggers 1-7 | Repeated detonation cycles; `wp` trigger-order. |
| 8 | `chase` | 1 | **kite** rats into holes/X | Lead the chasers through traps; `wp` your kite path. |
| 9 | `cooperation/tug_of_war` | 2 | mirror-symmetric → **mirrored moves** | Try symmetric P1/P2 pairs; may need sync. (2p `wp` not implemented — see §4.) |
| 10 | `cooperation/handoff` | 2 | **baton-pass**: one enables the other | Sequence P1-enables→P2-advances. |
| 11 | `cooperation/blocked_v2` | 2 | one player **blocked** | ⚠ coop agent's analysis suggested rat at `(6,10)` may be **permanently unreachable** (only entrance is an indestructible plank) → **possibly UNWINNABLE.** Verify rigorously (deep/exhaustive search) before sinking time. |

### Methods already tried & exhausted on all 11 (so don't repeat blindly)
- Heuristic search (gbfs/astar, weights 1-10, `PROGRESS_H`, depth 400-500, 240-450s each): **0/11**.
- PDDL (pyperplan): agent **crashed at 53 min**, 0.
- LLM-play agents (Opus×2 / Sonnet, oracle+trace in the loop, ~55 min): **0 verified** — but produced the leads above. They did **not** use the `wp` solver, which is likely why they stalled hand-tracing.

---

## 4. Planned next steps (NOT yet done — start here)

1. **Speed up the oracle (highest ROI, ~half-planned).** It's ~3,800 states/sec because
   every move round-trips through CSV `parse`+`serialize`. Add to `game/src/testing.rs`:
   `pub fn step_grid(grid:&Grid, actions:&[Action]) -> (Grid, PlayState)` (Grid is `Clone`),
   and a way to hash a `Grid`'s cells directly (derive/expose). Switch `solver/src/main.rs`
   `step()` and the visited-set hashing off strings. Est. **10-50×** → deep search becomes
   feasible. (I was reading `game/src/grid.rs` to do this when paused.)
2. **Auto-waypoint trigger-ordering mode.** New solver mode: enumerate trigger cells, try
   orderings (greedy + permutations), run `wp` segments between them + mop-up. Should crack
   `lock_in` / `release` / `reload_v3`.
3. **Workflow fan-out** (ultracode): one *focused* LLM agent per remaining level, each told
   to **use `wp`** (supply a plan, let search fill moves) + its specific lead, iterate
   `trace`/`verify`, and **return the verified move string**. Focus-per-level + `wp` is the
   fix for what stalled the earlier agents.
4. **Hand-solve stragglers** — `tinderbox` lead is the most promising concrete start.
5. **`blocked_v2`:** rigorously test winnability before assuming solvable.

---

## 5. File map

```
HANDOFF.md                          ← this file
levels/claude/                      5 new puzzles + gauntlet hub + README (all verified)
solver/                             Rust oracle crate
  src/main.rs                         all modes: solve / verify / trace / wp
  Cargo.toml
solver/solutions/
  SOLUTIONS.md                       27 verified solutions (table)
  final_solutions.json               machine-readable verified set
  autoplay.js                        browser console auto-player (1p + 2p)
  results/                           raw search outputs (results*.json, autoplay_data.json)
  tools/                             orchestration + verification scripts
    build_final.py, verify.py, gen_docs.py, run_all.py, run_failures.py, run_pass3.py
    scratch/                         historical Python engine (BUGGY) + agent IDA*/sim experiments
```
> Tooling scripts have paths hard-coded to the session workspace (`/tmp/infestation`); adjust on resume.

## 6. Session context
- A session Stop-hook with goal **"solve all the puzzles"** is active (blocks stopping until
  all 11 are solved; auto-clears on success). Resume by working §4.
- Fork created with `gh repo fork`; push with `gh auth setup-git --hostname github.com` then
  `git push fork claude/new-puzzles`. No PR was opened to upstream (`davidspies/infestation`).
