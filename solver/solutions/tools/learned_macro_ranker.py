#!/usr/bin/env python3
"""Learn a lightweight value model for macro/frontier ranking.

This is intentionally not a second Infestation engine. It asks the Rust oracle
for diagnostics, trains a small NumPy MLP from solved trajectories plus known
bad frontier states, and ranks candidate prefixes for later oracle verification.
The model is a guide for where to spend exact search, not proof of solvability.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import math
import pathlib
import re
import subprocess
import sys
from collections import defaultdict
from collections.abc import Iterable
from typing import Any

import numpy as np

from frontier_triage import Diag, parse_diag, record_json, warning_flags
from go_explore_portfolio import (
    Candidate,
    SOLVER,
    archive_candidates,
    coerce_score,
    static_candidates,
)


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLUTIONS = ROOT / "solver" / "solutions" / "final_solutions.json"


@dataclasses.dataclass(frozen=True)
class Example:
    level: str
    prefix: str
    diag: Diag
    target: float
    source: str


def move_tokens(moves: str, players: int) -> list[str]:
    if players == 2 or " " in moves:
        return moves.split()
    return list(moves)


def join_tokens(tokens: list[str], players: int) -> str:
    if players == 2:
        return " ".join(tokens)
    return "".join(tokens)


def sample_prefixes(moves: str, players: int, samples: int) -> list[tuple[str, float]]:
    tokens = move_tokens(moves, players)
    if not tokens:
        return []
    indices = sorted(
        {
            0,
            1,
            len(tokens) // 4,
            len(tokens) // 2,
            (3 * len(tokens)) // 4,
            max(0, len(tokens) - 2),
            max(0, len(tokens) - 1),
        }
    )
    if samples > len(indices):
        for index in np.linspace(0, len(tokens) - 1, samples, dtype=int).tolist():
            indices.append(int(index))
        indices = sorted(set(indices))
    result = []
    denominator = max(1, len(tokens) - 1)
    for index in indices[:samples]:
        prefix = join_tokens(tokens[:index], players)
        # A solved trajectory prefix is not "won" yet, but later prefixes should
        # be ranked higher because fewer precise choices remain.
        progress = index / denominator
        result.append((prefix, 0.35 + 0.65 * progress))
    return result


def run_diag(level: str, prefix: str, timeout_sec: float) -> Diag | None:
    try:
        completed = subprocess.run(
            [str(SOLVER), "diag", str(ROOT / "levels" / level), prefix],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout_sec,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None
    if completed.returncode != 0:
        return None
    return parse_diag(completed.stdout)


def load_solved_examples(samples_per_solution: int, diag_timeout_sec: float) -> list[Example]:
    data = json.loads(SOLUTIONS.read_text(encoding="utf-8"))
    examples = []
    for level, record in data.items():
        moves = record["sol"]
        players = int(record.get("players", 1))
        for prefix, target in sample_prefixes(moves, players, samples_per_solution):
            diag = run_diag(level, prefix, diag_timeout_sec)
            if diag is None or diag.state == "GameOver":
                continue
            examples.append(
                Example(
                    level=level,
                    prefix=prefix,
                    diag=diag,
                    target=target,
                    source="solved-trajectory",
                )
            )
    return examples


def triage_diag(record: dict[str, Any]) -> Diag | None:
    diag = record.get("diag")
    if not isinstance(diag, dict):
        return None
    features = diag.get("features")
    if not isinstance(features, dict):
        return None
    return Diag(
        state=str(diag.get("state", "Playing")),
        turns=int(diag.get("turns", 0)),
        features={key: int(value) for key, value in features.items()},
        reachable_cells=int(diag.get("reachable_cells", 0)),
        reachable_rats=int(diag.get("reachable_rats", 0)),
        total_rats=int(diag.get("total_rats", features.get("rats", 0))),
        reachable_triggers=int(diag.get("reachable_triggers", 0)),
        total_triggers=int(diag.get("total_triggers", features.get("triggers", 0))),
        trapped=int(diag.get("trapped", 0)),
        rat_lines={},
        trigger_lines={},
    )


def badness_from_flags(flags: Iterable[str]) -> float:
    joined = " ".join(flags)
    penalty = 0.0
    for marker, value in [
        ("no-reachable-rats", 0.35),
        ("no-remaining-mechanism", 0.40),
        ("sealed", 0.25),
        ("unreachable", 0.15),
        ("trapped", 0.10),
    ]:
        if marker in joined:
            penalty += value
    return min(0.85, penalty)


def load_triage_examples(paths: Iterable[pathlib.Path]) -> list[Example]:
    examples = []
    for path in paths:
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            diag = triage_diag(record)
            level = record.get("level")
            prefix = record.get("prefix")
            if diag is None or not isinstance(level, str) or not isinstance(prefix, str):
                continue
            flags = record.get("flags") or []
            if not isinstance(flags, list):
                flags = []
            target = max(0.02, 0.30 - badness_from_flags(str(flag) for flag in flags))
            if diag.total_rats and diag.reachable_rats == diag.total_rats and diag.trapped == 0:
                target = max(target, 0.35)
            examples.append(
                Example(
                    level=level.removeprefix("levels/"),
                    prefix=prefix,
                    diag=diag,
                    target=target,
                    source=str(path),
                )
            )
    return examples


def features_for(level: str, prefix: str, diag: Diag) -> np.ndarray:
    values = diag.features
    rats = max(1, diag.total_rats)
    triggers = max(1, diag.total_triggers)
    cells = max(1, diag.reachable_cells)
    return np.array(
        [
            values.get("rats", 0),
            values.get("explosives", 0),
            values.get("webs", 0),
            values.get("triggers", 0),
            values.get("planks", 0),
            values.get("walls", 0),
            diag.reachable_cells,
            diag.reachable_rats,
            diag.total_rats,
            diag.reachable_triggers,
            diag.total_triggers,
            diag.trapped,
            diag.total_rats - diag.reachable_rats,
            diag.reachable_rats / rats,
            diag.reachable_triggers / triggers,
            diag.trapped / rats,
            len(prefix.split()) if " " in prefix else len(prefix),
            1.0 if "cooperation/" in level or " " in prefix else 0.0,
            1.0 if diag.state == "Won" else 0.0,
            1.0 if diag.state == "Playing" else 0.0,
        ],
        dtype=np.float64,
    )


def train_mlp(x: np.ndarray, y: np.ndarray, epochs: int, seed: int) -> tuple[np.ndarray, ...]:
    rng = np.random.default_rng(seed)
    hidden = 32
    w1 = rng.normal(0.0, 0.12, size=(x.shape[1], hidden))
    b1 = np.zeros(hidden)
    w2 = rng.normal(0.0, 0.12, size=(hidden, 1))
    b2 = np.zeros(1)
    lr = 0.015

    for epoch in range(epochs):
        order = rng.permutation(len(x))
        for start in range(0, len(order), 32):
            batch = order[start : start + 32]
            xb = x[batch]
            yb = y[batch, None]
            h = np.tanh(xb @ w1 + b1)
            pred = 1.0 / (1.0 + np.exp(-(h @ w2 + b2)))
            error = pred - yb
            grad_out = error * pred * (1.0 - pred) / len(batch)
            grad_w2 = h.T @ grad_out
            grad_b2 = grad_out.sum(axis=0)
            grad_h = (grad_out @ w2.T) * (1.0 - h * h)
            grad_w1 = xb.T @ grad_h
            grad_b1 = grad_h.sum(axis=0)
            w2 -= lr * grad_w2
            b2 -= lr * grad_b2
            w1 -= lr * grad_w1
            b1 -= lr * grad_b1
        if epoch and epoch % 80 == 0:
            lr *= 0.75
    return w1, b1, w2, b2


def predict(x: np.ndarray, params: tuple[np.ndarray, ...]) -> np.ndarray:
    w1, b1, w2, b2 = params
    h = np.tanh(x @ w1 + b1)
    return (1.0 / (1.0 + np.exp(-(h @ w2 + b2)))).ravel()


def normalize_fit(x: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    mean = x.mean(axis=0)
    std = x.std(axis=0)
    std[std < 1e-6] = 1.0
    return mean, std


def candidate_key(candidate: Candidate) -> tuple[str, str]:
    return candidate.level.removeprefix("levels/"), candidate.prefix


def load_candidates(
    archives: Iterable[pathlib.Path],
    seed_files: Iterable[pathlib.Path],
    include_static: bool,
) -> list[Candidate]:
    candidates: list[Candidate] = []
    for archive in archives:
        candidates.extend(archive_candidates(archive))
    for seed_file in seed_files:
        if not seed_file.exists():
            continue
        for line in seed_file.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            level = record.get("level")
            prefix = record.get("prefix")
            if isinstance(level, str) and isinstance(prefix, str):
                diag = record.get("diag") if isinstance(record.get("diag"), dict) else {}
                diag_features = diag.get("features") if isinstance(diag.get("features"), dict) else {}
                top_features = record.get("features") if isinstance(record.get("features"), dict) else {}
                features = diag_features or top_features
                candidates.append(
                    Candidate(
                        level=level,
                        prefix=prefix,
                        source=str(record.get("source") or seed_file),
                        rats=int(record.get("rats", features.get("rats", diag.get("total_rats", 999)))),
                        reachable_rats=int(record.get("reachable_rats", diag.get("reachable_rats", -1))),
                        trapped=int(record.get("trapped", diag.get("trapped", 999))),
                        score=coerce_score(record.get("score")),
                    )
                )
    if include_static:
        candidates.extend(static_candidates())
    by_key = {}
    for candidate in candidates:
        by_key.setdefault(candidate_key(candidate), candidate)
    return list(by_key.values())


def score_candidates(
    candidates: list[Candidate],
    mean: np.ndarray,
    std: np.ndarray,
    params: tuple[np.ndarray, ...],
    diag_timeout_sec: float,
) -> list[tuple[float, Candidate, Diag, tuple[str, ...]]]:
    scored = []
    for candidate in candidates:
        level = candidate.level.removeprefix("levels/")
        diag = run_diag(level, candidate.prefix, diag_timeout_sec)
        if diag is None or diag.state == "GameOver":
            continue
        flags = warning_flags(f"levels/{level}", diag)
        x = features_for(level, candidate.prefix, diag)[None, :]
        model_score = float(predict((x - mean) / std, params)[0])
        # Keep a small hard-constraint correction so the learned model cannot
        # overrate obviously sealed/no-mechanism states.
        corrected = model_score - badness_from_flags(flags)
        scored.append((corrected, candidate, diag, flags))
    return sorted(scored, key=lambda row: row[0], reverse=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--seed-file", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--triage", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--include-static", action="store_true")
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--samples-per-solution", type=int, default=8)
    parser.add_argument("--epochs", type=int, default=320)
    parser.add_argument("--diag-timeout-sec", type=float, default=3.0)
    parser.add_argument("--top", type=int, default=30)
    parser.add_argument("--jsonl-out", type=pathlib.Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    solved = load_solved_examples(args.samples_per_solution, args.diag_timeout_sec)
    triage = load_triage_examples(args.triage)
    train = solved + triage
    if len(train) < 8:
        raise SystemExit("not enough training examples")

    x_raw = np.vstack([features_for(example.level, example.prefix, example.diag) for example in train])
    y = np.array([example.target for example in train], dtype=np.float64)
    mean, std = normalize_fit(x_raw)
    params = train_mlp((x_raw - mean) / std, y, args.epochs, seed=7)

    candidates = load_candidates(args.archive, args.seed_file, args.include_static)
    if args.only:
        candidates = [
            candidate
            for candidate in candidates
            if any(token in candidate.level or token in candidate.source for token in args.only)
        ]
    ranked = score_candidates(candidates, mean, std, params, args.diag_timeout_sec)

    print(
        f"train_examples={len(train)} solved_examples={len(solved)} "
        f"triage_examples={len(triage)} candidates={len(candidates)} ranked={len(ranked)}"
    )
    by_level: dict[str, list[tuple[float, Candidate, Diag, tuple[str, ...]]]] = defaultdict(list)
    for row in ranked:
        by_level[row[1].level].append(row)
    selected = []
    for level in sorted(by_level):
        print(f"\n== {level} ==")
        for score, candidate, diag, flags in by_level[level][: args.top]:
            selected.append((score, candidate, diag, flags))
            flag_text = ",".join(flags) if flags else "-"
            print(
                f"score={score:.3f} rats={diag.reachable_rats}/{diag.total_rats} "
                f"trig={diag.reachable_triggers}/{diag.total_triggers} "
                f"x={diag.features['explosives']} w={diag.features['webs']} "
                f"trapped={diag.trapped} turns={diag.turns} flags={flag_text} "
                f"source={pathlib.Path(candidate.source).name} prefix={candidate.prefix!r}"
            )

    if args.jsonl_out is not None:
        args.jsonl_out.parent.mkdir(parents=True, exist_ok=True)
        with args.jsonl_out.open("w", encoding="utf-8") as out:
            for score, candidate, diag, flags in selected:
                out.write(
                    json.dumps(
                        {
                            "level": candidate.level,
                            "prefix": candidate.prefix,
                            "source": candidate.source,
                            "learned_score": score,
                            "flags": list(flags),
                            "diag": {
                                "state": diag.state,
                                "turns": diag.turns,
                                "features": diag.features,
                                "reachable_cells": diag.reachable_cells,
                                "reachable_rats": diag.reachable_rats,
                                "total_rats": diag.total_rats,
                                "reachable_triggers": diag.reachable_triggers,
                                "total_triggers": diag.total_triggers,
                                "trapped": diag.trapped,
                            },
                        },
                        sort_keys=True,
                    )
                    + "\n"
                )
    return 0


if __name__ == "__main__":
    sys.exit(main())
