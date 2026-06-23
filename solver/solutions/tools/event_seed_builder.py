#!/usr/bin/env python3
"""Build transfer-rankable seeds from structural event successors.

This is a macro-expansion step between archived return cells and the learned
transfer ranker. It asks the Rust oracle for one layer of `events --families`
successors, keeps the full oracle move prefix for each structural event, and
emits JSONL records that `transfer_ranker.py` and `go_explore_portfolio.py` can
consume.

The script does not implement game rules. It only parses oracle output.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import pathlib
import re
import resource
import subprocess
import sys
from collections import defaultdict
from collections.abc import Iterable
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Any

from go_explore_portfolio import (
    Candidate,
    SOLVER,
    archive_candidates,
    seed_file_candidates,
    static_candidates,
    unique_best,
)


ROOT = pathlib.Path(__file__).resolve().parents[3]

EVENT_RE = re.compile(
    r"EVENT idx=(?P<idx>\d+) moves=(?P<moves>\d+) score=(?P<score>-?\d+) "
    r"key=(?P<key>.*?) features=Features \{ rats: (?P<rats>\d+), "
    r"explosives: (?P<explosives>\d+), webs: (?P<webs>\d+), "
    r"triggers: (?P<triggers>\d+), planks: (?P<planks>\d+), "
    r"walls: (?P<walls>\d+) \} reachable_rats=(?P<reachable_rats>\d+) "
    r"trapped=(?P<trapped>\d+)"
)


LEVEL_GUARDS: dict[str, tuple[str, ...]] = {
    "levels/tinderrectangle.csv": (
        "--min-rats",
        "16",
        "--max-trapped-rats",
        "0",
    ),
    "levels/release.csv": (
        "--min-rats",
        "20",
        "--min-reachable-rats",
        "18",
        "--max-trapped-rats",
        "3",
        "--min-explosives",
        "4",
    ),
    "levels/cyborg_rats/ai_takeover.csv": (
        "--min-rats",
        "20",
        "--min-reachable-rats",
        "10",
        "--max-trapped-rats",
        "4",
        "--min-explosives",
        "4",
    ),
    "levels/reload_v3.csv": (
        "--min-rats",
        "3",
        "--min-reachable-rats",
        "1",
        "--max-trapped-rats",
        "2",
        "--min-triggers",
        "14",
    ),
    "levels/old_levels/on_the_clock.csv": (
        "--min-rats",
        "8",
        "--min-explosives",
        "3",
        "--min-triggers",
        "13",
        "--max-trapped-rats",
        "4",
    ),
    "levels/cooperation/handoff.csv": (
        "--min-rats",
        "5",
        "--min-explosives",
        "6",
        "--max-trapped-rats",
        "3",
    ),
    "levels/cooperation/blocked_v2.csv": (
        "--min-rats",
        "9",
        "--min-reachable-rats",
        "5",
        "--max-trapped-rats",
        "1",
    ),
}


@dataclasses.dataclass(frozen=True)
class EventSeed:
    parent: Candidate
    idx: int
    moves: int
    score: int
    event_key: str
    prefix: str
    features: dict[str, int]
    reachable_rats: int
    trapped: int
    snapshot: dict[str, Any] | None


def canonical_level(level: str) -> str:
    if level.startswith("levels/"):
        return level
    return f"levels/{level}"


def set_memory_limit(mem_mb: int) -> None:
    if mem_mb <= 0:
        return
    limit = mem_mb * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (limit, limit))


def run_events(
    candidate: Candidate,
    depth: int,
    secs: float,
    max_events: int,
    mem_mb: int,
    extra_args: tuple[str, ...],
    use_level_guards: bool,
) -> str | None:
    level = canonical_level(candidate.level)
    guards = LEVEL_GUARDS.get(level, ()) if use_level_guards else ()
    cmd = [
        str(SOLVER),
        "events",
        level,
        "--prefix",
        candidate.prefix,
        "--depth",
        str(depth),
        "--secs",
        str(secs),
        "--max",
        str(max_events),
        "--families",
        *guards,
        *extra_args,
    ]
    try:
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=secs + 12,
            check=False,
            preexec_fn=(lambda: set_memory_limit(mem_mb)) if mem_mb > 0 else None,
        )
    except subprocess.TimeoutExpired:
        return None
    if proc.returncode != 0:
        return None
    return proc.stdout


def run_snapshot(
    level: str,
    prefix: str,
    timeout_sec: float,
    *,
    include_csv: bool = False,
) -> dict[str, Any] | None:
    cmd = [str(SOLVER), "trajectory-json", canonical_level(level), prefix]
    if not include_csv:
        cmd.append("--no-csv")
    try:
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout_sec,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None
    if proc.returncode != 0:
        return None
    last: dict[str, Any] | None = None
    for line in proc.stdout.splitlines():
        try:
            last = json.loads(line)
        except json.JSONDecodeError:
            return None
    return last


def parse_events(
    parent: Candidate,
    text: str,
    with_snapshots: bool,
    snapshot_timeout: float,
    include_csv: bool,
) -> list[EventSeed]:
    events: list[EventSeed] = []
    pending: dict[str, Any] | None = None
    for line in text.splitlines():
        event_match = EVENT_RE.match(line)
        if event_match is not None:
            pending = event_match.groupdict()
            continue
        if pending is not None and line.startswith("FULL_ASCII "):
            prefix = line.removeprefix("FULL_ASCII ").strip()
            level = canonical_level(parent.level)
            snapshot = (
                run_snapshot(level, prefix, snapshot_timeout, include_csv=include_csv)
                if with_snapshots
                else None
            )
            events.append(
                EventSeed(
                    parent=parent,
                    idx=int(pending["idx"]),
                    moves=int(pending["moves"]),
                    score=int(pending["score"]),
                    event_key=pending["key"],
                    prefix=prefix,
                    features={
                        name: int(pending[name])
                        for name in ("rats", "explosives", "webs", "triggers", "planks", "walls")
                    },
                    reachable_rats=int(pending["reachable_rats"]),
                    trapped=int(pending["trapped"]),
                    snapshot=snapshot,
                )
            )
            pending = None
    return events


def load_candidates(args: argparse.Namespace) -> list[Candidate]:
    candidates: list[Candidate] = []
    for archive in args.archive:
        candidates.extend(archive_candidates(archive))
    for seed_file in args.seed_file:
        candidates.extend(seed_file_candidates(seed_file))
    if args.include_static:
        candidates.extend(static_candidates())
    if args.only:
        candidates = [
            candidate
            for candidate in candidates
            if any(token in canonical_level(candidate.level) or token in candidate.source for token in args.only)
        ]
    return unique_best(candidates, args.per_level, args.rank_key)


def seed_record(
    seed: EventSeed,
    learned_score: float | None = None,
    rank_score: float | None = None,
) -> dict[str, Any]:
    diag: dict[str, Any] = {}
    if seed.snapshot is not None:
        diag = {
            "state": seed.snapshot["play_state"],
            "turns": seed.snapshot["turn"],
            "features": seed.snapshot["features"],
            "reachable_cells": seed.snapshot["reachability"]["player_cells"],
            "reachable_rats": seed.snapshot["reachability"]["rats"],
            "total_rats": seed.snapshot["features"]["rats"],
            "reachable_triggers": seed.snapshot["reachability"]["triggers"],
            "total_triggers": seed.snapshot["features"]["triggers"],
            "trapped": seed.snapshot["reachability"]["trapped_unreachable_rats"],
        }
    record: dict[str, Any] = {
        "level": canonical_level(seed.parent.level),
        "prefix": seed.prefix,
        "source": f"event:{seed.parent.source}",
        "parent_prefix": seed.parent.prefix,
        "event": {
            "idx": seed.idx,
            "moves": seed.moves,
            "key": seed.event_key,
            "oracle_score": seed.score,
        },
        "score": seed.score,
        "rats": seed.features["rats"],
        "reachable_rats": seed.reachable_rats,
        "trapped": seed.trapped,
        "diag": diag,
    }
    if learned_score is not None:
        record["learned_score"] = learned_score
    if rank_score is not None:
        record["rank_score"] = rank_score
    return record


def dedupe_events(events: Iterable[EventSeed]) -> list[EventSeed]:
    by_key: dict[tuple[str, str], EventSeed] = {}
    for event in events:
        key = (canonical_level(event.parent.level), event.prefix)
        previous = by_key.get(key)
        if previous is None or (
            event.trapped,
            event.features["rats"],
            -event.reachable_rats,
            event.score,
            len(event.prefix.replace(" ", "")),
        ) < (
            previous.trapped,
            previous.features["rats"],
            -previous.reachable_rats,
            previous.score,
            len(previous.prefix.replace(" ", "")),
        ):
            by_key[key] = event
    return list(by_key.values())


def transfer_scores(args: argparse.Namespace, events: list[EventSeed]) -> dict[tuple[str, str], tuple[float, float]]:
    if not args.transfer_rank:
        return {}
    if args.no_snapshots:
        raise SystemExit("--transfer-rank requires snapshots; remove --no-snapshots")

    try:
        from transfer_ranker import (
            DEFAULT_PRETRAINED_CHECKPOINT,
            DEFAULT_PRETRAINED_REPO,
            SnapshotExample,
            canonical_level as transfer_level,
            load_obligation_examples,
            load_solved_examples,
            load_triage_examples,
            normalize_pretrained_checkpoints,
            score_ensemble_snapshot,
            train_transfer_ensemble,
        )
    except ImportError as error:
        raise SystemExit(
            "--transfer-rank requires the ML environment used by transfer_ranker.py "
            "(PyTorch, Hugging Face tooling, msgpack)"
        ) from error

    pretrained_repo = args.pretrained_repo or DEFAULT_PRETRAINED_REPO
    pretrained_checkpoints = normalize_pretrained_checkpoints(
        args.pretrained_checkpoint or [DEFAULT_PRETRAINED_CHECKPOINT]
    )
    include_csv = args.encoder == "semantic-dense10"
    solved = load_solved_examples(
        args.samples_per_solution,
        args.oracle_timeout_sec,
        include_csv=include_csv,
    )
    triage = load_triage_examples(
        args.triage,
        args.oracle_timeout_sec,
        include_csv=include_csv,
    )
    obligations = load_obligation_examples(
        args.obligation_labels,
        args.oracle_timeout_sec,
        include_csv=include_csv,
    )
    train_examples = solved + triage + obligations
    if len(train_examples) < 16:
        raise SystemExit("not enough oracle examples for transfer head training")

    ensemble = train_transfer_ensemble(
        train_examples,
        pretrained_repo,
        pretrained_checkpoints,
        epochs=args.epochs,
        seed=args.seed,
        freeze_prior=not args.fine_tune_prior,
        head=args.head,
        encoder=args.encoder,
    )

    candidate_examples: list[SnapshotExample] = []
    for event in events:
        if event.snapshot is None or event.snapshot["play_state"] == "GameOver":
            continue
        level = transfer_level(canonical_level(event.parent.level))
        candidate_examples.append(
            SnapshotExample(
                level=level,
                prefix=event.prefix,
                snapshot=event.snapshot,
                target=0.0,
                source=f"event:{event.parent.source}",
            )
        )
    if not candidate_examples:
        return {}

    height = max(
        max(int(example.snapshot["height"]) for example in train_examples),
        max(int(example.snapshot["height"]) for example in candidate_examples),
    )
    width = max(
        max(int(example.snapshot["width"]) for example in train_examples),
        max(int(example.snapshot["width"]) for example in candidate_examples),
    )
    scores = {}
    for example in candidate_examples:
        key = (canonical_level(example.level), example.prefix)
        learned_score, rank_score = score_ensemble_snapshot(ensemble, example, height, width)
        scores[key] = (learned_score, rank_score)
    print(
        f"transfer_rank pretrained={pretrained_repo}/{' + '.join(pretrained_checkpoints)} "
        f"head={args.head} encoder={args.encoder} frozen_prior={not args.fine_tune_prior} "
        f"train={len(train_examples)} solved={len(solved)} triage={len(triage)} "
        f"obligations={len(obligations)} scored_events={len(scores)}",
        file=sys.stderr,
        flush=True,
    )
    return scores


def event_sort_key(
    event: EventSeed,
    learned_scores: dict[tuple[str, str], tuple[float, float]],
) -> tuple[float, ...]:
    scores = learned_scores.get((canonical_level(event.parent.level), event.prefix))
    if scores is not None:
        _learned_score, rank_score = scores
        return (
            0.0,
            -rank_score,
            event.trapped,
            event.features["rats"],
            -event.reachable_rats,
            event.score,
            len(event.prefix.replace(" ", "")),
        )
    return (
        1.0,
        0.0,
        event.trapped,
        event.features["rats"],
        -event.reachable_rats,
        event.score,
        len(event.prefix.replace(" ", "")),
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--seed-file", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--include-static", action="store_true")
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--per-level", type=int, default=4)
    parser.add_argument("--rank-key", choices=("score", "default"), default="score")
    parser.add_argument("--depth", type=int, default=70)
    parser.add_argument("--secs", type=float, default=12.0)
    parser.add_argument("--max-events", type=int, default=12)
    parser.add_argument("--mem-mb", type=int, default=850)
    parser.add_argument(
        "--jobs",
        type=int,
        default=0,
        help="parallel event-expansion jobs; 0 chooses a CPU/memory-aware default",
    )
    parser.add_argument("--event-arg", action="append", default=[])
    parser.add_argument(
        "--level-guards",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="apply built-in per-level resource/reachability guards to event expansion",
    )
    parser.add_argument("--no-snapshots", action="store_true")
    parser.add_argument("--snapshot-timeout-sec", type=float, default=4.0)
    parser.add_argument(
        "--transfer-rank",
        action="store_true",
        help="train the frozen-prior transfer head and score event successors before writing seeds",
    )
    parser.add_argument("--triage", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--obligation-labels", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--samples-per-solution", type=int, default=10)
    parser.add_argument("--epochs", type=int, default=80)
    parser.add_argument("--seed", type=int, default=11)
    parser.add_argument("--oracle-timeout-sec", type=float, default=4.0)
    parser.add_argument("--pretrained-repo")
    parser.add_argument(
        "--pretrained-checkpoint",
        action="append",
        help=(
            "pretrained checkpoint to use as a frozen spatial prior; repeat or "
            "comma-separate values to ensemble checkpoints"
        ),
    )
    parser.add_argument(
        "--head",
        choices=("linear", "small-mlp", "dense-linear", "dense-mlp"),
        default="linear",
    )
    parser.add_argument(
        "--encoder",
        choices=("legacy", "semantic-dense10"),
        default="legacy",
        help="board encoder to pass through to transfer_ranker.py",
    )
    parser.add_argument("--fine-tune-prior", action="store_true")
    parser.add_argument("--jsonl-out", type=pathlib.Path, required=True)
    return parser.parse_args()


def available_memory_mb() -> int | None:
    try:
        for line in pathlib.Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemAvailable:"):
                return int(line.split()[1]) // 1024
    except OSError:
        return None
    return None


def auto_jobs(requested: int, candidate_count: int, mem_mb: int) -> int:
    if requested > 0:
        return max(1, min(requested, candidate_count))
    cpu_count = os.cpu_count() or 1
    if mem_mb <= 0:
        return max(1, min(cpu_count, candidate_count))
    mem_available = available_memory_mb()
    if mem_available is None:
        return max(1, min(cpu_count, candidate_count))
    reserve_mb = 4096
    memory_workers = max(1, (mem_available - reserve_mb) // mem_mb)
    return max(1, min(cpu_count, memory_workers, candidate_count))


def expand_candidate(
    candidate: Candidate,
    index: int,
    total: int,
    args: argparse.Namespace,
    extra_args: tuple[str, ...],
    include_csv: bool,
) -> list[EventSeed]:
    print(
        f"[{index}/{total}] events {canonical_level(candidate.level)} "
        f"prefix_len={len(candidate.prefix.split()) if ' ' in candidate.prefix else len(candidate.prefix)} "
        f"source={candidate.source}",
        file=sys.stderr,
        flush=True,
    )
    text = run_events(
        candidate,
        depth=args.depth,
        secs=args.secs,
        max_events=args.max_events,
        mem_mb=args.mem_mb,
        extra_args=extra_args,
        use_level_guards=args.level_guards,
    )
    if text is None:
        return []
    return parse_events(
        candidate,
        text,
        with_snapshots=not args.no_snapshots,
        snapshot_timeout=args.snapshot_timeout_sec,
        include_csv=include_csv,
    )


def main() -> int:
    args = parse_args()
    if not SOLVER.exists():
        raise SystemExit(f"missing solver binary: {SOLVER}")
    include_csv = args.encoder == "semantic-dense10"

    candidates = load_candidates(args)
    if not candidates:
        raise SystemExit("no candidates to expand")

    extra_args = tuple(str(arg) for item in args.event_arg for arg in item.split())
    all_events: list[EventSeed] = []
    jobs = auto_jobs(args.jobs, len(candidates), args.mem_mb)
    print(
        f"expanding parents={len(candidates)} jobs={jobs} requested_jobs={args.jobs} "
        f"mem_available_mb={available_memory_mb()} level_guards={args.level_guards}",
        file=sys.stderr,
        flush=True,
    )
    if jobs == 1:
        for index, candidate in enumerate(candidates, start=1):
            all_events.extend(
                expand_candidate(candidate, index, len(candidates), args, extra_args, include_csv)
            )
    else:
        with ThreadPoolExecutor(max_workers=jobs) as executor:
            futures = [
                executor.submit(
                    expand_candidate,
                    candidate,
                    index,
                    len(candidates),
                    args,
                    extra_args,
                    include_csv,
                )
                for index, candidate in enumerate(candidates, start=1)
            ]
            for future in as_completed(futures):
                all_events.extend(future.result())

    selected = dedupe_events(all_events)
    learned_scores = transfer_scores(args, selected)
    by_level: dict[str, list[EventSeed]] = defaultdict(list)
    for event in selected:
        by_level[canonical_level(event.parent.level)].append(event)

    args.jsonl_out.parent.mkdir(parents=True, exist_ok=True)
    written = 0
    with args.jsonl_out.open("w", encoding="utf-8") as out:
        for level in sorted(by_level):
            ranked = sorted(
                by_level[level],
                key=lambda event: event_sort_key(event, learned_scores),
            )
            for event in ranked:
                scores = learned_scores.get((canonical_level(event.parent.level), event.prefix))
                learned_score = None
                rank_score = None
                if scores is not None:
                    learned_score, rank_score = scores
                out.write(
                    json.dumps(
                        seed_record(event, learned_score=learned_score, rank_score=rank_score),
                        sort_keys=True,
                    )
                    + "\n"
                )
                written += 1

    print(
        f"parents={len(candidates)} raw_events={len(all_events)} "
        f"deduped={len(selected)} written={written} out={args.jsonl_out}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
