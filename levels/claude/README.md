# Claude's Gauntlet — new Infestation puzzles

Five hand-designed single-player levels for [Infestation](../../README.md), each built
around **one distinct primitive ("trick")**. Every level here was **verified against the
real game logic** (the Rust engine compiled as a native oracle) — each is solvable, and
for the lure puzzles the *naive straight rush is verified to fail*, which is what makes
the trick necessary rather than decorative.

Enter them from the hub `gauntlet.csv` (`claude/gauntlet`), which portals to all five.

Cell legend: `#` wall · `.` floor · `w` web · `O` black hole · `X` explosive ·
`R` rat · `1` trigger · `◄▲►▼` you (facing).

| Level | Primitive / trick | Why the obvious move fails |
|---|---|---|
| **Sacrifice** | explosive-lure | Walking at the rat means stepping on the `X` — which kills *you*. You must let the rat walk onto it. |
| **Roach Motel** | black-hole lure | The direct path is through the hole; it swallows you too. Bait the rat in instead. |
| **Stampede** | crowd hole-lure | One hole, a whole pack — funnel them all in by holding position; you can't cross the hole. |
| **Remote Detonator** | twin-trigger remote chain | The rats are boxed where you can't reach them and you can't touch the `X`. Step the *far* trigger; its twin's zap detonates the chain. |
| **Web Lair** | web-shield + sword-facing | You're invulnerable inside the webs (rats can't enter). Step out only when your sword faces the rat. |

## Verified solutions

Replay any of these with the bundled solver:
`solver verify levels/claude/<level>.csv "<moves>"` → prints `result=Won`.
(`^`=up `v`=down `<`=left `>`=right `.`=stall)

| Level | Solution | Moves |
|---|---|---|
| Sacrifice | `<>` | 2 |
| Roach Motel | `^^` | 2 |
| Stampede | `^^^` | 3 |
| Remote Detonator | `^<<<<v` | 6 |
| Web Lair | `^^vvvv` | 6 |

## How they were verified

The `solver/` crate links the game's actual logic (`infestation::testing`) and exposes:
- `solver solve <csv>` — A*/greedy search over the real engine to find a winning line.
- `solver verify <csv> "<moves>"` — replay a line, print the true `PlayState`.
- `solver trace <csv> "<moves>"` — print the board after every turn (to watch rat reactions).

Because the oracle *is* the shipped game logic, "verified" here means it genuinely wins
in the real game — not in an approximate re-implementation.
