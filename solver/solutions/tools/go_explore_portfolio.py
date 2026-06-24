#!/usr/bin/env python3
"""Run archive-seeded, bounded Go-Explore-style solver probes.

The direct win reward is sparse on the hard Infestation levels. This runner
uses the log archive as a set of return cells: keep promising prefixes, restart
from them, and explore with several bounded solver modes. It is deliberately
conservative with memory and wall time so parallel runs do not OOM the host.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import resource
import shlex
import signal
import subprocess
import sys
import time
from collections import Counter, defaultdict, deque
from collections.abc import Iterable
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLVER = ROOT / "target" / "release" / "solver"
RUN_ROOT = pathlib.Path("/tmp/infestation-runs")
DEFAULT_ARCHIVE = RUN_ROOT / "archive_20260614_full.jsonl"
FINAL_SOLUTIONS = ROOT / "solver" / "solutions" / "final_solutions.json"
DEFAULT_OBLIGATION_LABELS = ROOT / "solver" / "solutions" / "tools" / "obligation_labels.jsonl"

LEVEL_RE = re.compile(r"\blevels/[^\s'\"]+?\.csv\b")


@dataclasses.dataclass(frozen=True)
class SourceMeta:
    level: str
    mode: str
    prefix: str


@dataclasses.dataclass(frozen=True)
class Candidate:
    level: str
    prefix: str
    source: str
    rats: int
    reachable_rats: int
    trapped: int
    score: int
    event_key: str | None = None
    flags: tuple[str, ...] = ()
    diag_known: bool = False


@dataclasses.dataclass(frozen=True)
class Job:
    name: str
    args: tuple[str, ...]
    timeout_sec: int
    mem_mb: int
    cpu_slots: int
    prune_dead: bool
    prune_stranded: bool
    progress_h: bool
    smart_h: bool


@dataclasses.dataclass
class ActiveJob:
    job: Job
    process: subprocess.Popen[None]
    log_path: pathlib.Path
    log_handle: Any
    started: float


STATIC_SEEDS: dict[str, list[tuple[str, str]]] = {
    "levels/chase.csv": [
        ("initial", ""),
    ],
    "levels/tinderrectangle.csv": [
        ("initial", ""),
        (
            "p57_near_miss",
            "<<<^<<^>>>>^>>v>v<v>>v^>^^>>v>.vvvv<<>^^^^^<<vvv<<^^^<<<<",
        ),
        (
            "t106_rectsep",
            "<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv"
            "^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv",
        ),
        ("old_lower_rat", "<<<^<^<>^>>>vv^^>>v>v.>>><>v"),
        (
            "p60_latch_open",
            "<^^<v<^^>vvv<<^^^>>>>>>vvv>>^^^>>vvv^^^<<vvv<<^^<^<<v<<<v<<^",
        ),
    ],
    "levels/release.csv": [
        ("early_t2", "v<vv^^>"),
        ("r20_trigger6", "v<vv^^>>vv>^<<v<<<^^"),
        ("p37_return", "v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>>"),
        ("fess_t2_frontier_a", "v<vv^^>>vv>"),
        ("fess_t2_frontier_b", "v<vv^^>>vv<"),
    ],
    "levels/reload_v3.csv": [
        ("bottom_station_short", "vvv<<<<<<vv<<<<"),
        ("t2_macro", ">>>^>>>>.>>.<.<<<<"),
        ("station", ">>>^>>>>vvv.v<^<<<v<<<<<<<^^<^"),
    ],
    "levels/cyborg_rats/ai_takeover.csv": [
        ("early_row5", "v<vv^^^"),
        ("early_row5_alt_t3", "v>vv^^^"),
        ("early_row5_safe_event", "v<vv^^^vvv>"),
        ("p38_clean", "v<vv^^^vvvv<<^^^<<<vv<<^^^^vv^^vv^^v^^"),
        (
            "release_skeleton",
            "vvvv<<^^^<<<vv<<^^^^vv^^vv",
        ),
    ],
    "levels/cooperation/tug_of_war.csv": [
        ("t1", "^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v"),
        ("pre_t3_baffle", "^v vv >< vv vv vv vv <> <> <> <>"),
    ],
    "levels/cooperation/handoff.csv": [
        ("initial", ""),
        ("courier", "v^ >^ >^ >^ >^ >^ ^^ .v .> v> v>"),
        ("pre_t1", "v^ >^ >^ >^ >^ >^"),
        (
            "bounded_solve_p20",
            "v^ >^ >^ v^ >v v> v> v^ ^v >< >< ^^ v^ ^^ ^^ v^ v^ vv <v ^^",
        ),
        (
            "bounded_solve_p35",
            "v^ >^ >^ v^ >^ vv v> v> v^ v^ v^ v^ v< ^^ >^ ^> ^v ^v ^v ^v "
            "^< ^< ^^ >v ^^ v^ ^^ ^^ v^ v^ vv <v ^^ ^^ ^^",
        ),
    ],
    "levels/cooperation/blocked_v2.csv": [
        (
            "b20",
            "^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v ^v v< ^> v> ^< v< ^^",
        ),
        ("pre_t5", "vv v^ vv <^ <v <^ <."),
        ("post_t5_pre_t1", "vv v^ vv <^ <v <^ <. <^ ^^ ^^"),
        (
            "lower_left_gate_p26",
            "v. v. v. <. <. <. <v <^ ^^ ^^ ^^ v^ <^ <^ <^ <^ <^ <^ <^ "
            "^v ^v ^v ^v ^v ^^ ^v",
        ),
    ],
    "levels/old_levels/on_the_clock.csv": [
        ("initial", ""),
        ("p18_pre_t9", ">>>^^>>>vvv><vvvvv"),
        ("p19_live_setup", ">>>^^>>>vvv><vvvvvv"),
        ("p24_sibling", ">>>^^>>>^^^^^vvv><vvvvvv"),
    ],
}

FESS_LEVEL_GUARDS: dict[str, tuple[str, ...]] = {
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
        "3",
        "--min-reachable-rats",
        "2",
        "--max-trapped-rats",
        "1",
    ),
}


def read_source_meta(source: str) -> SourceMeta | None:
    try:
        lines = pathlib.Path(source).read_text(encoding="utf-8", errors="replace").splitlines()
    except (FileNotFoundError, OSError):
        return None
    for line in lines[:8]:
        match = LEVEL_RE.search(line)
        if match is None:
            continue
        level = match.group(0)
        if not line.startswith("$ "):
            return SourceMeta(level=level, mode="", prefix="")
        try:
            tokens = shlex.split(line.removeprefix("$ "))
        except ValueError:
            return SourceMeta(level=level, mode="", prefix="")
        solver_idx = next(
            (
                index
                for index, token in enumerate(tokens)
                if token.endswith("/solver") or token == "target/release/solver"
            ),
            None,
        )
        if solver_idx is None or solver_idx + 1 >= len(tokens):
            return SourceMeta(level=level, mode="", prefix="")
        mode = tokens[solver_idx + 1]
        prefix = ""
        if "--prefix" in tokens:
            prefix_index = tokens.index("--prefix")
            if prefix_index + 1 < len(tokens):
                prefix = tokens[prefix_index + 1]
        return SourceMeta(level=level, mode=mode, prefix=prefix)
    return None


def join_prefix(prefix: str, suffix: str, two_player: bool) -> str:
    if not prefix:
        return suffix
    if not suffix:
        return prefix
    if two_player:
        return f"{prefix} {suffix}"
    return f"{prefix}{suffix}"


def archive_candidates(archive: pathlib.Path) -> list[Candidate]:
    candidates: list[Candidate] = []
    source_meta_cache: dict[str, SourceMeta | None] = {}
    if not archive.exists():
        return candidates

    for line in archive.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            record: dict[str, Any] = json.loads(line)
        except json.JSONDecodeError:
            continue

        full_ascii = record.get("full_ascii")
        prefix = full_ascii or record.get("ascii")
        source = record.get("source")
        if not isinstance(prefix, str) or not prefix.strip() or not isinstance(source, str):
            continue
        if source not in source_meta_cache:
            source_meta_cache[source] = read_source_meta(source)
        meta = source_meta_cache[source]
        if meta is None:
            continue
        full_ascii_is_global = full_ascii is not None and record.get("kind") == "event"
        if not full_ascii_is_global and record.get("kind") != "branch" and meta.prefix:
            prefix = join_prefix(
                meta.prefix,
                prefix.strip(),
                two_player=" " in meta.prefix or " " in prefix,
            )

        features = record.get("features") or {}
        candidates.append(
            Candidate(
                level=meta.level,
                prefix=prefix.strip(),
                source=source,
                rats=int(features.get("rats", 999)),
                reachable_rats=int(record.get("reachable_rats", -1)),
                trapped=int(record.get("trapped", 999)),
                score=int(record.get("score", 0)),
                diag_known=False,
            )
        )
    return candidates


def static_candidates() -> list[Candidate]:
    result = []
    for level, seeds in STATIC_SEEDS.items():
        for name, prefix in seeds:
            result.append(
                Candidate(
                    level=level,
                    prefix=prefix,
                    source=f"static:{name}",
                    rats=999,
                    reachable_rats=-1,
                    trapped=999,
                    score=0,
                    diag_known=False,
                )
            )
    return result


def coerce_score(value: Any, fallback: int = 0) -> int:
    if isinstance(value, int | float):
        return int(value)
    if isinstance(value, list) and all(isinstance(item, int | float) for item in value):
        score = 0
        # Preserve lexicographic ordering for frontier_triage score arrays.
        for item in value:
            score = score * 1_000_000 + int(item) + 500_000
        return score
    return fallback


def seed_file_candidates(seed_file: pathlib.Path) -> list[Candidate]:
    candidates: list[Candidate] = []
    if not seed_file.exists():
        raise SystemExit(f"missing seed file: {seed_file}")

    for line_number, line in enumerate(seed_file.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise SystemExit(f"{seed_file}:{line_number}: invalid json: {error}") from error
        level = record.get("level")
        prefix = record.get("prefix")
        if not isinstance(level, str) or not isinstance(prefix, str):
            raise SystemExit(f"{seed_file}:{line_number}: expected level and prefix strings")
        diag = record.get("diag") if isinstance(record.get("diag"), dict) else {}
        diag_features = diag.get("features") if isinstance(diag.get("features"), dict) else {}
        top_features = record.get("features") if isinstance(record.get("features"), dict) else {}
        features = diag_features or top_features
        structural_known = bool(diag) or (
            bool(features)
            and isinstance(record.get("reachable_rats"), int | float)
            and isinstance(record.get("trapped"), int | float)
        )
        flags = record.get("flags")
        if not isinstance(flags, list):
            flags = []
        rank_score = record.get("rank_score")
        learned_score = record.get("learned_score")
        score = coerce_score(record.get("score"))
        event = record.get("event") if isinstance(record.get("event"), dict) else {}
        event_key = event.get("key") if isinstance(event.get("key"), str) else None
        if isinstance(rank_score, int | float):
            score = int(-1_000_000 * float(rank_score))
        elif isinstance(learned_score, int | float):
            score = int(-1_000_000 * float(learned_score))
        candidates.append(
            Candidate(
                level=level,
                prefix=prefix,
                source=str(record.get("source") or seed_file),
                rats=int(record.get("rats", features.get("rats", diag.get("total_rats", 999)))),
                reachable_rats=int(record.get("reachable_rats", diag.get("reachable_rats", -1))),
                trapped=int(record.get("trapped", diag.get("trapped", 999))),
                score=score,
                event_key=event_key,
                flags=tuple(str(flag) for flag in flags),
                diag_known=structural_known,
            )
        )
    return candidates


def canonical_level(level: str) -> str:
    level = level.replace("\\", "/")
    if level.startswith("levels/"):
        return level.removeprefix("levels/")
    return level


def compact_moves(prefix: str) -> str:
    return "".join(prefix.split())


def load_negative_obligations(paths: Iterable[pathlib.Path]) -> dict[str, list[tuple[str, str]]]:
    obligations: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for path in paths:
        if not path.exists():
            continue
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8", errors="replace").splitlines(),
            start=1,
        ):
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as error:
                raise SystemExit(f"{path}:{line_number}: invalid json: {error}") from error
            label = str(record.get("label", "")).lower()
            if label not in {"negative", "violates", "violated", "dead", "bad"}:
                continue
            level = record.get("level")
            prefix = record.get("prefix")
            if not isinstance(level, str) or not isinstance(prefix, str):
                continue
            compact = compact_moves(prefix)
            if not compact:
                continue
            obligation_id = str(record.get("obligation_id", "negative"))
            obligations[canonical_level(level)].append((compact, obligation_id))
    for level in obligations:
        obligations[level].sort(key=lambda item: len(item[0]), reverse=True)
    return obligations


def negative_obligation_id(
    level: str,
    prefix: str,
    obligations: dict[str, list[tuple[str, str]]],
) -> str | None:
    compact = compact_moves(prefix)
    if not compact:
        return None
    for negative_prefix, obligation_id in obligations.get(canonical_level(level), []):
        if compact.startswith(negative_prefix):
            return obligation_id
    return None


def apply_negative_obligation_policy(
    candidates: Iterable[Candidate],
    obligations: dict[str, list[tuple[str, str]]],
    policy: str,
) -> tuple[list[Candidate], int]:
    if policy == "ignore" or not obligations:
        return list(candidates), 0
    result: list[Candidate] = []
    matched = 0
    for candidate in candidates:
        obligation_id = negative_obligation_id(candidate.level, candidate.prefix, obligations)
        if obligation_id is None:
            result.append(candidate)
            continue
        matched += 1
        if policy == "drop":
            continue
        if policy != "score":
            raise ValueError(f"unknown obligation policy: {policy}")
        flag = f"negative-obligation:{obligation_id}"
        if flag in candidate.flags:
            result.append(candidate)
        else:
            result.append(dataclasses.replace(candidate, flags=(*candidate.flags, flag)))
    return result, matched


def load_solved_levels(solutions_file: pathlib.Path) -> set[str]:
    try:
        data = json.loads(solutions_file.read_text(encoding="utf-8"))
    except (FileNotFoundError, OSError, json.JSONDecodeError):
        return set()
    if not isinstance(data, dict):
        return set()
    return {canonical_level(level) for level in data if isinstance(level, str)}


def unique_best(
    candidates: Iterable[Candidate],
    per_level: int,
    rank_key: str,
    dedupe_event_key: bool,
    max_flag_penalty: int | None,
) -> list[Candidate]:
    by_level: dict[str, list[Candidate]] = defaultdict(list)
    for candidate in candidates:
        by_level[candidate.level].append(candidate)

    def flag_penalty(candidate: Candidate) -> int:
        """Score oracle warning flags before sparse reward metrics.

        Lower remaining-rat counts are only useful when the state is still
        structurally live. Triage already labels common dead basins, so use
        those labels while selecting candidates instead of rediscovering the
        same failures with full solver jobs.
        """
        penalty = 0
        for flag in candidate.flags:
            if flag.startswith("state:"):
                penalty += 100
            elif flag in {"no-reachable-rats", "no-remaining-mechanism"}:
                penalty += 80
            elif flag.startswith("negative-obligation:"):
                penalty += 70
            elif flag == "tinder-dropped-rat":
                penalty += 60
            elif flag.endswith("-sealed"):
                penalty += 30
            elif flag.endswith("-unreachable") or flag.endswith("-unreachable-rat"):
                penalty += 20
            elif flag.startswith("trapped:"):
                try:
                    penalty += 10 * int(flag.removeprefix("trapped:"))
                except ValueError:
                    penalty += 10
            else:
                penalty += 5
        return penalty

    def viability_bucket(candidate: Candidate) -> int:
        """Prefer frontiers where remaining rats are still actionable.

        Archive records with reachability diagnostics are more informative than
        static seeds. A low rat count is tempting, but a frontier with zero
        reachable rats or trapped rats usually sends bounded probes into an
        already-dead basin. Unknown/static seeds are kept ahead of known-dead
        states so they can still seed exploration.
        """
        known_reachable = candidate.reachable_rats >= 0
        known_trapped = candidate.trapped < 999
        has_rats = candidate.rats > 0 and candidate.rats < 999
        if flag_penalty(candidate) >= 60:
            return 5
        if known_reachable and candidate.reachable_rats > 0:
            return 0 if not (known_trapped and candidate.trapped > 0) else 1
        if not known_reachable:
            return 2
        if has_rats and candidate.reachable_rats == 0:
            return 4 if known_trapped and candidate.trapped > 0 else 3
        return 2

    selected = []
    for level, level_candidates in sorted(by_level.items()):
        if rank_key == "score":
            ranked = sorted(
                level_candidates,
                key=lambda c: (
                    viability_bucket(c),
                    flag_penalty(c),
                    c.score,
                    c.rats,
                    -c.reachable_rats,
                    c.trapped,
                    len(c.prefix.replace(" ", "")),
                ),
            )
        else:
            ranked = sorted(
                level_candidates,
                key=lambda c: (
                    viability_bucket(c),
                    flag_penalty(c),
                    c.rats,
                    -c.reachable_rats,
                    c.trapped,
                    len(c.prefix.replace(" ", "")),
                    c.score,
                ),
            )
        seen: set[str] = set()
        deduped = []
        for candidate in ranked:
            if (
                max_flag_penalty is not None
                and flag_penalty(candidate) > max_flag_penalty
            ):
                continue
            dedupe_key = candidate.prefix
            if dedupe_event_key and candidate.event_key:
                dedupe_key = f"event:{candidate.event_key}"
            if dedupe_key in seen:
                continue
            seen.add(dedupe_key)
            deduped.append(candidate)
        if not deduped and max_flag_penalty is not None:
            # Do not silently starve a level when all known frontiers are bad;
            # keep the best flagged candidate so the dry-run accounting is
            # explicit and targeted exception runs remain possible.
            for candidate in ranked:
                dedupe_key = candidate.prefix
                if dedupe_event_key and candidate.event_key:
                    dedupe_key = f"event:{candidate.event_key}"
                if dedupe_key in seen:
                    continue
                deduped.append(candidate)
                break
        selected.extend(deduped[:per_level])
    return selected


def slug(text: str) -> str:
    digest = hashlib.sha1(text.encode("utf-8")).hexdigest()[:8]
    clean = re.sub(r"[^a-zA-Z0-9]+", "_", text).strip("_").lower()
    return f"{clean[:64]}_{digest}"


def jobs_for_candidate(
    candidate: Candidate,
    timeout_sec: int,
    mem_mb: int,
    lookup_mem_mb: int | None,
    prune_dead: bool,
    prune_stranded: bool,
    progress_h: bool,
    smart_h: bool,
    lookup_depth: int,
    lookup_maxnodes: int,
    lookup_weight: int,
    lookup_stagnation_secs: float,
    fess_jobs: int,
    fess_level_guards_enabled: bool,
) -> list[Job]:
    level = candidate.level
    prefix = candidate.prefix
    base = slug(f"{pathlib.Path(level).stem}_{candidate.source}_{prefix}")
    lookup_effective_mem_mb = lookup_mem_mb or mem_mb
    fess_level_guards = FESS_LEVEL_GUARDS.get(level, ()) if fess_level_guards_enabled else ()
    fess_args = (
        "fess",
        level,
        "--prefix",
        prefix,
        "--steps",
        "7",
        "--width",
        "96",
        "--per-bucket",
        "2",
        "--events",
        "18",
        "--segdepth",
        "110",
        "--segsecs",
        "2.5",
        "--secs",
        str(timeout_sec - 10),
        "--mopdepth",
        "420",
        "--mopsecs",
        "2.0",
        *fess_level_guards,
    )
    if fess_jobs > 1:
        fess_args = (*fess_args, "--jobs", str(fess_jobs))
    jobs = [
        Job(
            f"{base}_novelty",
            (
                "novelty",
                level,
                "--prefix",
                prefix,
                "--k",
                "2",
                "--depth",
                "220",
                "--secs",
                str(timeout_sec - 10),
            ),
            timeout_sec,
            mem_mb,
            1,
            prune_dead,
            prune_stranded,
            progress_h,
            smart_h,
        ),
        Job(
            f"{base}_lookup_win",
            (
                "lookup",
                level,
                "--prefix",
                prefix,
                "--goal",
                "win",
                "--order",
                "astar",
                "--depth",
                str(lookup_depth),
                "--secs",
                str(timeout_sec - 10),
                "--maxnodes",
                str(lookup_maxnodes),
                "--weight",
                str(lookup_weight),
            ),
            timeout_sec,
            lookup_effective_mem_mb,
            1,
            prune_dead,
            prune_stranded,
            progress_h,
            smart_h,
        ),
        Job(
            f"{base}_fess",
            fess_args,
            timeout_sec,
            mem_mb,
            max(1, fess_jobs),
            prune_dead,
            prune_stranded,
            progress_h,
            smart_h,
        ),
        Job(
            f"{base}_dropchain",
            (
                "dropchain",
                level,
                "--prefix",
                prefix,
                "--steps",
                "8",
                "--segdepth",
                "120",
                "--segsecs",
                "12",
                "--segnodes",
                "700000",
                "--results",
                "12",
                "--beam",
                "12",
                "--mopdepth",
                "420",
                "--mopsecs",
                "8",
            ),
            timeout_sec,
            mem_mb,
            1,
            prune_dead,
            prune_stranded,
            progress_h,
            smart_h,
        ),
    ]
    if lookup_stagnation_secs > 0.0:
        lookup_job = jobs[1]
        jobs[1] = dataclasses.replace(
            lookup_job,
            args=(
                *lookup_job.args,
                "--stagnation-secs",
                str(lookup_stagnation_secs),
            ),
        )
    return jobs


def expensive_filter_reason(candidate: Candidate, policy: str) -> str | None:
    """Return why expensive speculative modes should be skipped for a candidate.

    The cheap modes are useful smoke tests for almost any prefix. FESS and
    dropchain are much more costly and repeatedly timed out from states whose
    oracle diagnostics already showed sealed, partial, or otherwise failed
    basins. Keep the old exhaustive behavior behind --expensive-filter all.
    """
    if policy == "all":
        return None
    if not candidate.diag_known:
        return "unknown_diag"

    hard_flags = {
        flag
        for flag in candidate.flags
        if flag.startswith("state:")
        or flag in {
            "no-reachable-rats",
            "no-remaining-mechanism",
            "tinder-dropped-rat",
        }
    }
    if hard_flags:
        return "hard_flags:" + ",".join(sorted(hard_flags))

    if policy == "no-flags":
        if candidate.flags:
            return "flags:" + ",".join(sorted(candidate.flags))
        if candidate.rats <= 0:
            return "no_rats"
        if candidate.reachable_rats <= 0:
            return "no_reachable_rats"
        if candidate.trapped > 0:
            return "trapped_rats"
        return None

    if policy == "known-clean":
        if candidate.rats <= 0:
            return "no_rats"
        if candidate.reachable_rats < 0:
            return "unknown_reachability"
        if candidate.reachable_rats != candidate.rats:
            return "not_all_rats_reachable"
        if candidate.trapped > 0:
            return "trapped_rats"
        return None

    raise ValueError(f"unknown expensive filter policy: {policy}")


def filter_expensive_jobs(
    jobs: Iterable[Job],
    candidate: Candidate,
    expensive_filter: str,
) -> tuple[list[Job], Counter[str]]:
    reason = expensive_filter_reason(candidate, expensive_filter)
    kept = []
    skipped: Counter[str] = Counter()
    for job in jobs:
        if reason is not None and job.args[0] in {"fess", "dropchain"}:
            skipped[reason] += 1
            continue
        kept.append(job)
    return kept, skipped


def _limit_child_memory(mem_mb: int) -> None:
    limit = mem_mb * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (limit, limit))


def available_memory_mb() -> int | None:
    try:
        for line in pathlib.Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemAvailable:"):
                return int(line.split()[1]) // 1024
    except OSError:
        return None
    return None


def auto_jobs(requested: int, job_count: int) -> int:
    if job_count <= 0:
        return 0
    cpu_count = os.cpu_count() or 1
    requested_or_cpu = requested if requested > 0 else cpu_count
    return max(1, min(requested_or_cpu, job_count))


def auto_jobs_for_queue(requested: int, jobs: list[Job]) -> int:
    return auto_jobs(requested, len(jobs))


def memory_budget_mb(reserve_mb: int) -> int | None:
    mem_available = available_memory_mb()
    if mem_available is None:
        return None
    return max(1, mem_available - reserve_mb)


def start_job(job: Job, out_dir: pathlib.Path) -> ActiveJob:
    log_path = out_dir / f"{job.name}.log"
    command = [str(SOLVER), *job.args]
    env = None
    if job.prune_dead or job.prune_stranded or job.progress_h or job.smart_h:
        env = dict(os.environ)
    if job.prune_dead:
        env["PRUNE_DEAD"] = "1"
    if job.prune_stranded:
        env["PRUNE_STRANDED"] = "1"
    if job.progress_h:
        env["PROGRESS_H"] = "1"
    if job.smart_h:
        env["SMART_H"] = "1"

    log = log_path.open("w", encoding="utf-8")
    log.write("$ " + shlex.join(command) + "\n")
    if job.prune_dead:
        log.write("# env PRUNE_DEAD=1\n")
    if job.prune_stranded:
        log.write("# env PRUNE_STRANDED=1\n")
    if job.progress_h:
        log.write("# env PROGRESS_H=1\n")
    if job.smart_h:
        log.write("# env SMART_H=1\n")
    log.write(
        f"# mem_mb={job.mem_mb} cpu_slots={job.cpu_slots} "
        f"timeout_sec={job.timeout_sec}\n"
    )
    log.flush()
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
        preexec_fn=lambda: _limit_child_memory(job.mem_mb),
    )
    return ActiveJob(
        job=job,
        process=process,
        log_path=log_path,
        log_handle=log,
        started=time.monotonic(),
    )


def finish_job(active: ActiveJob, code: int) -> tuple[str, int, float, pathlib.Path, bool]:
    elapsed = time.monotonic() - active.started
    active.log_handle.write(f"\nEXIT {code}\n")
    active.log_handle.close()
    text = active.log_path.read_text(encoding="utf-8", errors="replace")
    solved = "SOLVED " in text or "result=Won" in text
    return active.job.name, code, elapsed, active.log_path, solved


def stop_timed_out_job(active: ActiveJob) -> int:
    active.log_handle.write(f"\nTIMEOUT after {active.job.timeout_sec}s\n")
    active.log_handle.flush()
    active.process.send_signal(signal.SIGTERM)
    try:
        active.process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        active.process.kill()
        active.process.wait()
    return 124


def stop_job(active: ActiveJob, reason: str) -> int:
    active.log_handle.write(f"\nSTOPPED {reason}\n")
    active.log_handle.flush()
    active.process.send_signal(signal.SIGTERM)
    try:
        active.process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        active.process.kill()
        active.process.wait()
    return 130


def run_jobs(
    jobs: list[Job],
    out_dir: pathlib.Path,
    max_processes: int,
    cpu_slots: int,
    mem_budget_mb: int | None,
    stop_on_solved: bool,
) -> bool:
    pending = deque(jobs)
    active: list[ActiveJob] = []
    found_solution = False

    def used_cpu_slots() -> int:
        return sum(active_job.job.cpu_slots for active_job in active)

    def used_mem_mb() -> int:
        return sum(active_job.job.mem_mb for active_job in active)

    def launchable_index() -> int | None:
        if len(active) >= max_processes or used_cpu_slots() >= cpu_slots:
            return None
        active_cpu = used_cpu_slots()
        active_mem = used_mem_mb()
        for index, job in enumerate(pending):
            if active_cpu + job.cpu_slots > cpu_slots:
                continue
            if mem_budget_mb is not None and active_mem + job.mem_mb > mem_budget_mb:
                continue
            return index
        # Preserve the old "always make forward progress" behavior when the
        # host budget snapshot is lower than a single job's cap.
        if not active and pending:
            return 0
        return None

    def pop_launchable(index: int) -> Job:
        pending.rotate(-index)
        job = pending.popleft()
        pending.rotate(index)
        return job

    def launch_ready() -> None:
        while pending and not (stop_on_solved and found_solution):
            index = launchable_index()
            if index is None:
                break
            job = pop_launchable(index)
            active.append(start_job(job, out_dir))

    launch_ready()
    while active:
        now = time.monotonic()
        for running in list(active):
            code = running.process.poll()
            if code is None and now - running.started >= running.job.timeout_sec:
                code = stop_timed_out_job(running)
            if code is None:
                continue
            active.remove(running)
            name, code, elapsed, log_path, solved = finish_job(running, code)
            found_solution = found_solution or solved
            marker = "SOLVED" if solved else "done"
            print(f"{marker} {name} code={code} elapsed={elapsed:.1f}s log={log_path}")
            sys.stdout.flush()
            if solved and stop_on_solved:
                pending.clear()
                for other in list(active):
                    active.remove(other)
                    stopped_code = stop_job(other, "after_solution")
                    stopped_name, _, stopped_elapsed, stopped_log_path, stopped_solved = finish_job(
                        other, stopped_code
                    )
                    found_solution = found_solution or stopped_solved
                    stopped_marker = "SOLVED" if stopped_solved else "stopped"
                    print(
                        f"{stopped_marker} {stopped_name} code={stopped_code} "
                        f"elapsed={stopped_elapsed:.1f}s log={stopped_log_path}"
                    )
                    sys.stdout.flush()
                break
            launch_ready()
        if active:
            time.sleep(0.25)
    return found_solution


def interleave_by_level(jobs: Iterable[Job]) -> list[Job]:
    """Round-robin jobs across levels so one hard level cannot occupy every worker."""
    by_level: dict[str, deque[Job]] = defaultdict(deque)
    for job in jobs:
        level = job.args[1] if len(job.args) > 1 else ""
        by_level[level].append(job)

    ordered = []
    levels = deque(sorted(by_level))
    while levels:
        level = levels.popleft()
        queue = by_level[level]
        ordered.append(queue.popleft())
        if queue:
            levels.append(level)
    return ordered


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=pathlib.Path, default=DEFAULT_ARCHIVE)
    parser.add_argument(
        "--seed-file",
        action="append",
        type=pathlib.Path,
        default=[],
        help="JSONL records with level/prefix fields, such as frontier_triage --seeds-out",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=0,
        help="maximum concurrent solver processes; 0 uses CPU count",
    )
    parser.add_argument(
        "--cpu-slots",
        type=int,
        default=0,
        help=(
            "maximum active CPU slots; 0 uses CPU count. FESS consumes "
            "--fess-jobs slots; other modes consume one"
        ),
    )
    parser.add_argument(
        "--mem-reserve-mb",
        type=int,
        default=4096,
        help=(
            "leave this much MemAvailable unused when scheduling mixed "
            "memory-cap jobs"
        ),
    )
    parser.add_argument(
        "--static",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="include built-in static seeds",
    )
    parser.add_argument("--per-level", type=int, default=3)
    parser.add_argument(
        "--rank-key",
        choices=["default", "score"],
        default="default",
        help="candidate ranking before job creation; use score for learned_ranker JSONL",
    )
    parser.add_argument("--timeout-sec", type=int, default=180)
    parser.add_argument("--mem-mb", type=int, default=1600)
    parser.add_argument(
        "--lookup-mem-mb",
        type=int,
        help="memory cap for lookup_win jobs; defaults to --mem-mb",
    )
    parser.add_argument(
        "--lookup-depth",
        type=int,
        default=220,
        help="depth for lookup_win jobs",
    )
    parser.add_argument(
        "--lookup-maxnodes",
        type=int,
        default=1_000_000,
        help="max node budget for lookup_win jobs",
    )
    parser.add_argument(
        "--lookup-weight",
        type=int,
        default=2,
        help="A* heuristic weight for lookup_win jobs",
    )
    parser.add_argument(
        "--lookup-stagnation-secs",
        type=float,
        default=18.0,
        help="stop speculative lookup_win jobs after this many seconds without heuristic improvement; 0 disables",
    )
    parser.add_argument(
        "--fess-jobs",
        type=int,
        default=1,
        help="threads per fess solver process; keep at 1 when running many fess processes concurrently",
    )
    parser.add_argument(
        "--fess-level-guards",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="apply level-specific FESS guard predicates before frontier expansion",
    )
    parser.add_argument(
        "--expensive-filter",
        choices=["known-clean", "no-flags", "all"],
        default="known-clean",
        help=(
            "gate expensive fess/dropchain probes; no-flags allows structurally "
            "clean frontier states, known-clean additionally requires all rats "
            "reachable and none trapped"
        ),
    )
    parser.add_argument(
        "--dedupe-event-key",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="collapse seed-file records with the same structural event key before per-level selection",
    )
    parser.add_argument(
        "--max-candidate-flag-penalty",
        type=int,
        help=(
            "drop candidates above this triage flag penalty before job creation; "
            "if all candidates for a level are dropped, keep the single best "
            "flagged candidate for visibility"
        ),
    )
    parser.add_argument(
        "--obligation-labels",
        action="append",
        type=pathlib.Path,
        default=[DEFAULT_OBLIGATION_LABELS],
        help="JSONL labels used to demote/drop already-closed negative prefix families",
    )
    parser.add_argument(
        "--obligation-policy",
        choices=["score", "drop", "ignore"],
        default="score",
        help="how to handle prefixes that extend negative obligation labels",
    )
    parser.add_argument(
        "--max-jobs",
        type=int,
        help="after interleaving, queue at most this many jobs; use --jobs for concurrency",
    )
    parser.add_argument(
        "--prune-dead",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="set PRUNE_DEAD=1 for solver children",
    )
    parser.add_argument(
        "--prune-stranded",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="set PRUNE_STRANDED=1 for solver children to prune mechanism-free trapped-rat basins",
    )
    parser.add_argument(
        "--progress-h",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="set PROGRESS_H=1 for solver children so setup work has a heuristic gradient",
    )
    parser.add_argument(
        "--smart-h",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="set SMART_H=1 for solver children to penalize unreachable/trapped rats",
    )
    parser.add_argument("--out-dir", type=pathlib.Path)
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
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument(
        "--skip-level",
        action="append",
        default=[],
        help="skip candidates whose level contains this token; repeatable",
    )
    parser.add_argument("--strategy", action="append", default=[])
    parser.add_argument(
        "--stop-on-solved",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="terminate remaining queued/running jobs after a solution marker appears",
    )
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not SOLVER.exists():
        raise SystemExit(f"missing solver binary: {SOLVER}")

    candidates = archive_candidates(args.archive)
    if args.static:
        candidates.extend(static_candidates())
    for seed_file in args.seed_file:
        candidates.extend(seed_file_candidates(seed_file))
    raw_candidate_count = len(candidates)
    skipped_solved = 0
    if args.skip_solved:
        solved_levels = load_solved_levels(args.solutions_file)
        before = len(candidates)
        candidates = [
            candidate
            for candidate in candidates
            if canonical_level(candidate.level) not in solved_levels
        ]
        skipped_solved = before - len(candidates)
    if args.only:
        candidates = [
            candidate
            for candidate in candidates
            if any(token in candidate.level or token in candidate.source for token in args.only)
        ]
    skipped_level = 0
    if args.skip_level:
        before = len(candidates)
        candidates = [
            candidate
            for candidate in candidates
            if not any(token in candidate.level for token in args.skip_level)
        ]
        skipped_level = before - len(candidates)
    negative_obligations = load_negative_obligations(args.obligation_labels)
    candidates, negative_obligation_matches = apply_negative_obligation_policy(
        candidates,
        negative_obligations,
        args.obligation_policy,
    )
    candidates = unique_best(
        candidates,
        args.per_level,
        args.rank_key,
        args.dedupe_event_key,
        args.max_candidate_flag_penalty,
    )
    jobs = []
    skipped_expensive: Counter[str] = Counter()
    for candidate in candidates:
        candidate_jobs, skipped = filter_expensive_jobs(
            jobs_for_candidate(
                candidate,
                args.timeout_sec,
                args.mem_mb,
                args.lookup_mem_mb,
                args.prune_dead,
                args.prune_stranded,
                args.progress_h,
                args.smart_h,
                args.lookup_depth,
                args.lookup_maxnodes,
                args.lookup_weight,
                args.lookup_stagnation_secs,
                args.fess_jobs,
                args.fess_level_guards,
            ),
            candidate,
            args.expensive_filter,
        )
        jobs.extend(candidate_jobs)
        skipped_expensive.update(skipped)
    if args.strategy:
        wanted = set(args.strategy)
        jobs = [
            job
            for job in jobs
            if job.args[0] in wanted
            or ("lookup_win" in wanted and job.args[0] == "lookup")
        ]
    if not jobs:
        raise SystemExit("no jobs selected")
    jobs = interleave_by_level(jobs)
    if args.max_jobs is not None:
        jobs = jobs[: args.max_jobs]

    timestamp = dt.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    out_dir = args.out_dir or RUN_ROOT / f"{timestamp}_go_explore"
    out_dir.mkdir(parents=True, exist_ok=True)

    cpu_count = os.cpu_count() or 1
    workers = auto_jobs_for_queue(args.jobs, jobs)
    cpu_slot_budget = args.cpu_slots if args.cpu_slots > 0 else cpu_count
    mem_budget = memory_budget_mb(args.mem_reserve_mb)
    job_mem_values = sorted({job.mem_mb for job in jobs})
    job_cpu_values = sorted({job.cpu_slots for job in jobs})
    print(
        f"raw_candidates={raw_candidate_count} selected_candidates={len(candidates)} "
        f"skipped_solved={skipped_solved} skipped_level={skipped_level} "
        f"negative_obligation_matches={negative_obligation_matches} "
        f"obligation_policy={args.obligation_policy} "
        f"skipped_expensive_jobs={sum(skipped_expensive.values())} "
        f"queued_jobs={len(jobs)} max_processes={workers} requested_processes={args.jobs} "
        f"cpu_count={cpu_count} cpu_slots={cpu_slot_budget} requested_cpu_slots={args.cpu_slots} "
        f"max_queued_jobs={args.max_jobs} "
        f"dedupe_event_key={args.dedupe_event_key} "
        f"max_candidate_flag_penalty={args.max_candidate_flag_penalty} "
        f"expensive_filter={args.expensive_filter} "
        f"fess_level_guards={args.fess_level_guards} "
        f"lookup_stagnation_secs={args.lookup_stagnation_secs} "
        f"job_mem_mb={job_mem_values} job_cpu_slots={job_cpu_values} "
        f"mem_available_mb={available_memory_mb()} mem_budget_mb={mem_budget} "
        f"mem_reserve_mb={args.mem_reserve_mb} logs={out_dir}",
        flush=True,
    )
    if skipped_expensive:
        reason_text = ", ".join(
            f"{reason}={count}" for reason, count in sorted(skipped_expensive.items())
        )
        print(f"skipped_expensive_reasons {reason_text}", flush=True)
    for candidate in candidates:
        flag_text = ",".join(candidate.flags) if candidate.flags else "-"
        diag_text = "known" if candidate.diag_known else "unknown"
        print(
            "candidate",
            candidate.level,
            f"rats={candidate.rats}",
            f"rr={candidate.reachable_rats}",
            f"trapped={candidate.trapped}",
            f"diag={diag_text}",
            f"flags={flag_text}",
            f"source={candidate.source}",
            f"prefix={shlex.quote(candidate.prefix)}",
            flush=True,
        )
    if args.dry_run:
        for job in jobs:
            print("$", shlex.join([str(SOLVER), *job.args]))
        return 0

    found_solution = run_jobs(
        jobs,
        out_dir,
        workers,
        cpu_slot_budget,
        mem_budget,
        args.stop_on_solved,
    )
    if not found_solution:
        print("no solved job in this go-explore portfolio")
    return 0


if __name__ == "__main__":
    sys.exit(main())
