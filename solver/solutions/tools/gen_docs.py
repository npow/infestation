import json

final = json.load(open("final_solutions.json"))  # {level: {sol, players, moves}}
# new puzzles (already verified earlier)
claude = {
    "claude/sacrifice.csv": {"sol": "<>", "players": 1, "moves": 2, "trick": "explosive-lure"},
    "claude/roach_motel.csv": {"sol": "^^", "players": 1, "moves": 2, "trick": "black-hole lure"},
    "claude/stampede.csv": {"sol": "^^^", "players": 1, "moves": 3, "trick": "crowd hole-lure"},
    "claude/remote_detonator.csv": {"sol": "^<<<<v", "players": 1, "moves": 6, "trick": "twin-trigger remote chain"},
    "claude/web_lair.csv": {"sol": "^^vvvv", "players": 1, "moves": 6, "trick": "web-shield + sword-facing"},
}

# convert arrow chars in final to ascii for portability/consistency
A2C = {"↑":"^","↓":"v","←":"<","→":">",".":"."}
def to_ascii(s):
    return "".join(A2C.get(ch, ch) for ch in s)

# ---- SOLUTIONS.md ----
md = ["# Infestation — verified solutions\n",
"All move-strings below were **verified against the real game engine** (`solver verify`) and print `result=Won`.\n",
"Keys: `^`=up `v`=down `<`=left `>`=right `.`=stall. Two-player turns are space-separated `P1P2` pairs.\n",
"\n## Original levels (22 solved)\n",
"| Level | Players | Moves | Solution |",
"|---|---|---|---|"]
for lv in sorted(final):
    d = final[lv]
    sol = to_ascii(d["sol"]) if d["players"]==1 else " ".join(to_ascii(t) for t in d["sol"].split())
    md.append(f"| `{lv}` | {d['players']} | {d['moves']} | `{sol}` |")
md.append("\n## New puzzles (Claude's Gauntlet)\n")
md.append("| Level | Trick | Moves | Solution |")
md.append("|---|---|---|---|")
for lv,d in claude.items():
    md.append(f"| `{lv}` | {d['trick']} | {d['moves']} | `{d['sol']}` |")
md.append("\n## Reproduce\n")
md.append("```\nsolver verify levels/<level>.csv \"<solution>\"   # prints result=Won\n```\n")
md.append("Or play in the browser: load `autoplay.js` in the dev console at "
          "https://davidspies.github.io/infestation/ , navigate to a level, then "
          "`infestation.play(\"rats\")` etc.\n")
open("/tmp/infestation/SOLUTIONS.md","w").write("\n".join(md))

# ---- autoplay.js data (verified only) ----
sols = {}
for lv,d in {**final, **claude}.items():
    name = lv.replace(".csv","").split("/")[-1]
    sols[name] = {"sol": (d["sol"] if "↑" not in d["sol"] else d["sol"]), "players": d["players"]}
# store ascii-normalized
out = {}
for lv,d in {**final, **claude}.items():
    name = lv.replace(".csv","").split("/")[-1]
    s = d["sol"]
    if d["players"]==1:
        out[name] = {"p":1,"s":to_ascii(s)}
    else:
        out[name] = {"p":2,"s":" ".join(to_ascii(t) for t in s.split())}
json.dump(out, open("/tmp/infestation/autoplay_data.json","w"), indent=0)
print("wrote SOLUTIONS.md and autoplay_data.json")
print(f"{len(final)} original + {len(claude)} new = {len(final)+len(claude)} total verified solutions")
