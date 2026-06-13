# Infestation solving campaign — HANDOFF

Resume doc for continuing the effort on another machine. **Goal: solve the 10
remaining rat-bearing CSV levels.** 35 verified solutions are recorded in
`solver/solutions/final_solutions.json`.

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

### Solved - 29/36 non-Claude levels + 5 new (all oracle-verified `result=Won`)
Move strings: **`solver/solutions/SOLUTIONS.md`** (machine-readable: `final_solutions.json`).
Browser auto-player: `solver/solutions/autoplay.js`. New puzzles: `levels/claude/`.

`HANDOFF.md` used to say 11 remained, but `solver/solutions/SOLUTIONS.md`
now includes verified wins for `tinderbox`, `no_retreat`, `lock_in`, and
`limited2`; the latest pass also adds `chase`.

### UNSOLVED - primary hard set

| # | Level | Players | Name-hint / trick | Best lead / recommended attack |
|---|---|---|---|---|
| 1 | `tinderrectangle` | 1 | pure ignition geometry | `ignitions` says a top-pack rat at `(0,0)` or `(16,0)` can detonate the rectangle and win. Directly cutting the left web from `(1,3)`/`(2,3)` kills the player. Treat this as a lure/facing puzzle: shape a top rat into the explosive corner, then make the one safe nudge. |
| 2 | `release` | 1 | release the caged rats, then mop | Strong human prefix: `v<vv^^>>v` consumes trigger 3 then 4, drops rats from 24 to 23, explosives from 35 to 5, webs from 47 to 27, and makes 21 rats reachable. Follow-up trigger 5 is reachable with suffix `v<>>>^`; next work is choosing between trigger 2 and 6, then mop-up. |
| 3 | `reload_v3` | 1 | fire/reload cycles; triggers 1-7 | Work bottom trigger row as reload stations, not as a global search. Likely order starts around trigger 1, then 2/3/4/5/6/7 as each detonation opens the next chamber. Use `triglookup` with explicit orders and inspect each irreversible change. |
| 4 | `cyborg_rats/ai_takeover` | 1 | `release` skeleton plus cyborgs/triggers 7-8 | Solve `release` first, then transfer the trigger skeleton. Extra triggers 7/8 and cyborg Dijkstra behavior are probably the intended differences. |
| 5 | `cooperation/tug_of_war` | 2 | mirror-symmetric tug | Needs paired role choreography with `wp2`: mirrored trigger pairs 1/2/3, side rats, then central rat. Avoid generic 2p search until the waypoint pairs encode the intended symmetry. |
| 6 | `cooperation/handoff` | 2 | baton pass | Small enough to hand-reason. P1 cannot simply reach trigger 1 first. P1 can reach trigger 2 first, but then trigger 1 is no longer useful/reachable; likely P1 opens the handoff and P2 finishes on the remote side. |
| 7 | `cooperation/blocked_v2` | 2 | one player blocked | Keep the previous warning: one rat may be permanently unreachable behind effectively indestructible structure. Before spending human-solving time, prove or disprove winnability with targeted reachability/exhaustive checks. |

Additional unsolved old-level CSVs in the current inventory:
`old_levels/on_the_clock.csv`, `old_levels/order_of_operations.csv`, and
`old_levels/overstep.csv`.

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
   - `solver branchdump` / `solver lookup --goal playerat:x,y|ratat:x,y|ratgone:x,y|trigger:n`
     for tactical subgoals. Use `ratcell:ratx,raty,cellx,celly,kind` for
     compound checks such as "rat is at `(6,6)` while `(7,6)` is still empty".
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
- `cooperation/handoff`: broader BFS (canonical and raw-hash variants) found a
  better-looking one-rat basin:
  `vv >^ >^ >^ >^ >^ ^^ v^ ^^ ^^ v^ v^ vv <v ^^ <^ <^ <^ <^ ^> ^< ^< ^< ^> ^<`.
  It leaves only rat `(10,6)`, but no explosives/triggers remain and the rat is
  in a size-1 component behind web `(10,5)`. This is also dead. The obvious
  rescue hypothesis, making that rat step onto the adjacent remote trigger
  `(11,7)`, was checked with `ratat:11,7` from the initial state and found no
  branch within depth 80 / 60s; A* plateaued with the rat still at `(10,6)`.
- `release`: `triganylookup` reconfirmed `v<vv^^>>v` as the best first event
  branch. Later trigger-6 variants again strand the `(18,4)` rat with the player
  sealed at bottom-right; this is the same bad basin as the earlier hand route.
- `release`: direct `branchdump trigger:2` from the initial state returned no
  branch within depth 120 / 45s, and explicit trigger order `2,3,4,5,6` reported
  trigger 2 has no reachable branches. Ordering `3,4,2,5,6` only rediscovers a
  variant of the known trigger-3/4 opener before falling back into the late
  trigger-6 dead basin. Trigger 2 probably has to be rat-triggered or made
  reachable by a different structural event, not reached directly by the player.
- `reload_v3`: trigger-order search that greedily picks trigger 7 first reaches a
  low-rat-count state with 0 reachable rats. Immediate rat reduction is the wrong
  objective; preserve lower reload access before reducing the count.
- `reload_v3`: explicit trigger-order lookup for `1,2,3,4,5,6,7` and
  `1,2,3,4,5,7,6` produced no reachable trigger-1 branch from the initial state
  within segment depth 160 / 10s. The older "start around trigger 1" assumption
  is probably wrong or missing a setup event. `7,1,2,3,4,5,6` reaches trigger 7,
  but then again strands access before trigger 1 can be used productively.
- `chase`: trigger-order search found another 6-rat branch,
  `>>>^vv^<<<>>v`, but continuations still strand separated rats. Treat it as a
  diagnostic sibling of the older `>>>.^v<<<>>>vv<^` branch, not a solved route.
- `solver`: `lookup` / `branchdump` now support `--goal playerat:x,y`. This is a
  diagnostic target for mechanism checks such as "can a player reach this lure
  cell before the trigger/explosive resources are consumed?" It does not change
  game rules or scoring outside lookup-goal handling.
- `cooperation/handoff`: the 8-move state
  `v^ >^ >^ >^ >^ >^ ^^ v^` was probed more tightly. From that state,
  `ratat:11,7`, `ratgone:10,6`, `cell:10,5`, `playerat:10,4`, and
  `playerat:11,7` all returned no branch in the tested budgets. `playerat:12,8`
  is reachable, but every returned branch has `explosives=0`, `triggers=0`, and
  `reachable_rats=0`; it reaches the far side only after left trigger 2 turns
  `(11,7)` into a wall, leaving `(10,6)` permanently sealed. Backing up to the
  6-move state found the same post-trigger dead access and no direct movement or
  removal of `(10,6)`. Do not continue the "go far side after trigger 2" family.
- `chase`: continuing from `>>>.^v<<<>>>vv<^`, ratdrop chaining produced better
  prefixes:
  `>>>.^v<<<>>>vv<^^>^^^^^^vvv>>>vv` leaves 5 rats, 3 reachable, and
  `>>>.^v<<<>>>vv<^^>^^^^^^vvv>>>vv^^^^<^^>^^^^^^>>>^^` leaves 4 rats, 3
  reachable. A further ratdrop reaches 3 rats, but the initial `(11,17)` rat
  remains in a one-cell component behind web `(11,16)`. Source confirms zaps do
  not affect webs/planks; explosions do, and rats can only break planks by
  moving through them. `cell:11,15`, `cell:12,15`, and `cell:11,16` returned no
  branch from the 16-, 32-, or 51-move prefixes in the tested budgets. This is a
  stronger winnability warning: either an earlier route must use a helper rat to
  alter those planks/web before the known ratdrop chain, or the level may be
  structurally unsolvable as authored.
- `release`: from the strong opener `v<vv^^>>v`, direct `trigger:2`,
  `cell:18,5`, and `ratgone:18,4` all returned no branch in the tested budgets.
  A* from the same prefix again ended in the known one-rat basin: only `(18,4)`
  remains, `(18,5)` is still web, and the player is sealed on the bottom-right
  side. The physical mechanism still appears to be "detonate the `(18,6..8)`
  column to clear `(18,5)` before the player is sealed", but direct player access
  to trigger 2 after `v<vv^^>>v` is not the route.
- `tinderrectangle`: from the old partial
  `<<<^<^<>^>>>vv^^>>v>v.>>><>v`, `ratat:0,0` returned no branch. `lure`
  targeting `rat 0,0` with player safe at `(1..4,3)` timed out with the best
  state at rat `(5,3)` / player `(4,3)`, still not ignitable. `geomlure` for
  either corner `(0,0)` or `(16,0)` cleared significant web but did not place a
  rat in a corner. `RECT_STRICT=1 TRAP_H=1 tinder` again converged to the lower
  rat near `(8,6)` with the player on the lower-right safe side. Treat this
  partial as a dead basin unless the earlier route changes how the lower rat is
  held while the player crosses.

### Current run notes - 2026-06-12

No new verified wins yet. The session focused on mechanism checks and stopped
the stale broad `triganylookup` runs for `reload_v3`, `tug_of_war`, and
`ai_takeover`.

- `solver`: the fast oracle path is already present (`step_grid`,
  `state_hash`, `search_hash`, and `solver::step()` using `step_grid`). Direct
  searches still only get roughly low tens of thousands of states/sec on the
  hard levels because state expansion and heuristic evaluation dominate.
- `solver`: `branchdump` now supports `--min-rats N`. Use this when a tactical
  goal such as `ratgone`, `playerat`, or `cellnot` should not be accepted after
  spending the rat needed for the next mechanism. Wins are still reported even
  if they have fewer than `N` rats.
- `release`: `v<vv^^>>v` remains the best opener. From that state, trigger 5 is
  easy (`v<>>>^`), but waypoint probes to trigger 6 at `(17,16)` / `(19,18)` and
  trigger 2 at `(0,16)` all returned `UNREACHABLE` in 40s budgets. Broader
  `cellnot:0,16,2` and `cellnot:19,7,2` probes from both the initial state and
  the opener found no branch, so neither player-trigger nor obvious rat-trigger
  activation of trigger 2 was found. Do not assume static reachability of
  trigger 6 means it is dynamically reachable; rat pressure prevents the route.
- `release`: an explosion-focused probe from `v<vv^^>>v` found a long diagnostic
  lead
  `v<vv^^>>v^^^^^^^vvvvv^^^>><>v^<<^^<^^^^^<vvvvvvvvvv>^v>^^^^^<^^^^^<vvvvvvvvvv<<^^^^^^^^^<vvvvvv<^^^^^^<vvvv><`.
  It moves a rat to `(2,16)`, adjacent to explosive `(1,16)`, but `trigger:2`,
  `cellnot:1,16,explosive`, and follow-up explosion probes found no branch. The
  rat prefers the black hole north unless the player can stand due west on
  `(0,16)`, which is the inaccessible trigger cell. Treat this as a dead
  diagnostic lead, not progress.
- `release`: explicit trigger order `5,6,2` from opener `v<vv^^>>v` reaches
  trigger 5 with suffix `v<>>>^`, but all trigger-6 attempts fall into the
  bottom-right basin. Verifying one representative branch plus the final down
  move onto `(19,18)` shows `result=Playing`, rats=1 at `(18,4)`, triggers=2
  (both trigger-2 cells), and `reachable_rats=0`; the right explosive column
  `(18,6..8)` is still intact. From the clean post-trigger-5 prefix
  `v<vv^^>>vv<>>>^`, `ratat:19,18`, `playerat:1,16`, and `trigger:6` all
  returned no branch in tested budgets. This rules out the current
  player-trigger-6 line; a solution needs trigger 6 fired while the player is
  positioned differently, or a different opener.
- `reload_v3`: the direct trigger-7 opener `^^^^^<<<<<<^^^` consumes the only
  reachable triggers and leaves zero reachable trigger continuations. A better
  non-trigger branch,
  `^^^^^<<<<<<^^<^^^^^vvvvvvv>>>>>>`, kills the middle rat while preserving the
  trigger resources. From there, trigger-2 and trigger-1 branches can reach a
  one-rat state, but the remaining bottom-left rat `(0,21)` is still sealed
  behind web/explosive; `trigger:6` and `win` from that state returned no
  branch. Trigger 7 after the one-rat state mostly clears upper webs and still
  leaves the bottom-left rat inaccessible.
- `reload_v3`: targeted lower-left structural probes found no branch for
  `cellnot:1,21,web` or `cellnot:2,21,explosive` from either the initial state or
  the 32-move non-trigger branch. Direct `playerat:9,4` and `trigger:6` probes
  also found no branch from the initial state / 32-move branch. Explicit orders
  `7,6`, `7,5,6`, and `7,4,5,6` repeatedly produced zero continuations after
  trigger 7; trigger 7 first is still the wrong objective unless a setup event
  changes access to the top trigger cage.
- `reload_v3`: rechecking the 32-move non-trigger branch on 2026-06-12
  reproduced the same blocker. `trigger:7` branches consume the upper trigger
  pair and can wall off parts of the 7-cage, but they still leave `(0,21)`
  sealed. Follow-up `playerat:9,4`, `trigger:6`, `cellnot:1,21,web`, and an
  explicit `triglookup` order `7,6` did not produce a route to the remote
  trigger-6 chain. The bottom-left rat still appears to require a setup event
  before the middle-rat kill, not after it.
- `reload_v3`: the plausible top reload-station hypothesis also failed in
  targeted checks. Preserving all 3 rats, probes for `trigger:5`,
  `cellnot:10,5,web`, and `ratat:9,5` from the initial state returned no
  branches. `events` again only listed the 32-move middle-rat kill family. Do
  not assume bottom trigger 5 can safely open access to top trigger 6; if bottom
  trigger 5 fires, the top trigger-5 cell becomes a wall.
- `tinderrectangle`: the ignition table confirms either rat corner `(0,0)` or
  `(16,0)` plus a player on row 3 can win in one move. A new useful partial,
  `<<<<<>^>>^>^>>v>vv>>^^^>>vvvv`, puts the lower rat at `(2,3)` and the player
  safely on the right side, but the rat is then pinned because `(1,2)`, `(2,2)`,
  and adjacent exits are still webs. From that settled state, `cellnot:1,2,web`,
  `cellnot:2,2,web`, `ratat:1,3`, and `ratat:1,2` all failed in tested budgets.
  The web cut must happen before the rat settles at `(2,3)`, or the route must
  target the right-corner ignition instead.
- `tinderrectangle`: full ignition enumeration also shows lower-rat winning
  placements `(2..6,6)` with the player on the far-right safe cells `(14,6)`,
  `(14,7)`, `(15,7)`, `(13,8)`, `(14,8)`, or `(15,8)`. Preserved-rat
  `playerat:14,7` / `playerat:14,8` branches from
  `<<<^<^<>^>>>vv^^>>v>v.>>><>v` are reachable, but diagnostics put the lower
  rat at `(8,6)`, outside the winning band. Composite `geomlure` searches for
  rat `(2..6,6)` plus those safe cells failed from both the initial state and
  that prefix, with and without `--preserve-rats`. The missing mechanism is
  holding the lower rat in `(2..6,6)` while crossing right; simply reaching the
  safe side is not enough.
- `tinderrectangle`: a hybrid timing test using the old exit-opening route plus
  the newer right-side crossing,
  `<<<^<^<>^>>>>>v>vv>>^^^>>vvvv`, is valid but not winning. It puts the player
  at `(14,7)` with the lower rat already overrun to `(8,6)`. The earlier typo
  variant `<<<^<^<>^>>>>v>vv>>^^^>>vvvv` dies on turn 21 and should be ignored.
  This narrows the lower-row hypothesis: opening `(3,4)` early lets the rat
  start too soon; opening it from the safe-side settled state spends the rat.
- `tinderrectangle`: targeted readiness probes on 2026-06-12 found no branch for
  `rectready` or `rectlower` from the initial state at depth 80. Narrow `tinder`
  probes for rat `(6,6)` / player `(14,7)` and rat `(4,6)` / player `(14,7)`
  both timed out in the same near-miss basin with a lure rat at `(2,3)` and the
  player near `(4,4)`. A preserved-rat corner lure for rat `(0,0)` plus safe
  row-3 cells also timed out, best state `rat=(2,3)` / player `(5,3)`. These are
  not one-step ignition states.
- `chase`: continuing from `>>>.^v<<<>>>vv<^` with `dropchain` repeats the known
  family and never changes `(11,16)` from web. Focused probes for
  `cellnot:11,16,web` and `cellnot:11,15,plank` from the 16-move prefix found no
  branches. The helper-rat alteration, if it exists, has to happen before the
  known ratdrop chain.
- `chase`: a direct A* probe found a lower-count but dead 3-rat lead,
  `>>v>v<^^<<>>>>^^^<^^^>>>^>>v^^^^^>^^^^vv<<vvvv<vvv<vvv>vv^^<<vvvvv>>>>>>>^>>>v>>>^^v<>^^^^^^^^^^^^^^^^^^<<<`.
  It leaves rats at `(15,0)`, `(11,17)`, and `(0,19)`, with zero explosives or
  triggers; the latter two are isolated behind webs. `win`, `cellnot:11,16,web`,
  and `ratgone:11,17` continuations immediately fail. Do not continue this
  3-rat lead.
- `cooperation/handoff`: the clean first event
  `v^ >^ >^ >^ >^ >^ ^^` kills the far-right rat without consuming triggers,
  but from that 7-move state `ratgone:10,6` and `cellnot:10,5,web` found no
  branch. `dropchain` and a 120s BFS both re-enter the same one-rat sealed basin
  around `(10,6)`. The next attempt should handle `(10,6)` before the P2 sweep,
  not after it.
- `cooperation/handoff`: the initial left rat at `(0,5)` is not a permanent-wall
  proof by itself; the `(1,2)` web boundary can be cleared. However,
  `cellnot:1,2,web` branches all converge to the same dead two-rat state:
  explosives=0, triggers=0, reachable_rats=0, rats at the left pocket and
  `(10,6)`. A direct `cellnot:10,5,web` probe from the initial state found no
  branch. The useful conclusion is narrower: clearing the left web after the
  common trigger/explosion line is too late, and `(10,6)` must be handled before
  that family spends the board.
- `cooperation/handoff`: from the trigger-1 branch
  `v^ >^ >^ >^ >^ >^ ^^ v^ ^^ ^^ v^ v^ vv <v`, trigger 2 at `(2,2)` is
  reachable and `wp2` can consume it, but doing so removes the remaining
  explosives/triggers and leaves two unreachable rats. This proves the
  trigger-1-then-trigger-2 family is also a dead mechanism, not a missing mop-up.
- `cooperation/blocked_v2`: `ratgone:x,y` is misleading here because rats move
  out of their starting cells. The branch
  `v< v^ v^ v^ <^ <^ <<` does not prove the `(14,19)` rat is solved; diagnostics
  show a lower rat still alive at `(15,18)`. Direct BFS and `dropchain` from this
  lead stall with separated lower-left / mid-bottom rats. Use rat count plus
  diagnostics, not just `ratgone`, when evaluating this level.
- `cooperation/blocked_v2`: the first useful event
  `v< v^ v^ << <^ <^ ^^ ^v ^^` leaves 7 rats and preserves structure, but the
  next structural event is trigger 1 and it cuts reachable rats from 6 to 4.
  From that 7-rat prefix, `allreachable` returned no branch and `events` only
  found trigger-1 walling variants. This is not a mop-up line.
- `cooperation/tug_of_war`: the trigger-1 prefix
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v` still looks like the best first event, but
  immediate ratdrop continuations reduce to three rats with only one reachable.
  A 120s BFS from the prefix did not find a win and over-walled the center in its
  best state.
- `cooperation/tug_of_war`: the top rat is structurally suspicious. It starts in
  the tiny component `{(7,0),(8,0)}`; after simple first moves such as `^v` it
  moves from `(7,0)` to `(8,0)`, so `ratgone:7,0` is misleading. `playerat:8,0`,
  `cellnot:7,1,web`, and `wp2` to `.|8,0` all returned no branch. Since zaps do
  not clear webs, this rat needs a concrete explosion/web-clear mechanism before
  trigger choreography can solve the level.
- `cooperation/tug_of_war`: the plausible central-rat plank-break mechanism was
  probed directly. Preserving all 7 rats, `ratat:7,3`, `cellnot:7,2,plank`, and
  `cellnot:7,1,web` all returned no branch. The known trigger-1 trace spends the
  center without moving the central rat up to the top planks, so trigger
  choreography still lacks a release mechanism for the top rat.
- `cooperation/blocked_v2`: stale broad trigger-order runs were stopped, then
  the promising staged line was checked structurally. From the correct
  16-turn `5 -> 1` prefix
  `v< v^ v^ << <^ <^ ^^ ^v ^^ ^v ^> <v <> ^^ ^^ ^^`, direct probes for
  `trigger:2`, `trigger:4`, `cellnot:1,15,web`, `cellnot:2,15,explosive`, and
  `reachable:0,15` all returned no branches. Backing up to the post-trigger-5
  prefix and firing trigger 3 before trigger 1 is only reachable after collapsing
  to 3 rats; follow-up `trigger:2` / lower-web probes from that state also
  return empty. Treat both `5 -> 1 -> 3` and `5 -> 3 -> 1` as dead mechanisms
  for the lower-left rat unless an earlier event changes the trigger-2 pocket.
- `tinderrectangle`: the lower ignition route now has a sharper timing failure.
  The old lower route reaches rat `(6,6)` / player `(10,6)`, but every legal
  next move or pivot pulls the rat to `(7,6)`, outside the synthetic winning
  band. The safe-side branch from that prefix crosses through row 3 but leaves
  the rat at `(8,6)`. Exact `geomlure` checks from the earlier `(5,5)` timing
  states for rat `(6,6)` plus safe cells `(14,6)`, `(14,7)`, `(15,7)`,
  `(13,8)`, `(14,8)`, `(15,8)` returned no solution. The route must synchronize
  the rat's final step to `(6,6)` with the player already on the safe side, or
  use a different earlier setup. Relaxing rat preservation from the 16-turn
  state also timed out; its best state was only the known near-miss with rat
  `(6,6)` and player `(10,6)`, not a safe-side placement.
- `release`: the left trigger-2 mechanism was checked directly from the strong
  opener `v<vv^^>>v`. Preserving at least 20 rats, probes for `ratat:0,16`,
  `cellnot:1,16,explosive`, `cellnot:19,7,2`, and `cellnot:18,5,web` returned
  empty. A true trigger-5-first prefix
  `^^^^^^v^vvvvvvv>>>v` preserves all 24 rats but leaves zero reachable rats;
  from it, the same trigger-2 structural probes fail, and trigger 3 or 4 simply
  re-enters the familiar 23-rat / 5-explosive family. Do not repeat
  trigger-5-first as a release mechanism without a new reason.
- `cooperation/handoff`: direct trigger-2 access from the initial state only
  reproduces the known 18-turn dead basin
  `v^ >^ >^ >^ >^ >^ ^^ v^ ^^ ^^ v^ v^ vv <v ^^ ^^ ^^ ^<`, with 2 rats,
  no explosives, no triggers, and `reachable_rats=0`. Direct initial probes for
  `cellnot:10,5,web` and `ratgone:10,6` returned empty. The `(10,6)` rat still
  has to be handled before the common right-side sweep/trigger-2 family.
- `cooperation/tug_of_war`: after the first useful ratdrop prefix
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^`, the top-rat release mechanism was checked
  directly. Preserving 5 rats, `cellnot:7,2,plank`, `cellnot:8,2,plank`,
  `ratat:7,3`, and `playerat:7,0` all returned empty. The central rat is not
  breaking the top planks from this staging line, so the top rat remains the
  structural blocker.
- `reload_v3`: from the all-rats-preserved trigger-7 prefix
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<<<^^^^^^<<^^^`, direct probes for
  `cellnot:1,21,web`, `cellnot:2,21,explosive`, `trigger:6`, and even the
  statically reachable `trigger:2` all returned empty with all 3 rats preserved.
  The preserved trigger-7 route is dynamically dead unless another setup event
  changes the middle-rat pressure or opens the bottom-left reload lane first.
- `chase`: from the initial state, preserving at least 6 rats, exact structural
  probes for `cellnot:11,16,web`, `cellnot:11,15,plank`,
  `cellnot:12,15,plank`, and `reachable:11,17` returned empty. This confirms
  the `(11,17)` rat blocker is the web itself, not merely a late mop-up pathing
  issue.
- `cyborg_rats/ai_takeover`: quick diagnostics confirm it is still the
  `release` skeleton with additional trigger-7/8 and cyborg behavior. No deeper
  search was run this pass; solve or invalidate the `release` trigger-2
  mechanism first.
- Parallel mechanism pass on 2026-06-12 found **no new verified wins**. It did
  improve the durable map of dead branches:
  - `release`: the trigger-5 / trigger-6 family can be pushed to one remaining
    rat, but that rat is always `(18,4)` behind web `(18,5)`. A representative
    one-rat pre-trigger-6 branch is
    `v<vv^^>>vv<>>>^^vvv<<<<<<^^^<<<<v^^^^^^^^^v>>>^^>>>><><><><><><>>>>>>><<<<<<<<<vvvvvv<<<<<^^^^^^>>>>>>>>>>v`;
    diagnostics show `rats=1`, player `(10,3)`, rat `(18,4)`,
    `reachable_rats=0/1`. Follow-up `lookup --goal win` from that state returns
    no solution quickly. Direct post-trigger-5 probes for `ratgone:18,4`,
    `cellnot:18,5,web`, `trigger:2`, and `ratat:19,7` with high rat
    preservation also returned empty. Treat this as a dead mechanism unless a
    setup handles `(18,4)` before the top sweep.
  - `tinderrectangle`: the old lower-rat route is still an overrun; when the rat
    reaches `(6,6)`, the player is only around `(10,6)`, and safe-side branches
    then leave the rat at `(8,6)`. New better lead: from prefix `<^^^`, the
    player can reach `(14,7)` with all 16 rats preserved while the lower rat is
    still held at `(2,3)`, e.g. `ASCII <^^^>>v>vv>>^^^>>vvvv`. The next
    mechanism to test is **prepare safe side first, then release lower rat**.
  - `reload_v3`: trigger 2 can be fired while preserving all 3 rats
    (`^>>>>>>>^^^vvv<<v<`), but diagnostics show rats at `(14,5)`, `(17,13)`,
    `(0,21)` with `reachable_rats=0/3`. Follow-up `cellnot:1,21,web`,
    `trigger:1`, and `trigger:6` from that prefix returned no branches.
  - `chase`: preserving as few as 4 rats, direct probes for
    `cellnot:11,16,web`, `reachable:11,17`, and `ratat:11,16` still returned no
    branches. Event enumeration did not change `(11,16)` or make `(11,17)`
    reachable.
  - `cooperation/handoff`: from the 7-turn setup
    `v^ >^ >^ >^ >^ >^ ^^`, `wp2` waypoints `.|10,6;.|11,7` are unreachable,
    and `ratat:11,7`, `cellnot:10,5,web`, and `reachable:10,6` return no
    branches. All-5-rat-preserved initial checks for `ratat:11,7` and
    `trigger:2` also returned empty.
  - `cooperation/tug_of_war`: the 17-turn partial
    `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v ^^ <^ <^ <^ v^ v^` verifies only as
    `Playing`. From there, `cellnot:7,1,web`, `cellnot:7,2,plank`, and
    `ratsle:2` returned empty. All-rats-preserved top enclosure checks still
    fail.
  - `cooperation/blocked_v2`: preserve-all-rats checks for `reachable:0,15`
    and `cellnot:1,15,web` returned empty. Relaxed `wp2` to `.|0,15` and
    `.|3,14;.|0,15` both report waypoint 0 unreachable.
- ASP/clingo idea: a quick prototype for one-player, one-rat suffixes exposed
  that a naive full-cell-state encoding grounds poorly even for a horizon-1
  `release` suffix. Do not repeat a direct cell-by-cell ASP dump. If revisiting
  ASP, encode only dynamic facts around reachable components/events, use fixed
  horizons, and verify every candidate with `target/release/solver verify`.
- Long parallel wave on 2026-06-12: 17 independent 900s structural probes were
  run concurrently (17-22 solver processes; memory, not CPU, was the limiting
  resource). No verified wins and no branch outputs. Empty branchdump results:
  `tinderrectangle` safe-side `rectlower` / `rectready`; `release` from
  `v<vv^^>>v` for `cellnot:18,5,web`, `cellnot:18,6,explosive`,
  `ratat:19,7`, and `trigger:2` with `--min-rats 20`; `reload_v3` from the
  45-move trigger-1 branch for `trigger:6`, `cellnot:1,21,web`, and
  `winready`; `chase` for preserved `cellnot:11,16,web` and
  `reachable:11,17`; `handoff` for preserved `cellnot:10,5,web` and
  `ratat:11,7`; `tug_of_war` for preserved `cellnot:7,1,web`; and
  `blocked_v2` for `reachable:0,15`. The non-branch `tinder` / `geomlure`
  processes ended without useful output under memory pressure, so treat those
  as inconclusive rather than proof.
- `tinderrectangle`: a better prepared-safe branch is
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv`.
  It verifies as `Playing` for 71 turns, with player `(14,7)`, all 16 rats
  alive, lower rat still `(2,3)`, and webs reduced from 43 to 30. This proves
  "safe side first, pre-open lower lane, return safe" is feasible. However,
  follow-up checks from this branch for `cellnot:3,4,web` with all 16 rats and
  for `rectlower` returned no branches in the tested budgets. The missing step
  is still releasing the pinned lower rat into `(2..6,6)` without overrun.
- `release`: a sharper staging branch is `v<vv^^>>v^vv><<`. It verifies as
  `Playing` for 15 turns with 23 rats, 5 explosives, 25 webs, player `(8,13)`,
  and rats at `(16,6)`, `(16,7)`, `(16,8)`, plus the sealed `(18,4)` rat.
  Trigger 2 remains unreachable; trigger 6 is player-reachable only at distance
  48/46. Follow-up `trigger:6` and `5,6,2` trigger-order checks did not open
  the right column. This narrows the blocker: moving rats near x=16 is possible,
  but the required event is opening/routing through the x=17 barrier before the
  pack collapses.
