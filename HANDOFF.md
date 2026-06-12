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

### Methods already tried (do not repeat blindly)
- Generic heuristic search (gbfs/astar, weights 1-10, `PROGRESS_H`, depth
  400-500, long budgets) solved several levels but stalled on the current 8.
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
2. **Continue `release` from a new hypothesis, not the trigger-5/6 one-rat
   family.** The known prefix `v<vv^^>>v` plus trigger 5 can reduce the board to
   a single `(18,4)` rat, but that mechanism strands it behind `(18,5)`. The next
   useful attack must handle `(18,4)` before the top sweep or open its web
   without spending the adjacent explosive chain.
3. **Solve `tinderrectangle` as a separation problem, not an ignition problem.**
   The ignition geometry is proven; use `--goal rectsep` to reject the known
   contact trap. The next useful hypothesis must create a side loop or delayed
   release before `(2,4)` opens, because opening `(2,4)` starts the lower rat
   immediately and the current row-6 route loses the timing race.
4. **Do not treat `ai_takeover` as a solved-by-`release` clone.** It has the
   right-side `(18,5)` opening that `release` lacks, but the standard opener
   still cannot reach or safely use triggers 7/8. Use `release` for trigger
   vocabulary only, not as a move skeleton.
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
- A session Stop-hook with goal **"solve all the puzzles"** may be active in
  some environments. The current remaining hard set is the 8 listed in §3.
  Resume by working §4.
- Fork created with `gh repo fork`; push with `gh auth setup-git --hostname github.com` then
  `git push fork claude/new-puzzles`. No PR was opened to upstream (`davidspies/infestation`).
