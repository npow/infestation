# Infestation solving campaign — HANDOFF

Resume doc for continuing the effort on another machine. **Goal: solve the 8
remaining hard levels.** 27/35 non-Claude playable CSV levels + 5 new puzzles
are already solved & shipped.

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

### Solved - 27/35 non-Claude levels + 5 new (all oracle-verified `result=Won`)
Move strings: **`solver/solutions/SOLUTIONS.md`** (machine-readable: `final_solutions.json`).
Browser auto-player: `solver/solutions/autoplay.js`. New puzzles: `levels/claude/`.

`HANDOFF.md` used to say 11 remained, but `solver/solutions/SOLUTIONS.md`
now includes verified wins for `tinderbox`, `no_retreat`, `lock_in`, and
`limited2`.

### UNSOLVED - the 8 (this is the job)

| # | Level | Players | Name-hint / trick | Best lead / recommended attack |
|---|---|---|---|---|
| 1 | `tinderrectangle` | 1 | pure ignition geometry | `ignitions` says a top-pack rat at `(0,0)` or `(16,0)` can detonate the rectangle and win. Directly cutting the left web from `(1,3)`/`(2,3)` kills the player. Treat this as a lure/facing puzzle: shape a top rat into the explosive corner, then make the one safe nudge. |
| 2 | `release` | 1 | release the caged rats, then mop | Strong human prefix: `v<vv^^>>v` consumes trigger 3 then 4, drops rats from 24 to 23, explosives from 35 to 5, webs from 47 to 27, and makes 21 rats reachable. Follow-up trigger 5 is reachable with suffix `v<>>>^`; next work is choosing between trigger 2 and 6, then mop-up. |
| 3 | `reload_v3` | 1 | fire/reload cycles; triggers 1-7 | Work bottom trigger row as reload stations, not as a global search. Likely order starts around trigger 1, then 2/3/4/5/6/7 as each detonation opens the next chamber. Use `triglookup` with explicit orders and inspect each irreversible change. |
| 4 | `chase` | 1 | kite rats into holes/X | All triggers are player-reachable, but only 4/8 rats are initially reachable. Solve as a route plan: trigger/kite the chasers through holes and the explosive lane, then mop. Do not let A* chase all rats directly. |
| 5 | `cyborg_rats/ai_takeover` | 1 | `release` skeleton plus cyborgs/triggers 7-8 | Solve `release` first, then transfer the trigger skeleton. Extra triggers 7/8 and cyborg Dijkstra behavior are probably the intended differences. |
| 6 | `cooperation/tug_of_war` | 2 | mirror-symmetric tug | Needs paired role choreography with `wp2`: mirrored trigger pairs 1/2/3, side rats, then central rat. Avoid generic 2p search until the waypoint pairs encode the intended symmetry. |
| 7 | `cooperation/handoff` | 2 | baton pass | Small enough to hand-reason. P1 cannot simply reach trigger 1 first. P1 can reach trigger 2 first, but then trigger 1 is no longer useful/reachable; likely P1 opens the handoff and P2 finishes on the remote side. |
| 8 | `cooperation/blocked_v2` | 2 | one player blocked | Keep the previous warning: one rat may be permanently unreachable behind effectively indestructible structure. Before spending human-solving time, prove or disprove winnability with targeted reachability/exhaustive checks. |

### Approach update - 2026-06-11

The remaining puzzles should not be attacked with another blind `solver solve`
run. Treat the oracle as a microscope for human hypotheses:

1. Run `solver diag <level>` first. Record reachable rats/triggers, explosive
   count, black holes, and which rats are in inaccessible components.
2. Identify the intended irreversible event: a trigger consumed, a rat dropped,
   an explosive chain, a web corridor opened, or a two-player handoff.
3. Use short-goal tools to validate only that event:
   - `solver ignitions` for one-step explosive geometry.
   - `solver trace` for exact rat movement after a proposed human line.
   - `solver branchdump` / `solver lookup --goal ratat:x,y|ratgone:x,y|trigger:n`
     for tactical subgoals.
   - `solver wp` / `solver wp2` after deciding the plan; waypoints should encode
     the human route, not discover the route from scratch.
   - `solver triglookup` with explicit trigger orders after the trigger plan is
     known. Use `triganylookup` only to find candidate trigger orders, then
     inspect the best branch and continue from its prefix.
4. Keep any partial that causes irreversible progress. A timeout with a lower
   rat/explosive/web count is a lead, not a failure.
5. Commit verified prefixes and observations even if they are not complete
   wins; the next iteration should continue from the best known state.

### Current run notes - 2026-06-11

No new verified wins yet. Useful observations to preserve:

- `release`: the 14-rat branch
  `v<vv^^>>vv<>>>^^vvv<<<<<<^^^<<<<v^^^^^^^^^v>>>^^>>>`
  is a strategic dead end for the right-side rat. From that state, `(18,5)`,
  `(17,5)`, `(19,7)`, and `(0,16)` are all unreachable by waypoint search, and
  `lookup --goal ratgone:18,4` / `cell:18,5` time out with no candidate. The
  trigger-6 suffix
  `><><><><><><>>>>>>><<<<<<<<<vvvvvv<<<<<^^^^^^>>>>>>>>>>v>>>>v>v>vvvvvvvvv>>>vvvv`
  reduces to one rat but seals the player bottom-right with `(18,4)` still
  unreachable. Trigger 2 is not player-reachable from checked prefixes at
  lengths 15, 23, 38, 42, or 51; if trigger 2 is part of the solution it likely
  has to be rat-triggered or reached before the current prefix family.
