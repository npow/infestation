# Infestation — verified solutions

All move-strings below were **verified against the real game engine** (`solver verify`) and print `result=Won`.

Keys: `^`=up `v`=down `<`=left `>`=right `.`=stall. Two-player turns are space-separated `P1P2` pairs.


## Original levels (37 solved)

| Level | Players | Moves | Solution |
|---|---|---|---|
| `blackhole_v2.csv` | 1 | 77 | `↓↓↓→→→→→→←←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→→→→→→→←←←←←←←↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑→→→→←←↓↓→→→→↓↓→` |
| `chase.csv` | 1 | 199 | `↑→→→↓↑↑↑→↑↑→→→→→↓→→↓↓↓↓↓↑↑↑↑↑↑←←←←↓←←←↓↑←↑←↑←←→→→→↓→→→→→↓↓↓↓↑↑↑↑←←←↑←↑←←↑↑↑↑↑↑↑↑←←←←←↑↑↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→→↓↓↑↑←←↓↓↓↓↓←→→→→→→→→→→→→→→↑↑↓←→↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑←←←←→→↓↓↓↓←↓↓↓↓↓↓↓↓↓↓↓↓←↓↓↓←←←←←←←←←←←←←←←` |
| `cooperation/coop_world_v3.csv` | 2 | 26 | `↑↑ ↑→ ↑↑ ↑↑ ↑↑ ↑↑ ↑↑ ↑→ ↑↑ ↓→ ←→ ↑→ ↑→ ↑→ ↑→ ↑→ ↑→ →↓ ←. ↑↓ →← ↓← .→ .↑ ↑↑ ↑↑` |
| `cooperation/cooperation.csv` | 2 | 28 | `↑↑ ↑→ ↑↑ ↑↑ ↑← ←↑ ↑→ ↑→ ↑→ ←→ ←↓ ←→ ←→ ←→ ←→ ↑→ →→ →→ ←→ ←← .← ←↓ ↓↓ ↓↓ ↓← ↓← ↓↓ ↓↓` |
| `cooperation/handoff.csv` | 2 | 85 | `vv >^ >^ >^ >^ >^ ^^ .v .v .> .> .> v> v< << ^< ^< ^> ^^ ^^ ^^ ^< ^< ^> <> >> .> v> >v >> >> >^ .^ >< ^< ^^ ^v >< ^^ ^^ ^^ ^v ^v ^> >> vv vv <v <> <^ v^ <^ v^ v^ v^ ^< ^< ^. >^ ^> >^ >> v< v. v> <v v< v. v^ <> <^ <> <v ^^ << <v <> << << ^< ^< << << ^> ^<` |
| `cooperation/tug_of_war.csv` | 2 | 98 | `^v vv >< vv vv vv vv <> <> <> <> <> >v << vv >> vv vv << >> << >> ^^ <^ ^^ >^ <v ^> ^< >^ >^ ^> ^^ ^< >< >< ^^ ^< ^> ^v ^^ ^^ ^^ ^^ ^^ >^ >^ ^^ ^^ v^ v^ <^ v^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ >^ >^ ^^ ^^ ^^ ^^ ^< ^< ^^ ^^ ^> ^> ^> ^> ^v ^v ^> ^> ^> ^^ ^^ ^> ^v ^v ^v ^v ^v ^v ^< ^< ^< ^< ^v ^v` |
| `cyborg_rats/cyborg_rats.csv` | 1 | 32 | `↑→→↑↑←←←→→→↑↑←←←→→→↑↑←←←→→→↑↑←←←` |
| `cyborg_rats/fakeout.csv` | 1 | 78 | `↑↑↓......→→←↓←↓................←←←→←←←←←→←→→←→↑→↑→↓←←↓↓↓↓↓↓↓↓↓↓→↑→↑↑→→→→→→→→→→` |
| `cyborg_rats/stalemate.csv` | 1 | 47 | `↑→→↑→↑→→↓↓←↓↓→←↑↑→↑↑→→→..............→←←←←←←←←←` |
| `cyborg_rats/unguided.csv` | 1 | 106 | `↓←←←←←←←←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓→↓↓→→↑←←↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑→→→→→→→→→→→→→→→↑↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→↓↓↓↓←←←←↑↑←←←←←←↑↑→→→→→→` |
| `explosives.csv` | 1 | 54 | `↓↓↓←←↑↑↑↑←←↓↓↓↓↓↓→→→→↓↓←←←←←→→→→→↑↑←←←←↑↑↑↑↑↑↑↑→→→→→→→` |
| `explosives2.csv` | 1 | 53 | `↑←↓↓↓↓↓↓→→→→→←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→←↑↓↑↑↑↑↑↑↑↑→→→→→→→` |
| `gimmicks/crushkill.csv` | 2 | 10 | `<> <> <> <> >^ ^v ^v ^< ^< ^<` |
| `gimmicks/platform.csv` | 1 | 29 | `vvv>vvv..^^.^^^^^^^^^<>>>>vvv` |
| `gimmicks/robotic_cheese.csv` | 2 | 7 | `>> ^> ^. ^. <. <^ ^^` |
| `guidance.csv` | 1 | 126 | `←←←←←←←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→→→←↑→→→→→←←←←←←←←←←←←←←↑↑↑↑↑↑←↓←↓→↑↑↑↑↑↑↑↑↑↑↑→→→→→↑↑↑↑↑↓↓←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→↑↑→→→→→→` |
| `intro.csv` | 1 | 54 | `↑↑↑→→→→↓↓↓←→↑↑↑←←←←↑↑↑↑↑↑←←↓↓↓↓↓←↓←↓↓↓↑↑↑→↑↑↑↑↑↑→→→↑→→` |
| `limited2.csv` | 1 | 149 | `→→→→→→→→→→→→→→←←←←←←←←←←←←←...←→.→→→→→→→→→→→→→←←←←←←←←←←←←←...←→....→→→→→→→→→→→→→←←←←←←←←←←←←←...←→→→→→→→←←←←←←←←←←←↑↑↑↑↑↑↑↑↑↑↑↑→→→→→→→→↑↑↑↑↑↑↑↑↑→→→→` |
| `lock_in.csv` | 1 | 69 | `↑↑→↓↓↓←↓↓←←↓←↓↓→↓←↑↑↑↑↑↓↓↓↓↓↓↑↑→→↑→→→→→↑↑↑↑↑↑↑↑↑↑↑↓←↓→←←←↑↑←←←←←←←←←←` |
| `more_rats.csv` | 1 | 29 | `↓←←→←→←→←→←→→↑↑↑↑↑↑←←→←→←→←←←` |
| `no_retreat.csv` | 1 | 76 | `↓←↑←←←←↑←→↑↑→→→↑↑←←↑←←←←→→→.←←←→←↑↑→→→→→→→→↑↑←←←←←←←←↑↑→→→→→→→→→→→→↑→→→→↑→→→` |
| `old_levels/old_levels.csv` | 1 | 9 | `↓↓↓↓←←←←←` |
| `old_levels/order_of_operations.csv` | 1 | 87 | `<^v^^<^^^>^^<vv^vv^^^^^^^^^^>^v>vv<vvvvvvvv>>>>>^^^^^^^^^^^^^^^>>>>>vvvvvvv>vvvvvvv>>vv` |
| `old_levels/overstep.csv` | 1 | 167 | `vvvvvv>>>>>>>>>v>><>><<^v^>>>^>^>^^^^^^^^^^^<<<<<>>>v>v>vvvvvvvvvvv<<<><<<<^<<<<^^<<^<<<^^<<^^^^^^>>>>>>>v>>>^^<<vvv<<>^^<<<<<vvv<<vvv>>vvvv<<<vv^^>>^^^^>^>>v^^^^^^^>>` |
| `order_of_operations_new_v2.csv` | 1 | 36 | `↓→→→→↑→↑↑↑→→↓→.....→→→↓←↓↓↓↓↓↓↓↓↓↓↓↓` |
| `planks.csv` | 1 | 100 | `→→→→←←←←←←←←←←↑↑↑↑↑↑←↓↓↓↓↓↓→→→→→→→→→→→→↑↑↑↑→↑↑↑↓↓↓←↓↓↓↓←←←←←←←←←←←←↑↑↑↑↑↑↑→→→↓→↓↓↓↓→→→→→→↑↑←↑↑↑←←←↓↓` |
| `rats.csv` | 1 | 13 | `↓←←↑↑↑→→→→←→↑` |
| `synchronicity.csv` | 1 | 50 | `→→↑↓→→↓↓↓←←↓↑↑↑↑←↓↑↓↑↓↑↑↑↓↓→↓↓↓→↓↓→→←←↑↑→→→→→→↑↑↑↑` |
| `tinderbox.csv` | 1 | 85 | `^>^^^^^<<<vv<<>.<>>>>^>v<<>>>v<v.><^v>><^^v<>^^<..v.^v^<v.>.vv^v<v..v^<^<<<<.vv^<^v>>` |
| `tinderbox_v2.csv` | 1 | 85 | `↑→↑↑↑↑↑←←←↓↓←←→.←→→→→↑→↓←←→→→↓←↓.→←↑↓→→←↑↑↓←→↑↑←..↓.↑↓↑←↓.→.↓↓↑↓←↓..↓↑←↑←←←←.↓↓↑←↑↓→→` |
| `trapped_rat.csv` | 1 | 75 | `↓↓↓↑↓.→→←→→→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑↑↑↑↑←` |
| `trapped_rat2_v2.csv` | 1 | 78 | `←↑↑↑↓→←↑↓→←↑↓→←↑↑←←←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↑↑↑↑↑↑→` |
| `triggering_explosives_v3.csv` | 1 | 72 | `←↓↓↓↓←←↓↓↓↓←←↓↑→→↑↑↑↑→→↑↑↑↑→→↓↓↓↓→→↓↓↓↑↑↑←↑↑→←↑←↑←←↑↑↓↓←←↑←←↓↓↓→↑↑↑→→←→↓` |
| `triggers.csv` | 1 | 71 | `↓↓↓→→↑↑→→↓↓→→→→↑→↑↑←↑←←←↑↑→→→→↑↑←←←→→↑↑←↑↑←←↓↓↓←↓←↓↓↓↓←←←←←↑↑↑↑↑↑↑→→→↓↓` |
| `triggers2.csv` | 1 | 104 | `↑↑→↑→↑↑↑↓↓↓←↓←←←←←↑↑↑↑→←→←↓↓↓↓→→→→→↑→↑↑↑↑↑↓↓↓↓↓→→↑↑↑↑→→↓→↓→↓↓↓↓↓→←↑↑↑↑↑←↑←↑↑→←←←→←↓↓↓↓↓←←←↓←↓↓↓↓↓→→↓→→→→` |
| `webs.csv` | 1 | 39 | `←←↑↑→→→↑→↑↑←←↑→→→→→→→→→↓↓↓↓↓←→←→←→←←←↑↑` |
| `world.csv` | 2 | 192 | `^^ ^< ^> ^< ^< ^< ^v ^< ^> <^ <^ v^ v^ v^ v^ v^ v^ <> <v v^ v^ v^ ^^ ^v ^< >> ^< ^< ^^ ^< ^< ^v >^ >< ^< ^< <v ^< ^^ v^ v^ >^ v^ v^ >^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ ^< >> >< >^ >v v< v< v^ <^ >< ^^ ^> ^v << << << <v ^> ^> ^< ^v ^> ^> ^^ ^< >> >< ^< ^^ v< v^ v^ v^ >^ >^ <v << ^< ^> >v >v ^^ ^^ vv vv <^ <v <^ <^ <> << << <^ ^> ^^ v^ v> >> >^ >> >> ^^ ^< ^< ^< << << << <^ >v ^^ ^^ ^v ^v ^^ ^v ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ v< v> v^ >v >v ^< ^< ^^ v< v> >^ >< ^< ^^ v< v> >^ >< ^^ ^^ v< v> << v^ v< v< v< v< v< >v >. v> v^ ^v ^v <^ <^ v^ v^ v^ v^ <^ v^ ^^ vv ^> <^ << v< v^` |

