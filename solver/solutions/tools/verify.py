#!/usr/bin/env python3
"""Verify Infestation solutions against the ground-truth Rust oracle."""
import subprocess, sys

SOLVER = "/tmp/infestation/repo/target/release/solver"
LV = "/tmp/infestation/repo/levels"

# (level_file, solution_arrows, nplayers)
SOLUTIONS = [
    ("rats.csv", "↓←←↑↑↑→→→→←→↑", 1),
    ("more_rats.csv", "↓←←→←→←→←→←→→↑↑↑↑↑↑←←→←→←→←←←", 1),
    ("webs.csv", "←←↑↑→→→↑→↑↑←←↑→→→→→→→→→↓↓↓↓↓←→←→←→←←←↑↑", 1),
    ("trapped_rat.csv", "↓↓↓↑↓.→→←→→→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑↑↑↑↑←", 1),
    ("trapped_rat2_v2.csv", "←↑↑↑↓→←↑↓→←↑↓→←↑↑←←←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↑↑↑↑↑↑→", 1),
    ("planks.csv", "→→→→←←←←←←←←←←↑↑↑↑↑↑←↓↓↓↓↓↓→→→→→→→→→→→→↑↑↑↑→↑↑↑↓↓↓←↓↓↓↓←←←←←←←←←←←←↑↑↑↑↑↑↑→→→↓→↓↓↓↓→→→→→→↑↑←↑↑↑←←←↓↓", 1),
    ("guidance.csv", "←←←←←←←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→→→←↑→→→→→←←←←←←←←←←←←←←↑↑↑↑↑↑←↓←↓→↑↑↑↑↑↑↑↑↑↑↑→→→→→↑↑↑↑↑↓↓←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→↑↑→→→→→→", 1),
    ("triggering_explosives_v3.csv", "←↑↑←←↓←←↓↓↓←↓↓↓↓↓→↓→→→→→→→", 1),
    ("triggers.csv", "↓↓↓→→↑↑→→↓↓→→→→↑→↑↑←↑←←←↑↑→→→→↑↑←←←→→↑↑←↑↑←←↓↓↓←↓←↓↓↓↓←←←←←↑↑↑↑↑↑↑→→→↓↓", 1),
    ("triggers2.csv", "↑↑→↑→↑↑↑↓↓↓←↓←←←←←↑↑↑↑→←→←↓↓↓↓→→→→→↑→↑↑↑↑↑↓↓↓↓↓→→↑↑↑↑→→↓→↓→↓↓↓↓↓→←↑↑↑↑↑←↑←↑↑→←←←→←↓↓↓↓↓←←←↓←↓↓↓↓↓→→↓→→→→", 1),
    ("blackhole_v2.csv", "↓↓↓→→→→→→←←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→→→→→→→←←←←←←←↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑→→→→←←↓↓→→→→↓↓→", 1),
    ("explosives.csv", "↓↓↓←←↑↑↑↑←←↓↓↓↓↓↓→→→→↓↓←←←←←→→→→→↑↑←←←←↑↑↑↑↑↑↑↑→→→→→→→", 1),
    ("explosives2.csv", "←↓↓↓↓↓↓→→→→→←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→←←↑↑↑↑↑↑↑↑→→→→→→→", 1),
    ("cyborg_rats/cyborg_rats.csv", "↑→→↑↑←←←→→→↑↑←←←→→→↑↑←←←→→→↑↑←←←", 1),
    ("cyborg_rats/stalemate.csv", "↓→→→←←←↑↑↑↑→↓↓↓↓→→→→→→→←→→→→→→→→→→→", 1),
    ("cooperation/coop_world_v3.csv", "↑→↑←←↑↑↑↑←↑↑←↑↑", 2),
]

def verify(level_file, sol, nplayers):
    path = f"{LV}/{level_file}"
    if nplayers == 2:
        # both players do the same move each turn
        turns = " ".join(f"{c}|{c}" for c in sol if c in "↑↓←→.")
        action_arg = turns
    else:
        action_arg = sol
    out = subprocess.run([SOLVER, "verify", path, action_arg],
                         capture_output=True, text=True)
    return out.stdout.strip(), out.stderr.strip()

passed, failed = 0, 0
for lf, sol, npl in SOLUTIONS:
    stdout, stderr = verify(lf, sol, npl)
    won = "result=Won" in stdout
    mark = "✓" if won else "✗"
    if won: passed += 1
    else: failed += 1
    print(f"{mark} {lf:42s} {stdout}  {stderr if not won else ''}")

print(f"\n{passed} WON, {failed} FAILED (ground-truth Rust oracle)")
