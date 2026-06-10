#!/usr/bin/env python3
"""Verify every solution against the ground-truth oracle and emit the final autoplay map."""
import subprocess, json, os

SOLVER = "/tmp/infestation/repo/target/release/solver"
LV = "/tmp/infestation/repo/levels"

# 12 confirmed-correct from the hand-ported pass (already oracle-verified earlier)
CONFIRMED = {
    "rats.csv": "↓←←↑↑↑→→→→←→↑",
    "more_rats.csv": "↓←←→←→←→←→←→→↑↑↑↑↑↑←←→←→←→←←←",
    "webs.csv": "←←↑↑→→→↑→↑↑←←↑→→→→→→→→→↓↓↓↓↓←→←→←→←←←↑↑",
    "trapped_rat.csv": "↓↓↓↑↓.→→←→→→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑←↓↓↓↓↓↓←←→→↑↑↑↑↑↑↑↑↑↑←",
    "trapped_rat2_v2.csv": "←↑↑↑↓→←↑↓→←↑↓→←↑↑←←←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↓↓↓↓↓↓←↑↑↑↑↑↑→←→←↑↑↑↑↑↑→",
    "planks.csv": "→→→→←←←←←←←←←←↑↑↑↑↑↑←↓↓↓↓↓↓→→→→→→→→→→→→↑↑↑↑→↑↑↑↓↓↓←↓↓↓↓←←←←←←←←←←←←↑↑↑↑↑↑↑→→→↓→↓↓↓↓→→→→→→↑↑←↑↑↑←←←↓↓",
    "guidance.csv": "←←←←←←←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→→→←↑→→→→→←←←←←←←←←←←←←←↑↑↑↑↑↑←↓←↓→↑↑↑↑↑↑↑↑↑↑↑→→→→→↑↑↑↑↑↓↓←←←←←↓↓↓↓↓↓↓↓↓↓↓↓↓↓↓→→→→→→→↑↑→→→→→→",
    "triggers.csv": "↓↓↓→→↑↑→→↓↓→→→→↑→↑↑←↑←←←↑↑→→→→↑↑←←←→→↑↑←↑↑←←↓↓↓←↓←↓↓↓↓←←←←←↑↑↑↑↑↑↑→→→↓↓",
    "triggers2.csv": "↑↑→↑→↑↑↑↓↓↓←↓←←←←←↑↑↑↑→←→←↓↓↓↓→→→→→↑→↑↑↑↑↑↓↓↓↓↓→→↑↑↑↑→→↓→↓→↓↓↓↓↓→←↑↑↑↑↑←↑←↑↑→←←←→←↓↓↓↓↓←←←↓←↓↓↓↓↓→→↓→→→→",
    "blackhole_v2.csv": "↓↓↓→→→→→→←←←←←←↑↑↑↑↑↑↑↑↓↓↓↓↓↓↓↓→→→→→→→←←←←←←←↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑→→→→←←↓↓→→→→↓↓→",
    "explosives.csv": "↓↓↓←←↑↑↑↑←←↓↓↓↓↓↓→→→→↓↓←←←←←→→→→→↑↑←←←←↑↑↑↑↑↑↑↑→→→→→→→",
    "cyborg_rats/cyborg_rats.csv": "↑→→↑↑←←←→→→↑↑←←←→→→↑↑←←←→→→↑↑←←←",
}

NPLAYERS = {}  # default 1; set 2 for coop

def detect_players(level_file):
    txt = open(f"{LV}/{level_file}").read()
    n = sum(txt.count(g) for g in "▲▼►◄△▽▷◁")
    return n

def verify(level_file, sol):
    n = detect_players(level_file)
    # solver verify accepts space-separated turns; for 2p each turn is a 2-char group.
    out = subprocess.run([SOLVER, "verify", f"{LV}/{level_file}", sol],
                         capture_output=True, text=True)
    return "result=Won" in out.stdout, out.stdout.strip(), n

def main():
    allsol = dict(CONFIRMED)
    for jf in ["results.json", "results_pass2.json"]:
        p = f"/tmp/infestation/{jf}"
        if os.path.exists(p):
            for k, v in json.load(open(p)).items():
                if v.get("status") == "SOLVED" and v.get("arrows"):
                    allsol[k] = v["arrows"]

    print(f"Verifying {len(allsol)} solutions against ground-truth oracle...\n")
    final = {}
    won = 0
    for lf in sorted(allsol):
        sol = allsol[lf]
        ok, msg, npl = verify(lf, sol)
        mark = "✓" if ok else "✗"
        moves = len([c for c in sol if c in "↑↓←→."])
        if npl == 2:
            moves = len(sol.split())
        if ok:
            won += 1
            final[lf] = {"sol": sol, "players": npl, "moves": moves}
        print(f"{mark} {lf:36s} p{npl} {moves:3d} moves  {msg}")
    print(f"\n{won}/{len(allsol)} verified WON")
    json.dump(final, open("/tmp/infestation/final_solutions.json", "w"),
              indent=2, ensure_ascii=False)
    print("Saved /tmp/infestation/final_solutions.json")

if __name__ == "__main__":
    main()
