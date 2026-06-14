#!/usr/bin/env python3
"""Run bounded solver portfolios with captured logs.

The hard levels need parallel probes, but unbounded fan-out can OOM the
workstation. This runner keeps each solver process under a virtual-memory cap,
limits wall time, and records stdout/stderr for every job.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import datetime as dt
import os
import pathlib
import resource
import shlex
import subprocess
import sys
import time
from collections import defaultdict, deque
from collections.abc import Iterable


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLVER = ROOT / "target" / "release" / "solver"
RUN_ROOT = pathlib.Path("/tmp/infestation-runs")


@dataclasses.dataclass(frozen=True)
class Job:
    name: str
    args: tuple[str, ...]
    timeout_sec: int = 90
    mem_mb: int = 1_200


TINDER_T106 = (
    "<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv"
    "^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv"
)

RELEASE_R20 = "v<vv^^>>vv>^<<v<<<^^"
RELEASE_P37 = "v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>>"
RELEASE_EARLY_T2 = "v<vv^^>"
RELOAD_T2 = ">>>^>>>>.>>.<.<<<<"
RELOAD_STATION = ">>>^>>>>vvv.v<^<<<v<<<<<<<^^<^"
AI_B75 = "^^^^^^v^vvvvvvv>>>vv^^^vv<<<v>>>>>^^^^v>>>v>vv>>vvvvv<<<<<<<<<<<<<<<<^^^<<<"
AI_EARLY_ROW5 = "v<vv^^^"
BLOCKED_B20 = "^< ^^ v^ ^^ v> v> v> vv vv ^v ^v ^< ^v ^v v< ^> v> ^< v< ^^"
TUG_T1 = "^< ^^ ^^ ^^ ^^ ^^ ^^ ^^ <^ ^v >v"
HANDOFF_COURIER = "v^ >^ >^ >^ >^ >^ ^^ .v .> v> v>"
HANDOFF_PRE_T1 = "v^ >^ >^ >^ >^ >^"
TUG_PRE_T3_BAFFLE = "^v vv >< vv vv vv vv <> <> <> <>"
BLOCKED_PRE_T5 = "vv v^ vv <^ <v <^ <."
BLOCKED_POST_T5_PRE_T1 = "vv v^ vv <^ <v <^ <. <^ ^^ ^^"


def current_jobs() -> list[Job]:
    """Small, mechanism-focused portfolio for the current hard set."""
    return [
        Job(
            "tinder_t106_rectsep",
            (
                "branchdump",
                "levels/tinderrectangle.csv",
                "--prefix",
                TINDER_T106,
                "--goal",
                "rectsep",
                "--depth",
                "120",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "16",
                "--no-canonical",
            ),
        ),
        Job(
            "release_r20_trigger6_open_right",
            (
                "branchdump",
                "levels/release.csv",
                "--prefix",
                RELEASE_R20,
                "--goal",
                "triggeronlycellnot:6,18,5,web",
                "--depth",
                "120",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "20",
                "--min-reachable-rats",
                "18",
                "--no-canonical",
            ),
        ),
        Job(
            "release_p37_break_return_plank",
            (
                "branchdump",
                "levels/release.csv",
                "--prefix",
                RELEASE_P37,
                "--goal",
                "cellnot:16,17,plank",
                "--depth",
                "120",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "20",
                "--min-reachable-rats",
                "18",
                "--no-canonical",
            ),
        ),
        Job(
            "release_early_t2_open_right_column",
            (
                "branchdump",
                "levels/release.csv",
                "--prefix",
                RELEASE_EARLY_T2,
                "--goal",
                "triggeronlycellnot:2,18,5,web",
                "--depth",
                "50",
                "--secs",
                "75",
                "--maxnodes",
                "700000",
                "--results",
                "6",
                "--min-rats",
                "23",
                "--min-reachable-rats",
                "20",
                "--max-trapped-rats",
                "1",
            ),
        ),
        Job(
            "release_early_t2_open_left_mouth",
            (
                "branchdump",
                "levels/release.csv",
                "--prefix",
                RELEASE_EARLY_T2,
                "--goal",
                "triggeronlycellnot:2,1,16,explosive",
                "--depth",
                "50",
                "--secs",
                "75",
                "--maxnodes",
                "700000",
                "--results",
                "6",
                "--min-rats",
                "23",
                "--min-reachable-rats",
                "20",
                "--max-trapped-rats",
                "1",
            ),
        ),
        Job(
            "reload_t2_preserve_trigger1",
            (
                "branchdump",
                "levels/reload_v3.csv",
                "--prefix",
                RELOAD_T2,
                "--goal",
                "triggeronly:1",
                "--depth",
                "120",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "3",
                "--min-reachable-rats",
                "1",
                "--min-reachable-triggers",
                "1",
                "--no-canonical",
            ),
        ),
        Job(
            "reload_station_open_bottom_fuse",
            (
                "branchdump",
                "levels/reload_v3.csv",
                "--prefix",
                RELOAD_STATION,
                "--goal",
                "cellnotratrectplayerrect:2,21,explosive,0,20,3,21,2,17,8,21",
                "--depth",
                "50",
                "--secs",
                "75",
                "--maxnodes",
                "700000",
                "--results",
                "6",
                "--min-rats",
                "3",
                "--min-explosives",
                "5",
                "--min-triggers",
                "14",
            ),
        ),
        Job(
            "ai_b75_use_explosive",
            (
                "branchdump",
                "levels/cyborg_rats/ai_takeover.csv",
                "--prefix",
                AI_B75,
                "--goal",
                "explosivesle:1",
                "--depth",
                "80",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "14",
                "--no-canonical",
            ),
        ),
        Job(
            "ai_early_row5_lure",
            (
                "branchdump",
                "levels/cyborg_rats/ai_takeover.csv",
                "--prefix",
                AI_EARLY_ROW5,
                "--goal",
                "ratplayer:18,5,5,5",
                "--depth",
                "60",
                "--secs",
                "75",
                "--maxnodes",
                "700000",
                "--results",
                "6",
                "--min-rats",
                "23",
                "--min-explosives",
                "9",
                "--min-triggers",
                "14",
            ),
        ),
        Job(
            "blocked_b20_open_left_web",
            (
                "branchdump",
                "levels/cooperation/blocked_v2.csv",
                "--prefix",
                BLOCKED_B20,
                "--goal",
                "cellnot:1,15,web",
                "--depth",
                "90",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "3",
                "--min-reachable-rats",
                "2",
            ),
        ),
        Job(
            "tug_t1_keep_resources_cleanup",
            (
                "branchdump",
                "levels/cooperation/tug_of_war.csv",
                "--prefix",
                TUG_T1,
                "--goal",
                "ratsle:3",
                "--depth",
                "90",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "3",
                "--min-triggers",
                "8",
                "--min-explosives",
                "3",
            ),
        ),
        Job(
            "handoff_courier_open_southeast",
            (
                "branchdump",
                "levels/cooperation/handoff.csv",
                "--prefix",
                HANDOFF_COURIER,
                "--goal",
                "cellnot:10,8,explosive",
                "--depth",
                "70",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "4",
                "--min-triggers",
                "4",
                "--min-explosives",
                "6",
            ),
        ),
        Job(
            "handoff_pre_t1_far_lure",
            (
                "branchdump",
                "levels/cooperation/handoff.csv",
                "--prefix",
                HANDOFF_PRE_T1,
                "--goal",
                "ratrectplayerrect:11,7,11,7,13,7,14,8",
                "--depth",
                "24",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "4",
                "--min-explosives",
                "8",
            ),
        ),
        Job(
            "tug_t3_baffle_then_trigger1",
            (
                "branchdump",
                "levels/cooperation/tug_of_war.csv",
                "--prefix",
                TUG_PRE_T3_BAFFLE,
                "--goal",
                "triggeronly:1",
                "--depth",
                "24",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "7",
                "--min-reachable-rats",
                "4",
            ),
        ),
        Job(
            "blocked_pre_t5_open_left_mouth",
            (
                "branchdump",
                "levels/cooperation/blocked_v2.csv",
                "--prefix",
                BLOCKED_PRE_T5,
                "--goal",
                "cellnot:1,15,web",
                "--depth",
                "30",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "9",
                "--min-explosives",
                "21",
            ),
        ),
        Job(
            "blocked_post_t5_pre_t1_trigger2",
            (
                "branchdump",
                "levels/cooperation/blocked_v2.csv",
                "--prefix",
                BLOCKED_POST_T5_PRE_T1,
                "--goal",
                "triggeronly:2",
                "--depth",
                "30",
                "--secs",
                "75",
                "--maxnodes",
                "900000",
                "--results",
                "6",
                "--min-rats",
                "9",
                "--min-explosives",
                "21",
            ),
        ),
    ]


def set_limits(mem_mb: int) -> None:
    mem_bytes = mem_mb * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (mem_bytes, mem_bytes))


def run_job(
    job: Job,
    out_dir: pathlib.Path,
    prune_dead: bool,
) -> tuple[str, int, float, pathlib.Path, bool]:
    log_path = out_dir / f"{job.name}.log"
    command = [str(SOLVER), *job.args]
    started = time.monotonic()
    env = None
    if prune_dead:
        env = dict(os.environ)
        env["PRUNE_DEAD"] = "1"
    with log_path.open("w", encoding="utf-8") as log:
        log.write("$ " + shlex.join(command) + "\n")
        if prune_dead:
            log.write("# env PRUNE_DEAD=1\n")
        log.flush()
        try:
            proc = subprocess.run(
                command,
                cwd=ROOT,
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=job.timeout_sec,
                preexec_fn=lambda: set_limits(job.mem_mb),
                check=False,
            )
            code = proc.returncode
        except subprocess.TimeoutExpired:
            log.write(f"\nTIMEOUT after {job.timeout_sec}s\n")
            code = 124
    elapsed = time.monotonic() - started
    text = log_path.read_text(encoding="utf-8", errors="replace")
    solved = "SOLVED " in text or "result=Won" in text
    return job.name, code, elapsed, log_path, solved


def interleave_by_level(jobs: Iterable[Job]) -> list[Job]:
    """Round-robin jobs across levels so the portfolio stays mechanism-diverse."""
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
    parser.add_argument("--jobs", type=int, default=3, help="maximum concurrent jobs")
    parser.add_argument("--preset", choices=["current"], default="current")
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        help="run only jobs whose name contains this substring; repeatable",
    )
    parser.add_argument(
        "--skip",
        action="append",
        default=[],
        help="skip jobs whose name contains this substring; repeatable",
    )
    parser.add_argument(
        "--prune-dead",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="set PRUNE_DEAD=1 for solver children",
    )
    parser.add_argument("--out-dir", type=pathlib.Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not SOLVER.exists():
        raise SystemExit(f"missing solver binary: {SOLVER}")
    timestamp = dt.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    out_dir = args.out_dir or RUN_ROOT / f"{timestamp}_bounded_portfolio"
    out_dir.mkdir(parents=True, exist_ok=True)

    jobs = current_jobs()
    if args.only:
        jobs = [job for job in jobs if any(token in job.name for token in args.only)]
    if args.skip:
        jobs = [job for job in jobs if not any(token in job.name for token in args.skip)]
    if not jobs:
        raise SystemExit("no jobs selected")
    jobs = interleave_by_level(jobs)
    print(f"running {len(jobs)} jobs with concurrency={args.jobs} logs={out_dir}")
    found_solution = False
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
        futures = [executor.submit(run_job, job, out_dir, args.prune_dead) for job in jobs]
        for future in concurrent.futures.as_completed(futures):
            name, code, elapsed, log_path, solved = future.result()
            found_solution = found_solution or solved
            marker = "SOLVED" if solved else "done"
            print(f"{marker} {name} code={code} elapsed={elapsed:.1f}s log={log_path}")
    if not found_solution:
        print("no solved job in this bounded portfolio")
    return 0


if __name__ == "__main__":
    sys.exit(main())