- `reload_v3`: the fresh 45-move trigger-1 branch
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<^<<<<<<<<v<v^v>>` verifies as `Playing`, but
  diagnostics show only two rats left, both unreachable: `(14,5)` and `(0,21)`;
  trigger 6 at `(9,4)` / `(2,22)` is also unreachable. The 900s follow-up
  probes for `trigger:6`, `cellnot:1,21,web`, and `winready` all returned empty.
  Treat this branch as another dead low-rat basin unless a prior setup changes
  bottom-left access.
- Two-player blockers tightened further. `handoff` 7-turn setup
  `v^ >^ >^ >^ >^ >^ ^^` leaves `(10,6)` sealed; no branches for clearing or
  reaching `(11,8)`, `(12,7)`, `(12,8)` while preserving all 5 rats, and reaching
  `(11,8)` after trigger 2 is too late (2 rats, no explosives/triggers,
  `reachable_rats=0`). `tug_of_war` 17-turn prefix
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v ^^ <^ <^ <^ v^ v^` leaves rats `(7,0)`,
  `(4,8)`, `(3,12)` with only 1/3 reachable; direct plank-break checks for
  `(7,2)` / `(8,2)` and staging waypoints failed. `blocked_v2` prefix
  `v< v^ v^ << <^ <^ ^^ ^v ^^` leaves the `(0,15)` rat unreachable; no branch
  for trigger 2, detonating `(2,15)`, row-17 black-hole lure staging, or
  `ratgone:0,15` under the tested constraints.
- Follow-up mechanism pass on 2026-06-12 found no verified wins, but sharpened
  three single-player blockers. `novelty` search with `--k 2` exhausted quickly
  on `tinderrectangle`, `release`, `release` staged at `v<vv^^>>v^vv><<`,
  `reload_v3`, the 45-move `reload_v3` branch, `chase`, and
  `cyborg_rats/ai_takeover`; it is not a useful frontier by itself.
- `tinderrectangle`: from the 71-turn prepared-safe branch, exact branchdump
  checks can move the lower rat down through `(2,4)`, `(2,5)`, and into the
  winning row `(2..6,6)` while preserving all 16 rats. Representative
  `(6,6)` branch:
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^>^^<<<vvv<<^^^<<<<<vvv<<<<>>>>>>`.
  It verifies as `Playing` for 105 turns, with player `(7,6)` and lower rat
  `(6,6)`. However, internal BFS from that state found no win within depth 40,
  and a solo `geomlure` from the prepared branch for lower row plus safe cells
  returned `NO_SOLUTION` at depth 180 / 161s. The problem is synchronization:
  the player is in contact with the lower rat when the rat reaches the winning
  band; moving down detonates the rectangle but kills the player.
- `reload_v3`: there is a preserved-rat branch that clears the right-side web
  `(16,7)`:
  `^^^^^<<<<<<^^<^^^^^^^>>>>>>>>>>vvvvv`. A six-move continuation
  `<<<<<` also clears the top-cage web band through `(11,8)`:
  `^^^^^<<<<<<^^<^^^^^^^>>>>>>>>>>vvvvvv<<<<<`. Both verify as `Playing` with
  all 3 rats. Diagnostics still show top trigger 6 `(9,4)` and trigger 5
  `(9,5)` unreachable. Follow-up branchdump checks for `reachable:9,4`,
  `reachable:9,5`, `reachable:10,5`, `trigger:6`, `cellnot:10,5,web`,
  `cellnot:9,6,explosive`, and `playerat:9,4` all returned empty. Clearing the
  outer right/top webs is not enough to enter the trigger-6 cage.
- `release`: from staged branch `v<vv^^>>v^vv><<`, direct structural checks for
  changing the x=17 wall cells `(17,5..8)`, opening `(18,5)`, detonating
  `(18,6)`, moving rats to `(17,6..8)` / `(18,6)` / `(19,7)`, and firing
  trigger 2 or 6 all returned empty with `--min-rats 20`. This rules out the
  obvious "rats lined up at x=16 eventually push through" hypothesis in the
  tested depth/budget; a solution needs a different earlier event.
- Follow-up human-mechanism pass on 2026-06-12 found no verified wins but ruled
  out several attractive next moves. `reload_v3` all-rat staged prefix
  `^^^^^<<<<<<^^<^^^^^^^>>>>>>>>>>vvvvvv<<<<<>>>>>^^^^^^<<<<<<<<<vvv`
  has player `(7,5)`, rats `(19,2)`, `(11,5)`, `(0,21)`, and only trigger 7
  reachable. Preserved-rat branchdumps from that state for `trigger:6`,
  `trigger:5`, and `cellnot:10,5,web` all returned empty. Consuming trigger 7
  with suffix `vvv` leaves zero reachable triggers; from that post-trigger-7
  state, six 240s branchdumps for `trigger:5`, `trigger:6`,
  `cellnot:10,5,web`, `ratat:10,5`, `cellnot:2,21,explosive`, and
  `cellnot:1,21,web` also returned empty. Treat this top-rat/trigger-7 staging
  as a dead mechanism unless a prior event changes bottom/top access.
- `tinderrectangle`: the prepared-safe 71-turn branch can cut a door near the
  lower rat and start a row-3 chase, but that route is a one-cell-deep corridor
  trap. Example continuation to rat `(6,3)` / player `(7,3)`:
  `^^^^<<vvv<<^^^<<<<<<<>>>>` after the 71-turn prefix. From there, every
  preserved-rat continuation is forced east until the wall at `(11,3)`, where
  the only non-losing move kills the lower rat. Stepping north into the web row
  at the earlier junction also loses because a top rat joins the chase. This
  narrows the lower-row solution further: the release must not create a
  one-cell tail chase on row 3.
- `tinderrectangle`: the alternative vertical-release hypothesis was also
  checked. From the 71-turn prepared-safe branch, preserved-rat branchdumps can
  clear `(2,4)`, `(2,5)`, and `(2,6)` and can place the lower rat at `(2,6)`;
  representative suffix:
  `>^^^^<<<vvv<<^^^<<<<<vvv<<<<>>`, total 101. That verifies as `Playing` with
  all 16 rats, lower rat `(2,6)`, player `(3,6)`. However, moving down ignites
  and kills the player, moving east only shifts the one-cell chase to
  `(3,6)`/`(4,6)`, and A*/branchdump continuations for `rectlower`,
  `playerat:14,7`, and `playerat:15,7` returned empty. So merely opening the
  x=2 vertical shaft is not enough; the release must give the player separation
  before the rat reaches the lower ignition row.
- `release`: the left trigger-2 route was sharpened. From `v<vv^^>>v`, `wp` to
  `(2,17)` is unreachable, and preserved-rat branchdumps for `ratat:2,17`,
  `cellnot:2,17,web`, `ratat:0,16`, early `trigger:6`,
  `cellnot:1,16,explosive`, and `cellnot:0,17,explosive` returned empty. The
  first-explosion probe again reached the known near-miss
  `v<vv^^>>v^vvv<<<<^<^^<<><`, with a rat at `(2,16)`, but continuing from
  there to clear `(1,16)` also returned empty. The blocker is concrete:
  `(1,16)` is an explosive between the rat and trigger 2, so the rat cannot
  activate left trigger 2 unless that explosive is cleared before the rat
  arrives.
- `chase`: static source inspection confirms explosions are only 3x3 and zaps
  do not affect webs/planks unless they detonate adjacent explosives. A
  90-second branchdump for `cellnot:11,16,web` from the initial state with
  `--min-rats 5` returned empty. Since no initial explosive is adjacent to
  `(11,16)`, the `(11,17)` rat remains a hard structural blocker unless a route
  first opens player access to the web or creates an adjacent explosion.
- `cyborg_rats/ai_takeover`: from opener `v<vv^^>>v`, relaxed branchdumps
  without `--min-rats` for `trigger:7`, `trigger:8`, and `ratgone:18,4`
  returned empty, and direct A* `lookup --goal win` returned `NO_SOLUTION`
  quickly. The extra triggers/cyborgs do not bypass the `release` skeleton from
  the standard opener in the tested budget.
- Mechanism-first parallel pass on 2026-06-12 found **no new verified wins**,
  but it tightened the current human map of the 8 hard levels. This pass
  deliberately tested blocker-changing mechanisms rather than broad direct
  `solve` runs.
- `tinderrectangle`: the 71-turn prepared-safe branch
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv`
  remains the best constructive lead. From it, branchdump can place the lower
  rat at `(2,6)` and `(6,6)` with all 16 rats alive; for example the
  `(6,6)` branch
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^>^^^<<<vvv<<^^^<<<<<vvv<<<<>>>>>>`
  verifies as `Playing`, with player `(7,6)` and lower rat `(6,6)`. However,
  every immediate ignition move from that contact state either kills the player
  or just continues the one-cell chase. BFS/A* continuations from the contact
  state found no win.
- `tinderrectangle`: a stronger safe-side state can put the player at `(15,7)`
  with all 16 rats alive:
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^`.
  From there, branchdump can still move the lower rat to `(2,6)` and `(6,6)`,
  but the returned branches again end with the player directly east of the rat.
  A combined `geomlure` for rat `(2..6,6)` plus safe cells
  `(14,6),(14,7),(15,7),(13,8),(14,8),(15,8)` returned `NO_SOLUTION` quickly
  from that state. Treat this as evidence that the lower-rat route needs a
  genuine separation loop, not just more safe-side preparation.
- `tinderrectangle`: full ignition enumeration shows an alternate winning
  geometry: rat `(13,8)` with the player on row 3 or row 6 can detonate the
  bottom explosive row. This suggests a possible top/right-pack drop instead
  of the lower-rat row-6 route. Manual checks show the immediate obstacle:
  clearing the rightmost row-2 web by stepping onto `(15,2)` lets the top rat
  enter the player's cell and causes `GameOver`. Branchdumps for `ratat:13,8`
  from both the initial state and the prepared state returned no branches in
  this pass, and a combined `geomlure` for `(13,8)` plus safe row-6 cells timed
  out with the known lower-rat near-miss, not a right-pack drop.
- `release`: parallel audit reconfirmed the concrete blocker. After
  `v<vv^^>>v`, `(18,4)` is still isolated behind web `(18,5)` and the right
  explosive column `(18,6..8)` is intact. Branchdumps from the opener and from
  post-trigger-5 for `cellnot:18,5,web` and `trigger:2` returned no branches;
  waypoint probes to `(19,7)`, `(0,16)`, `(18,5)`, and `(17,5)` are unreachable.
  The known dead sweep still verifies only as `Playing`, leaving exactly the
  `(18,4)` rat.
- `cyborg_rats/ai_takeover`: unlike `release`, `(18,5)` starts open, so the
  `(18,4)` rat is statically in the main component. That does not make the
  standard opener solve it: targeted branchdumps for `trigger:2`, `trigger:7`,
  `trigger:8`, and `ratgone:18,4` from the initial/opener states returned no
  candidates, and trigger-7 waypoints `(16,9)`, `(15,9)`, `(16,11)` are
  unreachable after the opener. Do not assume `release`'s skeleton transfers
  mechanically here.
- `reload_v3`: the bottom-left blocker is still `(0,21)` behind web `(1,21)`
  and explosive `(2,21)`. Initial and staged probes found no route to
  `cellnot:1,21,web`, `cellnot:2,21,explosive`, or `trigger:6`. The preserved
  trigger-2 branch `^>>>>>>>^^^vvv<<v<` and the top/right web-clearing branch
  `^^^^^<<<<<<^^<^^^^^^^>>>>>>>>>>vvvvvv<<<<<` both preserve all 3 rats but
  leave trigger 6 unreachable. The only clean mechanism still appears to be
  firing trigger 6 before access collapses; no route to do so is known.
- `chase`: the `(11,17)` rat remains isolated behind web `(11,16)`. Source
  inspection plus targeted branchdumps confirm zaps do not clear that web,
  no initial explosive is adjacent to it, and no helper-rat route to
  `(11,16)` / plank alteration was found before the known ratdrop chains.
  Waypoints to `(11,17)` remain unreachable from both the initial state and the
  16-move known prefix.
- `cooperation/tug_of_war`: top rat movement from `(7,0)` to `(8,0)` is only
  pocket motion, not progress. The top pocket's webs `(7,1)/(8,1)` and planks
  `(7,2)/(8,2)` did not change in targeted branchdumps, and the known
  trigger-1 prefix still leaves that rat unreachable.
- `cooperation/handoff`: the sealed `(10,6)` rat remains the blocker.
  Targeted checks did not move/remove it, place it on remote trigger `(11,7)`,
  clear web `(10,5)`, or reach/change `(11,7)` before the common trigger line.
  The traced trigger-2 family ends with `rats=2`, `explosives=0`,
  `triggers=0`, and `reachable_rats=0`.
- `cooperation/blocked_v2`: lower-left rat `(0,15)` still did not move or
  disappear, web `(1,15)` and explosive `(2,15)` did not change, and trigger 2
  was not consumed. Both trigger-2 cells `(3,14)` and `(5,11)` were waypoint
  unreachable for both players in targeted `wp2` probes.
- Parallel human-mechanism pass later on 2026-06-12 found **no new verified
  wins**, but produced better frontiers and stronger blocker evidence:
  - `solver`: `lookup` / `branchdump` now support
    `--goal ratcell:ratx,raty,cellx,celly,kind`. This is diagnostic-only and
    was useful for testing whether a rat can reach a target while an adjacent
    escape/stop cell remains unchanged.
  - `tinderrectangle`: a new all-rats-preserved left-wall pump is
    `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv>^^^^<<<vvv<<^^^<<<<<vvv<<<<`.
    It verifies as `Playing` for 99 turns, with player `(1,6)`, all 16 rats
    alive, and the lower rat at `(1,4)`. This is real separation, but immediate
    left-edge ignition kills the player; `>>` falls back into the known
    `(2,6)` rat / `(3,6)` player contact line. Suffix `^^^` kills the lower
    rat and puts the player at `(1,3)` with 15 top rats left, but the top row is
    still sealed by row-2 webs. Entering row 2 under the pack is unsafe because
    diagonal rats can enter the player cell despite the sword.
  - `tinderrectangle`: from the 99-turn pump and from the row-3 suffix, direct
    `ratat:0,0` / `win` continuations returned no branch in the tested budgets.
    The remaining viable idea is still a rat-triggered border explosive, but
    it likely requires opening a row-2 web without standing under a diagonal
    top rat.
  - `cyborg_rats/ai_takeover`: constrained trigger-order probes
    `4,5,7,...`, `3,5,7,...`, and related beams all failed when trying to
    reach trigger 7 after the first two central triggers. Direct lure probes
    for the `(18,4)` cyborg to `(18,5)`, `(18,6)`, `(19,7)`, or detonating
    `(18,6)` also returned no branches. Do not keep spending on the standard
    3/4/5 opener unless a new structural event appears before trigger 7.
  - `reload_v3`: new preserved-rat prefix `^^^^^<<<<<<^^<^^^^^vv` verifies as
    `Playing` for 21 turns with player `(6,6)` and rats `(19,2)`, `(11,5)`,
    `(0,21)`. It proves the top-cage rat can be moved to `(11,5)` while
    preserving all 3 rats, but `(10,5)` remains web, `(9,6)` remains explosive,
    and trigger 6 is still unreachable. Targeted `trigger:5`, `trigger:6`, and
    `cellnot:10,5,web` checks from this state returned empty.
  - `chase`: the better structural intermediate is the plank line, not the web:
    if `(11,15)` or `(12,15)` could be broken by a helper rat, the player might
    then clear `(11,16)` and reach the sealed `(11,17)` rat. Initial preserved
    checks for `cellnot:11,15,plank`, `cellnot:12,15,plank`, `ratat:11,15`,
    and `reachable:11,16` returned no branches in the tested budget.
  - `cooperation/tug_of_war`: the top pocket now looks structurally impossible
    as authored. The blocker cells `(7,1)`, `(8,1)`, `(7,2)`, and `(8,2)` have
    no adjacent explosive; zaps do not clear webs/planks unless they detonate
    an adjacent explosive, and rats cannot enter the web cells. Treat this as a
    likely authoring blocker unless source mechanics are changed.
  - `cooperation/handoff`: preserved checks still did not place the sealed
    `(10,6)` rat onto remote trigger `(11,7)`, clear `(10,5)`, or make
    `(10,6)` reachable before the common dead trigger line.
  - `cooperation/blocked_v2`: preserved-rat checks still did not remove
    `(0,15)` or consume trigger 2; paired waypoint access to trigger 2 failed.

- Mechanism pass on 2026-06-12 found **no new verified wins**, but tightened two
  active frontiers:
  - `tinderrectangle`: the lower-row ignition mechanism is now proven on
    virtual boards: any lower rat on `(2..6,6)` with the player safely at
    `(14,6)`, `(14,7)`, or `(14,8)` can win by stalling, because the rat steps
    onto the row-7 explosive chain while the player survives. Cells `(15,7)` and
    `(15,8)` are not safe for this purpose. The hard part is not ignition; it is
    separation.
  - `tinderrectangle`: the stronger safe-side staging branch
    `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^`
    verifies as `Playing` for 84 turns, with player `(15,7)`, all 16 rats alive,
    the lower rat still `(2,3)`, and 23 webs. From that state, branchdump can
    move the lower rat to `(6,6)`, but every returned branch again collapses to
    the one-cell contact line: player `(7,6)`, lower rat `(6,6)`. Example full
    branch:
    `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<<<<vv>v<<<<>>>>>>`.
    Immediate `^`, `v`, or `.` loses; moving east overruns the rat to `(8,6)`.
  - `tinderrectangle`: the best cut point in that branch is turn 113:
    `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<<<<vv>v<<<<`.
    It reproduces the left-wall pump shape with extra right-side cleanup:
    player `(1,6)`, lower rat `(1,4)`, all 16 rats alive, and only 19 webs.
    Direct suffixes still fail in the same way (`>>>>>>` gives player `(7,6)` /
    rat `(6,6)`, then all non-kill continuations fail), and branchdump for
    `winready` / `playerat:14,7` from this cut point returned no branch in the
    tested budgets. This suggests the missing trick is not more right-side
    preparation; it is a different release geometry that gives the player
    separation before the rat reaches row 6.
  - `cooperation/blocked_v2`: the best current frontier is
    `^< ^^ <^ <^ <v <^ v> .> <> vv ^v .v <v ^v v^ vv >v ^> ^> v< v> ^<`.
    It verifies as `Playing` for 22 turns with 3 rats at `(9,13)`, `(9,14)`,
    and `(0,15)`, zero planks, and trigger 2 still present. Trigger 2 is the
    plausible way to open the left rat, because it can zap/detonate the
    `(2,15)/(3,15)` explosives and clear web `(1,15)`. However, from both the
    15-turn pre-trigger-4 frontier and the 22-turn 3-rat frontier, targeted
    checks for `trigger:2`, `reachable:5,11`, `reachable:3,14`,
    `ratat:5,11`, `ratat:3,14`, `cellnot:1,15,web`,
    `cellnot:2,15,explosive`, `cellnot:6,11,explosive`,
    `reachable:6,11`, and `playerat:6,12` returned no branches in the tested
    budgets. Treat trigger-2 access as the current blocker, not the later mop-up.

- Follow-up targeted pass on 2026-06-12 found **no new verified wins**, but
  tightened the current search map:
  - `solver`: `branchdump` / `lookup` now support `--goal rectsep`
    (`rectangle-lower-separated`). This is diagnostic-only. It accepts only
    states where a lower rat is in the proven row-6 winning band `(2..6,6)` and
    the player is on a right-side one-move-winning staging cell. It rejects the
    old contact line where the rat reaches `(6,6)` with the player at `(7,6)`.
    The rectangle lower target set represents one-move-winning staging
    cells, not only cells safe for stalling: virtual ignition checks include
    `(13,8)`, `(15,7)`, and `(15,8)` because the player can move into column 14
    as the ignition happens. Treat `(14,6)`, `(14,7)`, and `(14,8)` as the true
    stall-safe cells.
  - `tinderrectangle`: `rectsep` returned no branches from the initial board
    at depth 150 / 120s, from the 84-turn safe-side staging branch at depth 95,
    or from the 113-turn left-wall pump at depth 80. That is stronger evidence
    than the older `rectlower` checks: the known lower-row route can place the
    rat in the winning band, but not with enough timing separation. Opening
    `(2,5)` / `(2,6)` while the rat remains parked at `(2,3)` is possible, but
    opening `(2,4)` starts the rat immediately and every direct escape tested
    is a diagonal-kill/contact trap. The top/corner alternative was also checked
    again: after killing the lower rat, row-2 entry still dies immediately and
    branchdumps did not open `(1,2)` / `(2,2)`, place a rat at `(0,0)` /
    `(16,0)`, or drop a right-pack rat to `(13,8)` in the tested budgets.
  - `reload_v3`: the lower-left staging prefix `vvv<<<<<<vv<<<<` is verified
    and clears `(3,21)`, putting the player at `(3,21)` with all 3 rats alive.
    It still leaves `(1,21)=web`, `(2,21)=explosive`, and the `(0,21)` rat in a
    size-1 component. Waypoints to `(2,22)` and trigger 6 `(9,4)` were
    unreachable; bounded branchdumps could not change `(1,21)`, detonate/remove
    `(2,21)`, reach trigger 6, or remove `(0,21)`, even when rat preservation
    was relaxed.
  - `cyborg_rats/ai_takeover`: after the standard opener `v<vv^^>>v`, static
    diagnostics report trigger 7 cells as distance-reachable, but this is
    dynamically misleading. `wp` to `(16,9)` from the opener returned
    `UNREACHABLE`, and a preserved-rat `trigger:7` branchdump returned no
    branch. Treat trigger 7 as a timing/pressure blocker, not a simple waypoint.
    A trigger-order beam found a slightly different 14-turn first branch,
    `v<vv^^<vv>>><^`, with 23 rats, 9 explosives, 32 webs, 12 triggers, and
    22/23 reachable rats. It is not a solution lead by itself: preserved
    branchdumps from that state for `trigger:7` and `ratgone:18,4` returned no
    branches, so it appears to be another version of the same dynamic-pressure
    blocker rather than a bypass.
  - `cooperation/blocked_v2`: from both strong 3-/4-rat frontiers, bounded
    checks still could not consume trigger 2, mutate either trigger-2 cell
    `(3,14)` / `(5,11)`, open `(1,15)`, or detonate/remove `(2,15)`. A directed
    `5 -> 2` trigger lookup from the 19-turn B frontier can consume trigger 5
    with suffix `^< ^< ^^ ^^ ^^ ^^ ^<`, leaving 3 rats, but both trigger-2 cells
    remain unreachable and `(0,15)` is still sealed. Do not continue the
    trigger-5-then-trigger-2 family unless an earlier prefix changes the
    lower-left pocket first.
  - `solver`: `lookup` / `branchdump` now support
    `--goal ratplayer:ratx,raty,playerx,playery`. This is a diagnostic-only
    compound target for timing puzzles where a rat location is useful only with
    the player in the matching lure cell.
  - `release`: trigger side matters. Consuming trigger 2 on the right side is a
    false success because the consumed trigger cell is overwritten before zap
    resolution, so only the left trigger-2 cell is zapped. To detonate the
    right explosive column and clear `(18,5)`, trigger 2 likely has to be
    consumed from the left cell `(0,16)` or the right column must be detonated
    directly. Targeted probes from the standard opener `v<vv^^>>v` found no
    route to `ratat:19,7`, `cellnot:18,6,explosive`, `cellnot:18,5,web`, or
    player staging cells `(19,6)`, `(19,8)`, `(17,7)` in tested budgets.
  - `chase`: the player can walk webs, so `(11,17)` is not web-blocked; the real
    blocker is plank `(11,15)`/`(12,15)`. The no-trigger pocket prefix
    `>>vv>>>>>>>>>^^^` reaches player `(13,16)` with helper rat `(13,17)`, but
    the rat is one turn too close. Targeted `ratplayer` timing checks from
    `>>vv>>>>>>>>>` for useful two-cell/offset stages and direct
    `cellnot:11,15,plank` / `cellnot:12,15,plank` returned empty in the tested
    budgets. Do not continue the adjacent-helper route unless a new entry keeps
    the helper separated before the climb.
  - `cooperation/blocked_v2`: a better-looking lower-trigger-3 prefix
    `^< ^^ <^ <^ <v <^ v> .> <> vv ^v .v <v ^< vv` leaves 8 rats, 6 reachable,
    and zero planks, but it still does not touch the lower-left mechanism. From
    the 13-turn pre-trigger prefix `^< ^^ <^ <^ <v <^ v> .> <> vv ^v .v <v`,
    `playerat:10,13`, `cellnot:10,13,spiderweb`, and
    `cellnot:12,11,explosive` returned empty in the tested budgets; lower
    trigger 3 only opens the top pack and leaves the x=10 seam unchanged.

- Parallel mechanism pass later on 2026-06-12 found **no new verified wins**,
  but added one useful diagnostic and ruled out another attractive human line:
  - `solver`: `lookup` / `branchdump` now support
    `--goal playerfacing:x,y,dir` and
    `--goal ratplayerfacing:ratx,raty,playerx,playery,dir`. Accepted
    directions are `north/south/east/west`, `n/s/e/w`, arrow glyphs, or
    `up/down/left/right`. These are diagnostic-only and exist for timing
    puzzles where facing changes the sword-block outcome.
  - `chase`: the helper-plank route has a sharper necessary condition. If the
    helper rat can be held at `(13,16)` while the player is at `(13,15)` facing
    south, the rat's direct north attack should be sword-blocked and it may
    enter a plank instead. However, exact `ratplayerfacing` probes from both the
    pre-pocket prefix `>>vv>>>>>>>>>` and the cleaner trigger frontier
    `>>>.^vvv<^` found no branch to `(rat=13,16, player=13,15, south)` in the
    tested horizons. Offset variants `(rat=13,16, player=14,15, south)`,
    `(rat=14,16, player=13,15, south)`, and the one-row-up timing
    `(rat=13,15, player=13,14, south)` also returned empty. The older
    `>>vv>>>>>>>>>^^^^` state is therefore confirmed as the wrong timing: it
    reaches player `(13,15)` facing north, which cannot force the plank break.
  - `tinderrectangle`: a stronger delayed side-loop frontier exists:
    `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<vv<<v<<<^>>^^>>>>>v>vv>>^^^>>vvvv`.
    It verifies as `Playing` for 135 turns with player `(14,7)`, lower rat
    `(2,3)`, all 16 rats alive, `(2,5)`/`(2,6)` open, and `(2,4)`/`(3,4)` still
    web. This is better delayed-release staging than the old 84/113-turn
    frontiers, but `rectsep` from it returned no branch; direct release still
    collapses into diagonal contact.
  - `release`: explicit `3,2` and `4,2` trigger-order probes still cannot make
    trigger 2 reachable. From the standard opener `v<vv^^>>v`, checks for
    `(18,5)` opening with the `(18,4)` rat preserved and for changing left
    trigger 2 `(0,16)` returned no branches. The right trigger-2 activation
    remains a false lead because it zaps the wrong side.
  - `reload_v3`: the known all-rat staging prefix
    `^^^^^<<<<<<^^<^^^^^vv` remains only a blocker witness: the top rat can be
    moved to `(11,5)` while `(10,5)` remains a web, but preserved checks still
    could not put a rat on `(10,5)`, clear `(10,5)`, clear `(9,6)`, open
    `(1,21)`/`(2,21)`, or reach trigger 6.
  - `cyborg_rats/ai_takeover`: cyborg movement did not create a bypass in the
    tested budgets. Relaxed probes still found no route to trigger 7/8 or
    right-side `(18,5)` staging from the standard opener.
  - `cyborg_rats/ai_takeover`: added strict trigger diagnostics:
    `branchdump --goal triggeronly:N` and `triglookup --strict`, where a trigger
    goal only succeeds if trigger `N` changes and every other trigger number is
    unchanged. This exposed a false assumption in earlier `triglookup` output:
    the loose trigger goal can accept branches that spent another trigger first.
    Strict checks show the clean top-release families still cannot reach trigger
    7, while the known right-side family
    `^^^^^^v^vvvvvvv>>>vv<<<<v<<^^^^^^^^<^<<<>>>>vvvvvvvvv>>>>>>>>^^^>>>vvv>>>vvvvv<<<<<<<<<<<<<<<<^^^<<<`
    opens `(18,4)` only after spending trigger 4 and leaving the top band sealed.
    Inserting trigger 3 at the turn-60 or turn-67 checkpoints consumes trigger 3
    but does not release the top row; it leaves 21 rats, 38-39 explosives, and
    zero reachable rats. Do not repeat simple trigger-order permutations here;
    the remaining blocker is physical herd separation/access to trigger 7 after
    top release.
  - `cooperation/handoff`: the reachable trigger-2 family is confirmed dead:
    after consuming it, the board can reach `rats=2`, `explosives=0`,
    `triggers=0`, `reachable_rats=0`, with rats at `(10,6)` and `(1,7)`. Checks
    for `cellnot:10,5,web`, `ratat:11,7`, and
    `ratcell:11,7,10,5,web` still found no route.
  - `cooperation/tug_of_war`: the top pocket remains a likely authoring blocker.
    Waypointing to `(8,0)` failed, and checks for changing `(7,1)` web or
    `(7,2)` plank returned empty.
  - `cooperation/blocked_v2`: trigger 2 remains the blocker. Split `wp2`
    checks for both players to `(5,11)` and `(3,14)` failed from the best
    early frontier; lookup also failed to consume trigger 2, open `(10,13)`,
    open `(1,15)`, or remove `(2,15)` in the tested budgets.
  - Environment note: neither `clingo` nor the Python `clingo` module is
    installed in this workspace, and `z3`/`ortools`/`pysat` are also absent.
    An ASP/SAT route would require first adding solver tooling rather than just
    writing an encoding against an available backend.

