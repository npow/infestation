#!/usr/bin/env python3
"""Extract solver run logs into a compact JSONL mechanism archive.

The solving workflow now produces many bounded logs under /tmp/infestation-runs.
This script turns the useful lines from those logs into machine-readable records
so later passes can sort, deduplicate, or mark dead mechanism families without
manually rereading every log.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
from collections.abc import Iterable
from typing import Any


FEATURES_RE = re.compile(
    r"Features \{ rats: (?P<rats>\d+), explosives: (?P<explosives>\d+), "
    r"webs: (?P<webs>\d+), triggers: (?P<triggers>\d+), "
    r"planks: (?P<planks>\d+), walls: (?P<walls>\d+) \}"
)
EVENT_RE = re.compile(
    r"EVENT idx=(?P<idx>\d+) moves=(?P<moves>\d+) score=(?P<score>-?\d+) "
    r"key=(?P<key>.+?) features=(?P<features>Features \{.+?\}) "
    r"reachable_rats=(?P<reachable_rats>\d+) trapped=(?P<trapped>\d+)"
)
BRANCH_RE = re.compile(
    r"BRANCH idx=(?P<idx>\d+) suffix=(?P<suffix>\d+) total=(?P<total>\d+) "
    r"score=(?P<score>-?\d+) features=(?P<features>Features \{.+?\}) "
    r"reachable_rats=(?P<reachable_rats>\d+)"
)
FESS_RE = re.compile(
    r"FESS_FRONTIER\[(?P<idx>\d+)\] score=(?P<score>-?\d+) path=(?P<path>\d+) "
    r"bucket=(?P<bucket>.+?) features=(?P<features>Features \{.+?\}) "
    r"reachable_rats=(?P<reachable_rats>\d+) trapped=(?P<trapped>\d+) "
    r"ascii=(?P<ascii>.*)"
)
BEST_RE = re.compile(r"\s*BEST_ASCII (?P<ascii>.*)")
BEST_FULL_RE = re.compile(r"\s*BEST_FULL_ASCII (?P<ascii>.*)")
SOLVED_RE = re.compile(r"SOLVED moves=(?P<moves>\d+) time=(?P<time>[0-9.]+)s")
END_RE = re.compile(r"END code=(?P<code>-?\d+) elapsed=(?P<elapsed>\d+)s")


def parse_features(text: str) -> dict[str, int]:
    match = FEATURES_RE.search(text)
    if match is None:
        return {}
    return {key: int(value) for key, value in match.groupdict().items()}


def log_files(inputs: Iterable[pathlib.Path]) -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for input_path in inputs:
        if input_path.is_dir():
            files.extend(sorted(input_path.rglob("*.log")))
        else:
            files.append(input_path)
    return files


def parse_log(path: pathlib.Path, include_best: bool) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    pending: dict[str, Any] | None = None
    last_best: dict[str, Any] | None = None

    def flush_pending() -> None:
        nonlocal pending
        if pending is not None:
            records.append(pending)
            pending = None

    for raw_line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw_line.rstrip("\n")

        if pending is not None and line.startswith("ASCII "):
            pending["ascii"] = line.removeprefix("ASCII ")
            continue

        if pending is not None and line.startswith("FULL_ASCII "):
            pending["full_ascii"] = line.removeprefix("FULL_ASCII ")
            continue

        if pending is not None and line.startswith("STATE"):
            flush_pending()
            continue

        match = EVENT_RE.search(line)
        if match is not None:
            flush_pending()
            pending = {
                "kind": "event",
                "source": str(path),
                "idx": int(match["idx"]),
                "moves": int(match["moves"]),
                "score": int(match["score"]),
                "event_key": match["key"],
                "features": parse_features(match["features"]),
                "reachable_rats": int(match["reachable_rats"]),
                "trapped": int(match["trapped"]),
            }
            continue

        match = BRANCH_RE.search(line)
        if match is not None:
            flush_pending()
            pending = {
                "kind": "branch",
                "source": str(path),
                "idx": int(match["idx"]),
                "suffix": int(match["suffix"]),
                "total": int(match["total"]),
                "score": int(match["score"]),
                "features": parse_features(match["features"]),
                "reachable_rats": int(match["reachable_rats"]),
            }
            continue

        match = FESS_RE.search(line)
        if match is not None:
            flush_pending()
            records.append(
                {
                    "kind": "fess_frontier",
                    "source": str(path),
                    "idx": int(match["idx"]),
                    "score": int(match["score"]),
                    "path": int(match["path"]),
                    "bucket": match["bucket"],
                    "features": parse_features(match["features"]),
                    "reachable_rats": int(match["reachable_rats"]),
                    "trapped": int(match["trapped"]),
                    "ascii": match["ascii"],
                }
            )
            continue

        match = BEST_RE.search(line)
        if match is not None and include_best:
            flush_pending()
            last_best = {
                "kind": "best",
                "source": str(path),
                "ascii": match["ascii"],
            }
            records.append(last_best)
            continue

        match = BEST_FULL_RE.search(line)
        if match is not None and include_best and last_best is not None:
            last_best["full_ascii"] = match["ascii"]
            continue

        match = SOLVED_RE.search(line)
        if match is not None:
            flush_pending()
            records.append(
                {
                    "kind": "solved_marker",
                    "source": str(path),
                    "moves": int(match["moves"]),
                    "time_sec": float(match["time"]),
                }
            )
            continue

        match = END_RE.search(line)
        if match is not None:
            flush_pending()
            records.append(
                {
                    "kind": "run_end",
                    "source": str(path),
                    "code": int(match["code"]),
                    "elapsed_sec": int(match["elapsed"]),
                }
            )

    flush_pending()
    return records


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("inputs", nargs="+", type=pathlib.Path)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument(
        "--include-best",
        action="store_true",
        help="include BEST_ASCII timeout/frontier markers, not just accepted branches/events",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    paths = log_files(args.inputs)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with args.out.open("w", encoding="utf-8") as out:
        for path in paths:
            for record in parse_log(path, args.include_best):
                out.write(json.dumps(record, sort_keys=True) + "\n")
                count += 1
    print(f"wrote {count} records from {len(paths)} log(s) to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
