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
from collections import defaultdict, deque
from collections.abc import Iterable
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLVER = ROOT / "target" / "release" / "solver"
RUN_ROOT = pathlib.Path("/tmp/infestation-runs")
DEFAULT_ARCHIVE = RUN_ROOT / "archive_20260614_full.jsonl"

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


@dataclasses.dataclass(frozen=True)
class Job:
    name: str
    args: tuple[str, ...]
    timeout_sec: int
    mem_mb: int
    prune_dead: bool


@dataclasses.dataclass
class ActiveJob:
    job: Job
    process: subprocess.Popen[None]
    log_path: pathlib.Path
    log_handle: Any
    started: float


STATIC_SEEDS: dict[str, list[tuple[str, str]]] = {
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

        prefix = record.get("full_ascii") or record.get("ascii")
        source = record.get("source")
        if not isinstance(prefix, str) or not prefix.strip() or not isinstance(source, str):
            continue
        if source not in source_meta_cache:
            source_meta_cache[source] = read_source_meta(source)
        meta = source_meta_cache[source]
        if meta is None:
            continue
        if record.get("kind") != "branch" and meta.prefix:
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
                )
            )
    return result


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
        features = diag.get("features") if isinstance(diag.get("features"), dict) else {}
        rank_score = record.get("rank_score")
        learned_score = record.get("learned_score")
        score = int(record.get("score", 0))
        if isinstance(rank_score, int | float):
            score = int(-1_000_000 * float(rank_score))
        elif isinstance(learned_score, int | float):
            score = int(-1_000_000 * float(learned_score))
        candidates.append(
            Candidate(
                level=level,
                prefix=prefix,
                source=str(record.get("source") or seed_file),
                rats=int(record.get("rats", features.get("rats", 999))),
                reachable_rats=int(record.get("reachable_rats", diag.get("reachable_rats", -1))),
                trapped=int(record.get("trapped", diag.get("trapped", 999))),
                score=score,
            )
        )
    return candidates


def unique_best(candidates: Iterable[Candidate], per_level: int, rank_key: str) -> list[Candidate]:
    by_level: dict[str, dict[str, Candidate]] = defaultdict(dict)
    for candidate in candidates:
        by_level[candidate.level].setdefault(candidate.prefix, candidate)

    selected = []
    for level, by_prefix in sorted(by_level.items()):
        if rank_key == "score":
            ranked = sorted(
                by_prefix.values(),
                key=lambda c: (
                    c.score,
                    c.rats,
                    -c.reachable_rats,
                    c.trapped,
                    len(c.prefix.replace(" ", "")),
                ),
            )
        else:
            ranked = sorted(
                by_prefix.values(),
                key=lambda c: (
                    c.rats,
                    -c.reachable_rats,
                    c.trapped,
                    len(c.prefix.replace(" ", "")),
                    c.score,
                ),
            )
        selected.extend(ranked[:per_level])
    return selected


def slug(text: str) -> str:
    digest = hashlib.sha1(text.encode("utf-8")).hexdigest()[:8]
    clean = re.sub(r"[^a-zA-Z0-9]+", "_", text).strip("_").lower()
    return f"{clean[:64]}_{digest}"


def jobs_for_candidate(
    candidate: Candidate,
    timeout_sec: int,
    mem_mb: int,
    prune_dead: bool,
) -> list[Job]:
    level = candidate.level
    prefix = candidate.prefix
    base = slug(f"{pathlib.Path(level).stem}_{candidate.source}_{prefix}")
    return [
        Job(
            f"{base}_fess",
            (
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
            ),
            timeout_sec,
            mem_mb,
            prune_dead,
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
            prune_dead,
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
                "220",
                "--secs",
                str(timeout_sec - 10),
                "--maxnodes",
                "1000000",
                "--weight",
                "2",
            ),
            timeout_sec,
            mem_mb,
            prune_dead,
        ),
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
            prune_dead,
        ),
    ]


def _limit_child_memory(mem_mb: int) -> None:
    limit = mem_mb * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (limit, limit))


def start_job(job: Job, out_dir: pathlib.Path) -> ActiveJob:
    log_path = out_dir / f"{job.name}.log"
    command = [str(SOLVER), *job.args]
    env = None
    if job.prune_dead:
        env = dict(os.environ)
        env["PRUNE_DEAD"] = "1"

    log = log_path.open("w", encoding="utf-8")
    log.write("$ " + shlex.join(command) + "\n")
    if job.prune_dead:
        log.write("# env PRUNE_DEAD=1\n")
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


def run_jobs(jobs: list[Job], out_dir: pathlib.Path, concurrency: int) -> bool:
    pending = deque(jobs)
    active: list[ActiveJob] = []
    found_solution = False

    def launch_ready() -> None:
        while pending and len(active) < concurrency:
            job = pending.popleft()
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
    parser.add_argument("--jobs", type=int, default=6)
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
        "--max-jobs",
        type=int,
        help="after interleaving, run at most this many jobs",
    )
    parser.add_argument(
        "--prune-dead",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="set PRUNE_DEAD=1 for solver children",
    )
    parser.add_argument("--out-dir", type=pathlib.Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--strategy", action="append", default=[])
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
    if args.only:
        candidates = [
            candidate
            for candidate in candidates
            if any(token in candidate.level or token in candidate.source for token in args.only)
        ]
    candidates = unique_best(candidates, args.per_level, args.rank_key)
    jobs = [
        job
        for candidate in candidates
        for job in jobs_for_candidate(
            candidate,
            args.timeout_sec,
            args.mem_mb,
            args.prune_dead,
        )
    ]
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

    print(
        f"candidates={len(candidates)} jobs={len(jobs)} concurrency={args.jobs} logs={out_dir}",
        flush=True,
    )
    for candidate in candidates:
        print(
            "candidate",
            candidate.level,
            f"rats={candidate.rats}",
            f"rr={candidate.reachable_rats}",
            f"trapped={candidate.trapped}",
            f"source={candidate.source}",
            f"prefix={shlex.quote(candidate.prefix)}",
            flush=True,
        )
    if args.dry_run:
        for job in jobs:
            print("$", shlex.join([str(SOLVER), *job.args]))
        return 0

    found_solution = run_jobs(jobs, out_dir, args.jobs)
    if not found_solution:
        print("no solved job in this go-explore portfolio")
    return 0


if __name__ == "__main__":
    sys.exit(main())