- Follow-up parallel/human pass later on 2026-06-12 found **no new verified
  wins**. All solver processes were stopped before handoff.
  - `release`: a sidecar audit and local checks both reconfirmed that the
    apparent trigger-6-to-left-trigger-2 route is blocked from the standard
    opener. Triggering right `2` zaps the wrong side, while the useful left
    trigger-2 cell `(0,16)` remains inaccessible. Preserved checks for
    clearing `(1,16)`, placing a rat on `(0,16)`, opening `(18,5)`, or using
    `ratcell:2,16,1,16,empty` returned no branch. Do not continue the
    trigger-5/6/right-column family without a different opener.
  - `reload_v3`: the trigger-2-first prefix `^>>>>>>>^^^vvv<<v<` should be
    deprioritized. A stronger trigger-1-first branch exists:
    `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<^<<<<<<<<v<v>`. It verifies as `Playing`
    with all 3 rats alive, player `(2,18)`, roaming rat `(4,18)`, lower
    trigger `1` consumed, and the bottom corridor partly opened. From there,
    trigger `2` is reachable only via
    `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<^<<<<<<<<v<v>^>^^^>>>>>>>>>>>>>>^`, which
    verifies as `Playing` but leaves only two rats, both unreachable. Follow-up
    `trigger:3`, `trigger:4`, `trigger:5`, and `cellnot:1,21,web` checks from
    that two-rat state returned empty; stricter pre-trigger-2 checks for
    `trigger:2`, `trigger:3`, and `trigger:5` with `--min-rats 3` also returned
    empty. The new lead is useful as a mechanism clue, but not a continuation
    yet: firing trigger 2 while preserving the roaming rat remains the blocker.
  - `tinderrectangle`: the 135-turn delayed staging branch
    `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<vv<<v<<<^>>^^>>>>>v>vv>>^^^>>vvvv`
    has player `(14,7)`, lower rat `(2,3)`, all 16 rats alive, and only 18
    webs. It can clear `(2,4)` while preserving rats, but that puts the player
    directly under the rat and re-enters the contact trap. It found no branch
    to clear `(3,4)` from that safe-side staging. Opening `(3,4)` early is easy
    (`<^^<v<^<<^`), but `geomlure` from that prefix immediately reports
    `NO_SOLUTION`, and direct safe-side/`rectsep` probes did not produce a
    separated lower-row ignition state. The missing trick is still a release
    geometry that opens `(3,4)` without starting an unescapable tail chase.
  - `chase`: the bottom-helper route was rechecked. The helper in
    `>>vv>>>>>>>>>^^^` is the `(0,19)` rat released through the bottom webs,
    not the right-side rat. It reaches `(13,17)` with the player at `(13,16)`
    facing north, but targeted checks still did not break `(12,15)` or
    `(11,15)` or clear `(11,16)`. Early trigger-4, trigger-5, and trigger-2
    variants did not affect the blocker cells. Treat the adjacent helper route
    as the wrong timing unless a different pre-release setup appears.
  - `cooperation/handoff`: sidecar checks again found no pre-trigger mechanism
    for sealed rat `(10,6)`: no branch to `ratat:11,7`,
    `cellnot:10,5,web`, or `reachable:10,6` from either the initial board or
    the common 7-turn setup. Reaching southeast lure cells after the common
    sweep spends all triggers and leaves the sealed rat unreachable.
  - `cooperation/blocked_v2`: the 33-turn upper-trigger-3 prefix is a dead
    basin for the lower-left trigger-2 pocket. Both trigger-2 cells are
    unreachable after upper 3, and checks for `trigger:2`,
    `cellnot:1,15,web`, `cellnot:2,15,explosive`,
    `cellnot:3,15,explosive`, and `cellnot:4,15,web` returned empty. A short
    continuation can stage P2 at `(11,14)` with a rat at `(9,15)`, but clearing
    `(10,15)` is an immediate contact trap and does not open the lower-left
    pocket.
- `solver`: added `ratgeom`, a synthetic one-turn mechanism diagnostic:
  `target/release/solver ratgeom <level> [prefix] --source x,y --target x,y`.
  It enumerates player placements/facings and stalls through the real engine to
  answer whether a specific rat step is locally possible. This is not a solver;
  use it to distinguish "the move cannot happen" from "the route to the
  required lure square is hard."
- `release`: a sidecar pass clarified a crucial trigger rule: stepping on a
  trigger does not zap its own neighbors; only same-number sibling triggers left
  on the board zap. Therefore stepping on right trigger 2 `(19,7)` would not
  detonate the adjacent right explosive column. The useful trigger-2 activation,
  if it exists, must be left trigger 2 `(0,16)` so sibling `(19,7)` zaps the
  right side. Direct probes for `ratat:0,16`, `playerat:0,16`, and
  `cellnot:0,16,2` while preserving at least 22 rats returned empty. The
  tempting `5 -> 6 -> 2` hypothesis also failed: after opener `v<vv^^>>v`,
  trigger-5 branch `v<>>>^`, no trigger-6 continuation was found; direct
  trigger-6 branchdumps from the opener and post-trigger-5 state also returned
  empty.
- `chase`: a real early helper-rat mechanism exists:
  `^>>>^^>^>^>>>>>v>v^^^>vvv` verifies as `Playing` and puts the mobile helper
  rat at `(12,15)`, breaking plank `(12,15)` before the known ratdrop family.
  `ratgeom` shows a synthetic placement can make that rat step left onto
  `(11,15)`, which would break the other plank, but actual branch searches from
  the prefix found no route to the needed left-side lure position and no
  `cellnot:11,15,plank` branch with 7+ rats. Treat this as a useful mechanism
  clue, not a solved route.
- `cooperation/handoff`: sidecar checks plus `ratgeom` confirm the sealed
  `(10,6)` rat has local geometry to step onto trigger `(11,7)` only under
  synthetic player placements, but route searches to those pre-trigger/body
  block positions return empty. The common 7-turn right sweep remains a dead
  family because it spends the board while `(10,5)` stays web.

### Current run notes - 2026-06-12

No new verified wins. The useful progress this run was pruning human-looking
mechanisms that repeatedly attracted search time.

- `solver`: added two diagnostic affordances:
  - `lookup --goal triggeropen:N,K` means trigger `N` has been consumed and the
    player still has at least `K` reachable cells. This was added to reject
    trigger activations that immediately self-seal.
  - `branchdump --no-canonical` uses full `state_hash()` instead of
    `search_hash()`, which is necessary when approach side or player position is
    the point of the diagnostic.
- `tinderrectangle`: the lower row-6 chase from `<^^<v<^<<vv<>` is locally
  dead. Continuing east keeps the lower rat one move behind; every north escape
  is `GameOver`, and turning west is only safe because it kills the lower rat.
  The side-loop prefix `<^^<vv<^^<<<` has the same tempo problem. Do not repeat
  the "make one row-6 gap" family unless a new blocker/tempo mechanism is found.
- `chase`: the helper route
  `^>>>^^>^>^>>>>>v>v^^^>vvv` really breaks `(12,15)`, but not `(11,15)`.
  `ratgeom`'s synthetic `(11,15)` break requires player `(11,16)` facing south;
  that cell is behind the plank being broken, so the staging is unreachable from
  the turn-22/24 frontier. Do not deepen this helper-left-plank family.
- `release`: the right isolated rat `(18,4)` cannot be lured through web
  `(18,5)`, and pushing the right-side released rats into x=17 is blocked by
  walls. Trigger 2 remains the plausible way to blast `(18,5)`, but direct
  trigger-2 probes from `v>v`, `v<vv^^`, `v>>>v`, and `v<vv^^>>v` returned no
  candidates. The next useful hypothesis must be rat-triggering or earlier
  lower-left setup, not another top sweep.
- `reload_v3`: the preserved roaming-rat family around
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<^<<<<<<<<v<v` is locally dead. Reaching trigger
  2 by the player sacrifices the roaming rat; rat-triggering trigger 2 preserves
  all three rats but strands them with zero reachable rats. Do not continue that
  family without a way to preserve post-trigger access.
- `cyborg_rats/ai_takeover`: prefix
  `v>v^>>v^<<vv^^<<vvv<<^^^<<<vv<<^^^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^^>^^^^^>>>^>>>>>>>>>>>`
  reaches player `(15,2)` with trigger-7 cells at distances 8-10, but only
  `(16,9)` is dynamically reachable. The branch `...vvv>vvvv` consumes trigger
  7 and leaves `player_reachable cells=1`; `triggeropen:7,2`,
  `playerat:15,9`, and `playerat:16,11` all failed from the relevant prefixes.
  Stop pursuing this trigger-7 lane; it structurally self-seals.
- `cooperation/handoff`: no viable pre-trigger handoff was found. The common
  7-turn sweep `v^ >^ >^ >^ >^ >^ ^^` opens the board but leaves `(10,6)` sealed
  with `(10,5)` still web. `ratgeom` for `(10,6)->(11,7)` works only with a
  player on the far-right island, and route searches to `playerat:14,7`,
  `playerat:14,8`, `ratat:11,7`, `cellnot:10,5,web`, and `reachable:10,6`
  returned empty in the checked windows.
- `cooperation/tug_of_war`: the top rat at `(7,0)` remains sealed in the
  two-cell pocket `{(7,0),(8,0)}`. Initial probes failed to change
  `(7,1)/(8,1)` webs or `(7,2)/(8,2)` planks. Trigger choreography `1/2/3`
  from the known trigger-1 route ends in 0- or 1-reachable-rat basins. Do not
  keep treating this as a symmetric trigger-order puzzle.
- `cooperation/blocked_v2`: the clean 36-turn frontier
  `^< ^^ <^ <^ <v <^ v> .> <> vv ^v .v <v ^v v^ vv >v ^> ^> v< v> ^< ^> ^< <> << <> << << <^ <^ v^ v> v> v> >v`
  leaves three rats and both trigger-2 cells unreachable. From that frontier,
  `trigger:2`, `reachable:5,11`, and `reachable:3,14` all failed. From the
  initial board, direct `trigger:2`, `reachablege:7`, and `allreachable` also
  failed in the checked budgets; the best all-reachable attempt still left the
  lower-left rat isolated. Treat trigger-2 access as the structural blocker.

### Human-mechanism pass - 2026-06-12 later

No new verified wins. The useful change in understanding was separating likely
authoring blockers from still-plausible mechanism chains.

- `cooperation/handoff` now looks structurally unwinnable as authored. The left
  rat component is sealed at the start:
  `{(0,3),(0,4),(0,5),(0,6),(0,7),(1,7)}`. Its boundary is walls or
  out-of-bounds, explosions do not destroy walls, zaps only add walls, and no
  explosive has a 3x3 blast that reaches the component. The common trigger-1
  branch
  `v^ >^ >^ >^ >^ >< ^^ v^ ^^ ^^ v^ v^ vv <v`
  is safe but leaves `(10,6)` and `(1,7)` sealed; consuming trigger 2 afterward
  with
  `v^ >^ >^ >^ >^ >< ^^ v^ ^^ ^^ v^ v^ vv <v ^^ ^^ ^^ ^<`
  removes the remaining explosives and still leaves both rats sealed.
- `cooperation/tug_of_war` also looks structurally unwinnable as authored. The
  top rat is confined to `{(7,0),(8,0)}` behind webs `(7,1),(8,1)` and planks
  `(7,2),(8,2)`. Static trigger/explosion analysis found no event that changes
  those cells; the nearest top explosive `(6,4)` misses them. A lower rat would
  have to break a plank, but the player positions needed to lure that require
  already being above the same web/plank barrier.
- `tinderrectangle`: the top/right breach failure is more specific than
  "cannot ignite." Opening `(13,2)` lets a neighboring top rat enter diagonally,
  bypassing the north-facing sword; the directly-above rat is not the only
  threat. The row-6 lower route can stage
  `<^^<v<<<v<>>` with lower rat `(3,6)` and player `(4,6)`, but the immediate
  north escape `^` is `GameOver` because the rat moves diagonally into the
  player's new cell. A credible solution needs a pre-existing gap in the top
  pack near the breach, or a real explosive/web clear before opening row 2.
- `chase`: the best real mechanism is still
  `^>>>^^>^>^>>>>>v>v^^^>vvv`, which breaks `(12,15)` and puts the helper rat at
  `(12,15)`, but `(11,15)` remains the blocker. Continuing with `<` holds that
  helper by sword-blocking it from `(13,15)` facing west; then `^` is
  `GameOver`, stalling freezes the local state, and `<<` kills the helper rat.
  Targeted route probes from the helper prefix could not stage the needed
  body-block cell `(11,14)`/`(11,13)`, and all initial explosives are too far
  from `(11,15)`, `(12,15)`, and `(11,16)` for a non-trigger explosion chain.
  The only remaining plausible mechanism is a simultaneous body-block frame
  with opener rat at `(12,15)`, another rat at or moving through `(11,14)`, and
  the player west/northwest, but targeted route probes did not stage it.
- `cyborg_rats/ai_takeover`: the currently reachable trigger 7 is `(16,9)`.
  Reaching it requires walking through `(16,8)`, destroying that web; when
  trigger 7 fires, sibling zaps from `(15,9)` and `(16,11)` turn empty
  neighbors into walls, including `(16,8)` and `(16,10)`, isolating the player.
  Verified dead branch:
  `v>v^>>v^<<vv^^<<vvv<<^^^<<<vv<<^^^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^^>^^^^^>>>^>>>>>>>>>>>vvv>vvvv`.
  The next plausible hypothesis is firing 7 without first emptying `(16,8)`,
  or having a non-player entity preserve that cell at zap time.
- `release`: static trigger analysis confirms the intended end event is left
  trigger 2 `(0,16)`: firing it leaves right trigger 2 `(19,7)` as the sibling
  zap source and detonates `(18,6..8)`, clearing web `(18,5)`. Firing right 2
  does the opposite and only clears the bottom-left. A reachable trigger-6 route
  from `v<vv^^>>v` exists:
  `v<vv^^>>vv<v<.........<<^^^^^^^^^<<<<v^v^v^v^v^^^^>>>>>>><><><>>>>>>v>v>v>vvvvvvvvv>>>vvvv`,
  but it kills the corridor rat and leaves only `(18,4)` sealed behind
  `(18,5)`. The viable variant must fire 6 while preserving a bottom/left actor
  that can consume left 2.
- `reload_v3`: static trigger analysis says the paper chain should be bottom
  `1,2,3,4,5` opening top `5/6`, then top `6` releasing `(0,21)`. Captured
  `triglookup` runs for `1,2,3,4,5,6`, `7,1,2,3,4,5,6`, and
  `1,7,2,3,4,5,6` all collapsed into two-rat or zero-reachable-rat basins
  before trigger 5/6. The chain is plausible, but no route preserved both the
  roaming rat and player access.
- `cooperation/blocked_v2`: there is a real mechanism lead. The initial
  `(0,7)` rat can be preserved and moved into the interior:
  `^< ^^ v^ ^^ vv ^v` leaves all 9 rats alive and puts that rat at `(3,8)`.
  The lower `3` trigger branch was the wrong event; it leaves `(6,10)` web
  intact. The upper trigger 3 route
  `v< vv v< << << << << <^ ^v ^< ^< v< << << << << << << <^`
  clears `(6,10)`/the central explosive wall but kills the top pack, leaving 5
  rats. From that state, relaxed continuations to `ratat:5,11`, `trigger:2`,
  `cellnot:1,15,web`, and a direct `cont` solve found no branch in the checked
  windows. The remaining plausible path is to combine the preserved `(3,8)` rat
  with the upper-3 clearing without losing the necessary lure geometry.

### Late human-mechanism pass - 2026-06-12

No new verified wins. This pass used four parallel explorer agents plus local
`release` / `ai_takeover` work, with short tactical oracle checks rather than
long direct solves.

- `release`: the most useful new constructive state is
  `v<vv^^>>vv><<v<<<^<^^<<><`. It verifies as `Playing` with 21 rats, player
  `(2,11)`, and a bottom actor staged at `(2,16)`. This is close to the human
  idea "open the lower-left lane, then use trigger 2", but it still fails:
  waypoint probes from that state to trigger 6 at `(17,16)` and `(19,18)` both
  return `UNREACHABLE`, and a `17,16;0,16` waypoint chain is also unreachable.
  Direct branchdumps from this state for `trigger:6`, `cellnot:1,16,explosive`,
  `ratat:19,18`, and `cellnot:16,17,plank` returned empty. The important
  insight is that a staged actor at `(2,16)` is possible, but if the player moves
  south before trigger 6 opens the lane the actor falls into the black-hole
  pocket; if the player leaves to the right first, current searches cannot keep
  the actor alive and reach trigger 6.
- `release`: the likely intended escape after trigger 6 would require a rat to
  traverse the newly opened row-17 lane and break plank `(16,17)`, letting the
  player return left to trigger 2. The checked prefixes did not reach this
  choreography. Do not just rerun trigger-5/6: strict `triglookup` order `5,6`
  from `v<vv^^>>v` again produced no trigger-6 continuation before falling into
  the old dead family.
- `tinderrectangle`: a stronger prepared-safe frontier is
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<vv<<v<<<^>>^^>>>>>v>vv>>^^^>>vvvv`.
  It verifies as `Playing` with player `(14,7)`, all 16 rats alive, lower rat
  `(2,3)`, and only 18 webs. From there, preserved release can clear `(2,4)`,
  but the lower rat and player enter a forced contact chase down the x=2 shaft:
  moving north kills the lower rat, moving sideways dies, and moving down just
  repeats the trap. Branchdumps / lookups for `rectsep`, `win`, and
  `cellnot:3,4,web` returned empty. This is now clearly a separation-loop
  problem, not an ignition-discovery problem.
- `reload_v3`: trigger 2 and trigger 7 are real early mechanisms, but spending
  them in either order still loses access to trigger 6. The trigger-2-first
  prefix `^>>>>>>>^^^^<<<<v<v>>` leaves two rats with `reachable_rats=0` and
  only trigger 7 reachable. The promising one-rat reload state
  `^>>>>>>>^^^^<<<<v<v>>^^<<<<<<<<<<^^<^^^^^vvvvvvv>>>>>><<<<<^^` leaves only
  `(0,21)` and both 7s reachable, but taking either 7 does not open trigger 6.
  The known 126-turn one-rat partial still has `(2,22)` unreachable, and pressing
  its remaining trigger 2 leaves zero reachable trigger continuations.
- `chase`: the suspected body-block square is reachable. Prefix
  `^>>>^^>^>^>>>>>v>v^^^vvv` verifies as `Playing` with all 8 rats, player
  `(13,15)`, helper rat `(11,14)`, and both throat planks intact. However,
  immediate `<`/`>` falls into the old `(12,15)` helper trap; `^v` dies; `^<`
  kills the helper. Branchdumps from the `(11,14)` / `(12,14)` staging states
  for `cellnot:11,15,plank` and `cellnot:11,16,web` returned empty even with
  `--min-rats 7`. Trigger 5 is reachable in the area but spends the geometry
  without opening the throat.
- `cooperation/blocked_v2`: the near-miss around `(6,11)` is now locally
  checked. From the final near-miss, one-turn enumeration shows north pulls the
  rat back to `(6,10)`, south pulls it to `(7,12)`, and east/west/stall leave it
  at `(6,11)`; the trigger count never changes. From the turn-28 predecessor,
  only P2 south moves the target rat, and it moves `(6,10) -> (6,11)`, not onto
  trigger `(5,11)`. Bounded branchdumps for `trigger:2`, `playerat:4,11`,
  `reachable:4,11`, and `ratat:5,11` from the upper-3 family returned empty.
  The preserved-rat prefix `^< ^^ v^ ^^ vv ^v` still cannot be combined with
  upper-3 clearing in checked budgets.
- `cooperation/blocked_v2`: a new 8-rat lower-trigger-3 branch exists:
  `^< ^^ v^ ^^ vv ^v v^ ^^ v> v> v> vv vv ^v ^v ^< ^v`. It verifies as
  `Playing`, but it consumes the lower 3, leaves the `(6,10)` / trigger-2 pocket
  untouched, and direct `trigger:2` checks from it returned empty. Treat it as a
  decoy basin unless a new route changes the trigger-2 pocket first.
- `cyborg_rats/ai_takeover`: after standard opener `v<vv^^>>v`, diagnostics are
  better than `release`: 22 of 23 rats are reachable and trigger 7 has static
  distances around 36-38. But dynamic checks still fail. Branchdumps from the
  opener for `trigger:7`, `ratat:16,9`, and `cellnot:16,10,empty` returned
  empty; waypoints to `(15,9)`, `(16,9)`, and `(16,11)` all returned
  `UNREACHABLE`; direct `lookup --goal win` from the opener returned
  `NO_SOLUTION` quickly. Do not transfer `release`'s opener mechanically.

### Continuation pass - 2026-06-12 evening

No new verified wins. Solution artifacts were intentionally left unchanged.
This pass shifted more work from broad search to human-style mechanism checks.

- `release`: a stronger row-17 actor frontier exists:
  `v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>>`. It verifies as `Playing` at 37
  turns with 21 rats, player `(13,14)`, and the bottom actor at `(13,17)`.
  This is better than the old `(2,16)` collapse: the actor can be carried east
  along row 17 before the player leaves the lower-left. However, the actual
  state cannot continue the actor to `(14,17)` or pair `(14,17)` with useful
  player lure squares; branchdumps for `ratat:14,17`,
  `ratplayer:14,17,16,14`, `ratplayer:14,17,18,14`, and
  `ratplayer:14,17,18,13` returned empty. `ratgeom` says `(13,17)->(14,17)`
  is locally possible from synthetic right-side placements, so the blocker is
  route/staging, not rat movement. Trigger 5 from this frontier is reachable
  while preserving 21 rats, but it still leaves no trigger-6 continuation.
- `tinderrectangle`: the exact separated winning geometry was checked rather
  than another ignition search. From the 84-turn safe-side prefix
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^`,
  branchdumps for `ratplayer:2,6,14,7`, `ratplayer:3,6,14,7`,
  `ratplayer:4,6,14,7`, and `ratplayer:6,6,14,7` returned empty. From the
  135-turn staging, the lower rat can be moved to `(1,4)` with all 16 rats
  preserved, but only with the player on the wrong side; target checks for
  `ratat:2,5` or `ratplayer:2,5,...` from that state returned empty. Treat the
  missing tactic as a release-gate geometry problem, not as missing cleanup.
  A sidecar pass also found no 16-rat branches for `cellnot:3,4,web`,
  `playerfacing:2,4,east|west|south`, or `rectsep` from the 135-turn family,
  nor for `cellnot:2,4,web` / `cellnot:3,4,web` / `rectsep` from the 113-,
  116-, or 84-turn pump cuts. Relaxing to 15 rats can open `(3,4)`, but only
  after killing the lower rat; short `win` checks from those 15-rat states also
  returned empty.
- `reload_v3`: trigger 7 first has real all-rat branches such as
  `^^^^^<<<<<<^^^`, but follow-up trigger 2 leaves all three rats with
  `reachable_rats=0`. The concrete `7 -> 2` branch
  `^^^^^<<<<<<^^^vvv>>>>>>>>>>>>>^vvvvv<<v<` verifies as `Playing` with
  triggers reduced but no reachable rats. A sidecar pass rechecked the all-rat
  trigger-1 rat-trigger frontier
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<^<<<<<<<<v<v>` and found no preserved branch for
  `triggeronly:2/3/4/5`, bottom trigger-2 cell access, `allreachable`,
  `cellnot:1,21,web`, or `triggeronly:6`. Relaxing to two rats only reaches the
  dead top-trigger-2 basin with `reachable_rats=0`. This reinforces that the
  chain fails on preserving post-trigger access, not on reaching early triggers.
- `chase`: the 51- and 107-turn helper frontiers are too late for the `(0,19)`
  pocket. In both, trigger 4 has already made `(2,19)` a wall, while `(1,19)`
  remains web and the rat at `(0,19)` is unreachable. The route
  `>.>>^^>^>>>^>><<v<^^v>>>>^^>>>^<^<<` from the 20-turn cut physically clears
  `(3,18)/(3,19)` while preserving lower trigger-4 cells, but it self-seals the
  player at `(13,9)` with one reachable cell. From the 20-turn cut,
  `cellnot:1,19,web`, `playerat:1,19`, `playerat:2,19`, `ratgone:0,19`, and
  `triggeronly:3` all returned empty; direct initial-board pocket checks also
  returned empty. Continue only if a route clears the lower explosives without
  self-sealing or without consuming trigger 4.
- `cooperation/blocked_v2`: the preserved-rat lead
  `^< ^^ v^ ^^ vv ^v` remains valid and keeps all 9 rats, but a sidecar pass
  found that the only concrete route to upper trigger 3 kills the preserved
  interior rat before trigger 3 fires. From the preserved frontier, checks for
  `cellnot:6,10,web`, `triggeronly:3`, and `playerat:13,4` returned empty
  across relaxed rat-count windows. From the resulting upper-3 basin, trigger 2
  and lower-left pocket access targets also returned empty. This now looks more
  like a structural trigger-2 blocker than a missed mop-up.
- `cooperation/handoff` and `cooperation/tug_of_war`: quick structural
  rechecks returned empty for the critical boundary changes (`handoff`
  `reachable:10,6` / `cellnot:10,5,web`; `tug_of_war`
  `cellnot:7,1,web` / `cellnot:8,1,web`). Keep treating both as likely
  authoring blockers unless a new event can change those sealed components.
- `cyborg_rats/ai_takeover`: from opener `v<vv^^>>v`, trigger-open checks for
  trigger 7/8 again returned empty in short windows. The useful mental model is
  still that reachable trigger 7 self-seals its approach unless a non-player
  actor preserves the relevant cell during the zap. Trigger 6 is statically
  reachable from the opener, but local checks for `triggeropen:6,2`,
  `triggeronly:6`, and strict `6,8,7` / `6,8,7,2` trigger orders produced no
  continuation; do not treat trigger 6 as a free bottom-door opener.

### Continuation pass - 2026-06-12 composite-goal/tooling pass

- Solver tooling now supports compound timing goals in `lookup` / `branchdump`:
  `cellnotratat:x,y,kind,ratx,raty`,
  `triggeronlycellis:n,x,y,kind`, and
  `triggeronlycellnot:n,x,y,kind`. `lookup` also accepts `--min-rats N`.
  These were added specifically for human-style timing checks such as "open
  this web while this actor is still alive" and "fire this trigger without
  walling the escape cell." `cargo build --release -p solver` and
  `cargo test -p solver` pass after the change.
- `release`: the promising clean trigger-6 + left-trigger-2 mechanism was
  checked more directly. From staged prefix
  `v<vv^^>>vv><<v<<<^<^^<<><`, `lookup --goal
  cellnotratat:1,16,explosive,2,16 --min-rats 20` ran to timeout with no
  branch; a matching `branchdump` also returned empty.
  The sidecar rat-geometry check also found no one-step way for the `(2,16)`
  actor to move onto `(1,16)` or `(0,16)` before trigger 6 clears the blocker.
  Treat the useful requirement as: clear `(1,16)` without spending the left
  actor, or find a different actor entirely.
- `release`: a separate 1-rat continuation from the macro best state
  `v<vv^^>>v...>>>>` also timed out trying to clear `(18,5)` or kill `(18,4)`.
  The board remains the same isolated-right-rat shape: the player can reach
  trigger 6, but the right rat is sealed behind `(18,5)`.
- `reload_v3`: the trigger-1 station is real but timing-sensitive. From
  `P41 = ^>>>>>>>^^vvvvvv<<<^^<<^^<<<<^<<<<<<<<v<v`, immediate `^` gives a
  new all-rat state with the roaming rat at `(4,17)`, but `lookup` found no
  all-rat branch from that state to `triggeronly:1`, `triggeronlycellnot:1,1,21,web`,
  or `ratat:10,22`; a BFS-style branchdump for `triggeronly:1` from this state
  was still running. The older P42/P43 trigger-1 follow-ups remain dead for
  all checked trigger-2/6/7 and lower-left web targets.
- `cyborg_rats/ai_takeover`: the body-block idea around trigger 7 was modeled
  exactly in `/tmp/ai_t7_local.py` using `/tmp/infestation-clingo/bin/python`.
  From the 113-turn trigger-7 lane prefix, the local state space saturated
  (`expanded=355`, `seen=71`, `trigger7_events=9`) with zero unsealed trigger-7
  events. In every event, `(16,8)` and `(16,10)` are empty before the zap and
  walls after it. The mechanical reason is `cyborg_rat.rs`'s `Dist` ordering:
  stepping into the cells orthogonally adjacent to the player is disfavored, so
  the closest body stops at `(16,7)`, one turn too high. Do not spend more time
  on this exact body-block lane unless the approach position changes.
- `blocked_v2`: a stronger all-rat staging path is
  `^< ^^ v^ ^^ vv ^v vv vv v^ vv v^ <v <^ <v <^ <> ^. ^. ^.`, with the
  follow-up `v^ v^ ^^ <^` reaching the upper rat wall while preserving all 9.
  From that wall state, `trigger:3` with `--min-rats 9` and `--min-rats 8`
  returns empty immediately; `cellnot:14,4,web --min-rats 7` from the previous
  staging also returns empty. Relaxed `trigger:3` checks with `--min-rats 7`
  and `--min-rats 6` also returned empty, so the upper-rat-wall route is not
  just one preserved-rat short.

### Continuation pass - 2026-06-12 trap/resource gates and mechanism lanes

- Solver tooling now has a broader `TrapConstraints` gate for lookup-style
  searches. In addition to reachable/trapped rats, `lookup`, `branchdump`,
  `triglookup`, and `triganylookup` accept:
  `--require-reachable-cell x,y` / `--cleanup-cell x,y`,
  `--min-explosives N`, `--min-triggers N`, and `--max-webs N`.
  These reject branches that reach a local milestone only after spending the
  trigger/explosive/corridor resource a human would know is needed for cleanup.
  `cargo test -p solver --no-run` and `cargo build --release -p solver` pass.
