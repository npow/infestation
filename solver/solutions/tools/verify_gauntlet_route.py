#!/usr/bin/env python3
"""Verify the Claude gauntlet portal route.

The normal solver verifies rat-bearing CSVs. The gauntlet hub has zero rats and
is completed by app-level portal stack state, so this verifier checks the hub
movement and delegates each child level to `solver verify`.
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[3]
LEVELS = ROOT / "levels"
SOLVER = ROOT / "target" / "release" / "solver"
ROUTE_FILE = ROOT / "solver" / "solutions" / "gauntlet_route.json"

DELTAS = {
    "^": (0, -1),
    "v": (0, 1),
    "<": (-1, 0),
    ">": (1, 0),
    ".": (0, 0),
}


def load_grid(path: pathlib.Path) -> tuple[list[list[str]], tuple[int, int]]:
    rows = [line.rstrip("\n").split(",") for line in path.read_text().splitlines()]
    player = None
    for y, row in enumerate(rows):
        for x, token in enumerate(row):
            if token not in {".", "#"}:
                if player is not None:
                    raise ValueError(f"multiple non-empty player-like cells in {path}")
                player = (x, y)
    if player is None:
        raise ValueError(f"no player found in {path}")
    return rows, player


def verify_child(portal: str, solution: str) -> None:
    csv_path = LEVELS / f"{portal}.csv"
    proc = subprocess.run(
        [str(SOLVER), "verify", str(csv_path), solution],
        check=False,
        text=True,
        capture_output=True,
    )
    if proc.returncode != 0 or "result=Won" not in proc.stdout:
        raise RuntimeError(
            f"{portal} failed child verification with {solution!r}:\n"
            f"stdout={proc.stdout}\nstderr={proc.stderr}"
        )


def main() -> int:
    route = json.loads(ROUTE_FILE.read_text())
    grid, pos = load_grid(LEVELS / route["level"])
    portals = {
        (entry["x"], entry["y"]): entry["level"]
        for entry in json.loads((LEVELS / "claude" / "gauntlet.json").read_text())["portals"]
    }
    completed: set[str] = set()

    for step_index, step in enumerate(route["route"], start=1):
        portal_name = step["portal"]
        for move_index, move in enumerate(step["hub_moves"], start=1):
            if move not in DELTAS:
                raise ValueError(f"bad hub move {move!r} in step {step_index}")
            dx, dy = DELTAS[move]
            next_pos = (pos[0] + dx, pos[1] + dy)
            x, y = next_pos
            if y < 0 or y >= len(grid) or x < 0 or x >= len(grid[y]):
                raise ValueError(f"hub move leaves grid at step {step_index}.{move_index}")
            if grid[y][x] == "#":
                raise ValueError(f"hub move hits wall at step {step_index}.{move_index}")
            pos = next_pos

            destination = portals.get(pos)
            if destination is not None and destination not in completed:
                if move_index != len(step["hub_moves"]):
                    raise ValueError(
                        f"step {step_index} enters {destination} before segment end"
                    )
                if destination != portal_name:
                    raise ValueError(
                        f"step {step_index} expected {portal_name}, reached {destination}"
                    )

        if portals.get(pos) != portal_name:
            raise ValueError(f"step {step_index} ended at {pos}, not {portal_name}")
        verify_child(portal_name, step["child_solution"])
        completed.add(portal_name)
        print(
            f"ok step {step_index}: hub {step['hub_moves']} -> "
            f"{portal_name}.csv child {step['child_solution']}"
        )

    expected = set(portals.values())
    if completed != expected:
        missing = ", ".join(sorted(expected - completed))
        raise ValueError(f"gauntlet route incomplete; missing: {missing}")

    print(f"GAUNTLET_ROUTE_VERIFIED completed={len(completed)} final_hub_pos={pos}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
