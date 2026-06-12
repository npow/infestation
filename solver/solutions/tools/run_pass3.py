#!/usr/bin/env python3
"""Pass 3: progress-heuristic + max parallelism. Each (level,strategy) is its own
process so all CPU cores stay busy; first solution per level wins."""
import subprocess, os, json, time
from concurrent.futures import ProcessPoolExecutor, as_completed

SOLVER = "/tmp/infestation/repo/target/release/solver"
LV = "/tmp/infestation/repo/levels"

LEVELS = [
    "tinderbox_v2.csv", "tinderrectangle.csv", "no_retreat.csv",
    "cyborg_rats/ai_takeover.csv", "release.csv", "lock_in.csv",
    "reload_v3.csv", "chase.csv",
    "cooperation/tug_of_war.csv", "cooperation/handoff.csv", "cooperation/blocked_v2.csv",
]

# per-level strategy variants — all run concurrently as separate processes
STRATS = [
    ("gbfs",  1, 240),
    ("astar", 2, 240),
    ("astar", 4, 240),
]

def run_job(level, strat, w, secs):
    path = f"{LV}/{level}"
    env = dict(os.environ); env["PROGRESS_H"] = "1"
    t0 = time.time()
    try:
        out = subprocess.run(
            [SOLVER, "solve", path, "--strategy", strat, "--weight", str(w),
             "--depth", "500", "--secs", str(secs)],
            capture_output=True, text=True, timeout=secs + 60, env=env)
    except subprocess.TimeoutExpired:
        return (level, strat, w, None, None, time.time()-t0)
    arrows, moves = None, "?"
    for line in out.stdout.splitlines():
        if line.startswith("SOLVED"): moves = line.split()[1]
        if line.startswith("ARROWS "): arrows = line[len("ARROWS "):]
    return (level, strat, w, arrows, moves, time.time()-t0)

def main():
    jobs = [(lv, s, w, secs) for lv in LEVELS for (s, w, secs) in STRATS]
    nworkers = max(2, (os.cpu_count() or 4))
    print(f"Pass 3: {len(jobs)} jobs ({len(LEVELS)} levels x {len(STRATS)} strats) on {nworkers} cores\n", flush=True)
    solved = {}
    with ProcessPoolExecutor(max_workers=nworkers) as ex:
        futs = {ex.submit(run_job, lv, s, w, secs): (lv, s, w) for (lv, s, w, secs) in jobs}
        for fut in as_completed(futs):
            level, strat, w, arrows, moves, el = fut.result()
            if arrows and level not in solved:
                solved[level] = {"arrows": arrows, "moves": moves, "strategy": f"{strat}/w{w}/PH"}
                print(f"✓ {level:40s} {moves} moves [{strat}/w{w}] ({el:.0f}s)", flush=True)
                print(f"    {arrows}", flush=True)
            elif not arrows:
                print(f"·  {level:40s} [{strat}/w{w}] no sol ({el:.0f}s)", flush=True)
    json.dump(solved, open("/tmp/infestation/results_pass3.json", "w"), indent=2, ensure_ascii=False)
    print(f"\nPass 3: solved {len(solved)}/{len(LEVELS)} levels.")
    for lv in LEVELS:
        if lv not in solved: print(f"   still open: {lv}")

if __name__ == "__main__":
    main()
