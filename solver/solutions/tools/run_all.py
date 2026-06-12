#!/usr/bin/env python3
"""Orchestrate the Rust oracle solver across all levels, escalating strategies."""
import subprocess, sys, os, json, time
from concurrent.futures import ProcessPoolExecutor, as_completed

SOLVER = "/tmp/infestation/repo/target/release/solver"
LV = "/tmp/infestation/repo/levels"

# Levels to solve (file relative to levels/). Confirmed-correct ones omitted.
LEVELS = [
    "triggering_explosives_v3.csv",
    "explosives2.csv",
    "synchronicity.csv",
    "tinderbox_v2.csv",
    "tinderrectangle.csv",
    "no_retreat.csv",
    "order_of_operations_new_v2.csv",
    "release.csv",
    "lock_in.csv",
    "chase.csv",
    "limited2.csv",
    "reload_v3.csv",
    "cyborg_rats/stalemate.csv",
    "cyborg_rats/ai_takeover.csv",
    "cyborg_rats/fakeout.csv",
    "cyborg_rats/unguided.csv",
    "cooperation/coop_world_v3.csv",
    "cooperation/tug_of_war.csv",
    "cooperation/handoff.csv",
    "cooperation/blocked_v2.csv",
    "cooperation/cooperation.csv",
]

# Strategy escalation: (strategy, weight, depth, secs)
ATTEMPTS = [
    ("gbfs",  5, 200, 25),
    ("astar", 3, 200, 35),
    ("astar", 10, 200, 35),
    ("astar", 1, 250, 45),   # closer to optimal/complete
]

def solve_level(level):
    path = f"{LV}/{level}"
    for (strat, w, depth, secs) in ATTEMPTS:
        try:
            out = subprocess.run(
                [SOLVER, "solve", path, "--strategy", strat,
                 "--weight", str(w), "--depth", str(depth), "--secs", str(secs)],
                capture_output=True, text=True, timeout=secs + 30)
        except subprocess.TimeoutExpired:
            continue
        for line in out.stdout.splitlines():
            if line.startswith("ARROWS "):
                arrows = line[len("ARROWS "):]
                moves = next((l.split()[1] for l in out.stdout.splitlines()
                              if l.startswith("SOLVED")), "?")
                return (level, "SOLVED", arrows, f"{strat}/w{w}", moves)
    return (level, "NO_SOLUTION", "", "", "")

def main():
    results = {}
    nworkers = max(2, (os.cpu_count() or 4) - 1)
    print(f"Running {len(LEVELS)} levels across {nworkers} workers...\n", flush=True)
    with ProcessPoolExecutor(max_workers=nworkers) as ex:
        futs = {ex.submit(solve_level, lv): lv for lv in LEVELS}
        for fut in as_completed(futs):
            level, status, arrows, strat, moves = fut.result()
            results[level] = {"status": status, "arrows": arrows, "strategy": strat, "moves": moves}
            if status == "SOLVED":
                print(f"✓ {level:42s} {moves} moves [{strat}]", flush=True)
                print(f"    {arrows}", flush=True)
            else:
                print(f"✗ {level:42s} NO SOLUTION", flush=True)
    with open("/tmp/infestation/results.json", "w") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    solved = sum(1 for r in results.values() if r["status"] == "SOLVED")
    print(f"\n{solved}/{len(LEVELS)} solved. Results saved to /tmp/infestation/results.json")

if __name__ == "__main__":
    main()