- `tinderrectangle`: the 16-rat branch
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<vv<<v<<<^>>^^>>>>>v>vv>>^^^>>vvvv^^^^<<vvv<<^^^<<<<<<<`
  is a clean diagnostic state, not proven progress. `ratgeom` from that state
  finds no one-step way to move the lower rat left/up; all safe carry geometry
  pulls it right until the player hits the `(11,3)` wall. Spending the lower rat
  cheaply gives better 15-rat staging states such as suffixes `<v>`, `><v<`,
  and `<vv>^^`, all with `reachable_rats=15/15`, `webs=15`, and
  `trapped_unreachable_rats=0`. Quick win and `cellnot:1,2,web` probes from
  those states returned empty, so the next real milestone is a safe top-row
  notch, not direct ignition.
- `reload_v3`: a useful staged prefix after the trigger-1 family is
  `>>>^>>>>vvv.v<^<<<v<<<<<<<^^<^<<<<<<<<v<v>><`. It verifies as `Playing` with
  all 3 rats alive: player `(1,18)`, roaming rat `(2,18)`, sealed rat `(0,21)`,
  and trigger 2 still intact. The active follow-up is a direct inverse test:
  `branchdump --prefix <that> --goal cellnot:2,21,explosive --min-rats 2`.
  If it succeeds, immediately verify whether the `(0,21)` rat becomes solvable;
  if it fails, this trigger-1 station likely stages the roaming rat too high.
- `chase`: the healthier staging prefix is
  `^>>>^^>^>^>>>>>v>vvvvv^^^^^^<<<^`, which places the player at `(10,12)` and
  helper rat at `(11,14)` while keeping all triggers reachable. This is better
  than letting the helper fall to `(12,15)`. Active lanes test whether this
  staging can change web `(11,16)` or reach `winready`; any low-rat branch that
  leaves `(11,16)` intact should be treated as another dead basin for the
  `(11,17)` rat.
- `tug_of_war`: do not rush trigger 2/3 from the symmetric opener. The useful
  asymmetric staging line `>< >< ^^ ^^ ^^ ^^ ^^ ^^ <^` leaves 5 rats,
  14 explosives, 15 triggers, and `reachable_rats=4/5`. Active lanes test
  `explosivesle:4` and `win` while preserving at least 10 triggers and keeping
  at least 4 rats reachable. The top pocket remains suspicious until
  `cellnot:7,1,web` or `cellnot:8,1,web` has a concrete mechanism.
- `handoff`: the current better branch
  `v^ >^ >^ >^ >^ >^ ^^ vv ^v .> .> .> v> v< <<` leaves 3 rats, 5 explosives,
  and 2 triggers. It is not enough that trigger 2 at `(2,2)` is reachable; the
  important questions are whether `(10,6)` can be removed before resources are
  gone or whether it can be moved to trigger `(11,7)`. Active lanes test
  `ratgone:10,6` and `ratat:11,7` with trigger/explosive preservation.
- `blocked_v2`: the correctly spaced staged line
  `^< ^^ v^ ^^ v^ ^^ v^ ^> v> ^> vv ^v v^ ^v vv ^v` verifies as `Playing` with
  8 rats, `webs=12`, no planks, and `trapped_unreachable_rats=0`. Use this
  instead of the malformed concatenated prefix. Active lane tests
  `triggeronly:3` while preserving 8 rats and at least 4 reachable rats.

### Continuation pass - 2026-06-12 current-oracle mechanism audit

No new verified wins. All candidate strings in this pass were checked with
`target/release/solver verify`; none printed `result=Won`, so do not update
`solver/solutions/final_solutions.json` or `solver/solutions/SOLUTIONS.md`.

- Repository/rules baseline: branch `claude/new-puzzles` is clean after
  `3851f2b Merge upstream input handling changes`. The current unresolved list
  is still exactly:
  `tinderrectangle.csv`, `release.csv`, `reload_v3.csv`, `chase.csv`,
  `cyborg_rats/ai_takeover.csv`, `cooperation/tug_of_war.csv`,
  `cooperation/handoff.csv`, and `cooperation/blocked_v2.csv`.
- Current-oracle recheck after the upstream rule/input merge:
  `cargo build --release -p solver` was a no-op on 2026-06-12, and all stored
  solution artifacts still replay under `target/release/solver verify`
  (`27/27` original entries in `final_solutions.json` plus the 5 Claude
  puzzle entries in `SOLUTIONS.md`). The invalidated old path is the previous
  `tinderbox.csv` entry; upstream replaced it with `tinderbox_v2.csv`, whose
  stored move string verifies as `result=Won`. The hub CSVs (`world.csv` and
  `claude/gauntlet.csv`) are not counted as puzzles for the remaining-8 target.
- The last six Release probes that were running at compaction exited before
  their stdout could be collected from this tool state. `tmux` history only
  showed redirected commands and `/tmp/infestation_tmux` was empty, so do not
  cite those runs as evidence. Rerun any valuable Release probe in a captured
  session before updating this doc with its result.
- `tinderrectangle`: cheap lower-rat spends from
  `TH=<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<vv<<v<<<^>>^^>>>>>v>vv>>^^^>>vvvv^^^^<<vvv<<^^^<<<<<<<`
  still produce good 15-rat diagnostic states (`<v>`, `><v<`, `<vv>^^`),
  but direct row-2 entry is `GameOver`, `events` from those states returns
  `NO_EVENTS`, and targeted row-2/top-row notch probes (`cellnot` on row-2 webs
  and top explosives) returned no branches. Treat these states as clean
  diagnostics, not progress to a mop-up.
- `reload_v3`: staged prefix
  `>>>^>>>>vvv.v<^<<<v<<<<<<<^^<^<<<<<<<<v<v>><` verifies as `Playing` with
  all 3 rats alive, but direct and inverse probes for removing `(2,21)`
  explosive or combining trigger 6 with that removal returned no branches, even
  relaxed without rat-preservation gates. The staged roaming rat appears too
  high to open the bottom-left sealed rat.
- `chase`: old AI partials decode to useful but still incomplete basins. The
  51-turn branch `>>>.^v<<<>>>vv<^^>^^^^^^vvv>>>vv^^^^<^^>^^^^^^>>>^^` leaves
  4 rats and all 12 triggers reachable, but `win` lookup timed out and
  trigger-1/2/4 continuations still leave isolated rats behind webs. Current
  helper lanes from `^>>>^^>^>^>>>>>v>vvvvv^^^^^^<<<^` also produced no branch
  for changing `(11,16)`, removing `(11,17)`, or breaking `(11,15)` while
  preserving the needed rats.
- `cooperation/tug_of_war`: the top rat gate is not solved by simply clearing
  the nearby `(6,4)` explosive. A relaxed lookup found
  `<v .< .v .v .v .v v> .v ^>` as a 9-turn `cellnot:6,4,explosive` goal, but
  diagnostics are bad (`reachable_rats=0/7`, `trapped_unreachable_rats=4`), so
  it is a false milestone. `wp2`/lookup to the lure squares `(7,3)` and `(7,1)`
  failed, and branchdumps for `ratat:7,4`, `ratat:7,3`, `ratat:8,3`,
  `cellnot:7,2,plank`, `cellnot:8,2,plank`, and `norats2:7,0,8,0` returned no
  useful branch under preservation gates.
- `cooperation/handoff`: `ratgeom` from the 7-turn sweep
  `v^ >^ >^ >^ >^ >^ ^^` shows the sealed `(10,6)` rat can locally step onto
  right trigger `(11,7)` if P2 reaches `(14,7)/(14,8)`, but this is probably a
  false mechanism: if the rat consumes the right trigger, the right trigger is
  no longer a sibling zap source. A route to P2 `(14,7)` exists, but only after
  all triggers/explosives are gone, leaving the same sealed rat. Stricter
  branchdumps preserving resources for `ratplayer:10,6,14,7`,
  `cellnot:10,5,web`, `reachable:10,5`, and
  `triggeronlycellis:2,10,6,empty` returned no branch. The useful condition
  must move or kill `(10,6)` without consuming the right trigger and before left
  trigger 2 is fired; current checked routes do not do that.
- `cooperation/blocked_v2`: `B+B1` can reach trigger 3 (representative suffix
  `vv`), but trigger 2/4, `(1,15)` web change, `(0,15)` rat removal, and
  reachability of the trigger cells all returned no branches under all-rat
  gates. The upper-trigger-3 family remains a dead basin for lower-left access.
- `release`: the standard opener `v<vv^^>>v` and trigger-5 family were again
  audited. Branchdumps from `P` for `ratat:19,7`, `cellnot:18,5,web`,
  `triggeronly:2`, `triggeronlycellnot:2,18,5,web`,
  `triggeronlycellnot:2,18,6,explosive`, and
  `cellnotratat:18,5,web,19,7` all returned no branches with high rat
  preservation. `events` from `P` finds only local non-right-pocket events.
  Continue only from a genuinely different opener.
- `cyborg_rats/ai_takeover`: unlike `release`, `(18,5)` is already open after
  `P`, so the release isolated-web blocker is not the main blocker here. Direct
  trigger-7, trigger-8, trigger-open, cyborg-parking, and `ratsle:18` probes
  from `P` and `P+S5` all returned no branches or no useful events. `P+S5` is
  especially barren under the updated oracle; do not treat it as transferable
  progress from `release`.
- `cooperation/blocked_v2`: the new lower lane is real progress but currently
  looks like another dead basin. Prefix
  `v< v^ v^ << <^ <^ ^^ ^v ^^ ^> ^> ^> ^> ^v ^v ^^ ^v ^v ^v ^v ^v ^^ ^. ^^`
  reaches a 4-rat state with rats at `(18,4)`, `(9,13)`, `(9,14)`, `(0,15)`.
  A 7-turn event
  `^^ >^ ^v ^v ^v ^v ^>` drops to the 3-rat B31 state
  `(9,13)`, `(9,14)`, `(0,15)`, but then direct `ratat:3,14`,
  staged `ratat:5,14` / `ratat:4,14`, `triggeronly:2`, and
  `cellnot:1,15,web` all returned no branch. Backing up did not help:
  `triggeronly:2` and `cellnot:1,15,web` from B22, and `triggeronly:2`
  from B20, also returned no branches under rat/reachability preservation.
  `triggeronly:5` from B24 is reachable but appears to be a trap: it leaves
  3 rats, 2 reachable rats, still-closed `(1,15)` web, and only 4 triggers;
  `triganylookup` chooses trigger 5 first and then reports no reachable step 2.
  Do not keep extending this B20/B22/B24/B31 family unless the new hypothesis
  changes the left-pocket mechanism, not just the cleanup order.
- `cooperation/handoff`: a preserved-resource branch
  `v^ >^ >^ >^ >^ >^ ^^ .v .v .> v> v> ^>` verifies as `Playing` and leaves
  two rats `(10,6)` and `(1,7)` with all 8 explosives and 4 triggers still
  live, but `reachable_rats=0/2`. This is important because it disproves the
  simpler "we spent resources too early" explanation for the known one-rat
  basin. `ratgeom` still says `(10,6)` can locally step onto `(11,7)` only when
  P2 is effectively at `(14,7)`/`(14,8)`, but preserved-resource searches to
  `playerat:14,8`, `ratat:11,7`, and `cellnot:10,5,web` from the 7-turn setup
  found no branch in the checked budgets.
- `cooperation/tug_of_war`: a new pre-trigger branch
  `.< v^ <^ >^ <^ <^ >^ <^` verifies as `Playing` with 5 rats, 14 explosives,
  all 15 triggers, and `reachable_rats=4/5`. The only unreachable rat is the
  top pocket rat `(7,0)`. This is a cleaner frontier than the old trigger-1
  route; the next question is specifically how the top pocket can be opened or
  consumed before any trigger choreography. A one-turn top-rat move such as
  `^v` only shifts it to `(8,0)` and leaves it unreachable in the same pocket.
  Source audit confirms zaps only turn empty cells into walls and queue
  explosions on explosives; explosions clear rats/webs/planks; rats cannot move
  through webs. From this frontier, branchdumps for `cellnot:7,1,web`,
  `cellnot:8,1,web`, `cellnot:7,2,plank`, `cellnot:8,2,plank`,
  `cellnot:6,0,wall`, `cellnot:9,0,wall`, and `norats2:7,0,8,0` all returned
  no branch. A `ratgone:7,0` branch is a false positive because it only moves
  the rat to `(8,0)`. Treat the top pocket as structurally blocked unless a
  new mechanism can affect one of those exact cells.
  Backing up to the initial state and testing the possible helper rat at
  `(7,5)` did not rescue the mechanism: initial-state branchdumps for
  `cellnot:7,2,plank`, `cellnot:8,2,plank`, `ratat:7,3`,
  `playerat:7,3`, and `playerat:7,1` all returned no branch. This suggests the
  apparent helper-rat plank-break route is circular too: the rat can be pulled
  upward locally, but no player can get above the plank to make it break the
  top gate.
- `release`: a new prefix
  `v<vv^^>>vv>^<<v<<<^^` verifies as `Playing` after the known 3/4 opener but
  before trigger 5 is consumed. It drops rats from 23 to 22 and webs from 27 to
  25 while preserving the remaining 5 explosives and all remaining triggers
  (5/6/2). Diagnostics: player `(5,12)`, `rats=22`, `explosives=5`,
  `webs=25`, `triggers=7`, reachable rats `19/22`, reachable triggers `3/7`;
  trigger 5 at `(11,12)` and trigger 6 at `(17,16)`/`(19,18)` are reachable,
  trigger 2 at `(19,7)`/`(0,16)` is still unreachable, and `(18,5)` remains
  web. Negative checks from post-3/4, post-5, and this new prefix:
  `triggeronlycellnot:2,18,5,web`, `ratat:0,16`, and `ratgone:18,4` found no
  branch in checked budgets. Trigger-6 routes are reachable but still collapse
  to the known dead one-rat `(18,4)` basin with `(18,5)` web closed.
- `tinderrectangle`: a shorter all-rats-live staging prefix
  `<<^<<^<>^>>>vv^` verifies as `Playing` with player `(7,4)`, lower rat
  `(5,5)`, all 16 rats alive, 43 explosives, and 51 webs. From that state,
  `lure --rat 5,6` and `lure --rat 6,6` with safe cells on the right-side row-6
  / row-8 staging area returned immediate `NO_SOLUTION`, and `rectlower` plus
  `ratat:0,0` / `ratat:16,0` bounded lookups returned no solution. It is still
  useful because it reaches the lower rat without the old `(2,3)` pin, but the
  next hypothesis needs a different separation target than simply pushing the
  lower rat to `(5,6)`/`(6,6)`.
  Event scan from this state finds all-rats-live siblings such as `^>` and a
  contact-close branch `<<^<<^<>^>>>vv^^<<v^` with player `(5,3)` and lower rat
  `(5,4)`. That branch is also likely a trap: only immediate `v` survives;
  `^`, `<`, `>`, and `.` are `GameOver`. Follow-up branchdumps from the 15-turn
  prefix, the `^>` event state, and the contact state for `rectsep`,
  `ratat:5,6`, and `ratat:6,6` returned no branch with all 16 rats preserved.
  A direct `win` lookup from the contact state timed out with best suffix `v^`,
  which kills/removes the lower rat and does not solve the level.
- `reload_v3`: best all-rat structural staging found in this pass is
  `^^^^^<<<<<<^^<^^^^^^^>>>>>>>>>>vvvvvv<<<<<>>>>>^^^^^^<<<<<<<<<vvv`
  (`Playing`, 65 turns). Diagnostics: 3 rats `(19,2)`, `(11,5)`, `(0,21)`,
  6 explosives, 11 webs, 14 triggers, 16 planks, player `(7,5)`,
  `reachable_rats=1/3`, `trapped_unreachable_rats=2`, and only trigger 7 is
  reachable (`(7,8)`/`(9,9)`). It preserves all rats and clears top/right
  structure, but still seals the bottom-left rat and trigger 6. Useful earlier
  cage-prep frontiers are `^>>>>>>>^^^^<^<<<v` near trigger 2 and
  `^>>>>>^>>^^^<<<<<<<<<<<<<^^<<<<v` near trigger 3, both with all 3 rats
  alive, 6 explosives, 18 webs, 14 triggers, and 15 planks. The key human
  point: firing a cage trigger is both opener and closure, so direct trigger 7
  or trigger 1 first is still the wrong objective; chain cage prep while
  preserving rats/access.
  Follow-up cage-prep checks from these two frontiers were negative under the
  current oracle: from `^>>>>>>>^^^^<^<<<v`, branchdumps for `ratat:17,13`,
  `triggeronly:2`, and `cellnot:17,14,plank` with `--min-rats 3` returned no
  branch; from `^>>>>>^>>^^^<<<<<<<<<<<<<^^<<<<v`, branchdumps for
  `ratat:4,11`, `triggeronly:3`, and `cellnot:4,12,plank` with
  `--min-rats 3` returned no branch. The simple "adjacent rat steps onto the
  local trigger while preserving all rats" mechanism is not enough.
- `chase`: best current staged continuation is
  `>>>.^v<<<>>>vv<^^>^^^^^^vvv>>>vv^^^^<^^>^^^^^^>>>^^vvvvvvvvv>>>>^^^^^^^^>>^^^`
  (`Playing`, 77 turns). Diagnostics: 4 rats `(15,0)`, `(19,8)`, `(11,17)`,
  `(0,19)`, 2 explosives `(3,18)/(3,19)`, 6 webs, 9 triggers, 2 planks,
  player `(17,0)`, `reachable_rats=2/4`, `trapped_unreachable_rats=2`. The
  top/right timing is improved, but the core blocker remains rat `(11,17)`
  isolated behind web `(11,16)`. Since zaps do not clear webs and the remaining
  explosives are far away, the intended route likely needs an earlier explosion
  or helper-rat event before the known ratdrop chain collapses that structure.
  Initial-state preservation probes also failed for that pocket:
  `cellnot:11,16,web`, `reachable:11,17`, `ratgone:11,17`,
  `cellnot:11,15,plank`, `cellnot:12,15,plank`, and `playerat:11,16` all
  returned no branch with high rat-preservation gates. This suggests the
  `(11,17)` pocket cannot simply be opened first; the useful event may need to
  alter global structure before the preservation gates make sense.

### Parallel mechanism audit - 2026-06-12

No new verified wins. All managed solver sessions and parallel explorer tasks
finished or timed out; no `target/release/solver` processes were left active at
checkpoint time. The practical status is still 8 unsolved, but `chase` gained a
real new mechanism frontier.

- Current verification baseline still holds locally: `target/release/solver`
  verifies all 27 original entries in `solver/solutions/final_solutions.json`
  and the 5 Claude puzzle entries in `SOLUTIONS.md`. There is no configured git
  remote in this checkout, so remote/upstream validation requires adding or
  restoring a remote first.
- `chase`: new best human-style frontier:
  `^>>>v^^^>^^>>>>>v>>v^^^vvvv`.
  It verifies as `Playing` and preserves all 8 rats while using the original
  left helper rat to break plank `(12,15)`. Diagnostics after the prefix:
  player `(13,16)`, rats `(10,0)`, `(15,1)`, `(13,3)`, `(12,15)`, `(8,16)`,
  `(19,16)`, `(11,17)`, `(0,19)`, features `rats=8`, `webs=19`,
  `triggers=19`, `planks=6`, and `reachable_rats=5`. This is the first
  confirmed half-step toward opening the `(11,17)` pocket. Immediate legal
  continuations are only `v` and `>`; `^`, `<`, and `.` are `GameOver` because
  the helper rat steps into the player. Follow-up branchdumps from this prefix
  for `ratat:11,15`, `cellnot:11,15,plank`, and `cellnot:11,16,web` with
  `--min-rats 8` returned no branches in checked budgets. The next useful
  probe is earlier west/southwest staging, e.g. `ratplayer:11,14,5,16`,
  `ratplayer:11,14,6,15`, `ratplayer:10,13,5,16`,
  `ratplayer:10,13,6,15`, `ratplayer:12,15,4,17`, and
  `ratplayer:12,15,3,17`. The human hypothesis is now specific: break
  `(12,15)` while already far enough west/southwest to pull that same helper
  into `(11,15)`, then only search for `(11,16)` / `(11,17)` cleanup.
- `tinderrectangle`: parallel analysis reconfirmed the intended win geometry:
  from the long lower staging prefix
  `P=<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv^^^>vv<vv<>>^^^^^<<<vvv<<^^^<<<vv<<v<<<^>>^^>>>>>v>vv>>^^^>>vvvv`,
  `ignitions` shows one-move wins when the lower rat is on row 6 (for example
  rat `(2,6)` or `(3,6)`) and the player is already at right safe cells such
  as `(14,7)`. The missing mechanism is separation, not ignition. Fresh
  release-frame probes from `P` returned no branches for
  `ratcell:2,3,3,4,empty`, `ratplayerfacing:2,5,4,6,east`, and `rectsep` with
  all 16 rats preserved. From the short staging prefix `<<^<<^<>^>>>vv^`, a
  stall gives a real all-rats-live state with the lower rat at `(6,5)`, but
  `lure` / `branchdump` still could not convert it into `(6,6)` plus right-side
  safety. Treat both the long lower-gate family and the short `(6,5)` family as
  contact traps unless the next hypothesis changes the separation target.
- `release`: a direct `lookup --goal cellnot:18,5,web` from the initial state
  timed out after 300s with best heuristic `1012`, but the best state still had
  `(18,5)` as web and remained in the familiar right-pocket basin. A stricter
  `branchdump --goal triggeronlycellnot:2,18,5,web` produced no branches. The
  newer prefix `v<vv^^>>vv>^<<v<<<^^` still verifies as `Playing`, but
  `triggeronly:6` from it returned no branches in the checked budget. Continue
  only from a materially different trigger-2/right-pocket hypothesis.
- `cyborg_rats/ai_takeover`: comparison against `release` confirms `(18,5)` is
  already open here, but that does not solve the right-pocket mechanism.
  From `v<vv^^>>v`, branchdumps for moving the `(18,4)` cyborg to `(18,5)`,
  `(18,6)`, or `(19,7)`, and for `triggeronly:2`, returned no branches. Lookup
  for player access to `(18,5)` / `(17,5)` and for `ratgone:18,4` also returned
  no solution in the checked budgets. `ratgeom` says `(18,4)->(18,5)` is
  synthetically possible under artificial player placements, but not from
  reachable play after the standard opener.
- `reload_v3`: no new good line. The short all-rat `triggeronly:7` branch
  `^>>>>>^>>^^^<<<<v<<^<<<<<<<^^^` still looks like a trap: diagnostics leave
  rats `(14,5)`, `(7,9)`, `(0,21)`, only `reachable_rats=1/3`, and only one
  reachable trigger. Follow-up checks for `triggeronly:6`, `triggeronly:1`, and
  `cellnot:2,21,explosive` returned no branches. A local `triganylookup` run
  found trigger-2 and trigger-7 first-step branches but did not produce a
  final solution before the managed session ended; rerun with captured output
  before citing it as evidence.
- `cooperation/tug_of_war`: independent audit confirmed the clean
  pre-trigger frontier `.< v^ <^ >^ <^ <^ >^ <^` has one unreachable top-pocket
  rat `(7,0)/(8,0)`. Ungated boundary checks from that frontier for
  `cellnot:7,1,web`, `cellnot:8,1,web`, `cellnot:7,2,plank`,
  `cellnot:8,2,plank`, `cellnot:6,0,wall`, `cellnot:9,0,wall`, and
  `norats2:7,0,8,0` returned no branches. A broader `lookup --goal win` from
  that frontier also timed out. Treat this as structurally blocked unless a
  new mechanism can affect one of those exact boundary cells from before the
  frontier.
- `cooperation/handoff`: the preserved-resource frontier
  `v^ >^ >^ >^ >^ >^ ^^ .v .v .> v> v> ^>` still leaves rats `(10,6)` and
  `(1,7)` with no reachable rats despite all resources being live. A 300s win
  lookup from that frontier timed out. Preserved-resource branchdumps for
  `ratat:11,7` / `cellnot:10,5,web` remain negative, so plain trigger-first or
  far-side-after-trigger routes should not be repeated.
- `cooperation/blocked_v2`: no new frontier beyond the B31 family. From the
  3-rat state `(9,13)`, `(9,14)`, `(0,15)`, preserved checks for
  `triggeronly:2`, `cellnot:1,15,web`, and `ratgone:0,15` returned no
  branches. Back up before B31 only if the hypothesis changes lower-left access.

### Continuation pass - 2026-06-12 chase solved

- `chase.csv` is now solved and recorded in `solver/solutions/final_solutions.json`,
  `solver/solutions/SOLUTIONS.md`, and the autoplay maps. The verified 199-move
  ASCII solution is:
  `^>>>v^^^>^^>>>>>v>>vvvvv^^^^^^<<<<v<<<v^<^<^<<>>>>v>>>>>vvvv^^^^<<<^<^<<^^^^^^^^<<<<<^^vvvvvvvvvvvvv>>>>>>>>vv^^<<vvvvv<>>>>>>>>>>>>>>^^v<>^^^^^^^^^^^^^^^^^^<<<<>>vvvv<vvvvvvvvvvvv<vvv<<<<<<<<<<<<<<<`.
- The useful human decomposition was not "chase every rat". It was:
  first use the early helper-rat/plank structure, then fire trigger 3 remotely
  from the top-left route before consuming trigger 4, then drop the reachable
  rats in sequence, and only then path to `(1,19)` facing west for the final
  rat. The critical intermediate after 161 turns has one rat left at `(0,19)`,
  no explosives, one web at `(1,19)`, and all remaining rat contact reachable.
  `solver wp levels/chase.csv --prefix <P161> --waypoints '1,19'` found the
  final mop-up suffix and `solver verify` returned `result=Won turns_applied=199`.
- All stale broad `chase` dropchain jobs were stopped after the verified win.
  Do not spend more search budget on `chase` unless upstream changes invalidate
  the recorded solution.

### Continuation pass - 2026-06-12 post-chase parallel audit

- `reload_v3`: the trigger-2-first branch
  `^>>>>>>>^^^vvv<<v<^` is a real 19-turn `triggeronly:2` state with all 3 rats
  alive and one reachable rat. It is better than the 18-turn variant, which
  strands all rats. However, bounded follow-ups from it found no preserved-rat
  branch to `triggeronly:1`, `triggeronly:3`, or a win. The trigger-1 branch
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<^<<<<<<<<<v<v>` remains the cleaner station
  after 42 turns: trigger 1 is spent, `(9,22)` is open, trigger 2 is reachable,
  and the roaming rat is still adjacent. But `wp` to trigger 2 spends that
  roaming rat, `lure` cannot carry it toward `(17,13)/(17,14)` with rat
  preservation, and branchdumps from this station found no preserved-rat
  `triggeronly:2`, `triggeronly:3`, or `triggeronly:6` branch. Treat both
  `2->...` and `1->2` as false milestones unless the next idea changes the
  roaming-rat position before the trigger is fired.
- `cooperation/blocked_v2`: a fresh trigger-4 mechanism improves the old B17
  frontier. From
  `^< ^^ v^ ^^ v^ ^^ v^ ^> v> ^> vv ^v v^ ^v vv ^v vv`, firing trigger 4 via
  suffix `^v v> ^> v< v>` reaches a 4-rat state; one more move `^<` reaches a
  3-rat state:
  `^< ^^ v^ ^^ v^ ^^ v^ ^> v> ^> vv ^v v^ ^v vv ^v vv ^v v> ^> v< v> ^<`.
  Diagnostics there: rats `(9,13)`, `(9,14)`, `(0,15)`, reachable rats `2/3`,
  triggers 5 and 2 still present, but trigger 2 is unreachable. Enlarged
  follow-ups from this B23 state found no `win`, no `ratsle:2`, no
  `triggeronly:2`, and no changes to `(1,15)` web or `(2,15)/(3,15)`
  explosives. Trigger 5 is reachable but only burns the trigger-5 family down
  to two triggers while leaving all 3 rats and the lower-left boundary intact.
  B23 is a better diagnostic basin, not a solved route.
- `tinderrectangle`: independent audit confirmed the P135/P160 lower-release
  family remains a contact trap. The useful buffer
  `PBUF = P135^^^^<<vvv<<^^^<<<v<<` verifies with all 16 rats alive, player
  `(5,4)`, and lower rat `(2,3)`, but it cannot open `(3,4)` safely or reach
  `rectsep`. Opening `(2,4)` from P135 while preserving all rats lands with
  player `(2,4)` and lower rat `(2,3)`; `rectsep` and `win` from that contact
  state returned no branch. The next hypothesis must change release geometry
  before `(2,4)` opens, not extend the same buffer.
- `release` / `cyborg_rats/ai_takeover`: these are now clearly different
  blockers. In `release`, after `v<vv^^>>v`, rat `(18,4)` is physically isolated
  behind web `(18,5)`; the useful trigger must be left trigger 2 `(0,16)`,
  because right trigger 2 zaps the wrong side. In `ai_takeover`, `(18,5)` is
  already open and the blocker is trigger-7 timing: the known route steps
  through and destroys `(16,8)` before firing trigger 7, then trigger 7 walls
  `(16,8)` behind the player and collapses reachability to a one-cell pocket.
  The next `ai_takeover` target is a non-collapsing trigger-7 event such as
  `triggeropen:7,50` from a route that does not first vacate/destroy `(16,8)`.

### Continuation pass - 2026-06-12 post-status targeted probes

- `cyborg_rats/ai_takeover`: the known 128-turn pre-trigger-7 state
  `v>v^>>v^<<vv^^<<vvv<<^^^<<<vv<<^^^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^vv^^^>^^^^^>>>^>>>>>>>>>>>vvv>vvv`
  is a precise trap, not just a slow search basin. Diagnostics before firing 7
  have player `(16,8)`, rats `(16,4)`, `(18,4)`, `(15,5)`, `(16,5)`,
  `(16,6)`, and player reachability `265` cells. Appending `v` fires trigger 7
  and leaves the player at `(16,9)` with reachability `1` and all 5 rats
  unreachable. Bounded branchdumps from that state for `triggeropen:7,50` and
  `triggeronlycellnot:7,16,8,wall` returned no branches. A separate south-access
  batch from the initial board also found no branch, with at least 5 rats
  preserved, for `cellnot:15,10,web`, `cellnot:16,12,web`,
  `cellnotratat:15,10,web,15,9`, or `cellnotratat:16,12,web,16,11`. The next
  `ai_takeover` idea needs an earlier top-release/trigger-6/trigger-8 mechanism;
  do not spend more time on one- or two-move wiggles at trigger 7.
- `reload_v3`: a better trigger-2 choreography exists:
  `vvv<<<<<<vvv><^^^>>>>>>>>>>>>>^^^^^v<<v<^`. It verifies as `Playing` at 41
  turns, fires trigger 2 with all 3 rats alive, keeps one rat reachable, and has
  already cleared bottom web `(8,22)`. Diagnostics: player `(17,15)`, rats
  `(14,5)`, `(17,14)`, `(0,21)`, `webs=15`, `triggers=12`. Follow-up probes
  from this prefix for preserved-rat `triggeronly:1`, `triggeronly:3`,
  `triggeronly:6`, and `cellnot:2,21,explosive` returned no branch. This is a
  better staging prefix than the 19-turn trigger-2 false milestone, but it still
  needs a rat-position change before the next irreversible event.
- `release`: the alternate opener `v<vv^^>>>>v` is verified `Playing` and is
  materially different from `v<vv^^>>v`: it fires trigger 5 before trigger 4 and
  leaves 23 rats, 5 explosives, 27 webs, 7 triggers, and `21/23` reachable rats.
  However, direct structural checks from the standard and alternate trigger
  families still found no way to open `(18,5)`, make left trigger 2 `(0,16)`
  reachable, break `(16,17)`, move a runner to `(17,19)`, or continue
  trigger-order `3,5,6` / `3,4,6` while preserving a useful rat. The staged
  `(2,16)` actor is real but is removed by every checked next action.
- `cooperation/blocked_v2`: higher-budget checks tightened the B17/B23 blocker.
  B17 still has 8 rats, 6 reachable, and trigger 2 cells `(5,11)` / `(3,14)`
  unreachable. B23 still has rats `(9,13)`, `(9,14)`, `(0,15)` with 2 reachable,
  and the lower-left pocket remains sealed by `(1,15)` web,
  `(2,15)/(3,15)` explosives, and `(4,15)` web. From both B17 and B23, waypoint
  probes to `. | 5,11` and `. | 3,14` were unreachable. Structural checks for
  `triggeronly:2`, `triggeropen:2,20`, `triggeronlycellnot:2,1,15,web`,
  `cellnot:1,15,web`, `cellnot:2,15,explosive`,
  `cellnot:3,15,explosive`, `ratgone:0,15`, and resource-gated `ratsle:2`
  returned no branches. A 100s A* lookup from B17 found only a worse 29-turn
  continuation with 3 rats and zero reachable triggers.
