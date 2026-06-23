#!/usr/bin/env python3
"""Verify final_solutions.json against the ground-truth Rust oracle."""

from __future__ import annotations

import json
import pathlib
import subprocess


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLVER = ROOT / "target" / "release" / "solver"
LEVELS = ROOT / "levels"
FINAL_SOLUTIONS = ROOT / "solver" / "solutions" / "final_solutions.json"


def verify(level_file: str, solution: str) -> tuple[bool, str, str]:
    proc = subprocess.run(
        [str(SOLVER), "verify", str(LEVELS / level_file), solution],
        capture_output=True,
        text=True,
        check=False,
    )
    stdout = proc.stdout.strip()
    stderr = proc.stderr.strip()
    return proc.returncode == 0 and "result=Won" in stdout, stdout, stderr


def main() -> int:
    final = json.loads(FINAL_SOLUTIONS.read_text(encoding="utf-8"))
    passed = 0
    failed = 0
    for level_file, record in sorted(final.items()):
        won, stdout, stderr = verify(level_file, record["sol"])
        mark = "OK" if won else "FAIL"
        if won:
            passed += 1
        else:
            failed += 1
        suffix = f"  {stderr}" if stderr and not won else ""
        print(f"{mark:4s} {level_file:42s} {stdout}{suffix}")

    print(f"\n{passed} WON, {failed} FAILED (ground-truth Rust oracle)")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
