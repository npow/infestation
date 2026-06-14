#!/usr/bin/env python3
"""Transfer-guided frontier ranker using a pretrained Sokoban spatial prior.

The pretrained network is not an Infestation oracle and does not emit moves.
It contributes frozen convolutional filters learned on Sokoban boards; the
trainable part is only a small head over real Infestation oracle snapshots and
macro features. All candidate prefixes still need Rust-oracle verification.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import math
import pathlib
import subprocess
import sys
from collections import defaultdict
from collections.abc import Iterable
from typing import Any

import msgpack
import numpy as np
import torch
from huggingface_hub import hf_hub_download
from torch import nn

from go_explore_portfolio import Candidate, SOLVER, archive_candidates, static_candidates


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLUTIONS = ROOT / "solver" / "solutions" / "final_solutions.json"
DEFAULT_PRETRAINED_REPO = "AlignmentResearch/learned-planner"
DEFAULT_PRETRAINED_CHECKPOINT = "drc11/eue6pax7/cp_2002944000"


@dataclasses.dataclass(frozen=True)
class SnapshotExample:
    level: str
    prefix: str
    snapshot: dict[str, Any]
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


def prefix_len(prefix: str) -> int:
    if " " in prefix:
        return len(prefix.split())
    return len(prefix)


def canonical_level(level: str) -> str:
    return level.removeprefix("levels/")


def level_path(level: str) -> pathlib.Path:
    level = canonical_level(level)
    return ROOT / "levels" / level


def run_trajectory_snapshot(level: str, prefix: str, timeout_sec: float) -> dict[str, Any] | None:
    try:
        proc = subprocess.run(
            [
                str(SOLVER),
                "trajectory-json",
                str(level_path(level)),
                prefix,
                "--no-csv",
            ],
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
    rows = []
    for line in proc.stdout.splitlines():
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            return None
    if not rows:
        return None
    return rows[-1]


def load_solved_examples(samples_per_solution: int, timeout_sec: float) -> list[SnapshotExample]:
    data = json.loads(SOLUTIONS.read_text(encoding="utf-8"))
    examples = []
    for level, record in data.items():
        moves = record["sol"]
        players = int(record.get("players", 1))
        tokens = move_tokens(moves, players)
        if not tokens:
            continue
        wanted = {
            0,
            1,
            len(tokens) // 4,
            len(tokens) // 2,
            (3 * len(tokens)) // 4,
            max(0, len(tokens) - 2),
            max(0, len(tokens) - 1),
            len(tokens),
        }
        if samples_per_solution > len(wanted):
            wanted.update(np.linspace(0, len(tokens), samples_per_solution, dtype=int).tolist())
        indices = sorted(index for index in wanted if 0 <= index <= len(tokens))[:samples_per_solution]
        denominator = max(1, len(tokens))
        for index in indices:
            prefix = join_tokens(tokens[:index], players)
            snapshot = run_trajectory_snapshot(level, prefix, timeout_sec)
            if snapshot is None or snapshot["play_state"] == "GameOver":
                continue
            progress = index / denominator
            target = 1.0 if snapshot["play_state"] == "Won" else 0.35 + 0.60 * progress
            examples.append(
                SnapshotExample(
                    level=level,
                    prefix=prefix,
                    snapshot=snapshot,
                    target=target,
                    source="solved-trajectory",
                )
            )
    return examples


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
    return min(0.90, penalty)


def snapshot_badness(snapshot: dict[str, Any]) -> float:
    features = snapshot["features"]
    reach = snapshot["reachability"]
    rats = int(features["rats"])
    penalty = 0.0
    if rats > 0 and int(reach["rats"]) == 0:
        penalty += 0.35
    if rats > 0 and int(features["triggers"]) == 0 and int(features["explosives"]) == 0:
        penalty += 0.35
    penalty += min(0.30, 0.08 * int(reach["trapped_unreachable_rats"]))
    if snapshot["play_state"] == "GameOver":
        penalty += 0.50
    return min(0.95, penalty)


def load_triage_examples(paths: Iterable[pathlib.Path], timeout_sec: float) -> list[SnapshotExample]:
    examples = []
    seen: set[tuple[str, str]] = set()
    for path in paths:
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            level = record.get("level")
            prefix = record.get("prefix")
            if not isinstance(level, str) or not isinstance(prefix, str):
                continue
            key = (canonical_level(level), prefix)
            if key in seen:
                continue
            seen.add(key)
            snapshot = run_trajectory_snapshot(level, prefix, timeout_sec)
            if snapshot is None:
                continue
            flags = record.get("flags") or []
            if not isinstance(flags, list):
                flags = []
            target = max(0.02, 0.35 - badness_from_flags(str(flag) for flag in flags))
            target = min(target, 0.35 - snapshot_badness(snapshot))
            examples.append(
                SnapshotExample(
                    level=canonical_level(level),
                    prefix=prefix,
                    snapshot=snapshot,
                    target=max(0.02, target),
                    source=str(path),
                )
            )
    return examples


def load_candidates(
    archives: Iterable[pathlib.Path],
    seed_files: Iterable[pathlib.Path],
    include_static: bool,
    candidate_limit: int,
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
                features = diag.get("features") if isinstance(diag.get("features"), dict) else {}
                candidates.append(
                    Candidate(
                        level=level,
                        prefix=prefix,
                        source=str(record.get("source") or seed_file),
                        rats=int(record.get("rats", features.get("rats", 999))),
                        reachable_rats=int(record.get("reachable_rats", diag.get("reachable_rats", -1))),
                        trapped=int(record.get("trapped", diag.get("trapped", 999))),
                        score=int(record.get("score", 0)),
                    )
                )
    if include_static:
        candidates.extend(static_candidates())

    by_key: dict[tuple[str, str], Candidate] = {}
    for candidate in candidates:
        key = (canonical_level(candidate.level), candidate.prefix)
        previous = by_key.get(key)
        if previous is None or (
            candidate.rats,
            -candidate.reachable_rats,
            candidate.trapped,
            candidate.score,
        ) < (
            previous.rats,
            -previous.reachable_rats,
            previous.trapped,
            previous.score,
        ):
            by_key[key] = candidate

    by_level: dict[str, list[Candidate]] = defaultdict(list)
    for candidate in by_key.values():
        by_level[canonical_level(candidate.level)].append(candidate)

    selected: list[Candidate] = []
    per_level = max(1, math.ceil(candidate_limit / max(1, len(by_level))))
    for level in sorted(by_level):
        ranked = sorted(
            by_level[level],
            key=lambda candidate: (
                candidate.rats,
                -candidate.reachable_rats,
                candidate.trapped,
                candidate.score,
                prefix_len(candidate.prefix),
            ),
        )
        selected.extend(ranked[:per_level])
    return selected[:candidate_limit]


def unpack_flax_array(code: int, data: bytes) -> Any:
    if code != 1:
        return msgpack.ExtType(code, data)
    shape, dtype_name, raw = msgpack.unpackb(data, raw=False, strict_map_key=False)
    return np.frombuffer(raw, dtype=np.dtype(dtype_name)).reshape(shape)


def load_pretrained_params(repo: str, checkpoint: str) -> dict[str, np.ndarray]:
    model_path = pathlib.Path(hf_hub_download(repo, f"{checkpoint}/model"))
    cfg_path = pathlib.Path(hf_hub_download(repo, f"{checkpoint}/cfg.json"))
    raw = msgpack.unpackb(
        model_path.read_bytes(),
        raw=False,
        strict_map_key=False,
        ext_hook=unpack_flax_array,
    )
    params = raw["params"]["params"]["network_params"]
    return {
        "cfg_path": np.array(str(cfg_path)),
        "conv0_kernel": params["conv_list_0"]["kernel"][0],
        "conv0_bias": params["conv_list_0"]["bias"][0],
        "conv1_kernel": params["conv_list_1"]["kernel"][0],
        "conv1_bias": params["conv_list_1"]["bias"][0],
    }


class TransferRanker(nn.Module):
    def __init__(self, macro_dim: int, pretrained: dict[str, np.ndarray], freeze_prior: bool) -> None:
        super().__init__()
        self.conv0 = nn.Conv2d(3, 32, kernel_size=4, padding="same")
        self.conv1 = nn.Conv2d(32, 32, kernel_size=4, padding="same")
        self.activation = nn.ReLU()
        self.pool = nn.AdaptiveAvgPool2d((1, 1))
        self.head = nn.Sequential(
            nn.Linear(32 + macro_dim, 96),
            nn.ReLU(),
            nn.Linear(96, 32),
            nn.ReLU(),
            nn.Linear(32, 1),
        )
        self.load_spatial_prior(pretrained)
        if freeze_prior:
            for parameter in list(self.conv0.parameters()) + list(self.conv1.parameters()):
                parameter.requires_grad = False

    def load_spatial_prior(self, pretrained: dict[str, np.ndarray]) -> None:
        conv0 = torch.tensor(pretrained["conv0_kernel"]).permute(3, 2, 0, 1)
        conv1 = torch.tensor(pretrained["conv1_kernel"]).permute(3, 2, 0, 1)
        with torch.no_grad():
            self.conv0.weight.copy_(conv0)
            self.conv0.bias.copy_(torch.tensor(pretrained["conv0_bias"]))
            self.conv1.weight.copy_(conv1)
            self.conv1.bias.copy_(torch.tensor(pretrained["conv1_bias"]))

    def forward(self, board: torch.Tensor, macro: torch.Tensor) -> torch.Tensor:
        spatial = self.activation(self.conv0(board))
        spatial = self.activation(self.conv1(spatial))
        spatial = self.pool(spatial).flatten(1)
        return self.head(torch.cat([spatial, macro], dim=1)).squeeze(1)


def add_points(channel: np.ndarray, positions: list[dict[str, Any]], value: float = 1.0) -> None:
    for point in positions:
        x = int(point["x"])
        y = int(point["y"])
        if 0 <= y < channel.shape[0] and 0 <= x < channel.shape[1]:
            channel[y, x] = value


def board_tensor(snapshot: dict[str, Any], height: int, width: int) -> np.ndarray:
    board = np.zeros((3, height, width), dtype=np.float32)
    positions = snapshot["positions"]
    add_points(board[0], positions["players"])
    add_points(board[1], positions["webs"])
    add_points(board[1], positions["planks"])
    add_points(board[1], positions["black_holes"])
    add_points(board[2], positions["rats"])
    add_points(board[2], positions["cyborg_rats"])
    add_points(board[2], positions["triggers"], 0.75)
    add_points(board[2], positions["explosives"], 0.50)
    for point in positions["black_holes"]:
        x = int(point["x"])
        y = int(point["y"])
        if 0 <= y < height and 0 <= x < width:
            board[2, y, x] = max(board[2, y, x], 0.35)
    return board


def macro_features(level: str, prefix: str, snapshot: dict[str, Any]) -> np.ndarray:
    features = snapshot["features"]
    reach = snapshot["reachability"]
    rats = max(1, int(features["rats"]))
    triggers = max(1, int(features["triggers"]))
    return np.array(
        [
            int(features["rats"]),
            int(features["explosives"]),
            int(features["webs"]),
            int(features["triggers"]),
            int(features["planks"]),
            int(features["walls"]),
            int(reach["player_cells"]),
            int(reach["rats"]),
            int(reach["triggers"]),
            int(reach["trapped_unreachable_rats"]),
            int(features["rats"]) - int(reach["rats"]),
            int(reach["rats"]) / rats,
            int(reach["triggers"]) / triggers,
            int(reach["trapped_unreachable_rats"]) / rats,
            prefix_len(prefix),
            1.0 if "cooperation/" in level or " " in prefix else 0.0,
            1.0 if snapshot["play_state"] == "Won" else 0.0,
            1.0 if snapshot["play_state"] == "Playing" else 0.0,
        ],
        dtype=np.float32,
    )


def tensors_for_examples(
    examples: list[SnapshotExample],
    mean: np.ndarray | None = None,
    std: np.ndarray | None = None,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, np.ndarray, np.ndarray]:
    height = max(int(example.snapshot["height"]) for example in examples)
    width = max(int(example.snapshot["width"]) for example in examples)
    boards = np.stack([board_tensor(example.snapshot, height, width) for example in examples])
    macros = np.stack(
        [macro_features(example.level, example.prefix, example.snapshot) for example in examples]
    )
    if mean is None or std is None:
        mean = macros.mean(axis=0)
        std = macros.std(axis=0)
        std[std < 1e-6] = 1.0
    macros = (macros - mean) / std
    targets = np.array([example.target for example in examples], dtype=np.float32)
    return (
        torch.tensor(boards),
        torch.tensor(macros, dtype=torch.float32),
        torch.tensor(targets),
        mean,
        std,
    )


def train_model(
    examples: list[SnapshotExample],
    pretrained: dict[str, np.ndarray],
    epochs: int,
    seed: int,
    freeze_prior: bool,
) -> tuple[TransferRanker, np.ndarray, np.ndarray]:
    torch.manual_seed(seed)
    boards, macros, targets, mean, std = tensors_for_examples(examples)
    model = TransferRanker(macros.shape[1], pretrained, freeze_prior=freeze_prior)
    optimizer = torch.optim.AdamW(
        [parameter for parameter in model.parameters() if parameter.requires_grad],
        lr=2e-3,
        weight_decay=1e-4,
    )
    loss_fn = nn.BCEWithLogitsLoss()
    for epoch in range(epochs):
        order = torch.randperm(len(targets))
        for start in range(0, len(order), 32):
            batch = order[start : start + 32]
            logits = model(boards[batch], macros[batch])
            loss = loss_fn(logits, targets[batch])
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
    return model.eval(), mean, std


def score_snapshot(
    model: TransferRanker,
    mean: np.ndarray,
    std: np.ndarray,
    example: SnapshotExample,
    height: int,
    width: int,
) -> float:
    board = torch.tensor(board_tensor(example.snapshot, height, width)[None, :, :, :])
    macro = macro_features(example.level, example.prefix, example.snapshot)
    macro = torch.tensor(((macro - mean) / std)[None, :], dtype=torch.float32)
    with torch.no_grad():
        score = torch.sigmoid(model(board, macro)).item()
    return float(score - snapshot_badness(example.snapshot))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--seed-file", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--triage", action="append", type=pathlib.Path, default=[])
    parser.add_argument("--include-static", action="store_true")
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--samples-per-solution", type=int, default=10)
    parser.add_argument("--candidate-limit", type=int, default=240)
    parser.add_argument("--top", type=int, default=8)
    parser.add_argument("--epochs", type=int, default=80)
    parser.add_argument("--seed", type=int, default=11)
    parser.add_argument("--oracle-timeout-sec", type=float, default=4.0)
    parser.add_argument("--pretrained-repo", default=DEFAULT_PRETRAINED_REPO)
    parser.add_argument("--pretrained-checkpoint", default=DEFAULT_PRETRAINED_CHECKPOINT)
    parser.add_argument("--fine-tune-prior", action="store_true")
    parser.add_argument("--jsonl-out", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not SOLVER.exists():
        raise SystemExit(f"missing solver binary: {SOLVER}")

    pretrained = load_pretrained_params(args.pretrained_repo, args.pretrained_checkpoint)
    solved = load_solved_examples(args.samples_per_solution, args.oracle_timeout_sec)
    triage = load_triage_examples(args.triage, args.oracle_timeout_sec)
    train_examples = solved + triage
    if len(train_examples) < 16:
        raise SystemExit("not enough oracle examples for transfer head training")

    model, mean, std = train_model(
        train_examples,
        pretrained,
        epochs=args.epochs,
        seed=args.seed,
        freeze_prior=not args.fine_tune_prior,
    )

    candidates = load_candidates(
        args.archive,
        args.seed_file,
        include_static=args.include_static,
        candidate_limit=args.candidate_limit,
    )
    if args.only:
        candidates = [
            candidate
            for candidate in candidates
            if any(token in candidate.level or token in candidate.source for token in args.only)
        ]

    candidate_examples: list[SnapshotExample] = []
    for candidate in candidates:
        level = canonical_level(candidate.level)
        snapshot = run_trajectory_snapshot(level, candidate.prefix, args.oracle_timeout_sec)
        if snapshot is None or snapshot["play_state"] == "GameOver":
            continue
        candidate_examples.append(
            SnapshotExample(
                level=level,
                prefix=candidate.prefix,
                snapshot=snapshot,
                target=0.0,
                source=candidate.source,
            )
        )

    if not candidate_examples:
        raise SystemExit("no candidate prefixes survived oracle replay")

    height = max(
        max(int(example.snapshot["height"]) for example in train_examples),
        max(int(example.snapshot["height"]) for example in candidate_examples),
    )
    width = max(
        max(int(example.snapshot["width"]) for example in train_examples),
        max(int(example.snapshot["width"]) for example in candidate_examples),
    )
    ranked = sorted(
        (
            (score_snapshot(model, mean, std, example, height, width), example)
            for example in candidate_examples
        ),
        key=lambda row: row[0],
        reverse=True,
    )

    by_level: dict[str, list[tuple[float, SnapshotExample]]] = defaultdict(list)
    for row in ranked:
        by_level[row[1].level].append(row)

    args.jsonl_out.parent.mkdir(parents=True, exist_ok=True)
    selected = []
    with args.jsonl_out.open("w", encoding="utf-8") as out:
        for level in sorted(by_level):
            for score, example in by_level[level][: args.top]:
                selected.append((score, example))
                record = {
                    "level": f"levels/{example.level}",
                    "prefix": example.prefix,
                    "source": example.source,
                    "learned_score": score,
                    "transfer": {
                        "repo": args.pretrained_repo,
                        "checkpoint": args.pretrained_checkpoint,
                        "frozen_prior": not args.fine_tune_prior,
                    },
                    "diag": {
                        "state": example.snapshot["play_state"],
                        "turns": example.snapshot["turn"],
                        "features": example.snapshot["features"],
                        "reachable_cells": example.snapshot["reachability"]["player_cells"],
                        "reachable_rats": example.snapshot["reachability"]["rats"],
                        "total_rats": example.snapshot["features"]["rats"],
                        "reachable_triggers": example.snapshot["reachability"]["triggers"],
                        "total_triggers": example.snapshot["features"]["triggers"],
                        "trapped": example.snapshot["reachability"]["trapped_unreachable_rats"],
                    },
                }
                out.write(json.dumps(record, sort_keys=True) + "\n")

    print(
        f"pretrained={args.pretrained_repo}/{args.pretrained_checkpoint} "
        f"train={len(train_examples)} solved={len(solved)} triage={len(triage)} "
        f"candidates={len(candidates)} replayed={len(candidate_examples)} selected={len(selected)} "
        f"out={args.jsonl_out}"
    )
    for score, example in selected[: min(40, len(selected))]:
        features = example.snapshot["features"]
        reach = example.snapshot["reachability"]
        print(
            f"{score:.3f} {example.level} rats={reach['rats']}/{features['rats']} "
            f"trig={reach['triggers']}/{features['triggers']} trapped={reach['trapped_unreachable_rats']} "
            f"turns={example.snapshot['turn']} source={pathlib.Path(example.source).name} "
            f"prefix={example.prefix!r}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