- `reload_v3`: an event-macro run from the new T2 prefix found the best current
  frontier, a verified 100-turn 1-rat state:
  `vvv<<<<<<vvv><^^^>>>>>>>>>>>>>^^^^^v<<v<^^>>^^^>^^^^^<<<<vv<<<<<>>>>>^^^^^^<<<<<<<<<vvvvv<vvvv>>>>>>`.
  Diagnostics: player `(12,11)`, final rat `(0,21)`, 5 explosives, 8 webs, 12
  triggers, `player_reachable cells=364`, but `reachable_rats=0/1` and only
  triggers 7 are reachable. Firing trigger 7 is possible, but it dead-ends:
  one representative branch
  `vvvvvvvv<<<<<vv<<<<^>>^>^^^^^^^^^^^>` leaves player `(7,8)`, final rat
  `(0,21)`, 7 webs, 10 triggers, and `player_reachable triggers=0/10`.
  Follow-up branchdumps from the 100-turn state found no direct win, no useful
  `triggeronly:6`, no removal of `(1,21)` web or `(2,21)` explosive, and no
  1-rat frontier with the final rat reachable or trigger 1 reachable. This is a
  real advance, but the next idea must change the final pocket before or during
  the macro route; firing 7 after the macro route is the wrong last event.

### Continuation pass - 2026-06-12 active parallel batch

- Solver tooling now supports compound lookup goals
  `ratslecellis:n,x,y,kind` and `ratslecellnot:n,x,y,kind`. These are
  diagnostic-only goals for mechanism searches where both cleanup and a blocker
  change must be true in the same accepted state. They do not change game rules.
- A parallel batch was started under `/tmp/infestation-runs` with long-running
  bounded `branchdump` jobs for:
  - `reload_v3`: from the T2 prefix, require 1-rat cleanup plus `(1,21)` web or
    `(2,21)` explosive changed, with a reachable rat and reachable trigger.
  - `blocked_v2`: from B17/B23, require 2-rat cleanup plus `(1,15)` web changed
    while preserving reachability/resources.
  - `cyborg_rats/ai_takeover`: from the initial board, search for a
    non-collapsing trigger-7 event (`triggeropen:7,50` or
    `triggeronlycellnot:7,16,8,wall`).
  - `release` and `tinderrectangle`: older long probes remain active for the
    alternate left-trigger-2 opener and early `(3,4)` rectangle separation.
  - `handoff`: initial-board search for firing trigger 2 without making
    `(11,7)` a wall.
- `cooperation/tug_of_war` has a new verified trigger-2 frontier after the
  trigger-1 setup:
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v ^^ <^ <^ <^ v^ v^ v^ v^ v^ <^ v^`
  verifies as `Playing` at 22 turns. Diagnostics: players `(0,0)` and `(10,16)`;
  rats `(7,0)`, `(4,8)`, `(5,14)`; 0 explosives; 7 webs; 5 trigger-3 cells
  reachable; `reachable_rats=1/3`. This is progress over the previous
  "trigger 2 unreachable" boundary.
- Immediate `tug_of_war` follow-up is still a trap. Firing the nearest
  lower-right trigger 3 with `>.` leaves all 3 rats unreachable and zero
  triggers. The reachable lower rat can be lured to `(5,13)` with suffix
  `^^ ^^ ^^`, but quick checks did not produce a `ratsle:2` cleanup or side
  placement at `(5,15)` / `(6,14)`. Long probes for trigger-3 timing and opening
  the top `(7,1)` web remain active.
- `cyborg_rats/ai_takeover` has a new safe trigger-7 family from the initial
  board. Representative verified frontier:
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>^>^`
  verifies as `Playing` at 44 turns. It fires trigger 7 without the old
  `(16,8)` wall trap, leaving 14 rats, 6 explosives, 28 webs, 7 triggers, and
  `reachable_rats=13/14`; player is at `(16,9)`. Appending `^` gives another
  live 45-turn frontier at `(16,8)` with one fewer web:
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>^>^^`.
  This disproves the earlier assumption that trigger 7 itself is always fatal.
- Follow-up `ai_takeover` checks from the safe trigger-7 frontier found no
  direct branch to trigger 6, trigger 8, trigger 2, or `ratsle:10`, and strict
  event-order probes `6,8,2`, `8,6,2`, `6,2,8`, plus `triganylookup` and
  `macro`, returned no solution. Waypoint checks to trigger cells `(17,16)`,
  `(19,18)`, and `(18,19)` from the 45-turn state were dynamically
  unreachable, even though static diagnostics list those cells in the reachable
  component. The next `ai_takeover` search should treat the 44/45-turn safe-7
  line as a staging mechanism and look for a route-shaping move before trying
  6/8, not repeat direct trigger-order searches.
- Latest `ai_takeover` progress: the lower trigger-7 route is now the best
  constructive lead. Verified prefix
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvvv<`
  reaches a live 60-turn post-trigger-8 state with 16 rats, 5 explosives, and
  trigger 2 still present. A better pre-trigger-2 drain is
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvvv<<<<<<<<<<<<<<<^^^v>>>>>>>>>><<<<<<<<`
  (`J`), verified `Playing` at 96 turns with 6 rats, 5 explosives, 26 webs,
  trigger 2 still present, and 5/6 rats reachable.
- From `J`, targeted branchdumps for the bottom-row cyborg cluster found
  verified 3-rat trigger-preserving frontiers. The shortest useful one is
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvvv<<<<<<<<<<<<<<<^^^v>>>>>>>>>><<<<<<<<>>>>`,
  verified `Playing` at 100 turns. Diagnostics: player `(10,19)`, rats
  `(18,4)`, `(11,9)`, `(12,19)`, 5 explosives, 26 webs, 2 triggers, and 2/3
  rats reachable. A more open variant is
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvvv<<<<<<<<<<<<<<<^^^v>>>>>>>>>><<<<<<<<<<<^^^v>>>>>><<<<`,
  verified `Playing` at 113 turns with rats `(18,4)`, `(11,9)`, `(3,17)` and
  25 webs.
- Firing left trigger 2 from the 100-turn 3-rat state with suffix
  `<<<<<<<^^^<<<` is live and changes the shape: full prefix
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvvv<<<<<<<<<<<<<<<^^^v>>>>>>>>>><<<<<<<<>>>><<<<<<<^^^<<<`
  verifies `Playing` at 113 turns with all 3 rats reachable:
  rats `(18,4)`, `(11,9)`, `(2,16)`, player `(0,16)`, 2 explosives, 25 webs,
  and no triggers. This is much better than the old post-trigger-2 basin, but
  no cleanup is known yet.
- Failed continuations from that post-trigger 3-rat state: `events` reports
  `NO_EVENTS`, `lookup --goal win` returns `NO_SOLUTION` quickly, and
  `branchdump --goal ratsle:2` found no branch in the tested depth/budget.
  Immediate tactical probes show `>` is safe but `>>` dies; `v`, `<`, `.>`,
  and `..>` are safe displacement moves. A `ratgone:2,16` result with suffix
  `v` is only coordinate movement, not a kill. The next useful attack is a
  hand-modeled cleanup/lure for the local `(2,16)` cyborg or a route that uses
  the two remaining explosives at `(14,12)` / `(15,12)` before the local rat
  becomes a contact trap.

### Status check - 2026-06-12 22:45Z

- Re-fetched remotes and re-ran the current `final_solutions.json` through this
  checkout's `target/release/solver`; all 28 recorded solutions still verify
  as `result=Won`. An earlier failed sweep in `/tmp/infestation-runs` used the
  wrong JSON key and only replayed empty strings; ignore it.
- No new `Won` candidate was found in this pass.
- `release`: the latest post-trigger-5 branch
  `v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>><<^^vv>` verifies as `Playing` at
  44 turns with the lower actor at `(12,17)`, but bounded preserved-rat
  branchdumps found no branch to `ratat:14,17`, exact
  `ratplayer:14,17,16,14`, `triggeronly:6`, or `cellnot:18,5,web`. This makes
  the trigger-5/row-17 actor family look like a staging loop rather than the
  intended route. Next `release` work should back up before trigger 5 and find
  a mechanism for `(18,4)` / `(18,5)`, not keep pushing trigger 6 from this
  family.
- `cyborg_rats/ai_takeover`: from the 113-turn all-reachable 3-rat post-trigger
  state, appending `>` is live and shifts the local cyborg `(2,16)->(2,17)` and
  the far cyborg `(18,4)->(18,5)`. Follow-up branchdumps from that shifted state
  found no `ratsle:2` cleanup and no detonation of `(14,12)` or `(15,12)`.
  The only `ratgone:2,17` branch is suffix `<`, which just moves the local
  cyborg back to `(2,16)` while keeping 3 rats, so it is not progress.
- `reload_v3`: from the verified 48-turn `2 -> 1` state
  `^>>>>>>>^vvvvvv<<<^^^<<^^<<<<<<<<<^vvvvv<<<^^<^>`, bounded branchdumps found
  no preserved-rat branch to change `(2,21)` explosive, change `(1,21)` web, or
  station the player at `(3,21)`. From the 19-turn trigger-2 state, preserving
  all 3 rats while changing `(3,21)` web also returned empty.

### Status check - 2026-06-12 23:15Z

- Fetched `fork` again; the branch matched `fork/claude/new-puzzles` before this
  pass. After adding `old_levels/old_levels.csv`, a fresh sweep of
  `solver/solutions/final_solutions.json` verifies all 29 recorded original /
  non-Claude solutions as `result=Won` on this checkout.
- No new hard puzzle has a verified `result=Won`.
- The portal-linked old-level hub `old_levels/old_levels.csv` is now recorded in
  the solution artifacts. It verifies with `vvvv<<<<<` (`result=Won`,
  9 turns). The three child old levels remain intentionally labeled broken in
  their hub metadata and quick current-oracle solves did not find wins.
- `cooperation/blocked_v2`: found a better all-rat-preserving trigger-3
  frontier:
  `^< ^^ v^ ^^ vv ^v v^ ^^ v> v> v> vv vv ^v ^v ^v ^v`.
  It verifies as `Playing` at 17 turns with 8 rats, 15 explosives, 11 webs,
  9 triggers, and `reachable_rats=6/8`. Follow-up checks from that state for
  `reachable:0,15`, `reachable:14,19`, `trigger:2`, `trigger:5`,
  `cellnot:10,13,web`, and `cellnot:10,14,web` returned no branch in the tested
  budgets, so it is a real frontier but not yet an access solution.
- `cooperation/tug_of_war`: a late trigger-3 branch from the 22-turn frontier
  was found and verified:
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v ^^ <^ <^ <^ v^ v^ v^ v^ v^ <^ v^ ^v ^v ^v ^v ^v ^v ^v ^v ^v ^v ^v ^v ^v vv vv vv v> v> v> >>`.
  It is not useful: it leaves 3 rats, 0 explosives, 0 triggers, and only
  `reachable_rats=1/3`.
- `cyborg_rats/ai_takeover`: the 113-turn 3-rat cleanup state was checked more
  tightly. `playerat:3,17`, 18-hold detonation of `(14,12)`, 18-hold removal of
  `(11,9)`, and 45-hold `ratsle:2` all returned empty. The 45-hold `v>` trace
  explains the near miss: it drops to one rat only by killing the player in the
  black-hole row.
- `release`: backing up to the 24-turn lower-actor state did not produce a
  branch for `cellnot:1,16,explosive` or
  `triggeronlycellnot:6,1,16,explosive`; backing up to 23 turns did not stage
  `ratplayer:5,17,16,14`. This weakens the hypothesis that the lower actor can
  be preserved through trigger 6 by simply undoing the final actor-killing move.
- `tinderrectangle`: from the fresh separated state `<<^<<^<>^>>>vv^^<<`,
  preserving the lower rat at `(5,5)` while moving the player to `(13,7)` or
  `(14,7)`, and opening `(13,2)` while preserving that rat, returned no branch.

### Status check - 2026-06-12 23:45Z

- Rechecked the upstream-rule concern. This checkout only has `fork` configured
  as a remote, but a stale local `upstream/ai-solutions` ref contains rule/source
  differences and an encoded-solution verifier. Exporting that branch into
  `/tmp`, building `verify_solution`, converting the pushed `SOLUTIONS.md`, and
  replaying all 34 encoded solutions there produced `Won` for every recorded
  solution, including `old_levels/old_levels`. The recorded wins are therefore
  still valid under that branch's game code.
- No new hard puzzle has a verified `result=Won`. All solver processes from this
  wave finished; no `target/release/solver` processes were left active at this
  checkpoint.
- Long continuations from the previous batch finished with `NO_SOLUTION`:
  `tinderrectangle` from `<<^<<^<>^>>>vv^^<<`, `release` from
  `v<vv^^>>vv><<v<<<^<^^<<>`, `blocked_v2` from the B17 trigger-3 frontier, and
  `tug_of_war` from the 22-turn frontier. Their best states did not produce a
  win candidate.
- `cooperation/handoff` / `release` branchdumps from the 23:28Z batch ended
  with no accepted branches for the tested resource-preserving goals. The
  `ai_takeover` safe-trigger-7 route-shaping probes also ended empty for direct
  trigger-6/8 opening, right-column detonation, `(17,19)` web opening, reachable
  staging cells near 6/8, and moving the `(18,4)` rat to `(18,5)`.
- A k=2 novelty-search lane was run for `release`, `ai_takeover`, `handoff`,
  `tug_of_war`, `blocked_v2`, and `tinderrectangle`. All returned
  `NO_SOLUTION` quickly; treat this as a cheap negative portfolio result, not a
  proof of unsolvability.
- `tinderrectangle`: a first `cellnotratat:3,4,web,5,5` probe was a bad goal:
  `(3,4)` was already open, so the returned branches were trivial. Corrected
  lower-door checks from `<<^<<^<>^>>>vv^^<<` for `(5,6)`, `(4,6)`, and `(3,6)`
  opening while keeping the lower rat at `(5,5)` returned no branches; `rectsep`
  from the same prefix also returned no branch. The lower release still needs a
  different timing/setup, not a direct cut from this separated state.
- `reload_v3`: from staged prefix `^^^^^<<<<<<^^<^^^^^vv`, branchdumps found
  real but likely dead diagnostic branches. One opens `(3,21)` by turn 83:
  `^^^^^<<<<<<^^<^^^^^vv^^^^>>>>>>>>>>vvvvvv<<>>^^^^^^<<<<<<<<<vvvvv<vvvvvvvvvvvvvv<<<`.
  It verifies `Playing` with rats `(19,2)`, `(11,5)`, `(0,21)`, 6 explosives,
  13 webs, all triggers preserved, and only `reachable_rats=1/3`; follow-up
  `trigger:6` from that state returned no branch. Another fires trigger 7 by
  turn 71:
  `^^^^^<<<<<<^^<^^^^^vvvv>^^^^^^>>>>>>>>>vvvvvv<<<<<>>>>>^^>>>>vvvvvvvv<<`.
  It leaves trigger 2 nearby but still only one reachable rat; strict trigger-2
  and compound lower-left cleanup checks from that state returned no branches.

### Continuation pass - 2026-06-13 bounded mechanism wave

No new verified wins. Solution artifacts were intentionally left unchanged.
All live solver processes from the earlier high-memory wave were stopped first;
the long bounded wave ran under `/tmp/infestation-runs/20260613T005515Z_longbounded2`
with per-process virtual-memory caps and left no solver process running.

- Current artifact inventory still shows 7 non-hub hard unsolved levels:
  `tinderrectangle.csv`, `release.csv`, `reload_v3.csv`,
  `cyborg_rats/ai_takeover.csv`, `cooperation/tug_of_war.csv`,
  `cooperation/handoff.csv`, and `cooperation/blocked_v2.csv`. `chase.csv`,
  `old_levels/old_levels.csv`, and `tinderbox_v2.csv` all still verify as
  `result=Won` in this checkout.
- `release`: the corrected dependency is left trigger 2 `(0,16)`, not right
  trigger 2 `(19,7)`. Triggering left 2 leaves right 2 as the sibling zap
  source for the `(18,6..8)` explosive stack that can clear `(18,5)`. A
  900-second capped lookup from `v<vv^^>>v` for the concrete carrier setup
  `ratplayer:18,17,17,0` returned `NO_SOLUTION`; the best state got the player
  to `(17,0)` but did not stage a carrier near `(18,17)`.
- `blocked_v2`: terrain reasoning says the useful sequence is upper trigger 3
  at `(13,4)` followed by trigger 2 at `(5,11)`, but an oracle lookup from B9
  for `triggeronlycellnot:3,6,11,explosive` with at least 7 rats returned
  `NO_SOLUTION`. The accessible upper-3 route appears to require trigger 5 and
  upper-pocket rat loss, then still strands access to useful trigger 2.
- `handoff`: H15 is a useful timing frontier, but the 900-second
  resource-preserving branchdump for `playerat:14,7` returned no branch. The
  local `(10,6)->(11,7)` rat geometry needs a far-right lure before trigger 2
  turns `(11,7)` into a wall; no trigger-preserving route to that lure was
  found.
- `tug_of_war`: the pre-trigger frontier
  `.< v^ <^ >^ <^ <^ >^ <^` still leaves the top rat sealed. A long
  resource-preserving branchdump for `cellnot:7,2,plank` returned no branch,
  reinforcing that the top pocket is a structural blocker, not a trigger-order
  cleanup issue.
- `tinderrectangle`: P135/P160-style release remains a spacing problem. A
  long branchdump for `ratplayer:3,6,4,6` from P135 returned no branch. The
  only local pre-release move found by `ratgeom` is `(2,3)->(1,4)` with the
  player at `(1,5)/(1,6)`; moving toward the useful row-6 side loop requires
  the player to already have spacing that current routes cannot realize.
- `reload_v3`: a low-beam macro from the 41-turn trigger-2 prefix found a
  shorter one-rat diagnostic state,
  `vvv<<<<<<vvv><^^^>>>>>>>>>>>>>^^^^^v<<v<^` +
  `^<<^^^<<<<<<<<^^<^^^^^vvvvvvv>>>>>>`, verified `Playing` at 76 turns.
  It is the known dead basin in smaller form: only rat `(0,21)` remains,
  `(1,21)` is still web, `(2,21)` is still explosive, and only trigger 7 is
  reachable.
- `cyborg_rats/ai_takeover`: from the safe trigger-7 frontier at 45 turns,
  longer `triggeropen:8,50` and low-beam macro checks produced no branches or
  solution. Continue from a new structural event before safe-7, not from direct
  trigger-6/8 ordering.

### Continuation pass - 2026-06-13 careful bounded follow-up

No new verified wins. Solution artifacts are still unchanged. This pass kept
solver concurrency low, used only capped 1M-2M node probes, and ended with no
solver processes running. Reverification of `solver/solutions/final_solutions.json`
still reports 29 verified wins and 0 failures under the current Rust oracle.

- `release`: a local `ratgeom` clue shows that if a rat is staged at `(18,17)`
  while the player is at `(19,19)`, the rat can trigger right-side `6` at
  `(19,18)`. This would leave left `6` as a zap source and could clear the
  `(0,17)` / `(1,16)` blocker before left trigger 2. However, from the standard
  opener `v<vv^^>>v`, bounded lookups for `ratat:18,17`, `ratat:19,18`, and the
  compound `ratplayer:18,17,19,19` all returned no solution. Treat right-6 as a
  valid final nudge but not a staged plan from the known opener.
- `reload_v3`: the all-rat 42-turn prefix
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<<<^^^^^^<<^^^` does make trigger 2 reachable,
  but it is another trap. `triglookup` orders `2,1,3,4,5,6` and `2,3,4,5,6`
  both collapse to a one-rat bottom-left basin before any next trigger can
  continue. Stricter branchdumps requiring trigger 1, 3, or 4 to be reachable
  immediately after `triggeronly:2` all returned empty while preserving all
  three rats.
- `tinderrectangle`: the near-miss state with lower rat `(6,6)` / player
  `(7,6)` is still only one move from the ignition but has no safe separation
  move. Moving left sacrifices the lower rat and leaves a 15-rat top-only state,
  but follow-up `win`, `rectready`, and top-corner `ratat:0,0` / `ratat:16,0`
  probes from that state returned no branch. The solution still needs separation
  before the lower rat reaches row 6, not a post-contact recovery.
- `cyborg_rats/ai_takeover`: from the A45 safe-7 frontier, branchdumps for
  `triggeronly:8`, `triggeronlycellnot:8,2,19,explosive`, and `triggeronly:6`
  returned empty with rat preservation. `wp` and `lookup` also report the right
  trigger-8 cell `(18,19)` and right trigger-6 cell `(19,18)` dynamically
  unreachable from A45. Do not continue direct trigger 6/8 from that frontier.
- `cooperation/handoff`: exact local geometry for the sealed `(10,6)` rat says
  `(10,6)->(11,7)` is possible if P2 is staged at `(14,8)`. `wp2` can reach
  P2 `(14,8)`, but only after consuming all explosives/triggers and leaving
  `(10,6)` in a size-1 component behind web `(10,5)`. All-rats-preserved
  branchdumps for the same far-lure target returned empty. The far lure has to
  happen before the common resource-spending sweep, and no such route is known.
- `cooperation/tug_of_war`: a bounded independent pass found no top-pocket
  release. Direct checks for helper-rat `(7,5)->(7,4)`, breaking `(7,2)` /
  `(8,2)`, clearing `(7,1)` / `(8,1)`, or combining those with the top rat
  still alive all returned empty under 1.2M-2M node caps. The top rat remains
  the structural blocker.

### Continuation pass - 2026-06-13 all-clean follow-up

No new verified wins. The raw CSV scan shows 10 missing solution entries, but
three are the child files inside `levels/old_levels/`; their hub metadata labels
them broken and the portal-linked hub `old_levels/old_levels.csv` is already a
verified win. The current hard working set remains the 7 listed above.

- `cooperation/blocked_v2`: rechecked the all-rat frontiers instead of the
  low-rat basins. From the 17-turn trigger-3 state
  `^< ^^ v^ ^^ vv ^v v^ ^^ v> v> v> vv vv ^v ^v ^v ^v`, trigger 5 does not
  produce a useful continuation: requiring trigger 2 to remain reachable after
  `triggeronly:5` returned no branch with either 8 rats or a relaxed 7-rat
  floor, and the compound `triggeronlycellnot:5,2,15,explosive` was also empty.
  Backing up to the 16-turn state
  `^< ^^ v^ ^^ v^ ^^ v^ ^> v> ^> vv ^v v^ ^v vv ^v`, `triggeronly:3` itself is
  easy and preserves all 8 rats, but requiring trigger 2 to be reachable after
  it returned empty, and `triggeronlycellnot:3,6,11,explosive` also returned
  empty. This rules out the current `3 -> 5 -> 2` access story from those
  frontiers.
- `tinderrectangle`: from prepared-safe P71
  `<^^^>>v>vv>>^^^>>vvvv^^^^><<<vvv<<^^^<<<<vvv<<<^>^>>>^>>v>vv>>^^^>>vvvv`,
  individual safe-side compound goals (`ratplayer` for lower rat
  `(2,4)/(2,5)/(2,6)/(4,6)/(6,6)` with player `(14,7)` and row-8 safe cells)
  returned no solution in capped checks. A native `rectlower` lookup from P71
  found a sharper best diagnostic suffix
  `^^^^<<vvv<<^^^<<<<<vvv<<<<>`, giving P98 with rat `(1,5)` and player `(2,6)`.
  From P98, every one-step move except `>` is `GameOver`; `>` reaches the known
  contact trap with rat `(2,6)` and player `(3,6)`. The player can escape back
  to `(14,7)` only if rat preservation is dropped, yielding P125 with 15 rats
  and the lower rat gone. A no-preservation `rectlower` lookup from the contact
  trap still returned no solution. This is stronger evidence that the lower
  route needs separation before P98, not a recovery after contact.

### Continuation pass - 2026-06-13 resource-capped mechanism probes

No new hard-level win was found. The pass kept solver concurrency to at most two
short searches at a time, with 20s-60s time caps and 200k-1M node caps, and
ended with no `solver`/`timeout`/`clingo` processes running.

- Solution artifact cleanup: the five `levels/claude/*` child levels already
  listed in `SOLUTIONS.md` and autoplay artifacts were missing from
  `solver/solutions/final_solutions.json`. They were reverified with
  `target/release/solver verify` and added:
  `claude/sacrifice.csv` (`<>`), `claude/roach_motel.csv` (`^^`),
  `claude/stampede.csv` (`^^^`), `claude/remote_detonator.csv` (`^<<<<v`), and
  `claude/web_lair.csv` (`^^vvvv`). These are not new hard-puzzle discoveries;
  they are verified artifact sync.
- Solver tooling: `lookup` now supports `ratfar:x,y,d`, accepting a state where
  a rat is at `(x,y)` and the nearest player is at least Manhattan distance `d`
  away. This is diagnostic-only and was added to test separation states without
  over-specifying the exact player square.
- `tinderrectangle`: P71 separation probes for `ratfar:2,6,3`,
  `ratfar:4,6,3`, and `ratfar:6,6,3` all returned `NO_SOLUTION` while
  preserving all 16 rats. This reinforces the P71 conclusion: its natural
  release is the doomed left-shaft timing, not a recoverable lower-row
  separation.
- `tinderrectangle`: the short lower-rat family
  `<<^<<^<>^>>>vv^` / `<<^<<^<>^>>>vv^.` was checked as a separate mechanism.
  Diagnostics show the lower rat at `(5,5)` or `(6,5)` with all 16 rats alive,
  but local one-step checks show a timing trap: from the dotted state only `^`
  survives, and it pulls the lower rat up to `(7,4)`; attempts to stage
  `(6,5)` with player `(8,6)` / `(10,6)`, or lure `(7,6)` with safe row-6
  cells, returned immediate `NO_SOLUTION`. The useful target is not
  `ratplayer:7,6,14,*` from this exact prefix unless the pre-stall timing is
  changed.
- `release`: an independent mechanism audit reconfirmed the likely dependency:
  left trigger 2 must be made usable while a left-side actor survives, because
  firing left 2 leaves right 2 as the zap source for the `(18,6..8)` explosive
  stack that can open `(18,5)`. Short capped probes from the near-miss
  `v<vv^^>>v^vvv<<<<^<^^<<><` for `ratcell:2,16,1,16,empty`,
  `triggeronlycellnot:6,1,16,explosive` with trigger 2 reachable, and from
  opener `v<vv^^>>v` for `cellnotratat:1,16,explosive,2,16` all missed. Best
  states got a rat near `(2,16)` but left `(1,16)` explosive intact.
- `reload_v3`: constrained trigger-order checks with
  `--min-reachable-rats 1 --min-reachable-triggers 1` reject the naive paper
  chain sooner. Strict `1,2,3,4,5,6` failed at trigger 1 under the reachability
  gate. A strict `2,1,3,4,5,6` check was also attempted under a 120s wrapper and
  stopped without a usable branch; process cleanup was verified afterward.

### Continuation pass - 2026-06-13 `world.csv` solved

`world.csv` now has an oracle-verified win and has been added to the solution
artifacts. The final replay is:

```sh
target/release/solver verify levels/world.csv '^^ ^< ^> ^< ^< ^< ^v ^< ^> <^ <^ v^ v^ v^ v^ v^ v^ <> <v v^ v^ v^ ^^ ^v ^< >> ^< ^< ^^ ^< ^< ^v >^ >< ^< ^< <v ^< ^^ v^ v^ >^ v^ v^ >^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ ^< >> >< >^ >v v< v< v^ <^ >< ^^ ^> ^v << << << <v ^> ^> ^< ^v ^> ^> ^^ ^< >> >< ^< ^^ v< v^ v^ v^ >^ >^ <v << ^< ^> >v >v ^^ ^^ vv vv <^ <v <^ <^ <> << << <^ ^> ^^ v^ v> >> >^ >> >> ^^ ^< ^< ^< << << << <^ >v ^^ ^^ ^v ^v ^^ ^v ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ v< v> v^ >v >v ^< ^< ^^ v< v> >^ >< ^< ^^ v< v> >^ >< ^^ ^^ v< v> << v^ v< v< v< v< v< >v >. v> v^ ^v ^v <^ <^ v^ v^ v^ v^ <^ v^ ^^ vv ^> <^ << v< v^'
# result=Won turns_applied=192
```

Mechanism notes:

- The direct `world.csv` solve repeatedly converged to a two-rat residue:
  top-left `(1,0)` plus one bottom rat. The missing human step was to clear the
  bottom-left and bottom-right rats before letting the solver run north.
- A 22-turn prefix moved/removes the initially isolated `(2,30)` rat. A later
  staged prefix at 65 turns removes bottom-right `(6,30)/(6,31)`, leaving all
  remaining rats reachable.
- Greedy count descent then exposed a dead trigger-1 lockout at seven rats, so
  the final solution backs up and uses explicit top-web and final-pair cleanup
  instead of accepting that dead seven-rat state.

After this pass, the raw missing non-old CSVs are the seven hard levels plus
`claude/gauntlet.csv`. `claude/gauntlet.csv` is a zero-rat portal hub, and the
game rule intentionally reports zero-rat levels as `Won` only if the level
started with rats. Keep it out of the oracle-verified solution artifacts.

### Continuation pass - 2026-06-13 post-world sidecar audit

No additional hard-level `result=Won` was found. The branch was already pushed
with the verified `world.csv` solution as commit `c5e59a8`. This pass kept
solver concurrency low, used bounded sidecar probes, and ended with no
`solver`, `timeout`, or `clingo` processes running.

- `tinderrectangle`: the P113 `ratplayerfacing` lead is now a false positive.
  From the 113-turn cut point, the suffix `^` reaches
  `ratplayerfacing:1,4,1,5,north`, but it is only a sword-pin trap: `<` and
  `>` are immediate `GameOver`, `^` kills the lower rat, `v` returns to
  adjacent contact, and `.` loops. Follow-up `ratfar:1,5,3`,
  `ratfar:2,6,3`, `ratfar:6,6,3`, and `rectsep` from that cut point all
  returned `NO_SOLUTION` in capped checks.
- `tinderrectangle`: P135/PBUF right-side release is also a suicide release.
  At `PBUF = P135^^^^<<vvv<<^^^<<<v<<`, diagnostics show player `(5,4)`,
  lower rat `(2,3)`, and all 16 rats alive. `ratgeom` still only finds the
  local `(2,3)->(1,4)` geometry with player `(1,5)/(1,6)`, and no safe
  `(2,3)->(3,4)`, `(2,4)`, or row-5 release. The actual `PBUF <<` opens
  `(3,4)`, moves the lower rat to `(3,4)`, and is `GameOver`. Do not extend the
  P113/P135/PBUF family without a different pre-release geometry.
- `release`: the local right-side trigger-6 carrier nudge is real but not
  reachable from the standard opener. `ratgeom` confirms a synthetic rat at
  `(18,17)` can step onto right trigger 6 `(19,18)` with player `(19,19)`, but
  bounded checks from `v<vv^^>>v` found no route to `ratplayer:18,17,19,19`,
  no waypoint route to any trigger-6 cell, and no `triggeronlycellnot:2,18,5,web`
  branch with at least 20 rats. The missing mechanism is staging the carrier
  before the top sweep, not pushing trigger 5/6 after the opener.