## New puzzles (Claude's Gauntlet)

| Level | Trick | Moves | Solution |
|---|---|---|---|
| `claude/sacrifice.csv` | explosive-lure | 2 | `<>` |
| `claude/roach_motel.csv` | black-hole lure | 2 | `^^` |
| `claude/stampede.csv` | crowd hole-lure | 3 | `^^^` |
| `claude/remote_detonator.csv` | twin-trigger remote chain | 6 | `^<<<<v` |
| `claude/web_lair.csv` | web-shield + sword-facing | 6 | `^^vvvv` |

## Portal Hub Route

`claude/gauntlet.csv` has zero rats, so `solver verify` correctly remains `Playing`; completion is app-level portal stack state. The route below is verified by `solver/solutions/tools/verify_gauntlet_route.py`, which checks hub movement against `gauntlet.json` and verifies each child level with `solver verify`.

Total movement: 16 hub moves + 19 child moves (plus one confirm after each child win).

| Step | Hub Moves | Portal | Child Solution |
|---|---|---|---|
| 1 | `^^` | `claude/roach_motel.csv` | `^^` |
| 2 | `<<` | `claude/sacrifice.csv` | `<>` |
| 3 | `vvvv` | `claude/remote_detonator.csv` | `^<<<<v` |
| 4 | `>>>>` | `claude/web_lair.csv` | `^^vvvv` |
| 5 | `^^^^` | `claude/stampede.csv` | `^^^` |

## Reproduce

```
solver verify levels/<level>.csv "<solution>"   # prints result=Won
python3 solver/solutions/tools/verify_gauntlet_route.py
```

Or play in the browser: load `autoplay.js` in the dev console at https://davidspies.github.io/infestation/ , navigate to a level, then `infestation.play("rats")` etc.