- `tinderrectangle`: `TRAP_H=1` exposes the intended lower trap row. Best useful
  partial so far is
  `<<<^<^<>^>>>vv^^>>v>v.>>><>v`, with the lower rat at `(6,6)` and player at
  `(10,6)`. Waypointing to `(13,8)`, `(14,8)`, or `(15,8)` is possible, but the
  rat tracks to `(8,6)` and gets pinned instead of stepping onto the explosive
  row. The missing trick is timing/facing that holds the rat near `(6,6)` while
  the player reaches the safe side.
- `reload_v3`: direct smart/regular search finds a strong-looking but dead
  1-rat partial:
  `^^^^^<<<<<<^^^^^^^^^>>>>>>>>>vv>>>vv>vvvvvvv<v<^^>>^^^^^^<<<<vv<<<<>>>>^^>>>>vvvvvvv<<^^<<<<<<<<<<<^^^^^^^vvvvvvv>>>>>>>>>>>vv`.
  It leaves only rat `(0,21)`, but trigger 6 / waypoint `(2,22)` are unreachable
  from that state. The final cage must be opened before this prefix closes the
  lower route.
- `cooperation/handoff`: trigger-first branches reduce to two unreachable rats.
  Example trigger-2 branch:
  `v^ >^ >^ >^ >^ >^ ^^ v^ ^^ ^^ v^ v^ vv <v`.
  `triganylookup`, `branchdump trigger:1`, `branchdump trigger:2`, and `macro`
  all converge to the same stuck shape. The handoff likely needs positional rat
  handling before either trigger is consumed.
- `chase`: first useful ratdrop branches include `>>>.^v<<<>>>vv<^`, reducing to
  six rats with more structure opened, but smart direct search still ends with
  multiple isolated components. Continue from branch states, not from the initial
  generic heuristic.
- `tinderrectangle`: strict lower-trap and corner-ignition probes did not produce
  a win. `RECT_STRICT=1 TRAP_H=1` repeatedly converges to the lower rat at
  `(2,3)` with the player either pinned at `(3,3)` or nearby at `(4,4)`. The
  synthetic ignition table says rat `(0,0)` plus player cells around row 3 would
  win, but direct `ratat:0,0` / `ratat:1,2` branches from those bottlenecks timed
  out or returned no branch. Do not add a heuristic that merely rewards the
  corner target; it still drifts into the same pin.
- `cooperation/tug_of_war`: trigger 1 is the best first irreversible event found:
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v` leaves 4 rats, 2 reachable. A short
  continuation `^^ <^ <^ <^ v^ v^` leaves 3 rats, 1 reachable, but no ratdrop
  branch was found from there within depth 80. Trigger 2 and trigger 3 first are
  worse: they strand rats or leave 6+ rats.
- `cooperation/handoff`: pre-trigger `ratgone:8,5`, `trigger:1`, and `trigger:2`
  all re-enter the same two-rat unreachable family. The fastest reachable-rat
  kill is not progress unless it preserves access to a remote rat.
- `release`: `triganylookup` reconfirmed `v<vv^^>>v` as the best first event
  branch. Later trigger-6 variants again strand the `(18,4)` rat with the player
  sealed at bottom-right; this is the same bad basin as the earlier hand route.
- `reload_v3`: trigger-order search that greedily picks trigger 7 first reaches a
  low-rat-count state with 0 reachable rats. Immediate rat reduction is the wrong
  objective; preserve lower reload access before reducing the count.
- `chase`: trigger-order search found another 6-rat branch,
  `>>>^vv^<<<>>v`, but continuations still strand separated rats. Treat it as a
  diagnostic sibling of the older `>>>.^v<<<>>>vv<^` branch, not a solved route.

### Methods already tried (do not repeat blindly)
- Generic heuristic search (gbfs/astar, weights 1-10, `PROGRESS_H`, depth
  400-500, long budgets) solved several levels but stalled on the current 8.
- PDDL and LLM-play agents produced no verified final wins on the current hard
  set. Their useful output was mechanism hints, not move strings.
- The current productive path is mechanism-first decomposition plus short
  oracle checks.

---

## 4. Planned next steps (start here)

1. **Speed up the oracle (highest ROI, ~half-planned).** It's ~3,800 states/sec because
   every move round-trips through CSV `parse`+`serialize`. Add to `game/src/testing.rs`:
   `pub fn step_grid(grid:&Grid, actions:&[Action]) -> (Grid, PlayState)` (Grid is `Clone`),
   and a way to hash a `Grid`'s cells directly (derive/expose). Switch `solver/src/main.rs`
   `step()` and the visited-set hashing off strings. Est. **10-50×** → deep search becomes
   feasible. (I was reading `game/src/grid.rs` to do this when paused.)
2. **Continue `release` from the known prefix.** Start with
   `v<vv^^>>v` (triggers 3 then 4). Test trigger 5 via suffix `v<>>>^`, then
   branch explicitly on trigger 2 vs 6 and inspect the resulting state.
3. **Solve `tinderrectangle` as an ignition lure.** The winning geometry is a
   top rat entering `(0,0)` or `(16,0)`. Find the safe facing/timing that opens
   the web without letting the rat kill the player.
4. **Use `release` as the template for `ai_takeover`.** Once `release` is
   solved, port its trigger order to `ai_takeover` and account for trigger 7/8
   and cyborg pathing.
5. **For two-player levels, work in `wp2` waypoint pairs.** Start with
   `handoff` because it is small; then use mirrored plans for `tug_of_war`.
6. **`blocked_v2`:** rigorously test winnability before assuming solvable.

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