- `reload_v3`: direct trigger-2 progress remains a trap. From
  `vvv<<<<<<vv<<<<`, appending `<` detonates `(2,21)` and opens the lower-left
  pocket, but the oracle reports `GameOver`. From the 42-turn all-rat
  trigger-2 staging prefix
  `^>>>>>>>^^vvvvvv<<<^^<<^^<<<<<<^^^^^^<<^^^`, `wp` can reach trigger 2, but
  follow-up search again falls into the dead `(0,21)` basin. A gated
  `triggeronly:2` branch requiring trigger 1 to remain reachable found no
  accepted branch. Trigger 2 being reachable is insufficient; the route must
  remote-open the lower-left gate or preserve a next trigger station during the
  trigger-2 event.
- `cyborg_rats/ai_takeover`: the 113-turn all-reachable 3-rat post-trigger
  state is a dead cleanup basin. Direct `cont` from that prefix returns
  `NO_SOLUTION` immediately, bounded branchdumps found no `ratsle:2` branch and
  no way to change explosives `(14,12)` or `(15,12)`, and the only local
  geometry around rat `(2,16)` does not become a mop-up. Do not spend down to
  the no-trigger 3-rat state; the solution needs an earlier structural event
  before trigger 2 is consumed, likely before or during the safe trigger-7/8
  chain.
- `cooperation/handoff`: a survivable trigger-1 route
  `v^ >^ >^ >^ >^ >^ ^^ v^ ^^ ^^ v^ v^ vv <v` followed by trigger 2 via
  `^^ ^^ ^^ ^<` spends both trigger systems but leaves rats `(10,6)` and
  `(1,7)`, zero explosives, zero triggers, and `reachable_rats=0`. This is a
  sharper blocker: the level can consume both trigger systems, but that route
  has no remaining mechanism to affect either rat.
- `cooperation/tug_of_war`: trigger 1 remains the only promising first lever.
  Prefix `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v` leaves 4 rats, 3 explosives,
  10 triggers, and `reachable_rats=2`. Early trigger 3 is poison, and bounded
  trigger-2 follow-up from the trigger-1 scaffold found no survivable
  continuation. Work the two reachable rats before any trigger-2/trigger-3
  attempt.
- `cooperation/blocked_v2`: early trigger 5 is poison. The useful branch is
  trigger 1 then trigger 3:
  `^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v`, which reaches 8 rats with
  `reachable_rats=6` and removes the plank. Triggering 5 after that drops to a
  4-rat state with `reachable_rats=0`, so trigger 5 must be delayed or avoided
  until the lower/remote rats are already controlled.

### Continuation pass - 2026-06-13 blocked-v2 frontier advance

No new verified win. This pass kept solver concurrency to one or two bounded
processes, reduced `branchdump` caps after one probe reached about 1.2 GB RSS,
and ended with no `solver`, `timeout`, or `clingo` processes running.

- `cooperation/blocked_v2`: the trigger-1/trigger-3 line can be pushed much
  further before the lower-left blocker. Starting from
  `^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v`, the rat-drop suffix
  `^v v< ^> v> ^< v< ^^` verifies as `Playing` at 20 turns with only three
  rats left: `(9,13)`, `(9,14)`, and `(0,15)`. Diagnostics: 15 explosives,
  10 webs, 9 triggers, `reachable_rats=2/3`, and all remaining rats are in the
  rat component. This is the best current `blocked_v2` frontier and supersedes
  the older B17/B23 8-/3-rat notes as the recommended start point.
- `cooperation/blocked_v2`: from that 20-turn frontier, direct continuation
  timed out with no win; resource-preserving `ratsle:2`, `cellnot:1,15,web`,
  `cellnot:2,15,explosive`, `cellnot:10,13,web`, `cellnot:10,14,web`, and
  `triggeronly:2` probes returned no branch under capped checks. The lower-left
  blocker is still exactly `(0,15)` behind `(1,15)` web and `(2,15)/(3,15)`
  explosives.
- `cooperation/blocked_v2`: late trigger 5 from the 20-turn frontier is no
  longer an immediate total-collapse branch, but it still does not open the
  lower-left pocket. A representative trigger-5 branch
  `v^ v^ v^ v^ vv <v <> <> <> <v` leaves the same three rats, 15 explosives,
  9 webs, 4 triggers, `reachable_rats=2/3`, and every remaining trigger is
  unreachable. Do not use trigger 5 after the 20-turn frontier unless a new
  reason explains how it helps open `(1,15)`.
- `cooperation/tug_of_war`: a small dropchain from the trigger-1 scaffold
  `^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v` reconfirmed the known side-rat cleanup
  suffix `^^ <^ <^ <^ v^ v^`, but then remained in the 3-rat/1-reachable top
  pocket basin. No new top-pocket release mechanism was found.
- `cooperation/handoff`: the `(10,6)` sealed rat is now the hard invariant to
  preserve against. The common trigger-2 finish spends the remote `(11,7)`
  trigger and leaves `(10,6)` behind web `(10,5)` with no triggers/explosives
  remaining. Capped checks from the 8-turn setup found no branch for
  `ratat:11,7`, `ratgone:10,6`, or `playerat:12,8`; trigger-1 branches all
  re-enter the same sealed-rat family. The best alternate family is the courier
  prefix `v^ >^ >^ >^ >^ >^ ^^ .v .> v> v>`, which preserves all triggers and
  leaves central rats at `(7,5)` and `(8,7)`. The next useful test is whether a
  central rat can detonate the bottom-right explosive chain before trigger 2,
  opening southeast lure cells for `(10,6)`. Immediate capped resource-preserved
  checks from that courier prefix found no branch for clearing `(10,8)`,
  `(12,6)`, or `(12,7)`.
- `reload_v3`: a stronger trigger-2-first prefix is
  `>>>^>>>>.>>.<.<<<<`. It safely fires trigger 2 with 3 rats, 5 explosives,
  16 webs, and 12 triggers; `(12,22)` is no longer explosive, while `(2,21)`
  still blocks the lower-left rat. From that prefix, capped branchdumps for
  trigger 1, 3, or 4 returned no branch, even without reachable-rat constraints;
  the only found trigger continuation is trigger 7, which again leaves two
  unreachable rats. The next hypothesis should focus on reaching/rat-activating
  top trigger 6 before bottom trigger 5; synthetic checks show bottom 5 first
  seals top 6 and leaves the lower-left rat closed.
- `tinderrectangle`: the lower-row ignition geometry is real but still lacks
  separation. A useful near-miss is the family ending at `P101`, where the
  lower rat reaches `(2,6)` and synthetic ignition from a separated right-side
  player wins, but actual `P101 "."` is `GameOver` and `P101 ">"` starts the
  known overrun. A shorter staged state `<^^^` puts the lower rat at `(2,3)`;
  walking left to `(1,3)` sacrifices that rat and leaves 15 top rats, but capped
  checks from that state found no `ratat:0,0` or immediate ignition. Do not
  extend the P113/P135/PBUF family unless a new gate gives the player right-side
  separation before the lower rat enters `(2,6)`.

### Continuation pass - 2026-06-13 mechanism split

No new verified win. This pass kept solver concurrency low, used
`ulimit -v 1500000` for bounded searches, and ended with no
`solver`/`timeout`/`clingo` processes running.

- `reload_v3`: the trigger-2-first basin is now more tightly bounded. From
  `>>>^>>>>.>>.<.<<<<.^`, diagnostics show two rats left at `(14,5)` and
  `(0,21)`, with the player able to reach only the two trigger-7 cells. Capped
  raw mechanism checks found no branch for `triggeronly:6`, `reachable:9,4`,
  `cellnot:10,5,web`, or `cellnot:9,6,explosive`. Guarded trigger-7 probes that
  required either `(9,4)` or `(1,21)` to stay reachable also returned empty.
  Backing up to the 18-turn three-rat prefix did not recover `triggeronly:6` or
  the `(10,5)` gate. Treat this trigger-2-first route as a dead basin unless a
  different pre-trigger-2 event leaves a new actor/resource.
- `tinderrectangle`: a `rectsep` lookup produced a cleaner staged-safe lead even
  though it did not solve:
  `<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv`.
  It verifies as `Playing` at 58 turns with all 16 rats and all 43 explosives
  preserved, lower rat `(2,3)`, and player already in the right safe pocket at
  `(14,6)`. From this state, capped `rectsep` continuation found no branch. A
  direct `cellnot:2,4,web` branch is possible, but all returned branches put the
  player back around `(2,4)` with the rat still at `(2,3)`, losing the very
  separation this lead was meant to preserve. Next useful work is a door-opening
  mechanism that changes `(2,4)` or `(3,4)` while the player can remain or return
  to the right pocket before the lower rat starts moving.
- `cyborg_rats/ai_takeover`: an independent pass found a better trigger chain
  than the old 113-turn three-rat basin:
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>v>vv>>vvvvv<<<<<<<<<<<<<<<<^^^<<<`.
  It verifies as `Playing` at 75 turns with 16/16 rats reachable, two
  explosives left at `(14,12)` and `(15,12)`, and no triggers. The immediate
  suffix `v` is safe and moves the player to `(0,17)`, but the pocket has no
  shallow payoff: `events` from `B75 + v` returned `NO_EVENTS`, and bounded
  checks for `ratsle:14` or changing `(14,12)` returned empty. Pure stalling
  walks the upper cyborg past `(14,12)` / `(15,12)` rather than onto them. This
  is a better frontier than the old dead basin, but still lacks a cleanup
  mechanism.
- `release`: the row-17 carrier lead remains blocked at plank `(16,17)`.
  `P37 = v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>>` verifies as `Playing` with
  player `(13,14)` and carrier rat `(13,17)`. From both `P37` and the earlier
  `P25 = v<vv^^>>vv><<v<<<^<^^<<><`, raw-state capped branchdumps for
  `cellnot:16,17,plank` returned empty while preserving at least 20 rats. The
  intended local mechanism is still plausible, but the current blocker is
  routing a player/lure to the top-row side before the carrier path is fixed.
- `cooperation/tug_of_war`: the top pocket remains structurally suspicious. The
  real invariant is not `ratgone:7,0`; the rat can shuffle between `(7,0)` and
  `(8,0)`. The important blockers are webs `(7,1)` and `(8,1)`. Capped checks
  for changing either web, or for `norats2:7,0,8,0`, returned empty. Unless a
  hidden mechanism changes one of those exits, the authored level may be missing
  a release for the top rat.
- `cooperation/blocked_v2`: an independent pass found a better first event than
  the older trigger-1 line:
  `v< v^ v^ <^ <^ << <^ >v` fires trigger 4 first and leaves 8 rats, 15
  explosives, 25 webs, 14 triggers, 6/8 rats reachable, and no trapped
  unreachable rats. Follow-up checks for the lower-left mechanism,
  `cell:1,15` and `cell:2,15`, returned empty. This trigger-4-first line is a
  better frontier to compare against the previous 20-turn trigger-1/3 frontier.

### Continuation pass - 2026-06-13 tinder/ai refinement

No new verified win. This pass found a more explanatory `tinderrectangle`
near-solution, and ruled out the new `ai_takeover` B75 frontier more sharply.
It ended with no `solver`/`timeout`/`clingo` processes running.

- `tinderrectangle`: the staged-safe T58 lead can open a side return lane while
  preserving every rat. A useful full prefix is:
  `<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv`.
  This verifies `Playing` at 106 turns with player `(14,6)`, lower rat `(2,3)`,
  all 16 rats, and all 43 explosives. It differs from older P101/P135 families
  because the player returns to the right pocket after opening `(3,5)`, while
  the lower rat remains parked at `(2,3)`.
- `tinderrectangle`: T106 still cannot release the rat from the safe side.
  Stalling or nudging from `(14,6)` does not move the lower rat because `(2,4)`
  and `(3,4)` remain webs. A bounded raw `rectsep` lookup from T106 returned
  empty. Returning left and opening `(2,4)` reaches the sword-pin state:
  `...T106 + ^^^><<<vvv<<^^^<<<<<v<v<<^` at 132 turns, with player `(2,4)` and
  lower rat `(2,3)`.
- `tinderrectangle`: the P132/P137 row-6 chase explains the remaining failure.
  From P132, the lower rat can be walked to `(2,6)` / `(3,6)` while preserving
  all 16 rats. The representative P137 prefix from the sidecar is:
  `<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv><v^>v^<^>v<vv<>^^>vv^^<^^>vv<^^^<<vvv<<^^<^<<vvv<<<<<>>>>>^<<<>>>^^<vv<<<v<<>>`.
  At P137, `>` keeps all 16 rats and moves the lower rat to `(3,6)`, but the
  row-6 chase dead-ends near `(8,6)`. The tempting move from player `(6,6)` /
  rat `(5,6)` is `v`, which detonates the rectangle but kills the player:
  `GameOver` with `rats=0`, not `Won`. A capped `winready` branchdump from P137
  returned empty. The missing step is not getting the lower rat to row 6; it is
  making the rat hit the lower explosive strip without the player standing on
  the same blast cell.
- `cyborg_rats/ai_takeover`: B75 is now ruled out as a cleanup frontier under
  targeted checks. B75 is
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>v>vv>>vvvvv<<<<<<<<<<<<<<<<^^^<<<`
  and verifies with 16/16 rats reachable, explosives `(14,12)/(15,12)`, and no
  triggers. Immediate `v` is safe, but every action after `B75+v` is
  `GameOver`. Long stalling from B75 leaves `rats=16`, `explosives=2`, and
  `triggers=0`; no blackhole drain or explosive use occurs. Targeted raw
  lookups for `ratdrop` and `explosivesle:1` from both B75 and `B75+v` returned
  `NO_SOLUTION` under depth-40 caps. Treat B75 as a better diagnostic trap, not
  a cleanup route.

### Continuation pass - 2026-06-13 bounded probe cleanup

No new verified win. This pass deliberately kept solver searches short after a
machine OOM warning. One accidental four-way `ratgeom` diagnostic fan-out on
`blocked_v2` was stopped after roughly 30s; RSS was tiny, but future probes
should still wrap even `ratgeom` in `timeout` when the target may be unreachable.
The pass ended with no `solver`/`timeout`/`clingo` processes running.

- `tinderrectangle`: T106 was retested with the concrete lower-ready staging
  goal. `branchdump --goal rectlower` from
  `<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv`
  returned no branch under depth 80 / 20s / 120k nodes. From the P137 sidecar,
  the safe `>` continuation reaches player `(4,6)` and lower rat `(3,6)`;
  walking the pair right to rat `(7,6)` is easy, but it pins the player at
  `(8,6)`. A direct escape probe for `playerat:14,8` from the row-6 walking
  state returned no branch. The lower route still needs a separation mechanism
  before the rat reaches the row-6 fuse lane.
- `release`: the P37 carrier state is now more clearly a diagnostic dead end.
  `ratgeom` says a synthetic player on the upper/right side can make the row-17
  carrier step east, but from actual P37 a capped `ratat:14,17` branchdump
  returned empty. Capped checks for changing the upper plank `(13,11)` from both
  P25 and P37 also returned empty, and nearby upper rats do not have one-step
  geometry into that plank. The likely issue is earlier routing/access, not just
  choosing a better suffix from P37. Local continuations explain why: `P37>`,
  `P37.`, `P37^`, and `P37^^` all leave the carrier parked at `(13,17)`;
  `P37<` makes it retreat to `(12,17)`; `P37v` is `GameOver`. The plank at
  `(16,17)` is not the immediate rules blocker because rats can traverse planks;
  the immediate blocker is getting the player east of the carrier without first
  losing the carrier position.
- `reload_v3`: trigger 7 first is reachable and produces several preserved-rat
  branches, for example
  `^>>>>>^>>^^^<<<<v<<^<<<<<<<^^^`, but a follow-up `triggeronly:2` probe from
  that branch returned empty under depth 60 / 15s / 80k nodes. This reinforces
  that immediate 7-then-2 is not the missing reload order; any useful 7-first
  route must change access to a different trigger or actor before aiming at 2.
- `cooperation/blocked_v2`: the trigger-4-first state was inspected directly.
  The lower-left trigger-2 cells remain player-unreachable, so the plausible
  next mechanism is rat activation rather than player activation. An attempted
  `ratgeom` fan-out for rats near `(9,13)/(9,14)/(0,15)/(15,16)` toward the
  trigger-2 cells did not finish quickly and was killed; rerun only as separate
  timed diagnostics or replace it with a purpose-built bounded goal.
- `cooperation/handoff`: pre-trigger geometry confirms the sealed `(10,6)` rat
  would move to `(11,7)` if a player could stand southeast of it, but the
  reachable pre-trigger player area does not include such a lure cell. A capped
  `ratat:11,7` branchdump under depth 20 / 11s / 60k nodes returned no branch.
  The safe staging prefix `v. >. >. >. >. >. v. >.` verifies `Playing` at 8
  turns and puts P1 at `(9,8)` with all 5 rats and all triggers preserved, but
  it still does not move the sealed rat or change web `(10,5)`. Do not continue
  staging toward `(9,8)` unless it is paired with a new way to reach a true
  southeast lure cell.

### Continuation pass - 2026-06-13 OOM-safe mechanism audit

No new verified win. This pass fetched remotes, confirmed the branch was already
current, kept solver probes memory-capped, and ended with no
`solver`/`timeout`/`clingo` processes running.

- `cooperation/blocked_v2`: from the 20-turn three-rat frontier
  `^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v ^v v< ^> v> ^< v< ^^`,
  `wp2` can put P2 at `(11,14)` east of the two reachable rats with suffix
  `v^ v^ v> v> >> vv <v <> ^v ^v ^v ^< ^< ^< ^< ^< ^<`, but opening the
  adjacent webs from `(11,13)`, `(11,14)`, or `(11,15)` is immediate
  `GameOver`. Firing trigger 5 from that staged position only walls off more of
  the board; it does not open the lower-left pocket.
- `tinderrectangle`: the row-6 route is now an exact local spacing trap, not
  just a vague near miss. At the P137 sidecar, `>` walks the lower rat along row
  6, but `v` detonates the rectangle and kills the player, while continuing
  right pins the player at `(8,6)` with the rat behind. A capped `rectlower`
  branchdump from the P132 gate-open state returned no branch under depth 70 /
  20s / 100k nodes.
- `reload_v3`: the short trigger-2-first line
  `>>>^>>>>.>>.<.<<<<` is confirmed as a rat-triggered event: the right rat
  steps onto trigger 2 while the player stays at `(17,15)`. Trying to keep that
  actor for the next station fails locally; after suffix `vvv`, the player is at
  `(17,17)` and the rat at `(17,16)`, and every side/pivot move is `GameOver`
  except moving back up, which kills the rat and returns to the known dead
  two-rat basin.
- `cooperation/tug_of_war`: the central-rat top-gate hypothesis remains
  blocked. Preserving all seven rats, capped branchdumps for `ratat:7,4` and
  `ratat:8,4` returned no branch, so the central rat was not even pulled one row
  upward toward the top planks in the tested window.
- `cyborg_rats/ai_takeover`: B75 is slightly more nuanced than the previous
  note: immediate `v`, `<`, and `.` are safe. Repeated stalling/wall-pivoting
  lets the remote cyborg walk around the two remaining explosives, but source
  confirms cyborg pathfinding treats explosives as blocked. Local escape checks
  after several stall counts still have `>`/`^` as `GameOver`; only `v`, `<`,
  or more waiting survive, so B75 remains a diagnostic trap rather than a
  cleanup route.

### Continuation pass - 2026-06-13 bounded frontier audit

No new verified win. This pass kept solver processes capped with `timeout` and
`ulimit -v` except for one accidental unwrapped `ratgeom`, which was killed by
PID after ~30s. The final process table was clear before committing.

- `solver`: `lookup` / `branchdump` now support
  `--goal cellnotplayer:x,y,kind,playerx,playery`. This is a diagnostic-only
  compound goal for checks like "door web is open while the player is back in a
  safe pocket."
- `solver`: added a diagnostic `frontier` mode:
  `solver frontier <csv> --prefix <moves> --depth N --secs S --maxnodes N`.
  It enumerates first paths to distinct local configurations and can be gated
  with `--min-rats`, `--states`, and `--no-canonical`. Use it to answer finite
  local-state questions before spawning many separate `branchdump` probes.
- `reload_v3`: the all-rats trigger-1 station
  `>>>^>>>>vvv.v<^<<<v<<<<<<<^^<^<<<<<<<<v<v.`
  is now tightly bounded. `frontier` from that prefix with `--min-rats 3`,
  depth 20, and both canonical and raw hashes fully exhausts after only 20
  states. The only real local mechanism is the released rat chewing plank
  `(3,18)` via suffix `<`; after that, the rat can shadow to `(2,17)`,
  `(2,18)`, or `(2,19)`, but cannot be carried lower or into the `(2,21)`
  explosive without killing the player or sacrificing the actor. Exact capped
  checks for changing `(4,17)` or `(4,19)` planks from `P1<` returned no
  branch. Treat this trigger-1 station as a finite local dead end unless a
  different pre-trigger-1 setup changes the geometry before the rat is released.
- `tinderrectangle`: `frontier` from T106
  `<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv`
  confirms that, while preserving all 16 rats, the lower rat remains parked at
  `(2,3)`; local states are mostly the player clearing right-side webs. This
  supports the current model: T106 is a prepared-safe pocket, but it still lacks
  a delayed release for `(2,4)` / `(3,4)`. A later subagent pass found one
  non-winning but concrete mechanism from T106: suffix
  `^^^>v^<vvvv^^^^<<vvv<<^^^<<<vvv<^^<^<<>` reaches player `(4,3)` with the
  lower rat at `(3,3)`; stepping west into `(3,3)` can sword-block the rat's
  east move and safely open the upper door. Direct descent still dies, so the
  next useful test is an inserted blocker/delay before descending, not another
  immediate rectangle ignition.
- `release`: from opener `v<vv^^>>v`, the intended dependency is still left
  trigger 2 opening the right-side isolated rat, but the route to trigger 6 is
  dynamically unreachable. Capped `branchdump` checks for `triggeronly:6` and
  for trigger-6 plus left explosives `(1,16)` / `(0,17)` changing returned no
  branch; `wp` to `(17,16)` and `(19,18)` is `UNREACHABLE`, even via the simple
  trigger-5 waypoint `(11,12)`.
- `cooperation/blocked_v2`: the 20-turn three-rat frontier was checked for the
  black-hole shortcut: the last unreachable rat at `(0,15)` has a black hole
  directly below at `(0,16)`, but capped checks for `ratgone:0,15` and
  `ratsle:2` from that state returned no branch. Do not assume the black hole
  is enough without a reachable lure.
- `clingo`: no global `clingo` CLI or Python module is installed in this
  environment. A temporary venv at `/tmp/infestation-clingo-venv` can provide
  Python `clingo`, but an ASP route should still start from a bounded subproblem;
  the new `frontier` command is currently the lower-risk way to get finite local
  proofs from the real Rust oracle.

### Continuation pass - 2026-06-13 bounded parallel sweep

No new verified win. This pass used two read-only side agents plus local capped
probes, kept solver runs under `timeout` / `ulimit`, and ended with no
`solver`, `timeout`, or `clingo` processes running.

- Solution catalog sanity: all 35 entries in
  `solver/solutions/final_solutions.json` still verify as `result=Won` against
  the current binary. The active hard set remains the 7 normal rat puzzles in
  section 3. `levels/claude/gauntlet.csv` is a zero-rat portal hub; direct
  `solver verify` reports `Playing` because zero-rat levels only become `Won`
  if the level started with rats. The five gauntlet sublevels still verify.
- `tinderrectangle`: the T106 upper-door latch lead is real but finite. From
  the latch state with player `(4,3)` and lower rat `(3,3)`, preserving all 16
  rats, `frontier` only finds the player walking right while the rat shadows on
  row 3; direct descent is contact death and `rectsep` returned no branch. From
  the pre-latch `(3,3)` / `(2,3)` state the all-rats frontier is the same
  rightward shadow path. The right-safe P84 top-pack hypothesis also failed in
  capped checks: no branch opened row-2 webs `(13,2)`, `(14,2)`, or `(15,2)`
  while returning the player to `(15,7)` with all rats preserved. Do not keep
  extending T106/P84 unless a different staging state changes the release
  timing.
- `reload_v3`: the stronger trigger-2 prefix
  `vvv<<<<<<vvv><^^^>>>>>>>>>>>>>^^^^^v<<v<^` is now bounded with `frontier`.
  It has only three all-rats local states: the carrier at `(17,14)`, `(17,15)`,
  or `(17,16)`. Capped raw checks still found no branch changing `(2,21)` from
  explosive. This confirms the 41-turn trigger-2 choreography is another tiny
  dead basin unless a different pre-trigger-2 event changes the lower-left
  pocket.
- `release`: a side pass retested early cage opening, alternate opener
  `v<vv^^>>>>v`, and staged prefix `v<vv^^>>vv>^<<v<<<^^`. No branch changed
  `(18,5)` web, moved the isolated `(18,4)` rat to `(18,5)`, reached trigger 6
  via `(19,18)`, or staged `ratplayer:18,17,19,19` under the caps. The solution
  still needs a setup before the known central-trigger family that either opens
  `(18,5)` or gives the isolated rat useful motion. A follow-up relaxed check
  with no rat-count floor also found no `triggeronlycellnot:2,18,5,web` branch
  from the initial state, standard opener `v<vv^^>>v`, or alternate opener
  `v<vv^^>>>>v`; the useful left-trigger-2 event is unreachable in those basins,
  not just overconstrained by preservation.
- `cyborg_rats/ai_takeover`: the lower safe-7/8 route
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvvv<`
  is still the better frontier than old B75. It can reach trigger 2 with suffix
  `<<<<<<<<<>>>>>>>><<<<<<<<<<<<<<^^^<<<`, producing C97: 8 rats, 2 explosives,
  0 triggers, all 8 rats reachable. But capped `ratsle:7` from C97 returned no
  branch, and B75 follow-ups for `ratsle:14` / changing `(14,12)` explosive also
  returned no branch. A longer raw `frontier` from C97 showed the safe local
  actions are essentially repeated left wall-pivots and occasional down toggles:
  the nearby cyborg cluster stays fixed while only the remote cyborg walks
  around the map. A relaxed depth-120 `ratsle:7` branchdump still returned no
  branch. Treat C97 and B75 as cleanup basins, not solutions.
- `cooperation/tug_of_war`: capped checks from the initial state and the
  trigger-1 scaffold found no branch opening pocket webs `(7,1)` / `(8,1)` or
  breaking planks `(7,2)` / `(8,2)` while preserving the required rats. The
  top-pocket release via central-rat / trigger-1 staging is ruled out more
  sharply in the tested bounds.
- `cooperation/handoff`: from the initial state and the prefix
  `v^ >^ >^ >^ >^ >^ ^^`, capped checks found no branch moving sealed rat
  `(10,6)` to `(11,7)`, clearing `(10,5)`, or making `(10,6)` reachable.
- `cooperation/blocked_v2`: from initial, 9-turn, 16-turn, and 20-turn basins,
  capped checks still found no trigger-2 event and no change to lower-left gate
  cells `(1,15)` / `(2,15)`. The 20-turn three-rat frontier remains the best
  diagnostic start, but not a solved path.

### Continuation pass - 2026-06-13 OOM-safe parallel follow-up

No new verified win. This pass used two read-only side agents and local capped
oracle probes. Local searches were wrapped in `timeout` / `ulimit -v`; the pass
kept the process table clean between probe groups.

- `release`: the trigger-6 hypothesis is now weaker. From the standard opener
  `v<vv^^>>v`, direct `wp` to trigger-6 cells `(17,16)` and `(19,18)` reports
  `UNREACHABLE`. Capped branchdumps for `triggeronly:6` while preserving a
  reachable trigger 2 returned no branch from the opener, the P37 carrier state
  `v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>>`, or the trigger-5 state
  `v<vv^^>>vv<>>>^`. Early trigger-6 checks from the initial board with
  `--min-rats 24`, and trigger-6 checks allowing the standard opener's one rat
  loss with `--min-rats 23`, also returned empty. Treat trigger 6 as a late
  diagnostic trap unless a new setup changes access; do not continue widening
  P37/P5-trigger probes blindly.
- `tinderrectangle`: T106 delayed-release checks for opening `(3,4)` or `(2,4)`
  while returning the player to the right safe pocket returned no branch under
  raw-state capped searches. The latch suffix
  `^^^>v^<vvvv^^^^<<vvv<<^^^<<<vvv<^^<^<<>` from T106 reaches player `(4,3)`
  with lower rat `(3,3)`, but the local all-rats frontier is finite: only the
  player walking right while the lower rat shadows on row 3. Killing the lower
  rat with suffix `>>>>>><` leaves a clean 15-top-rat state with player `(8,3)`,
  all 43 explosives intact, and all 15 rats reachable, but a capped `rectready`
  branchdump from that state returned empty. The lower-rat route still needs a
  timing/separation idea before release, not another immediate descent.
- `cyborg_rats/ai_takeover`: a side-agent found a better pre-trigger-2 route.
  Prefix
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<`
  verifies `Playing` at 64 turns with 15 rats, 5 explosives, 2 triggers, and
  14/15 rats reachable. Taking the usual trigger-2 trip after it,
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<<<<<<<<<<>>>>>>>><<<<<<<<<<<<<<^^^<<<`,
  verifies `Playing` at 101 turns with 7 rats, 2 explosives, 0 triggers, and
  all 7 rats reachable. This improves the older C97 8-rat triggerless basin,
  but capped `ratsle:6` / event checks from D101 still found no cleanup branch.
  Continue from Q64 before trigger 2, not from D101 cleanup.

### Latest bounded pass - 2026-06-13

No new verified win. All checks in this pass were run with low concurrency and
short `timeout`/`ulimit` caps after the previous OOM.

- `cyborg_rats/ai_takeover`: Q64 can reach a better 3-rat triggerless basin:
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<<<<<<<<<<<<<<>>>>>>>>>>>><<<<<<<<<<<<<<^^^<<<`
  verifies `Playing` at 109 turns with rats/cyborgs at `(18,4)`, `(11,9)`,
  and `(2,16)`, explosives only at `(14,12)` and `(15,12)`, no triggers, and
  all 3 rats reachable. However `ratsle:2` from this basin returned no branch
  in bounded checks, and the local move table shows the left-pocket rat/cyborg
  immediately constrains cleanup.
- `cyborg_rats/ai_takeover`: the human mechanism is now clearer. The remote
  cyborg at `(18,4)` does not start moving until trigger 2 clears the explosive
  column at `(18,6..8)`, so the known trigger-2 line spends the trap before the
  cyborg can enter it. A stricter `triggeronlycellnot:2,18,4,cyborg` probe did
  find 4-rat trigger variants, but the best inspected state had player `(0,17)`,
  cyborgs at `(1,16)`/`(2,16)`, and no playable successors. Continue searching
  from Q64 for a pre-trigger structural event, not from the 3- or 4-rat basins.
