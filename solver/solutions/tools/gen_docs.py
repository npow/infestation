#!/usr/bin/env python3
"""Regenerate SOLUTIONS.md plus browser autoplay data from final_solutions.json."""

from __future__ import annotations

import json
import pathlib


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLUTIONS_DIR = ROOT / "solver" / "solutions"
FINAL_SOLUTIONS = SOLUTIONS_DIR / "final_solutions.json"
AUTOPLAY_DATA = SOLUTIONS_DIR / "results" / "autoplay_data.json"

CLAUDE_TRICKS = {
    "claude/sacrifice.csv": "explosive-lure",
    "claude/roach_motel.csv": "black-hole lure",
    "claude/stampede.csv": "crowd hole-lure",
    "claude/remote_detonator.csv": "twin-trigger remote chain",
    "claude/web_lair.csv": "web-shield + sword-facing",
}


def autoplay_name(level: str) -> str:
    return level.removesuffix(".csv").split("/")[-1]


def write_solutions_markdown(final: dict[str, dict[str, object]]) -> None:
    original = {level: record for level, record in final.items() if not level.startswith("claude/")}
    claude = {level: final[level] for level in CLAUDE_TRICKS if level in final}

    lines = [
        "# Infestation — verified solutions\n",
        "All move-strings below were **verified against the real game engine** (`solver verify`) and print `result=Won`.\n",
        "Keys: `^`=up `v`=down `<`=left `>`=right `.`=stall. Two-player turns are space-separated `P1P2` pairs.\n",
        f"\n## Original levels ({len(original)} solved)\n",
        "| Level | Players | Moves | Solution |",
        "|---|---|---|---|",
    ]
    for level, record in original.items():
        lines.append(
            f"| `{level}` | {record['players']} | {record['moves']} | `{record['sol']}` |"
        )

    lines.extend(
        [
            "\n## New puzzles (Claude's Gauntlet)\n",
            "| Level | Trick | Moves | Solution |",
            "|---|---|---|---|",
        ]
    )
    for level, record in claude.items():
        lines.append(
            f"| `{level}` | {CLAUDE_TRICKS[level]} | {record['moves']} | `{record['sol']}` |"
        )

    lines.extend(
        [
            "\n## Reproduce\n",
            "```",
            'solver verify levels/<level>.csv "<solution>"   # prints result=Won',
            "```\n",
            "Or play in the browser: load `autoplay.js` in the dev console at "
            "https://davidspies.github.io/infestation/ , navigate to a level, then "
            '`infestation.play("rats")` etc.\n',
        ]
    )
    (SOLUTIONS_DIR / "SOLUTIONS.md").write_text("\n".join(lines), encoding="utf-8")


def write_autoplay(final: dict[str, dict[str, object]]) -> None:
    autoplay = {
        autoplay_name(level): {"p": record["players"], "s": record["sol"]}
        for level, record in final.items()
    }
    AUTOPLAY_DATA.write_text(
        json.dumps(autoplay, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    js = f'''/**
 * Infestation auto-player — verified solutions (oracle-confirmed result=Won).
 * Paste into the dev console at https://davidspies.github.io/infestation/
 *   infestation.list()          list solved levels
 *   infestation.play("rats")    auto-play a level (navigate to it first)
 *   infestation.stop()          stop;  infestation.speed = 150  (ms/move)
 * Two-player levels drive P1 with arrow keys and P2 with WASD.
 */
(function () {{
  const SOL = {json.dumps(autoplay, indent=2, ensure_ascii=False)};
  const P1 = {{ '^':'ArrowUp','v':'ArrowDown','<':'ArrowLeft','>':'ArrowRight' }};
  const P2 = {{ '^':'w','v':'s','<':'a','>':'d' }};
  const KC = {{ ArrowUp:38, ArrowDown:40, ArrowLeft:37, ArrowRight:39,
               w:87, a:65, s:83, d:68, ' ':32 }};
  let stop=false, timer=null;
  function key(k){{
    if(!k) return;
    const kc = KC[k]||0;
    for(const t of ['keydown','keypress','keyup']){{
      const e=new KeyboardEvent(t,{{key:k,code:k,keyCode:kc,which:kc,bubbles:true}});
      document.dispatchEvent(e);
      const c=document.querySelector('canvas'); if(c) c.dispatchEvent(e);
    }}
  }}
  window.infestation = {{
    speed: 200,
    list(){{ console.log('Verified solutions:'); for(const n in SOL)
      console.log('  infestation.play("'+n+'")  ('+SOL[n].p+'p, '+
        (SOL[n].p==1?SOL[n].s.length:SOL[n].s.split(' ').length)+' moves)'); }},
    play(name){{
      const e = SOL[name];
      if(!e){{ console.warn('unknown level: '+name); return; }}
      const turns = e.p==1 ? e.s.split('') : e.s.split(' ');
      console.log('Playing '+name+' ('+turns.length+' turns @ '+this.speed+'ms)…');
      stop=false; if(timer) clearInterval(timer); let i=0;
      timer=setInterval(()=>{{
        if(stop||i>=turns.length){{ clearInterval(timer); if(!stop) console.log('✓ done '+name); return; }}
        const t=turns[i++];
        if(e.p==1){{ key(P1[t]); }}
        else {{ const a=t[0], b=t[1]; key(P1[a]); key(b==='.'?' ':P2[b]); }}
      }}, this.speed);
    }},
    stop(){{ stop=true; if(timer) clearInterval(timer); console.log('stopped'); }}
  }};
  infestation.list();
}})();
'''
    (SOLUTIONS_DIR / "autoplay.js").write_text(js, encoding="utf-8")


def main() -> int:
    final = json.loads(FINAL_SOLUTIONS.read_text(encoding="utf-8"))
    write_solutions_markdown(final)
    write_autoplay(final)
    claude_count = sum(1 for level in final if level.startswith("claude/"))
    print(
        f"wrote {len(final)} verified solutions "
        f"({len(final) - claude_count} original + {claude_count} claude)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
