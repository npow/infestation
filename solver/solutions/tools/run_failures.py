#!/usr/bin/env python3
"""Second pass: larger budgets + strategy variety on the hard unsolved levels."""
import subprocess, os, json, time
from concurrent.futures import ProcessPoolExecutor, as_completed

SOLVER = "/tmp/infestation/repo/target/release/solver"
LV = "/tmp/infestation/repo/levels"

FAILURES = [
    "tinderbox_v2.csv",
    "tinderrectangle.csv",
    "lock_in.csv",
    "no_retreat.csv",
    "cyborg_rats/ai_takeover.csv",
    "limited2.csv",
    "release.csv",
    "chase.csv",
    "reload_v3.csv",
    "cooperation/tug_of_war.csv",
    "cooperation/handoff.csv",
    "cooperation/blocked_v2.csv",
]

# escalating attempts: (strategy, weight, depth, secs)
ATTEMPTS = [
    ("gbfs",  1, 400, 60),
    ("astar", 2, 400, 90),
    ("astar", 4, 400, 90),
    ("astar", 8, 400, 90),
    ("astar", 1, 400, 120),
]

def solve_level(level):
    path = f"{LV}/{level}"
    t0 = time.time()
    for (strat, w, depth, secs) in ATTEMPTS:
        try:
            out = subprocess.run(
                [SOLVER, "solve", path, "--strategy", strat,
                 "--weight", str(w), "--depth", str(depth), "--secs", str(secs)],
                capture_output=True, text=True, timeout=secs + 60)
        except subprocess.TimeoutExpired:
            continue
        arrows = None
        moves = "?"
        for line in out.stdout.splitlines():
            if line.startswith("SOLVED"):
                moves = line.split()[1]
            if line.startswith("ARROWS "):
                arrows = line[len("ARROWS "):]
        if arrows is not None:
            return (level, "SOLVED", arrows, f"{strat}/w{w}", moves, time.time()-t0)
    return (level, "NO_SOLUTION", "", "", "", time.time()-t0)

def main():
    results = {}
    nworkers = max(2, (os.cpu_count() or 4) - 1)
    print(f"Pass 2: {len(FAILURES)} hard levels across {nworkers} workers...\n", flush=True)
    with ProcessPoolExecutor(max_workers=nworkers) as ex:
        futs = {ex.submit(solve_level, lv): lv for lv in FAILURES}
        for fut in as_completed(futs):
            level, status, arrows, strat, moves, el = fut.result()
            results[level] = {"status": status, "arrows": arrows, "strategy": strat, "moves": moves}
            if status == "SOLVED":
                print(f"✓ {level:42s} {moves} moves [{strat}] ({el:.0f}s)", flush=True)
                print(f"    {arrows}", flush=True)
            else:
                print(f"✗ {level:42s} NO SOLUTION ({el:.0f}s)", flush=True)
    with open("/tmp/infestation/results_pass2.json", "w") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    solved = sum(1 for r in results.values() if r["status"] == "SOLVED")
    print(f"\nPass 2: {solved}/{len(FAILURES)} solved.")

if __name__ == "__main__":
    main()