- `reload_v3`: the live-rat plank-cutting hypothesis from the trigger-1 family
  was checked from
  `>>>^>>>>vvv.v<^<<<v<<<<<<<^^<^<<<<<<<<v<v.<`. The helper rat cleanly cuts
  plank `(3,18)`, but capped `cellnot:4,17,plank` and
  `cellnot:4,19,plank` probes with `--min-rats 3` returned no branch. Killing
  the helper rat is easy and appears to be the wrong objective.
- `tinderrectangle`: the upper-door sword-latch branch
  `<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv^^^>v^<vvvv^^^^<<vvv<<^^^<<<vvv<^^<^<<>`
  reaches player `(4,3)` with the lower rat at `(3,3)`. Running right then
  stepping left (`>>>>><`) safely kills the lower rat and leaves a clean
  15-top-rat state with player `(8,3)`, but a capped `winready` probe and
  direct `ratat:0,0` / `ratat:16,0` corner probes returned no branch. This
  latch is useful evidence, but not a solution route by itself.
- `old_levels/on_the_clock.csv`, `old_levels/order_of_operations.csv`, and
  `old_levels/overstep.csv`: quick capped direct A* probes timed out without
  wins. They remain in the unsolved rat inventory, but are lower priority than
  the active hard set.
- `tinderrectangle`: `geomlure` against the actual ignition targets
  `(0,0)` / `(16,0)` found a new left-edge latch:
  `<^^<<<^vv<v>>^>v<<<<>^>^^>>v<<v<<>>^>>>^>>v>vv>>^^^>>vvv>vv<<>^^^^^<<vvv<<^^<^<<v<<<^<`
  verifies `Playing` at 86 turns with player `(3,3)`, lower rat `(2,3)`, all
  16 rats alive, and 21 webs left. Stepping left kills the lower rat safely;
  stepping left again detonates the left border and kills all rats, but it is
  `GameOver` because the player triggers the blast. From this latch, capped
  checks found no playable branch clearing top-left webs `(1,2)` or `(2,2)`
  while preserving either 16 rats or the post-lower-rat 15-rat state. This is
  the same underlying failure as the upper-door latch: the border blast is
  sufficient, but only if a rat triggers it.
- `cyborg_rats/ai_takeover`: from Q64, capped `cellnot:18,6,explosive` and
  `ratat:18,5` probes with `--min-triggers 2` returned no branch. That weakens
  the idea that the remote cyborg can be lured into the `(18,6..8)` explosive
  column before trigger 2 fires.
- `old_levels/overstep.csv` and `old_levels/order_of_operations.csv`:
  capped `dropchain` probes found partial rat drops but timed out without a
  verified win. No solver/timeout processes were left running afterward.
- `release`: a bounded macro/event probe found a new-looking 13-rat state
  `^^^^^^v^vvvvvvv>>>vv^^^^vv<^v<<<<<^^^<<<<.<>>>>vvv^v^^^^^^^^^<<<^`
  with player `(1,4)`, 13 rats, 5 explosives, 9 triggers, and static distances
  to trigger-6 cells `(17,16)` / `(19,18)`. Dynamic `wp` to both trigger-6
  cells from this exact prefix returned `UNREACHABLE`, and capped
  `triggeronly:6` / `cellnot:18,5,web` branchdumps returned no branch. Treat
  this macro state as another sealed trigger-6 mirage unless a new route changes
  the rat/player timing before the top sweep.
- `old_levels/on_the_clock.csv`: trigger-order search produced a 2-rat partial
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvv.vv>>><vvv<<^^^>>>><<vvv<<<<v<vvvv>>><<<<<<>>>^^^^^^>^^>>>>>>>vvvv>>>>>>>vv<<<<<<<<<v`
  with no explosives/triggers left. It leaves rat `(16,6)` in a size-1
  unreachable component and rat `(10,14)` adjacent to the player, so it is a
  dead basin, not a continuation frontier.
- Long capped background solves on `release`, `reload_v3`,
  `tinderrectangle`, and `cyborg_rats/ai_takeover` found no verified wins.
  `tinderrectangle` hit its per-process `ulimit` and aborted safely; the
  process table and system memory were clean afterward.
- `tinderrectangle`: added bounded multi-target lookup goals
  `ratrectplayerrect` and `cellnotplayerrect` to avoid launching one process
  per exact coordinate. These are search-oracle helpers only; they do not
  change game rules.
- `tinderrectangle`: a better human-style precursor was verified. Let
  `T106=<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv`,
  `PREOPEN=^^^>v^<vvvv^^^^<<vvv<<^^^<<<vvv<^^<<v<<>`, and
  `RET=>>>>^^>>>vvv>>^^^>>vvv`. `T106+PREOPEN+RET` verifies `Playing` at
  168 turns with player `(14,6)`, the lower rat still parked at `(2,3)`, all
  16 rats alive, and `(2,5)` open. This proves the intended direction is
  delayed release/separation, not simply dragging the lower rat across row 6.
- `tinderrectangle`: from `T106+PREOPEN+RET`, opening `(2,4)` while preserving
  all 16 rats is reachable; the best branch found returns to
  `player=(2,4), rat=(2,3)` with `(2,4)` open. However, that branch is still a
  contact trap: `v` / `vv` keeps the rat adjacent, horizontal escape is
  immediate `GameOver`, and `cellnotplayerrect:2,4,web,13,6,15,8` found no
  branch that opens `(2,4)` and returns the player to the right/bottom safe
  pocket under the cap. Do not repeat the adjacent-open branch unless a new
  separation idea is added.
- `old_levels/order_of_operations`: found a much shorter productive trigger
  family than the previous 2-rat dead basin. Prefix
  `<<<<<<^v^^^^>^^^vvvv^vv^^vvvv>>>>>^` verifies `Playing` at 35 turns with
  4 rats, 1 explosive, and all remaining rats reachable. A shorter trigger-8
  sibling
  `<<<<<<^v^^^^>^^^vvvv^vv^^vvvv>>>>>^^>>>>^^^^vvvv<<<<^<<<`
  reaches a 4-rat state at 56 turns. Appending `v>>>>>>>^>>^` reaches a
  2-rat state at 68 turns with rats `(9,3)` and `(17,18)`, all remaining rats
  reachable, 1 explosive, and 7 triggers.
- `old_levels/order_of_operations`: the 68-turn 2-rat state is not solved.
  Bounded checks found no direct `ratsle:1` with `max_trapped_rats=0`, no
  `ratgone:9,3`, and no `cellnot:9,4,web`. Trigger 5 from this family walls
  off the top rat; trigger 2 from the related 60-turn branch also strands
  rats. The next useful hypothesis must handle top rat `(9,3)` before the
  trigger-8/trigger-5 cleanup family, or use a different trigger-8 sibling that
  changes the top enclosure.
- `old_levels/order_of_operations`: after the commit above, a trigger-3-first
  alternative was also checked. Prefix
  `<<<<<<^v^^^^>^^^vvvv^vv^^^vvvv` reaches a clean 5-rat / 1-explosive state
  at 30 turns with all 5 rats reachable. From there, trigger 8 can drop to
  4 rats at 44 turns via
  `<<<<<<^v^^^^>^^^vvvv^vv^^^vvvv^^^^^^^^^^^>>>`, still with all rats and
  triggers reachable. Continuing to
  `<<<<<<^v^^^^>^^^vvvv^vv^^^vvvv^^^^^^^^^^^>>>^^^^<<<vvvvvvvvvvvvvvvv>>>>>^^>>>>^^>>v`
  reaches a 2-rat state at 83 turns, but it is the same obstruction: residual
  rats `(9,3)` and `(17,18)`. Bounded `ratsle:1` / `ratgone:9,3` /
  `cellnot:9,4,web` checks still fail. This weakens the idea that merely
  choosing a different trigger-8 sibling fixes the top rat.
- `reload_v3`: a bounded trigger-any pass found only a trigger-7 opening
  `^^^^^<<<<<<^^^` at 14 turns. It verifies `Playing`, but diagnostics show
  only 1 of 3 rats reachable and 0 of 12 remaining triggers reachable, so this
  is a dead macro family, not a continuation frontier.

### Current run notes - 2026-06-13

No new verified wins. The session used low-concurrency capped probes after a
prior machine OOM: every solver run was wrapped with `ulimit -v` plus `timeout`,
and process-table checks confirmed no lingering `solver` / `timeout` / `clingo`
processes before and after batches.

- Current inventory check: `solver/solutions/final_solutions.json` has 35
  entries, and all 35 replay as `result=Won` under `target/release/solver
  verify`. The unsolved rat-bearing CSVs are:
  `cooperation/blocked_v2.csv`, `cooperation/handoff.csv`,
  `cooperation/tug_of_war.csv`, `cyborg_rats/ai_takeover.csv`,
  `old_levels/on_the_clock.csv`, `old_levels/order_of_operations.csv`,
  `old_levels/overstep.csv`, `release.csv`, `reload_v3.csv`, and
  `tinderrectangle.csv`.
- `old_levels/order_of_operations`: continued from the trigger-1/3 top-column
  family. The promising prefix
  `B=<<<<<<^v^^^^>^^vv^vv^vv^^^vvvv^^^^^^^^^^^>>^>>vvvvvv<vv<`
  reaches a 4-rat state with all rats/triggers reachable. From `B`, bounded
  `ratsle:3` finds a clean drop to
  `D=<<<<<<^v^^^^>^^vv^vv^vv^^^vvvv^^^^^^^^^^^>>^>>vvvvvv<vv<v>vv>>>>>>^^^^vv>>`
  with rats `(9,3)`, `(10,11)`, `(17,18)`, all reachable. From `D`, one step
  `v` gives
  `E=<<<<<<^v^^^^>^^vv^vv^vv^^^vvvv^^^^^^^^^^^>>^>>vvvvvv<vv<v>vv>>>>>>^^^^vv>>v`,
  a 2-rat state with rats `(9,3)` and `(17,18)`, all triggers still reachable.
- `old_levels/order_of_operations`: the `B/D/E` family is probably a dead
  branch. From `D` and `E`, capped `playerat:9,4`, `playerat:9,6`,
  `playerat:8,6`, and `ratplayerfacing:9,3,9,4,north` all returned empty.
  From `E`, `trigger:7`, `trigger:8`, and `trigger:2` are reachable while
  preserving both rats, but they do not make `(9,4)` usable and do not produce a
  `ratsle:1` cleanup. Killing the right rat first via `>>>>>>vv` strands the
  top rat with `reachable_rats=0`. A static shortest path to the top route
  steps through trigger terrain and walls off the map, leaving the player near
  `(14,14)` rather than at the top rat.
- `old_levels/order_of_operations`: broader capped checks from
  `P1=<<<<<<^v^^^^>^^vv^vv^vv^^^vvvv` for `playerat:9,4` with 4 rats preserved
  and `ratsleplayer:3,9,4` also returned empty. Do not repeat the `P1 -> B/D/E`
  route unless a new idea changes the lower-pair disposal before the trigger
  terrain seals the route.
- `tinderrectangle`: rechecked the two tempting local frontiers with strict
  caps. At
  `P137=<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv><v^>v^<^>v<vv<>^^>vv^^<^^>vv<^^^<<vvv<<^^<^<<vvv<<<<<>>>>>^<<<>>>^^<vv<<<v<<>>`,
  the all-rats-preserving frontier is forced: repeated `>` walks player/rat
  along row 6 until `P137+>>>>>` has player `(8,6)` and lower rat `(7,6)`.
  Every next move (`>`, `^`, `v`, `.`, etc.) is `GameOver` because `(9,6)` is a
  wall and the rat catches the player. This is a local forced contact trap, not
  a search-width issue.
- `tinderrectangle`: at the earlier contact-release state
  `P132=T106+^^^><<<vvv<<^^^<<<<<v<v<<^` (player `(2,4)`, lower rat `(2,3)`),
  `^` kills the lower rat and enters the 15-top-rat branch; `v` / `.v` preserve
  all 16 rats but keep the lower rat directly behind the player; horizontal
  escape dies immediately. Bounded `rectsep` and `ratfar:2,5,3` checks from
  P132 returned empty. From the lower-rat-killed branch `P132+^`, bounded
  `winready`, `ratat:0,0`, and `ratat:16,0` checks also returned empty. Treat
  P132/P137 as exhausted unless an earlier route changes the release spacing.
- `reload_v3`: the short rat-triggered trigger-2 prefix
  `>>>^>>>>.>>.<.<<<<` was rechecked as a possible reload station. Diagnostics
  at the prefix show 3 rats, 5 explosives, 12 triggers, but `reachable_rats=0`
  and only 2 reachable triggers. Local frontier can make the adjacent middle
  rat reachable, but capped continuations for `trigger:1`, `trigger:6`,
  `reachablege:2`, `cellnot:1,21,web`, and `ratat:1,21` all returned empty.
  Treat this as a one-actor local basin, not a path to the bottom-left rat.
- `old_levels/on_the_clock`: after the machine OOM, this pass used only
  low-concurrency capped probes (`ulimit -v 800000; timeout ...`) and left no
  solver processes running. No verified win was found, but the mechanism map is
  sharper. The standard trigger-5 frontier
  `>>>^<^>>>>vvv><^vvvvvv^^>>>` strands the initial lower-right rat inside the
  sealed `(12,14)` component. That component is surrounded by walls with only
  web gate `(14,15)`, and remaining triggers/explosions do not open a player
  route to that web. Continuing `5 -> 8 -> 6 -> 7` or the symmetric order is a
  dead mechanism unless the lower-right rat is handled before trigger 5.
- `old_levels/on_the_clock`: the older nonstandard 2-rat basin exposed a better
  timing idea. In the old route, the top-right rat fires trigger 9 on `(16,6)`,
  self-sealing into a one-cell unreachable pocket. Replacing the local
  `.`/turning sequence around turn 40 with earlier south moves gives the useful
  frontier
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvvvvvv`
  at 44 turns: player `(7,8)`, all 8 rats alive, top-right rat `(15,8)`, and
  11 triggers left. Stepping south once more fires trigger 4:
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvvvvvvv`
  at 45 turns has all 8 rats and all 9 remaining triggers player-reachable
  (`reachable_rats=8/8`, `trapped_unreachable_rats=0`). This is the best new
  constructive state.
- `old_levels/on_the_clock`: the 45-turn all-reachable state is only a one-turn
  window. On the next player action, the rat at `(14,9)` steps onto trigger 9
  at `(13,9)`, leaving `triggers=0` and only 5 reachable rats. Immediate
  `ratdrop` / `ratsle:7` / generic `win` checks from that state return no
  branch. Backing up to the 44-turn frontier, strict trigger-9-first branches
  fire too early and leave only 2 reachable rats; strict trigger-3 and
  rat-triggered trigger-4 alternatives were not found in the checked caps. The
  next useful hypothesis is to alter the turn-40-to-45 timing so trigger 4 can
  fire while the top-right rat is not forced onto `(13,9)` on the following
  turn, or to find a way to consume/disable that trigger-9 step first.

### OOM-safe follow-up - 2026-06-13

No new verified win. This pass was intentionally conservative after the prior
machine OOM: every local solver run used `ulimit -v 800000` plus `timeout`, no
more than two solver processes were active at once, and final process-table
checks showed no `solver`, `timeout`, or `clingo` processes.

- Inventory sanity: this checkout currently has 35 entries in
  `solver/solutions/final_solutions.json`. The missing CSVs are the 10
  rat-bearing files
  `cooperation/blocked_v2.csv`, `cooperation/handoff.csv`,
  `cooperation/tug_of_war.csv`, `cyborg_rats/ai_takeover.csv`,
  `old_levels/on_the_clock.csv`, `old_levels/order_of_operations.csv`,
  `old_levels/overstep.csv`, `release.csv`, `reload_v3.csv`, and
  `tinderrectangle.csv`, plus the zero-rat `claude/gauntlet.csv` portal hub.
- `release`: `frontier` and `events` from the standard opener `v<vv^^>>v`
  only found local top-pack/web-shaving variations and 22-rat drops. None of
  the listed event successors changed `(18,5)`, made left trigger 2 reachable,
  reached trigger 6, or moved the isolated `(18,4)` rat. Treat this as direct
  evidence that the standard opener is a local-events mirage; back up before
  the central-trigger family instead of widening it again.
- `cyborg_rats/ai_takeover`: Q64
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<`
  still has only left trigger 2 reachable. `events` from Q64 found only the
  one-step web shave `<`; preserved-resource checks for changing either
  explosive `(14,12)` or `(15,12)` returned no branch. From the known 3-rat
  triggerless basin
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<<<<<<<<<<<<<<>>>>>>>>>>>><<<<<<<<<<<<<<^^^<<<`,
  alternating `><` safely pins the local cyborg while the remote cyborg walks
  around the map for about 20 pairs, but the pattern collapses at turn 154.
  The tempting suffix `>v` drops to two rats only as `GameOver`; a capped
  `ratsle:2` check from the pre-collapse state returned no legal branch. Treat
  this as a cleanup trap; continue from a pre-trigger-2 structural event, not
  from D109-style cleanup.
- `old_levels/overstep`: a short strict trigger-any probe found a concrete
  first-event branch
  `v<<^^^^^^^>>>>>>>>>>>>><>><^<<<<<vvv<<`, verified `Playing` at 38 turns
  with 5 rats, 3 explosives, 5 webs, 8 triggers, and only 2 reachable rats.
  From that state, a `ratsle:4` branch exists but spends all explosives and
  triggers, leaving only 2 reachable rats; a resource-preserving `triggeronly:1`
  continuation returned no branch. This is a useful first lever for mapping
  `overstep`, but the immediate rat-drop objective is a dead milestone.
- `cooperation/blocked_v2`: the current 23-move 3-rat basin was rechecked for
  `triggeronly:2` under resource and reachable-rat gates; no branch was found.
  A side explorer was started for `blocked_v2` but did not return before the
  safety cutoff and was closed; no visible solver process was left behind.

### OOM-safe continuation - 2026-06-13

No new verified win. This pass again used only capped local probes
(`ulimit -v 800000` plus `timeout`) and ended with no solver processes active.

- `old_levels/on_the_clock`: the 45-turn all-reachable frontier is confirmed as
  a forced trigger-9 collapse. Enumerating every immediate legal action from
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvvvvvvv` shows that all surviving
  moves consume trigger 9 on the next rat step, leaving `triggers=0`,
  `reachable_rats=5/8`, and three trapped rats. The next viable idea has to
  change timing before this 45-turn state; no local rescue exists at that
  state.
- `old_levels/overstep`: the initial event scan found a better early lever
  than the previous long branch: `>>>` preserves all 6 rats and most resources
  (`rats=6`, `explosives=6`, `webs=6`, `triggers=20`), but capped
  `reachablege:1` / `allreachable` probes from `>>>` returned no branch.
  A macro run found a stronger-looking 68-turn state
  `v<<^^^^^^>>>>>>>v>>>^^<<v<<<<<vv><v<<<vvv>vvvv<<vv^^>>^^^^>^>>>^v^.v`
  with 3 rats, 1 explosive, 1 web, and 5 triggers, but diagnostics show the
  player has only 3 reachable cells and no rats reachable. The only local
  follow-up is trigger 4 (`^^`), which spends the last explosive/web and still
  leaves all 3 rats unreachable. Treat both `>>>` and the macro 3-rat basin as
  diagnostic dead ends unless a different first event opens rat reachability.
- `old_levels/order_of_operations`: the top rat blocker was tested earlier than
  the known two-rat residue. From
  `P1=<<<<<<^v^^^^>^^vv^vv^vv^^^vvvv`, `playerat:9,4` is dynamically
  unreachable, and both preserved and relaxed `ratgone:9,3` checks from the
  initial board and P1 returned no branch. The top rat is statically
  player-reachable in diagnostics, but current route families cannot actually
  remove it before the trigger terrain seals the map.
- `release`: an initial-board `events` scan mostly rediscovers central trigger
  variants and local rat movements; it did not expose a first event that touches
  `(18,5)`, left trigger 2, or trigger 6. Continue only from a hypothesis that
  changes the right-pocket dependency before the standard 3/4/5 family.
- `cooperation/blocked_v2`: the trigger-4-first frontier
  `v< v^ v^ <^ <^ << <^ >v` verifies with 8 rats, 15 explosives, 25 webs, and
  14 triggers, but its structural event scan only produced immediate top-pack
  rat drops. A short `dropchain` from that frontier drops to 3 rats while
  reducing reachable rats from 6 to 0; it does not touch the lower-left
  trigger-2 gate. Do not continue cleanup-first from this frontier.

### Guarded mechanism pass - 2026-06-13

No new verified win. This pass was deliberately OOM-safe after the prior
machine OOM: before/after solver runs, check
`ps -eo pid,ppid,comm,stat,pcpu,pmem,rss,etime,args | awk '$3=="solver" || $3=="timeout" || $3=="clingo" {print}'`.
Run local solver probes as `ulimit -v 800000; timeout <N>s
target/release/solver ...`, and keep concurrency to one or two bounded solver
processes. All probes below ended with no visible `solver` / `timeout` /
`clingo` processes.

- `solver`: `lure` and `geomlure` now accept `--maxnodes`. On timeout or node
  limit they print the best path/state seen so the search can be stopped before
  it grows into the memory cap. This is a diagnostic guard only; it does not
  change game rules.
- `tinderrectangle`: a short independent audit found a clean left-column latch:
  `<^^^vvv<<<<<<` verifies `Playing` with all 16 rats alive, player `(1,6)`,
  and lower rat `(1,4)`. This is a useful human-readable reproduction of the
  row-6 contact trap: from the latch, `>` carries the lower rat through
  `(2,6)` ... `(6,6)`, but the player remains one cell ahead; `^` has only the
  safe all-rats continuation `v`, and `rectsep` from the latch returned no
  branch under the cap. The row-3 chase `<^^^<<<<>>` also moves the lower rat
  cleanly, but opening upward lets the top pack drop and kills the player.
- `tinderrectangle`: the exact local mechanism is now clear. `ratgeom` says the
  pinned lower rat can step `(2,3)->(1,4)` only with the player at `(1,5)` or
  `(1,6)`; no one-step `(2,3)->(1,3)`, `(2,4)`, or `(3,4)` geometry exists.
  Actual branchdumps from both `<^^^` and the 58-turn safe-pocket frontier
  failed to reach those lure squares while preserving all rats, so the next
  useful idea must create a new side loop or top-pack blocker before release.
- `reload_v3`: the live-rat plank station from
  `>>>^>>>>vvv.v<^<<<v<<<<<<<^^<^<<<<<<<<v<v` is real, and both `^v` / `v^`
  variants verify with 3 rats alive. However, exact preserved-resource probes
  from both variants returned no branch for the helper rat firing trigger 1
  (`ratcell:4,18,9,22,empty`) or for changing the lower-left blocker
  (`ratcell:3,20,2,21,explosive` / `cellnot:2,21,explosive`). Park this plank
  station unless a different pre-station event changes the delivery geometry.
- `cooperation/blocked_v2`: from trigger-4-first
  `v< v^ v^ <^ <^ << <^ >v`, exact checks for
  `triggeronlycellnot:2,1,15,web`, `triggeronlycellnot:2,2,15,explosive`,
  `cellnot:6,11,explosive`, and resource-preserving `ratgone:0,15` all
  returned empty. The alternate staged prefix
  `^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v` also failed
  `triggeronlycellnot:2,1,15,web`. Current frontiers still do not provide
  trigger-2 access or a lower-left bypass.
- `cyborg_rats/ai_takeover`: Q64 still verifies, but right-middle
  cyborg-specific structural probes returned empty for `cellnot:17,8,plank`
  and `cellnot:16,8,web` while preserving at least 12 rats and a trigger.
  A stricter non-collapsing trigger-7 check,
  `triggeronlycellnot:7,18,6,explosive` with trigger 2 still reachable, also
  returned empty. Continue from a pre-Q64 structural event, not D101 cleanup.
- `release`: from the standard opener `v<vv^^>>v`, the left-trigger-2/right-web
  formulation `triggeronlycellnot:2,18,5,web` returned empty under resource
  gates. The standard opener still does not turn trigger 2 into the isolated
  `(18,4)` fix.
