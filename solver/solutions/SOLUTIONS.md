# Infestation — verified solutions

All move-strings below were **verified against the real game engine** (`solver verify`) and print `result=Won`.

Keys: `^`=up `v`=down `<`=left `>`=right `.`=stall. Two-player turns are space-separated `P1P2` pairs.


## Original levels (30 solved)

| Level | Players | Moves | Solution |
|---|---|---|---|
| `blackhole_v2.csv` | 1 | 77 | `vvv>>>>>><<<<<<^^^^^^^^vvvvvvvv>>>>>>><<<<<<<^^^^^^^^^^^^^^^^^>>>><<vv>>>>vv>` |
| `chase.csv` | 1 | 199 | `^>>>v^^^>^^>>>>>v>>vvvvv^^^^^^<<<<v<<<v^<^<^<<>>>>v>>>>>vvvv^^^^<<<^<^<<^^^^^^^^<<<<<^^vvvvvvvvvvvvv>>>>>>>>vv^^<<vvvvv<>>>>>>>>>>>>>>^^v<>^^^^^^^^^^^^^^^^^^<<<<>>vvvv<vvvvvvvvvvvv<vvv<<<<<<<<<<<<<<<` |
| `cooperation/coop_world_v3.csv` | 2 | 26 | `^^ ^> ^^ ^^ ^^ ^^ ^^ ^> ^^ v> <> ^> ^> ^> ^> ^> ^> >v <. ^v >< v< .> .^ ^^ ^^` |
| `cooperation/cooperation.csv` | 2 | 28 | `^^ ^> ^^ ^^ ^< <^ ^> ^> ^> <> <v <> <> <> <> ^> >> >> <> << .< <v vv vv v< v< vv vv` |
| `cyborg_rats/cyborg_rats.csv` | 1 | 32 | `^>>^^<<<>>>^^<<<>>>^^<<<>>>^^<<<` |
| `cyborg_rats/fakeout.csv` | 1 | 78 | `^^v......>><v<v................<<<><<<<<><>><>^>^>v<<vvvvvvvvvv>^>^^>>>>>>>>>>` |
| `cyborg_rats/stalemate.csv` | 1 | 47 | `^>>^>^>>vv<vv><^^>^^>>>..............><<<<<<<<<` |
| `cyborg_rats/unguided.csv` | 1 | 111 | `^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^vv>>>>vvvvv>^^^^^<<<<<<<<<<<<<<<vvvvvvvvvvvvvvv>>>>>v<<>^>>>^^>>>>>>` |
| `explosives.csv` | 1 | 54 | `vvv<<^^^^<<vvvvvv>>>>vv<<<<<>>>>>^^<<<<^^^^^^^^>>>>>>>` |
| `explosives2.csv` | 1 | 53 | `^<vvvvvv>>>>><<<<<^^^^^^^^vvvvvvvv><^v^^^^^^^^>>>>>>>` |
| `gimmicks/platform.csv` | 1 | 29 | `vvv>vvv..^^.^^^^^^^^^<>>>>vvv` |
| `gimmicks/robotic_cheese.csv` | 2 | 7 | `>> ^> ^. ^. <. <^ ^^` |
| `guidance.csv` | 1 | 126 | `<<<<<<<<<<<vvvvvvvvvvvvvvvv>>>>>>>>><^>>>>><<<<<<<<<<<<<<^^^^^^<v<v>^^^^^^^^^^^>>>>>^^^^^vv<<<<<vvvvvvvvvvvvvvv>>>>>>>^^>>>>>>` |
| `limited2.csv` | 1 | 149 | `>>>>>>>>>>>>>><<<<<<<<<<<<<...<>.>>>>>>>>>>>>><<<<<<<<<<<<<...<>....>>>>>>>>>>>>><<<<<<<<<<<<<...<>>>>>>><<<<<<<<<<<^^^^^^^^^^^^>>>>>>>>^^^^^^^^^>>>>` |
| `lock_in.csv` | 1 | 69 | `^^>vvv<vv<<v<vv>v<^^^^^vvvvvv^^>>^>>>>>^^^^^^^^^^^v<v><<<^^<<<<<<<<<<` |
| `more_rats.csv` | 1 | 29 | `v<<><><><><>>^^^^^^<<><><><<<` |
| `no_retreat.csv` | 1 | 76 | `v<^<<<<^<>^^>>>^^<<^<<<<>>>.<<<><^^>>>>>>>>^^<<<<<<<<^^>>>>>>>>>>>>^>>>>^>>>` |
| `old_levels/old_levels.csv` | 1 | 9 | `vvvv<<<<<` |
| `order_of_operations_new_v2.csv` | 1 | 36 | `v>>>>^>^^^>>v>.....>>>v<vvvvvvvvvvvv` |
| `planks.csv` | 1 | 100 | `>>>><<<<<<<<<<^^^^^^<vvvvvv>>>>>>>>>>>>^^^^>^^^vvv<vvvv<<<<<<<<<<<<^^^^^^^>>>v>vvvv>>>>>>^^<^^^<<<vv` |
| `rats.csv` | 1 | 13 | `v<<^^^>>>><>^` |
| `synchronicity.csv` | 1 | 50 | `>>^v>>vvv<<v^^^^<v^v^v^^^vv>vvv>vv>><<^^>>>>>>^^^^` |
| `tinderbox_v2.csv` | 1 | 85 | `^>^^^^^<<<vv<<>.<>>>>^>v<<>>>v<v.><^v>><^^v<>^^<..v.^v^<v.>.vv^v<v..v^<^<<<<.vv^<^v>>` |
| `trapped_rat.csv` | 1 | 75 | `vvv^v.>><>>>>^^^^^^<vvvvvv<<>>^^^^^^<vvvvvv<<>>^^^^^^<vvvvvv<<>>^^^^^^^^^^<` |
| `trapped_rat2_v2.csv` | 1 | 78 | `<^^^v><^v><^v><^^<<<vvvvvv<^^^^^^><><vvvvvv<^^^^^^><><vvvvvv<^^^^^^><><^^^^^^>` |
| `triggering_explosives_v3.csv` | 1 | 72 | `<vvvv<<vvvv<<v^>>^^^^>>^^^^>>vvvv>>vvv^^^<^^><^<^<<^^vv<<^<<vvv>^^^>><>v` |
| `triggers.csv` | 1 | 71 | `vvv>>^^>>vv>>>>^>^^<^<<<^^>>>>^^<<<>>^^<^^<<vvv<v<vvvv<<<<<^^^^^^^>>>vv` |
| `triggers2.csv` | 1 | 104 | `^^>^>^^^vvv<v<<<<<^^^^><><vvvv>>>>>^>^^^^^vvvvv>>^^^^>>v>v>vvvvv><^^^^^<^<^^><<<><vvvvv<<<v<vvvvv>>v>>>>` |
| `webs.csv` | 1 | 39 | `<<^^>>>^>^^<<^>>>>>>>>>vvvvv<><><><<<^^` |
| `world.csv` | 2 | 192 | `^^ ^< ^> ^< ^< ^< ^v ^< ^> <^ <^ v^ v^ v^ v^ v^ v^ <> <v v^ v^ v^ ^^ ^v ^< >> ^< ^< ^^ ^< ^< ^v >^ >< ^< ^< <v ^< ^^ v^ v^ >^ v^ v^ >^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ ^< >> >< >^ >v v< v< v^ <^ >< ^^ ^> ^v << << << <v ^> ^> ^< ^v ^> ^> ^^ ^< >> >< ^< ^^ v< v^ v^ v^ >^ >^ <v << ^< ^> >v >v ^^ ^^ vv vv <^ <v <^ <^ <> << << <^ ^> ^^ v^ v> >> >^ >> >> ^^ ^< ^< ^< << << << <^ >v ^^ ^^ ^v ^v ^^ ^v ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ ^^ v< v> v^ >v >v ^< ^< ^^ v< v> >^ >< ^< ^^ v< v> >^ >< ^^ ^^ v< v> << v^ v< v< v< v< v< >v >. v> v^ ^v ^v <^ <^ v^ v^ v^ v^ <^ v^ ^^ vv ^> <^ << v< v^` |

## New puzzles (Claude's Gauntlet)

| Level | Trick | Moves | Solution |
|---|---|---|---|
| `claude/sacrifice.csv` | explosive-lure | 2 | `<>` |
| `claude/roach_motel.csv` | black-hole lure | 2 | `^^` |
| `claude/stampede.csv` | crowd hole-lure | 3 | `^^^` |
| `claude/remote_detonator.csv` | twin-trigger remote chain | 6 | `^<<<<v` |
| `claude/web_lair.csv` | web-shield + sword-facing | 6 | `^^vvvv` |

## Reproduce

```
solver verify levels/<level>.csv "<solution>"   # prints result=Won
```

Or play in the browser: load `autoplay.js` in the dev console at https://davidspies.github.io/infestation/ , navigate to a level, then `infestation.play("rats")` etc.
