#!/usr/bin/env python3
"""Rank archived frontier prefixes with oracle diagnostics.

Archive records are cheap to collect, but many of them only carry local event
features. This tool reconstructs full prefixes, replays them with the Rust
oracle's `diag` mode, and ranks the resulting states by structural obligations:
reachable rats, reachable triggers, trapped rats, remaining resources, and
level-specific warning flags. It is a triage tool, not a game reimplementation.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import pathlib
import re
import shlex
import subprocess
from collections import defaultdict
from collections.abc import Iterable
from typing import Any

from go_explore_portfolio import (
    FINAL_SOLUTIONS,
    Candidate,
    SOLVER,
    archive_candidates,
    canonical_level,
    load_solved_levels,
    static_candidates,
)


FEATURES_RE = re.compile(
    r"features rats=(?P<rats>\d+) explosives=(?P<explosives>\d+) "
    r"webs=(?P<webs>\d+) triggers=(?P<triggers>\d+) planks=(?P<planks>\d+) "
    r"walls=(?P<walls>\d+)"
)
REACHABLE_RE = re.compile(
    r"player_reachable cells=(?P<cells>\d+) rats=(?P<reachable_rats>\d+)/(?P<rats>\d+) "
    r"triggers=(?P<reachable_triggers>\d+)/(?P<triggers>\d+)"
)
TRAPPED_RE = re.compile(r"trapped_unreachable_rats=(?P<trapped>\d+)")
STATE_RE = re.compile(r"state=(?P<state>\w+) turns_applied=(?P<turns>\d+)")
RAT_LINE_RE = re.compile(r"rat \((?P<x>-?\d+),(?P<y>-?\d+)\) (?P<rest>.*)")
TRIGGER_LINE_RE = re.compile(r"trigger Trigger\((?P<number>\d+)\) at \((?P<x>-?\d+),(?P<y>-?\d+)\) (?P<rest>.*)")


@dataclasses.dataclass(frozen=True)
class Diag:
    state: str
    turns: int
    features: dict[str, int]
    reachable_cells: int
    reachable_rats: int
    total_rats: int
    reachable_triggers: int
    total_triggers: int
    trapped: int
    rat_lines: dict[tuple[int, int], str]
    trigger_lines: dict[tuple[int, int], tuple[int, str]]


@dataclasses.dataclass(frozen=True)
class TriageRecord:
    candidate: Candidate
    diag: Diag
    flags: tuple[str, ...]
    score: tuple[int, ...]


def moves_len(prefix: str) -> int:
    if " " in prefix:
        return len(prefix.split())
    return len(prefix)


def parse_diag(text: str) -> Diag | None:
    state_match = STATE_RE.search(text)
    features_match = FEATURES_RE.search(text)
    reachable_match = REACHABLE_RE.search(text)
    trapped_match = TRAPPED_RE.search(text)
    if (
        state_match is None
        or features_match is None
        or reachable_match is None
        or trapped_match is None
    ):
        return None

    rat_lines: dict[tuple[int, int], str] = {}
    trigger_lines: dict[tuple[int, int], tuple[int, str]] = {}
    for line in text.splitlines():
        rat_match = RAT_LINE_RE.match(line)
        if rat_match is not None:
            point = (int(rat_match["x"]), int(rat_match["y"]))
            rat_lines[point] = rat_match["rest"]
            continue
        trigger_match = TRIGGER_LINE_RE.match(line)
        if trigger_match is not None:
            point = (int(trigger_match["x"]), int(trigger_match["y"]))
            trigger_lines[point] = (
                int(trigger_match["number"]),
                trigger_match["rest"],
            )

    return Diag(
        state=state_match["state"],
        turns=int(state_match["turns"]),
        features={key: int(value) for key, value in features_match.groupdict().items()},
        reachable_cells=int(reachable_match["cells"]),
        reachable_rats=int(reachable_match["reachable_rats"]),
        total_rats=int(reachable_match["rats"]),
        reachable_triggers=int(reachable_match["reachable_triggers"]),
        total_triggers=int(reachable_match["triggers"]),
        trapped=int(trapped_match["trapped"]),
        rat_lines=rat_lines,
        trigger_lines=trigger_lines,
    )


def run_diag(candidate: Candidate, timeout_sec: float) -> Diag | None:
    try:
        proc = subprocess.run(
            [str(SOLVER), "diag", candidate.level, candidate.prefix],
            cwd=SOLVER.parents[2],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout_sec,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None
    if proc.returncode != 0:
        return None
    return parse_diag(proc.stdout)


def unique_by_prefix(candidates: Iterable[Candidate]) -> list[Candidate]:
    by_level_prefix: dict[tuple[str, str], Candidate] = {}
    for candidate in candidates:
        key = (candidate.level, candidate.prefix)
        previous = by_level_prefix.get(key)
        if previous is None or candidate.rats < previous.rats:
            by_level_prefix[key] = candidate
    return list(by_level_prefix.values())


def coarse_rank(candidate: Candidate) -> tuple[int, int, int, int, int]:
    return (
        candidate.rats,
        -candidate.reachable_rats,
        candidate.trapped,
        moves_len(candidate.prefix),
        candidate.score,
    )


def trigger_reachable(diag: Diag, number: int) -> bool:
    return any(
        trigger_number == number and "player_dist=unreachable" not in rest
        for trigger_number, rest in diag.trigger_lines.values()
    )


def rat_unreachable(diag: Diag, point: tuple[int, int]) -> bool:
    rest = diag.rat_lines.get(point)
    return rest is not None and "player_dist=unreachable" in rest


def warning_flags(level: str, diag: Diag) -> tuple[str, ...]:
    flags: list[str] = []
    if diag.state != "Playing":
        flags.append(f"state:{diag.state}")
    if diag.total_rats > 0 and diag.reachable_rats == 0:
        flags.append("no-reachable-rats")
    if diag.total_rats > 0 and diag.reachable_triggers == 0 and diag.features["explosives"] == 0:
        flags.append("no-remaining-mechanism")
    if diag.trapped:
        flags.append(f"trapped:{diag.trapped}")

    if level.endswith("release.csv"):
        if rat_unreachable(diag, (18, 4)):
            flags.append("release-right-rat-sealed")
        if not trigger_reachable(diag, 2):
            flags.append("release-trigger2-unreachable")
    elif level.endswith("reload_v3.csv"):
        if rat_unreachable(diag, (0, 21)):
            flags.append("reload-top-left-rat-sealed")
        if not trigger_reachable(diag, 2):
            flags.append("reload-trigger2-unreachable")
    elif level.endswith("tug_of_war.csv"):
        if rat_unreachable(diag, (7, 0)):
            flags.append("tug-top-rat-sealed")
        if diag.reachable_rats < 2 and diag.total_rats > 1:
            flags.append("tug-low-reachability")
    elif level.endswith("handoff.csv"):
        if rat_unreachable(diag, (10, 6)):
            flags.append("handoff-hard-rat-sealed")
    elif level.endswith("blocked_v2.csv"):
        if diag.reachable_rats < diag.total_rats:
            flags.append("blocked-unreachable-rat")
    elif level.endswith("chase.csv"):
        if rat_unreachable(diag, (11, 17)):
            flags.append("chase-sealed-rat")
        if diag.total_rats > 0 and diag.reachable_rats < diag.total_rats:
            flags.append("chase-unreachable-rat")
    elif level.endswith("tinderrectangle.csv"):
        if diag.total_rats < 16:
            flags.append("tinder-dropped-rat")
        if diag.total_rats == 16 and diag.reachable_rats < 16:
            flags.append("tinder-unreachable-rat")
    return tuple(flags)


def oracle_score(candidate: Candidate, diag: Diag, flags: tuple[str, ...]) -> tuple[int, ...]:
    flag_weights = {
        # These are usually post-failure frontiers. Ranking them below states
        # with live mechanisms saves full solver timeouts on already sealed
        # continuations.
        "no-reachable-rats": 8,
        "no-remaining-mechanism": 8,
        "chase-unreachable-rat": 3,
        "tinder-unreachable-rat": 3,
        "release-trigger2-unreachable": 3,
        "reload-trigger2-unreachable": 3,
        "tug-low-reachability": 3,
        "tinder-dropped-rat": 8,
    }
    hard_flag_penalty = sum(flag_weights.get(flag, 0) for flag in flags)
    hard_flag_penalty += sum(2 for flag in flags if flag.endswith("-sealed"))
    return (
        hard_flag_penalty,
        diag.total_rats,
        diag.trapped,
        diag.total_rats - diag.reachable_rats,
        -diag.reachable_rats,
        -diag.reachable_triggers,
        -diag.features["explosives"],
        -diag.features["triggers"],
        moves_len(candidate.prefix),
    )


def select_for_diag(
    candidates: Iterable[Candidate],
    per_level_before_diag: int,
) -> list[Candidate]:
    by_level: dict[str, list[Candidate]] = defaultdict(list)
    for candidate in unique_by_prefix(candidates):
        by_level[candidate.level].append(candidate)

    selected = []
    for level, level_candidates in sorted(by_level.items()):
        ranked = sorted(level_candidates, key=coarse_rank)
        selected.extend(ranked[:per_level_before_diag])
    return selected


def record_json(record: TriageRecord) -> dict[str, Any]:
    return {
        "level": record.candidate.level,
        "prefix": record.candidate.prefix,
        "source": record.candidate.source,
        "score": list(record.score),
        "flags": list(record.flags),
        "diag": {
            "state": record.diag.state,
            "turns": record.diag.turns,
            "features": record.diag.features,
            "reachable_cells": record.diag.reachable_cells,
            "reachable_rats": record.diag.reachable_rats,
            "total_rats": record.diag.total_rats,
            "reachable_triggers": record.diag.reachable_triggers,
            "total_triggers": record.diag.total_triggers,
            "trapped": record.diag.trapped,
        },
    }


def diagnostic_signature(record: TriageRecord) -> tuple[Any, ...]:
    diag = record.diag
    features = diag.features
    return (
        record.flags,
        diag.total_rats,
        diag.reachable_rats,
        diag.reachable_triggers,
        diag.total_triggers,
        diag.trapped,
        features["explosives"],
        features["webs"],
        features["triggers"],
        features["planks"],
    )


def select_diverse_records(
    records: list[TriageRecord],
    limit: int,
    per_signature: int,
) -> list[TriageRecord]:
    selected: list[TriageRecord] = []
    signature_counts: dict[tuple[Any, ...], int] = defaultdict(int)

    for record in records:
        signature = diagnostic_signature(record)
        if per_signature <= 0 or signature_counts[signature] < per_signature:
            selected.append(record)
            signature_counts[signature] += 1
            if len(selected) >= limit:
                return selected
    return selected


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("archives", nargs="*", type=pathlib.Path)
    parser.add_argument("--include-static", action="store_true")
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument(
        "--solutions-file",
        type=pathlib.Path,
        default=FINAL_SOLUTIONS,
        help="final_solutions.json used by --skip-solved",
    )
    parser.add_argument(
        "--skip-solved",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="skip candidates whose level is already present in final_solutions.json",
    )
    parser.add_argument("--per-level-before-diag", type=int, default=24)
    parser.add_argument("--per-level", type=int, default=8)
    parser.add_argument(
        "--per-signature",
        type=int,
        default=2,
        help="select at most this many records with the same diagnostic signature per level; 0 disables the cap",
    )
    parser.add_argument(
        "--max-flag-penalty",
        type=int,
        default=7,
        help="drop records whose weighted hard-flag penalty is above this value when lower-penalty records exist; negative disables the filter",
    )
    parser.add_argument("--diag-timeout-sec", type=float, default=3.0)
    parser.add_argument("--jsonl-out", type=pathlib.Path)
    parser.add_argument("--seeds-out", type=pathlib.Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    candidates: list[Candidate] = []
    for archive in args.archives:
        candidates.extend(archive_candidates(archive))
    if args.include_static:
        candidates.extend(static_candidates())
    if args.only:
        candidates = [
            candidate
            for candidate in candidates
            if any(token in candidate.level or token in candidate.source for token in args.only)
        ]
    if args.skip_solved:
        solved_levels = load_solved_levels(args.solutions_file)
        candidates = [
            candidate
            for candidate in candidates
            if canonical_level(candidate.level) not in solved_levels
        ]

    selected = select_for_diag(candidates, args.per_level_before_diag)
    records: list[TriageRecord] = []
    for candidate in selected:
        diag = run_diag(candidate, args.diag_timeout_sec)
        if diag is None:
            continue
        flags = warning_flags(candidate.level, diag)
        records.append(
            TriageRecord(
                candidate=candidate,
                diag=diag,
                flags=flags,
                score=oracle_score(candidate, diag, flags),
            )
        )

    by_level: dict[str, list[TriageRecord]] = defaultdict(list)
    for record in records:
        by_level[record.candidate.level].append(record)

    final: list[TriageRecord] = []
    for level, level_records in sorted(by_level.items()):
        ranked = sorted(level_records, key=lambda record: record.score)
        eligible = ranked
        if args.max_flag_penalty >= 0:
            eligible = [
                record for record in ranked if record.score[0] <= args.max_flag_penalty
            ]
            if not eligible and ranked:
                eligible = ranked[:1]
        selected = select_diverse_records(eligible, args.per_level, args.per_signature)
        final.extend(selected)
        print(f"\n== {level} ==")
        for record in selected:
            diag = record.diag
            flags = ",".join(record.flags) if record.flags else "-"
            print(
                f"score={record.score} rats={diag.reachable_rats}/{diag.total_rats} "
                f"trig={diag.reachable_triggers}/{diag.total_triggers} "
                f"x={diag.features['explosives']} w={diag.features['webs']} "
                f"trapped={diag.trapped} turns={diag.turns} flags={flags} "
                f"source={pathlib.Path(record.candidate.source).name} "
                f"prefix={shlex.quote(record.candidate.prefix)}"
            )

    if args.jsonl_out is not None:
        args.jsonl_out.parent.mkdir(parents=True, exist_ok=True)
        with args.jsonl_out.open("w", encoding="utf-8") as out:
            for record in final:
                out.write(json.dumps(record_json(record), sort_keys=True) + "\n")
    if args.seeds_out is not None:
        args.seeds_out.parent.mkdir(parents=True, exist_ok=True)
        with args.seeds_out.open("w", encoding="utf-8") as out:
            for record in final:
                out.write(json.dumps(record_json(record), sort_keys=True) + "\n")

    print(f"\ntriaged={len(records)} selected={len(final)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
