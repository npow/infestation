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
2. **Continue `release` from the known prefix.** Start with
   `v<vv^^>>v` (triggers 3 then 4). Test trigger 5 via suffix `v<>>>^`, then
   look for a way to make trigger 2 change indirectly; direct player routes to
   trigger 2 and trigger 6 have failed dynamically.
3. **Solve `tinderrectangle` as an ignition lure.** The winning geometry is
   either a corner rat at `(0,0)` / `(16,0)` with the player on row 3, or the
   lower rat at `(2..6,6)` with the player on the far-right safe cells. The next
   attack should hold the lower rat in that band while crossing right; existing
   safe-side branches let it drift to `(8,6)`.
4. **Use `release` as the template for `ai_takeover`.** Once `release` is
   solved, port its trigger order to `ai_takeover` and account for trigger 7/8
   and cyborg pathing.
5. **For two-player levels, work in `wp2` waypoint pairs.** Start with
   structural access checks (`cellnot` / `playerat`) before trigger choreography;
   `tug_of_war` currently needs a mechanism for the top rat component, and
   `handoff` needs a mechanism for the `(10,6)` sealed rat before the P2 sweep.
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