- `old_levels/on_the_clock`: the proposed timing point
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^` verifies at 35 turns, but diagnostics
  show `reachable_rats=0` even before the known P45 all-reachable/trigger-9
  collapse. A local frontier from that point only varies terrain collapse and
  rat drift; it does not create a rescue. Back up earlier than this timing
  family.
- `claude/gauntlet.csv`: this is a zero-rat empty room / portal-hub style level.
  Empty replay is `Playing`, and bounded `lookup` returns `NO_SOLUTION`
  immediately. Do not count it with the rat-bearing hard puzzles.

### Current run notes - 2026-06-13

No new verified wins yet. This session used one memory-capped solver process at
a time (`ulimit -v 800000`) after the previous OOM, with parallelism limited to
read-only subagent reasoning and shell reads.

- `solver`: added a bounded diagnostic goal
  `noratsrect:x1,y1,x2,y2` for `lookup` / `branchdump`. It is equivalent to
  asking that no rat/cyborg remains in an inclusive rectangle, and is intended
  for sealed-component tests like the `on_the_clock` right pocket. This is only
  a search-target helper; it does not change game rules.
- `old_levels/on_the_clock`: the previous "top/right rat pocket" blocker was
  reframed and improved. Trigger 1 can first move the top/right rat out of the
  original pocket:
  `v>>><^^` gives all 8 rats alive, top/right rat at `(19,1)`, and trigger 9
  still reachable. Holding that rat in `(19,0..2)` through trigger 9 returned
  empty, but a better bridge exists:
  `v>>><^^v>^^>>>vvv><vvvv` reaches a 23-turn state with the top/right rat at
  `(16,7)`, all 8 rats alive, and `reachable_rats=2`. From there,
  `v>>><^^v>^^>>>vvv><vvvvvv^` fires trigger 9 with all 8 rats alive and
  `reachable_rats=4`; the top/right rat is no longer the final isolated
  blocker.
- `old_levels/on_the_clock`: the new bridge route can be chained much farther
  than the stale early-trigger-9 chain:
  `v>>><^^v>^^>>>vvv><vvvvvv^^>>>>^` -> 7 rats / 3 reachable,
  `v>>><^^v>^^>>>vvv><vvvvvv^^>>>>^^^v` -> 6 rats / 2 reachable,
  `v>>><^^v>^^>>>vvv><vvvvvv^^>>>>^^^vvv><<<<<<<^<<<<v^>>>>v>>>>>>vvv^>>`
  -> 5 rats / 3 reachable with 1 explosive and 2 triggers, then firing trigger
  8 gives a triggerless 5-rat state with 3 reachable. Cleanup can continue to
  4 rats / 3 reachable, then 3 rats / 2 reachable, then 2 rats / 1 reachable.
  The best inspected 2-rat prefix is
  `v>>><^^v>^^>>>vvv><vvvvvv^^>>>>^^^vvv><<<<<<<^<<<<v^>>>>v>>>>>>vvv^>>v^<^vv>>>>>>>vv^^<<<<<<^<^^<<<<<<<^^^^^vvvvvvvv<vvvvvvvvv^^^^^<<>>>>><`,
  with rats at `(12,14)` and `(7,17)`.
- `old_levels/on_the_clock`: the new remaining blocker is not the old top rat;
  it is the sealed right component. Component analysis shows the trapped rat
  lives in cells `(12,14),(13,14),(14,14),(15,14),(16,14),(16,15),(16,16),
  (16,17),(16,18),(16,19),(17,14),(17,15)`, whose only interesting boundary is
  web `(14,15)`. Exact probes to clear `cellnot:14,15,web` from P23/P26/P35
  returned empty. `noratsrect:12,14,17,19` from P23 also returned empty. The
  route is a near-solution but needs an earlier way to kill/displace the
  bottom-right component rat before the trigger-9/trigger-8 cleanup sequence.
- `old_levels/on_the_clock`: follow-up bottom-right timing checks from P23
  showed the component rat can move to `(16,18)`, but only after the `(16,17)`
  explosive has already been spent. The exact pre-blast timing target
  `ratcell:16,18,16,17,explosive` returned empty. Backing the component-empty
  goal to `v>>><^^` with `noratsrect:12,14,17,19` also returned empty with
  either 8 or 7 rats preserved, but that rectangle is broad enough to include a
  separate bottom rat at `(14,19)`, so do not over-interpret it.
- `cyborg_rats/ai_takeover`: Q64 and Q58 both failed the concrete enemy-stage
  targets `ratat:1,16` and `ratat:2,16` while preserving trigger resources.
  Q58 also failed the mid-collar target `cellnot:16,12,web` with trigger 2
  reachable. This weakens the "enemy fires left trigger 2 from the Q64 family"
  idea; back up before Q58 if continuing that hypothesis.
- `old_levels/on_the_clock`: a narrower `noratsrect:16,14,17,19` target from
  P23 found short displacement branches such as
  `v>>><^^v>^^>>>vvv><vvvvvv^^^>`, which moves the sealed-component rat to
  `(15,14)` with all 8 rats alive, but `ratsle:7` from that P29 state returned
  empty even without a reachability gate. A stricter
  `noratsrect:15,14,17,19` target found P30 branches such as
  `v>>><^^v>^^>>>vvv><vvvv^vvv^^^`, moving the rat to `(14,14)`, but that
  branch also returned empty for `ratsle:7` with `min_reachable_rats=2`.
  Treat these as displacement diagnostics, not cleanup frontiers.
- `old_levels/on_the_clock`: trigger 6 and trigger 5 from P25
  `v>>><^^v>^^>>>vvv><vvvvvv` returned empty under preserved-rat/reachability
  gates. Adjacent pre-blast timing targets around `(16,17)` also returned
  empty: `ratcell:16,16,16,17,explosive`,
  `ratcell:15,17,16,17,explosive`, `ratcell:17,17,16,17,explosive`, and the
  earlier `ratcell:16,18,16,17,explosive`.
- `reload_v3`: initial `events` found a clean 2-rat basin without spending
  trigger/explosive resources:
  `^^^^^>>>>>>><<<<<<<<<<<<<^^<^^^^^^^^^^^vvvvvvvvv>>>>>>` leaves rats at
  `(11,11)` and `(0,21)`, all 6 explosives, 14 triggers, and the central rat
  reachable. From that basin, `cellnot:1,21,web` and
  `cellnot:2,21,explosive` returned empty while preserving useful rat counts.
  `triggeronly:7` preserving both rats returned empty; relaxed `trigger:7`
  only kills the reachable rat and leaves the bottom-left rat unreachable.
  A broader initial-board `cellnot:1,21,web` probe hit the per-process memory
  cap at `ulimit -v 800000`; rerun only with lower `--maxnodes` or a sharper
  mechanism target.
- `old_levels/on_the_clock`: corrected the over-broad right-component rectangle
  check. `noratsrect:12,14,17,18` from P23 immediately returns
  `v>>><^^v>^^>>>vvv><vvvvv`, but this only excludes row 19; diagnostics show
  the actual bottom-right rat remains at `(16,19)` in the same sealed component.
  Do not treat this as a component-empty success.
- Bounded queue `/tmp/infestation_safe_probes_20260613_085106_c.log` completed
  cleanly with no branches. Empty checks:
  `reload_v3` initial `cellnot:1,21,web` (`--maxnodes 180000`),
  `reload_v3` initial `cellnot:2,21,explosive` (`--maxnodes 180000`),
  `reload_v3` 2-rat basin `reachable:0,21`,
  `on_the_clock` P23 `cellnot:14,15,web`, and
  `on_the_clock` P23 `ratcell:16,18,16,17,explosive`.
- `old_levels/on_the_clock`: the stale 35-turn timing family is confirmed
  misleading. Two shim variants
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvvvvvv<>v` and
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvvvvv<>vv` still have
  `reachable_rats=0` and only trigger 4 reachable. The third shim
  `^>>vv>vvv<<<v^^^^^>>>^^>>^^^^^^^^^^>vvvvvvvv^vv` exposes 5 rats, but
  bounded `win` and `ratsle:7` branches returned empty; the reachable rats are
  not capturable in that terrain.
- `old_levels/on_the_clock`: a new trigger route is better. Early trigger 9
  from the initial state is possible with all 8 rats preserved; shortest useful
  prefix is `>>>v>vv^`. A small `triganylookup` then found
  `>>>v>vv^^^<^^>>>vvvvvvvvv`, which preserves all 8 rats and gives
  `reachable_rats=2`. Rat-count chaining from there produced:
  `>>>v>vv^^^<^^>>>vvvvvvvvv^^>>>>>` -> 7 rats / 4 reachable,
  `>>>v>vv^^^<^^>>>vvvvvvvvv^^>>>>><` -> 6 rats / 3 reachable,
  `>>>v>vv^^^<^^>>>vvvvvvvvv^^>>>>><>v>v<v>>>>>>>>vv` -> 5 rats / 2 reachable,
  and
  `>>>v>vv^^^<^^>>>vvvvvvvvv^^>>>>><>v>v<v>>>>>>>>vv^^<<<<<<<^^^^<vvvv>>>>>>>>vv<<<<<<<<<vv`
  -> 4 rats / 1 reachable. Continuing blindly to 3 rats spends all triggers and
  explosives, leaving the right-side rat pocket unsolved.
- `old_levels/on_the_clock`: the real remaining blocker on the early-trigger-9
  route is the top/right rat pocket. From the 4-rat state, `ratgone:17,5` only
  moves that rat to `(18,5)`. The stricter `norats2:17,5,18,5` returned empty
  from the 25-, 49-, and 88-turn prefixes while preserving contact/resources.
  Trigger 1 before trigger 9 is reachable (`>>v>>^^` then trigger 9 branches),
  but a bounded trigger-order lookup from `>>v>>^^<^v<<vvvv^` reached no
  branch with reachable rats and ended `NO_SOLUTION`. Next useful attack:
  solve or release the top/right pocket before or during the first trigger-9
  event, not after the lower cleanup chain.
- `old_levels/overstep`: the 26-turn reachable-rat lead
  `v<<^^^^^^^>>>>>>>>>>>>><>>` exposes rat `(15,17)`, and `ratsle:5` quickly
  kills it, but every first-kill state has `reachable_rats=0`. Adding
  `--min-reachable-rats 1` returns empty. Backing up to `v<<^^^^>>>>`,
  `reachablege:3` and `triggeronly:3` both returned empty under caps. Park this
  trigger-7 family unless a new earlier event appears.
- `release`: from `v<vv^^`, strict
  `triggeronlycellnot:2,18,5,web` returned empty. From `v<vv^^>>`,
  `reachable:0,16` and `cellnot:1,16,explosive` also returned empty while
  preserving rats/resources. This supports the earlier conclusion that the
  standard opener cannot make trigger 2 solve `(18,4)`.
- `old_levels/order_of_operations`: from cutoff
  `<<<<<<^v^^^^>^^vv^vv^vv`, both
  `ratplayerfacing:9,3,9,4,north` and `playerat:9,4` returned empty. From
  trigger-3-first prefix `<<<<<<^v^^^^>^^^vvvv^vv^^^vvvv`, `ratgone:9,3`
  also returned empty. The top rat still needs an earlier interrupt.
- `cooperation/handoff`: exact southeast lure
  `ratplayer:10,6,14,8` with all rats and trigger 2 reachable returned empty.
  From courier prefix `v^ >^ >^ >^ >^ >^ ^^ .v .> v> v>`,
  `cellnot:10,8,explosive` and `cellnot:11,8,explosive` returned empty with
  trigger 2 preserved. Park the P2 sweep/courier family.

### OOM-safe follow-up - 2026-06-13 later pass

No new verified win. This pass kept one capped solver process active at a time
(`ulimit -v 800000; timeout ...`) and ended each foreground probe with no
`solver` / `timeout` / `clingo` processes active.

- `old_levels/on_the_clock`: the bridge route has a new diagnostic frontier.
  From P23 `v>>><^^v>^^>>>vvv><vvvv`, suffix `^^^vvvvv` fires trigger 4
  after walking the sealed-component rat upward/leftward. The resulting P31
  `v>>><^^v>^^>>>vvv><vvvv^^^vvvvv` has all 8 rats alive, the former
  bottom-right component rat at `(13,14)`, 3 reachable rats, and triggers 6/7/8
  still present. However, from P31, `cellnot:14,15,web` and `ratat:14,15`
  returned empty. A ratdrop branch
  `v>>><^^v>^^>>>vvv><vvvv^^^vvvvv^^>>>>^` reaches 7 rats with the component
  rat at `(12,14)`, but follow-up `ratsle:6`, `triggeronly:6`, and
  `triggeronly:8` returned empty under reachability gates.
- `old_levels/on_the_clock`: the early-trigger-9 route also has a sharper map.
  Prefix `>>>v>vv^^^<^^>>>vvvvvvvvv` puts the lower-right component rat at
  `(13,14)` before cleanup, avoiding the old `(16,19)` sealed pocket. Firing
  trigger 3 immediately with suffix `^^^` gives all 8 rats alive and 5 reachable
  rats, but the top/right rat remains parked at `(17,5)`. From that trigger-3
  state, `triggeronly:6` and `triggeronly:8` returned empty. Backing up to
  `>>v>>^^`, preserved-trigger staging targets `ratat:19,6` and `ratat:19,7`
  both returned empty.
- `release`: the post-5 prefix `v<vv^^>>vv<>>>^` still looks structurally
  useful but did not produce the left-trigger-2 carrier. From that state,
  `cellnotratat:1,16,explosive,3,17` and relaxed
  `cellnot:1,16,explosive` both returned empty while preserving the right-side
  explosive column and useful rat/resource counts.
- `release`: a relaxed actor-staging probe from the same post-5 prefix did find
  a concrete branch:
  `v<vv^^>>vv<>>>^vv<<<<.<<^^^<<<>` leaves a rat at `(3,17)` with 21 rats,
  5 explosives, and 5 triggers. Appending `<` reaches `(2,16)` while keeping
  `(1,16)` explosive, but every local continuation drops that actor without
  detonating the explosive. From the `(3,17)` staged state, `cellnot:1,16`,
  `triggeronly:2`, `triggeronly:6`, and `cellnot:18,5,web` all returned empty
  under resource gates. `ratgeom` also found no one-step geometry from
  `(3,17)` to `(1,16)`, `(0,16)`, or `(0,17)`. Treat this as a useful finite
  diagnostic, not the missing carrier route.
- `old_levels/order_of_operations`: resumed the interrupted P35 top-rat stance
  checks from `<<<<<<^v^^^^>^^^vvvv^vv^^vvvv>>>>>^`. Both
  `playerfacing:9,4,north` and sibling `playerfacing:9,2,south` returned no
  branches with 4 rats / 4 reachable rats preserved. Strict trigger 8 is
  reachable from P35, but it leaves `(9,4)` as web and the top rat `(9,3)`
  isolated; strict trigger 2 and strict trigger 5 returned empty under the same
  gates. Treat the P35/P56 route as exhausted unless a new earlier interrupt
  changes the top enclosure before this state.
- `old_levels/overstep`: an event scan from the clean first lever `>>>` found
  `>>>^^` as the best immediate structural continuation, but diagnostics at
  that state still have zero reachable rats. The event removes resources without
  opening actual rat access; do not continue `>>> -> ^^` as a cleanup route.
- `cooperation/blocked_v2`: from the 20-turn three-rat frontier, the attractive
  human idea "lure lower-left rat `(0,15)` into the adjacent black holes" was
  checked by asking for reachable south lure cells. `playerat:0,17`,
  `playerat:1,17`, and `playerat:2,17` all returned empty with all 3 rats
  preserved. Note that `ratgeom` is the wrong tool for this specific check
  because it expects a rat to remain at the target, while black holes remove it.
  The lower-left pocket still requires trigger/explosion access, not a
  south-side player lure from this frontier.
- `cyborg_rats/ai_takeover`: Q41
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>` has the upper trigger-7 cluster
  live and high player reachability. The intended alternate "enemy upper-7"
  checks `cellnotratat:16,12,web,15,9` and
  `cellnotratat:16,12,web,16,9` both returned empty. The bottom-actor staging
  target `ratcell:4,19,2,19,explosive` also returned empty under resource gates.
- OOM-safe serial queue `/tmp/infestation_safe_bg_test_fg.log` completed under
  `RLIMIT_AS=800MB` with no solver processes left active. Empty checks:
  `blocked_v2` B20 `ratsle:2` with the two survivors reachable and no trapped
  rats; `ai_takeover` Q64 `cellnot:14,12,explosive` and
  `cellnot:15,12,explosive` while preserving 12 reachable rats and a trigger;
  `tinderrectangle` T106 safe-door checks for both `(3,4)` and `(2,4)` webs;
  and `reload_v3` trigger-2 prefix `triggeronly:6`.
- `cyborg_rats/ai_takeover`: the same queue reproduced the known Q64
  `triggeronlycellnot:2,18,4,cyborg` family. Best branch
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<<<<<<<<<<>>>>>>>><<<<<<<<<<<<<<^^^<<<v`
  reaches 7 rats/cyborgs, all reachable, no triggers, and explosives at
  `(14,12)` / `(15,12)`, but it is a hard contact trap: from player `(0,17)`,
  every immediate action (`^`, `v`, `<`, `>`, `.`) is `GameOver`. A capped
  `lookup --goal win` from that exact state returned `NO_SOLUTION`
  immediately. Continue from pre-trigger Q64 structure, not this 7-rat basin.
- OOM-safe tmux queue `/tmp/infestation_safe_bg_20260613_093208_live_tmux.log`
  also completed with one solver at a time and no leftover processes. Empty
  structural checks:
  `old_levels/order_of_operations` initial `ratgone:9,3` with at least 5 rats
  and 4 reachable rats; `old_levels/on_the_clock` from `>>v>>^^` for
  `norats2:17,5,18,5` with all 8 rats and 5 reachable rats;
  `release` standard opener `v<vv^^>>v` for `cellnot:18,5,web` preserving
  23 rats / 20 reachable rats; `cooperation/handoff` initial
  `cellnot:10,5,web` preserving all 5 rats; and
  `cooperation/tug_of_war` initial `cellnot:7,1,web` preserving all 7 rats.
  These rechecks further support the current model that the hard blockers need
  different earlier mechanisms, not wider searches on the known frontiers.
- Added solver diagnostic mode `ratdeathgeom` for human-style local trap checks:
  `solver ratdeathgeom <csv> [prefix] --source x,y [--target x,y] --kind blackhole|explosive|any`.
  It enumerates synthetic player stances where a stalled turn removes a source
  rat without killing the player. This is useful for distinguishing real lure
  geometry from unreachable cleanup ideas.
- `cooperation/blocked_v2`: `ratdeathgeom` confirms the lower-left rat `(0,15)`
  can be lured into the adjacent black-hole row only from synthetic stances such
  as player `(13,16)` / `(14,16)` or `(0,17)` / `(1,17)`. Capped real-state
  `playerat` checks for those cells from the B20 frontier returned empty, so the
  black-hole self-delete idea is geometrically real but dynamically unreachable
  from the current best frontier.
- `tinderrectangle`: `ratdeathgeom` confirms the exact row-6 winning geometry:
  from P137, if the lower rat is at `(2,6)` and the player is safely at
  `(14,7)` or `(14,8)`, stalling detonates the rectangle and wins. The same is
  true one chase step later with the lower rat at `(3,6)`. However, capped
  `ratplayer` checks from P137/P138 for `(rat,player)=(2,6,14,7)`,
  `(2,6,14,8)`, and `(3,6,14,7)` all returned empty. The missing trick is now
  very narrow: create that right-pocket separation while the lower rat stays on
  row 6.
- `tinderrectangle`: tightened the separation target further with
  `ratrectplayerrect:2,6,7,6,14,7,14,8` (lower rat anywhere on the viable
  row-6 strip, player in the exact safe pocket). From both T106 and earlier
  T58, capped branchdumps preserving all 16 rats returned empty. This rules out
  "same route, better end choreography"; the route needs a different door timing
  before T58 or a different lower-rat release geometry.
- `old_levels/on_the_clock`: applied `ratdeathgeom` to the early-trigger-9 plus
  trigger-3 state `>>>v>vv^^^<^^>>>vvvvvvvvv^^^`; the top/right pocket rat at
  `(17,5)` has no one-step black-hole/explosive self-delete geometry under the
  checked synthetic stances. This supports the existing conclusion that the
  pocket must be released structurally before/during the trigger-9 route.
- `cooperation/blocked_v2`: a new safe rat-trigger-4 variant exists. From the
  B17 prefix
  `^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v ^v v< ^> v>`, two stalls
  (`.. ..`) hold P2 on the x=15 file so the lower transient rat steps onto
  trigger 4, while P1 stays out of the top-row blast. Adding one more stall
  self-deletes that rat and leaves the familiar 3-rat shape, but with the top
  explosive strip cleared:
  `^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v ^v v< ^> v> .. .. ..`.
  Capped follow-ups from that state for `cellnot:1,15,web`,
  `triggeronly:2`, `playerat:1,17`, `playerat:0,17`, and a bounded `win`
  lookup all returned empty. Treat this as a real but insufficient mechanism:
  trigger 4 is no longer the missing step unless it is combined with an earlier
  lower-left access change.
- `old_levels/on_the_clock`: late cleanup from the promising 5-rat bridge state
  is now bounded more tightly. From
  `v>>><^^v>^^>>>vvv><vvvvvv^^>>>>^^^vvv><<<<<<<^<<<<v^>>>>v>>>>>>vvv^>>`,
  a capped direct `win` lookup reached only the same two-rat basin: a reachable
  left rat and the sealed right-component rat. Exact P69 checks for
  `noratsrect:12,14,17,19` and `cellnot:14,15,web` returned empty, even when
  rat preservation was relaxed. The bridge route is still the best near-solve,
  but the right-component rat has to be displaced or killed earlier than P69.
- `old_levels/on_the_clock`: a side analysis found a sharper P66/P69 failure
  mode. From P66
  `v>>><^^v>^^>>>vvv><vvvvvv^^>>>>^^^vvv><<<<<<<^<<<<v^>>>>v>>>>>>vvv`,
  `ratslecellnot:5,13,14,rat` can avoid the exact P69 parking cell, but every
  returned branch spends all triggers/explosives and still leaves the right
  component rat sealed (for example at `(14,14)`). Requiring `--min-triggers 1`
  makes that target empty. The companion trigger-8 inversion target
  `triggeronlycellnot:8,12,14,rat` from P56 also returned empty. This closes
  the "same bridge, different final cleanup timing" idea; continue before the
  trigger-9 bridge commits the right-component rat to the 12-cell pocket.
- `old_levels/on_the_clock`: the P23/P24/P25 bottom-right trigger mechanism is
  now mapped. From P23
  `v>>><^^v>^^>>>vvv><vvvv`, suffix `v` makes the rat at `(11,12)` fire trigger
  2, which clears `(16,18)` while preserving all 8 rats. One more `v` moves the
  bottom-right component rat to `(16,18)` and makes trigger 6 player-reachable,
  but the next rat wave forces trigger 9 before trigger 6 can be used. Exact
  P23/P24/P25 checks for `triggeronlycellnot:6,13,19,web` returned empty, even
  relaxed. The all-rats P25 side branch `^^^>` moves the component rat to
  `(15,14)`, but it is a contact trap: only `>` survives, and every action from
  the resulting P30 state is `GameOver`.
- `old_levels/order_of_operations`: the P35 route
  `<<<<<<^v^^^^>^^^vvvv^vv^^vvvv>>>>>^` was rechecked with direct cleanup and
  exact blocker targets. A capped `win` lookup from P35 reached the same
  two-rat basin and returned `NO_SOLUTION`. Exact `ratgone:9,3` branchdumps
  from P35 returned empty both with and without the `--min-reachable-rats 3`
  viability gate, so the top rat is not removable from this frontier in the
  tested bounds. Continue before P35; do not spend more time on late cleanup.

### OOM-safe continuation - 2026-06-13 current pass

No new verified win. This pass kept main-thread solver probes serial, wrapped
with short `timeout` and `ulimit -v 800000` caps where search was involved, and
checked the process table between probes. `clingo` is not installed in this
workspace (`clingo` binary absent and Python `import clingo` fails), so no ASP
encoding was attempted in this pass.

- `cyborg_rats/ai_takeover`: re-expanded the Q64
  `triggeronlycellnot:2,18,4,cyborg` family with a bounded raw branchdump. It
  found all-reachable 4-rat triggerless branches, but these are still contact
  traps. Representative branch A
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<<<<<<<<<<<<<>>>>>>>>>>><<<<<<<<<<<<<<^^^<<<v`
  has player `(0,17)` and rats/cyborgs `(18,5)`, `(11,9)`, `(1,16)`,
  `(2,16)`; every immediate action is `GameOver`. Sibling branch B
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<<<<<<<<<<<<<>>>>>>>>>>><<<<<<<<<<<<<<^^^<<<<`
  allows `v`, `<`, and `.` tempos, but capped `ratsle:3` and `win` lookups
  returned empty. A side audit also confirms the Q3/Q6 cleanup basin is blocked
  by a fixed two-cell local-cyborg shadow: the player can enter row-16 webs, but
  the cyborg moves into the newly cut web cell on the next westward progress
  step. Continue before Q64/Q6 if pursuing this level; look for a wider local
  cyborg offset or lower-pocket redirection before stepping on trigger 2.
- `release`: current-rule replay sharpened the lower-carrier failure. With
  `BASE=v<vv^^>>vv<>>>^vv<<<<.<<^^^<<<`, `BASE+><` verifies `Playing` at
  32 turns with the lower actor at `(2,16)`, explosive `(1,16)` still intact,
  right explosive stack `(18,6..8)` intact, and triggers 2/6 still present.
  Immediate moves delete the actor without detonating `(1,16)`, and a capped
  `cellnot:1,16,explosive` branchdump from that state returned empty. `BASE+>`
  gives the documented `(3,17)` actor, but that branch also remains finite. The
  actor can be carried right only to about `(5,17)` before the player hits the
  row-11 wall at x=6. Side checks from `BASE+>` for
  `ratplayer:18,17,19,19` and even `ratat:14,17` returned empty. The current
  carrier family can stage an actor beside the explosive but cannot convert it
  into the left-trigger-2/right-stack mechanism.
- `reload_v3`: the pre-trigger-1 helper station is real but finite. From
  `P41=>>>^>>>>vvv.v<^<<<v<<<<<<<^^<^<<<<<<<<v<v`, `^` safely chews plank
  `(4,17)` and `v` safely chews `(4,19)` before trigger 1 fires. A bounded
  frontier from `P41+^` exhausted after 21 nodes / 15 states: the helper can
  move through `(3,17)`, `(3,18)`, `(3,19)`, `(2,16..18)`, and `(1,16..18)`,
  but attempts to push farther toward the bottom-left explosive become contact
  deaths. Capped `cellnot:2,21,explosive` from `P41+^` returned empty. Treat
  this as a real local mechanism that still needs a different earlier setup,
  not as a reload-order continuation.
- `tinderrectangle`: a side audit found a new local latch but ruled it out. At
  P115 from the known P135 family, suffix `<>` safely opens `(3,3)` and moves
  the lower rat to `(3,3)` while the player returns to `(4,3)`. However,
  `P115<> >v` dies on diagonal contact, and bounded checks found no branch to
  `rectsep` or even to a live `ratat:4,4` state. This closes the tempting
  east-door variant; the level still needs an earlier release-spacing change.
- `old_levels/on_the_clock`: a side audit sharpened the P23/P25 tempo blocker.
  `P23=v>>><^^v>^^>>>vvv><vvvv`; `P23+v` lets the central rat fire trigger 2,
  and `P23+vv` puts the lower-right component rat at `(16,18)` while trigger 6
  is player-reachable. The intended-looking plan is to use trigger 6 to detonate
  the `(12,19)` explosive before the trigger-9/trigger-8 cleanup, but at P25 the
  top rat is already at `(14,9)`, one move from trigger 9 at `(13,9)`. Ordinary
  movement toward trigger 6 lets trigger 9 fire first. `ratdeathgeom` from P25
  found no one-step explosive/black-hole self-delete for `(16,18)`, and a
  bounded P23 check for `ratcell:13,18,12,19,explosive` with trigger 6 reachable
  returned empty. The next useful idea must diverge before or at P23 to delay
  the top trigger-9 rat or pre-stage the lower-right rat earlier.

### OOM-safe continuation - 2026-06-13 follow-up

No new verified win. This pass used serial solver probes with
`ulimit -v 800000` and short `timeout` wrappers after the previous machine OOM;
parallelism was limited to read-only mechanism audits. Process checks found no
leftover `target/release/solver`, `timeout`, or `clingo` jobs after the probes.

- Inventory check: `solver/solutions/final_solutions.json` still contains 35
  stored solutions. The current missing CSV inventory is 11 files, but
  `claude/gauntlet.csv` has zero rats. The remaining rat-bearing set is the 10
  hard CSVs: `cooperation/blocked_v2.csv`, `cooperation/handoff.csv`,
  `cooperation/tug_of_war.csv`, `cyborg_rats/ai_takeover.csv`,
  `old_levels/on_the_clock.csv`, `old_levels/order_of_operations.csv`,
  `old_levels/overstep.csv`, `release.csv`, `reload_v3.csv`, and
  `tinderrectangle.csv`.
- `old_levels/on_the_clock`: the P23/P25 trigger-6 rescue is now closed more
  tightly. From `P20=v>>><^^v>^^>>>vvv><v`, exact pre-stage checks returned no
  branches for the lower-right rat at `(16,18)` with the player already near
  left trigger 6 (`ratplayer:16,18,5,7`, `ratplayer:16,18,5,8`, and
  `ratplayer:16,18,4,7`) while trigger 6 was reachable and at least 13 triggers
  remained. The delayed-upper-rat variants also returned empty:
  `ratcell:16,18,15,8,rat` and `ratcell:16,18,16,7,rat`. A rat-trigger-6
  bypass target `triggeronlycellnot:6,12,19,explosive` returned empty, as did
  relaxed direct staging `ratcell:13,18,12,19,explosive` from P20 and P21. This
  rules out the obvious "same bridge, better tempo" fixes; continue before the
  P20/P23 bridge or find a different bridge route entirely.
- `release`: the "handle `(18,4)` before the top sweep" hypothesis was checked
  directly from the initial board. `cellnotratat:18,5,web,18,4` returned empty
  with all 24 rats preserved, and also empty when allowing one unrelated rat
  loss. The stronger adjacent target `cellnotratat:18,6,explosive,18,4` also
  returned empty. This closes the simple pre-sweep right-door variant; do not
  retry it without a new mover or trigger-order reason.
- `cyborg_rats/ai_takeover`: the Q64 wider-offset idea is real but still
  finite. From
  `Q64=^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>^^>>><<>vvv>vvvv^^vvv<`,
  `triggeronlycellis:2,3,17,cyborg` found reachable branches, best:
  `Q64+<<<<<>>>><<<<<<<<<<<<<<^^^<<<`. It verifies `Playing` at 93 turns with
  11 enemies, 2 explosives, 0 triggers, all 11 enemies reachable, player
  `(0,16)`, and enemies at `(18,4)`, `(11,9)`, and `(2..7,16..19)` along the
  lower-left lanes. Unlike the older Q64 4-rat branch, this is not immediate
  `GameOver`; however, bounded `ratsle:10` checks from that exact state returned
  empty even relaxed to depth 60 / 120k nodes. Treat it as an improved diagnostic
  frontier, not a cleanup path.
- `cyborg_rats/ai_takeover`: the safe-trigger-7 family was checked as a shaping
  route rather than a trigger-order continuation. The live prefix
  `^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>^>^^` verifies with 14 enemies,
  6 explosives, 7 triggers, and 13 reachable enemies. Some old collar targets
  such as `(16,12)` and `(14,12)` are already changed by this prefix, so they
  are not meaningful follow-ups. The remaining right-side stack targets
  `cellnot:18,6,explosive` and `cellnot:18,8,explosive` both returned empty
  under 35-depth / 120k-node caps while preserving at least 12 enemies, 10
  reachable enemies, and 5 triggers. Safe-7 still needs a different concrete
  target before it is worth revisiting.
- `tinderrectangle`: quick one-step `ratdeathgeom` checks for the top-corner
  ignition idea found no candidate synthetic stances from adjacent top rats
  `(1,1)->(0,0)` or `(15,1)->(16,0)`, nor for lower rat `(1,6)->(0,6)`.
  This does not disprove a multi-step top lure, but it confirms there is no
  simple local corner-ignition stance to replace the known row-6 separation
  problem.
- `overstep`: a fresh look at the clean first lever `>>>` and a short `events`
  scan reconfirmed the documented dead pattern. The best immediate event
  `>>>^^` leaves six rats, three explosives, three webs, 15 triggers, and still
  zero reachable rats. Do not continue the `>>> -> ^^` line as a cleanup route.
- `cooperation/blocked_v2`: the B20 three-rat frontier was checked from the
  right side of the lower-left pocket as well as the previously documented left
  blockers. From
  `B20=^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v ^v v< ^> v> ^< v< ^^`,
  direct `cellnot:4,15,web` returned empty while preserving three rats, two
  reachable rats, and four triggers. The strict trigger-4 variant
  `triggeronlycellnot:4,4,15,web` also returned empty. The lower-left rat is
  blocked from both sides of the `(1..4,15)` barrier from this frontier; keep
  looking before B20 or for a different actor/trigger path, not for another
  local B20 cleanup.
- `old_levels/order_of_operations`: the top-rat blocker was checked at the P30
  cutoff as an earlier alternative to the exhausted P35/P56 route. With
  `P30=<<<<<<^v^^^^>^^vv^vv^vv^^^vvvv`, all four remaining rats are still
  reachable in diagnostics, but `cellnot:9,4,web` returned empty under a
  four-rat / four-reachable-rat gate, and the direct stance
  `ratplayerfacing:9,3,9,4,north` also returned empty. The initial synthetic
  `ratdeathgeom` check for `(9,3)` found no one-step explosive/black-hole
  self-delete. Continue before P30 or with a route that changes the top enclosure
  before the player is committed to the lower-left cleanup.

### Methods already tried (do not repeat blindly)
- Generic heuristic search (gbfs/astar, weights 1-10, `PROGRESS_H`, depth
  400-500, long budgets) solved several levels but stalled on the current
  10-level inventory.
- PDDL and LLM-play agents produced no verified final wins on the current hard
  set. Their useful output was mechanism hints, not move strings.
- The current productive path is mechanism-first decomposition plus short
  oracle checks.

---

## 4. Planned next steps (start here)

1. **Do not repeat broad direct searches.** The grid-step/hash speedup is already
   in the tree, but the current hard cases still fail because the heuristic
   prefers irreversible dead basins. Use mechanism-specific goals and inspect
   diagnostics after every irreversible event.
2. **Continue `release` from a new hypothesis, not the trigger-5/6 family.**
   The known prefix `v<vv^^>>v` plus trigger 5 can reduce the board to a single
   `(18,4)` rat, but that mechanism strands it behind `(18,5)`. Recent bounded
   checks also failed to reach trigger 6 in a way that preserves reachable
   trigger 2. The next useful attack must handle `(18,4)` before the top sweep
   or open its web without spending the adjacent explosive chain.
3. **Solve `tinderrectangle` as a separation problem, not an ignition problem.**
   The ignition geometry is proven; use `--goal rectsep` to reject the known
   contact trap. The next useful hypothesis must create a side loop or delayed
   release before `(2,4)` opens, because opening `(2,4)` starts the lower rat
   immediately and the current row-6 route loses the timing race.
4. **Continue `ai_takeover` from Q64, not D101 cleanup.** Q64 is the current
   best preserved frontier before trigger 2; D101 improves the triggerless basin
   to 7 rats but still has no cleanup branch in bounded checks. Use `release`
   for trigger vocabulary only, not as a move skeleton.
5. **For two-player levels, work in `wp2` waypoint pairs.** Start with
   structural access checks (`cellnot` / `playerat`) before trigger
   choreography. `tug_of_war` may be unwinnable as authored because the top
   pocket has no legal web/plank-clearing event; `handoff` still needs a
   mechanism for the `(10,6)` sealed rat before the P2 sweep.
6. **`blocked_v2`:** rigorously test winnability before assuming solvable.

---

## 5. File map

```
HANDOFF.md                          ← this file
levels/claude/                      5 verified puzzles + gauntlet portal hub + README
solver/                             Rust oracle crate
  src/main.rs                         all modes: solve / verify / trace / wp
  Cargo.toml
solver/solutions/
  SOLUTIONS.md                       28 verified original solutions + 5 Claude puzzles
  final_solutions.json               machine-readable verified set
  autoplay.js                        browser console auto-player (1p + 2p)
  results/                           raw search outputs (results*.json, autoplay_data.json)
  tools/                             orchestration + verification scripts
    build_final.py, verify.py, gen_docs.py, run_all.py, run_failures.py, run_pass3.py
    scratch/                         historical Python engine (BUGGY) + agent IDA*/sim experiments
```
> Tooling scripts have paths hard-coded to the session workspace (`/tmp/infestation`); adjust on resume.

## 6. Session context
- A session Stop-hook with goal **"solve all the puzzles"** may be active in
  some environments. The current remaining hard set is the 10 rat-bearing CSVs
  listed in §3 plus the additional old-level CSVs there. Resume by working §4.
- Fork created with `gh repo fork`; push with `gh auth setup-git --hostname github.com` then
  `git push fork claude/new-puzzles`. No PR was opened to upstream (`davidspies/infestation`).
