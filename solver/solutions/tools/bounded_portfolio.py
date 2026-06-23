#!/usr/bin/env python3
"""Run bounded solver portfolios with captured logs.

The hard levels need parallel probes, but unbounded fan-out can OOM the
workstation. This runner keeps each solver process under a virtual-memory cap,
limits wall time, and records stdout/stderr for every job.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import os
import pathlib
import re
import resource
import shlex
import subprocess
import sys
import time
from collections import defaultdict, deque
from collections.abc import Iterable
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLVER = ROOT / "target" / "release" / "solver"
RUN_ROOT = pathlib.Path("/tmp/infestation-runs")


@dataclasses.dataclass(frozen=True)
class Job:
    name: str
    args: tuple[str, ...]
    timeout_sec: int = 90
    mem_mb: int = 1_200


@dataclasses.dataclass
class ActiveJob:
    job: Job
    process: subprocess.Popen[None]
    log_path: pathlib.Path
    log_handle: Any
    started: float
    effective_mem_mb: int


BEST_H_RE = re.compile(r"\bbest_h=(-?\d+)\b")


TINDER_T106 = (
    "<<>^v<<>>^<v<<>>>^^vv<<^v>>^^<vv<<<>>>>^^^>>v>vv>>^^^>>vvv"
    "^^^><<<vvv<<^^^<<<<<v<v<>^>^>>>>>vvv>>^^^>>><vvv"
)

RELEASE_R20 = "v<vv^^>>vv>^<<v<<<^^"
RELEASE_P37 = "v<vv^^>>vv><<v<<<^<^^<<>>>vvv>>>>>>>>"
RELEASE_EARLY_T2 = "v<vv^^>"
RELOAD_T2 = ">>>^>>>>.>>.<.<<<<"
RELOAD_STATION = ">>>^>>>>vvv.v<^<<<v<<<<<<<^^<^"
CHASE_P109 = (
    "^>>>v^^^>^^>>>>>v>>vvvvv^^^^^^<<<<v<<<>v<<^^^<<<^^^^^^>>>>>>vvv>>>>"
    "^^<^^^^vv<<v<<<vvvvvvvvvvvvvv<<>^^^^^>>>vv"
)
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
            "chase_initial_open_11_16_with_rat",
            (
                "branchdump",
                "levels/chase.csv",
                "--goal",
                "cellnotratrect:11,16,web,11,17,11,17",
                "--depth",
                "140",
                "--secs",
                "75",
                "--maxnodes",
                "1200000",
                "--results",
                "8",
                "--min-rats",
                "4",
                "--min-reachable-rats",
                "4",
                "--max-trapped-rats",
                "0",
                "--no-canonical",
            ),
        ),
        Job(
            "chase_p109_open_11_16",
            (
                "branchdump",
                "levels/chase.csv",
                "--prefix",
                CHASE_P109,
                "--goal",
                "cellnot:11,16,web",
                "--depth",
                "90",
                "--secs",
                "75",
                "--maxnodes",
                "1200000",
                "--results",
                "8",
                "--min-rats",
                "4",
                "--min-reachable-rats",
                "4",
                "--max-trapped-rats",
                "0",
                "--no-canonical",
            ),
        ),
        Job(
            "chase_p109_move_or_remove_11_17",
            (
                "branchdump",
                "levels/chase.csv",
                "--prefix",
                CHASE_P109,
                "--goal",
                "ratgone:11,17",
                "--depth",
                "90",
                "--secs",
                "75",
                "--maxnodes",
                "1200000",
                "--results",
                "8",
                "--min-rats",
                "3",
                "--min-reachable-rats",
                "3",
                "--max-trapped-rats",
                "0",
                "--no-canonical",
            ),
        ),
        Job(
            "chase_p109_merge_11_17_component",
            (
                "branchdump",
                "levels/chase.csv",
                "--prefix",
                CHASE_P109,
                "--goal",
                "ratrectcomponentge:11,17,11,17,2",
                "--depth",
                "90",
                "--secs",
                "75",
                "--maxnodes",
                "1200000",
                "--results",
                "8",
                "--min-rats",
                "4",
                "--min-reachable-rats",
                "4",
                "--max-trapped-rats",
                "0",
                "--no-canonical",
            ),
        ),
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


def available_memory_mb() -> int | None:
    try:
        for line in pathlib.Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemAvailable:"):
                return int(line.split()[1]) // 1024
    except OSError:
        return None
    return None


def auto_jobs(requested: int, jobs: list[Job], mem_mb: int | None) -> int:
    if requested > 0:
        return max(1, min(requested, len(jobs)))
    cpu_count = os.cpu_count() or 1
    effective_mem_mb = mem_mb or max(job.mem_mb for job in jobs)
    mem_available = available_memory_mb()
    if mem_available is None:
        return max(1, min(cpu_count, len(jobs)))
    reserve_mb = 4096
    memory_workers = max(1, (mem_available - reserve_mb) // effective_mem_mb)
    return max(1, min(cpu_count, memory_workers, len(jobs)))


def classify_log(text: str, returncode: int) -> str:
    if "SOLVED " in text or "result=Won" in text:
        return "SOLVED"
    if "BRANCH " in text or "\nGOAL " in text:
        return "HIT"
    if "PREFIX_STOP" in text:
        return "PREFIX_STOP"
    if "NO_EVENTS" in text:
        return "NO_EVENTS"
    if "NO_SOLUTION" in text:
        return "NO_SOLUTION"
    if "branchdump:" in text:
        return "NO_BRANCH"
    if "TIMEOUT" in text or returncode == 124:
        best_h = BEST_H_RE.findall(text)
        if best_h:
            tail = best_h[-3:]
            if len(tail) >= 2 and len(set(tail)) == 1:
                return f"TIMEOUT_STALLED_H={tail[-1]}"
            return f"TIMEOUT_BEST_H={best_h[-1]}"
        return "TIMEOUT"
    if returncode != 0:
        if "memory allocation" in text or "Cannot allocate memory" in text:
            return "MEMORY"
        return f"EXIT_{returncode}"
    best_h = BEST_H_RE.findall(text)
    if best_h:
        tail = best_h[-3:]
        if len(tail) >= 2 and len(set(tail)) == 1:
            return f"STALLED_H={tail[-1]}"
    return "DONE"


def start_job(
    job: Job,
    out_dir: pathlib.Path,
    prune_dead: bool,
    mem_mb: int | None,
) -> ActiveJob:
    log_path = out_dir / f"{job.name}.log"
    command = [str(SOLVER), *job.args]
    effective_mem_mb = mem_mb or job.mem_mb
    env = None
    if prune_dead:
        env = dict(os.environ)
        env["PRUNE_DEAD"] = "1"

    log = log_path.open("w", encoding="utf-8")
    log.write("$ " + shlex.join(command) + "\n")
    if prune_dead:
        log.write("# env PRUNE_DEAD=1\n")
    log.write(f"# mem_mb={effective_mem_mb} timeout_sec={job.timeout_sec}\n")
    log.flush()
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
        preexec_fn=lambda: set_limits(effective_mem_mb),
    )
    return ActiveJob(
        job=job,
        process=process,
        log_path=log_path,
        log_handle=log,
        started=time.monotonic(),
        effective_mem_mb=effective_mem_mb,
    )


def finish_job(active: ActiveJob, code: int) -> tuple[str, int, float, pathlib.Path, bool, str]:
    elapsed = time.monotonic() - active.started
    active.log_handle.write(f"\nEXIT {code}\n")
    active.log_handle.close()
    text = active.log_path.read_text(encoding="utf-8", errors="replace")
    solved = "SOLVED " in text or "result=Won" in text
    return (
        active.job.name,
        code,
        elapsed,
        active.log_path,
        solved,
        classify_log(text, code),
    )


def stop_timed_out_job(active: ActiveJob) -> int:
    active.log_handle.write(f"\nTIMEOUT after {active.job.timeout_sec}s\n")
    active.log_handle.flush()
    active.process.terminate()
    try:
        active.process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        active.process.kill()
        active.process.wait()
    return 124


def run_jobs(
    jobs: list[Job],
    out_dir: pathlib.Path,
    prune_dead: bool,
    mem_mb: int | None,
    workers: int,
) -> bool:
    pending = deque(jobs)
    active: list[ActiveJob] = []
    found_solution = False

    def launch_ready() -> None:
        while pending and len(active) < workers:
            job = pending.popleft()
            active_job = start_job(job, out_dir, prune_dead, mem_mb)
            active.append(active_job)
            print(
                f"start {job.name} pid={active_job.process.pid} "
                f"mem={active_job.effective_mem_mb}MB timeout={job.timeout_sec}s"
            )
            sys.stdout.flush()

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
            name, code, elapsed, log_path, solved, signal = finish_job(running, code)
            found_solution = found_solution or solved
            marker = "SOLVED" if solved else signal
            print(f"{marker} {name} code={code} elapsed={elapsed:.1f}s log={log_path}")
            sys.stdout.flush()
            launch_ready()
        if active:
            time.sleep(0.25)
    return found_solution


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
    parser.add_argument(
        "--jobs",
        type=int,
        default=0,
        help="maximum concurrent jobs; 0 chooses a CPU/memory-aware default",
    )
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
    parser.add_argument(
        "--mem-mb",
        type=int,
        help="override per-child virtual-memory cap",
    )
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
    workers = auto_jobs(args.jobs, jobs, args.mem_mb)
    mem_available = available_memory_mb()
    print(
        f"running {len(jobs)} jobs with concurrency={workers} "
        f"requested_jobs={args.jobs} mem_available_mb={mem_available} logs={out_dir}"
    )
    found_solution = run_jobs(jobs, out_dir, args.prune_dead, args.mem_mb, workers)
    if not found_solution:
        print("no solved job in this bounded portfolio")
    return 0


if __name__ == "__main__":
    sys.exit(main())
