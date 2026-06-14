#!/usr/bin/env python3
"""Run bounded, human-obligation predicate probes in parallel.

These probes are not global solvers. They ask the Rust oracle for short,
structural proofs such as "open this web while the rat is still recoverable".
Any full solution found downstream still needs `solver verify`.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import datetime as dt
import pathlib
import re
import resource
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[3]
SOLVER = ROOT / "target" / "release" / "solver"


@dataclasses.dataclass(frozen=True)
class PredicateJob:
    name: str
    level: str
    args: tuple[str, ...]
    timeout_sec: int


PREDICATES: tuple[PredicateJob, ...] = (
    PredicateJob(
        name="blocked_v2_rat_5_11",
        level="levels/cooperation/blocked_v2.csv",
        args=(
            "branchdump",
            "levels/cooperation/blocked_v2.csv",
            "--goal",
            "ratat:5,11",
            "--depth",
            "60",
            "--secs",
            "30",
            "--maxnodes",
            "350000",
            "--results",
            "8",
            "--min-rats",
            "9",
            "--min-reachable-rats",
            "5",
            "--max-trapped-rats",
            "1",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=45,
    ),
    PredicateJob(
        name="blocked_v2_open_lower_left_mouth",
        level="levels/cooperation/blocked_v2.csv",
        args=(
            "branchdump",
            "levels/cooperation/blocked_v2.csv",
            "--prefix",
            "vv v^ vv <^ <v <^ <.",
            "--goal",
            "cellnotratrect:1,15,web,0,15,2,16",
            "--depth",
            "70",
            "--secs",
            "35",
            "--maxnodes",
            "450000",
            "--results",
            "8",
            "--min-rats",
            "9",
            "--min-reachable-rats",
            "5",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=50,
    ),
    PredicateJob(
        name="handoff_move_10_6_rat",
        level="levels/cooperation/handoff.csv",
        args=(
            "branchdump",
            "levels/cooperation/handoff.csv",
            "--prefix",
            "v^ >^ >^ >^ >^ >^",
            "--goal",
            "ratrectplayerrect:10,6,11,7,13,7,14,8",
            "--depth",
            "60",
            "--secs",
            "30",
            "--maxnodes",
            "350000",
            "--results",
            "8",
            "--min-rats",
            "5",
            "--min-explosives",
            "20",
            "--min-triggers",
            "4",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=45,
    ),
    PredicateJob(
        name="handoff_open_10_5_web",
        level="levels/cooperation/handoff.csv",
        args=(
            "branchdump",
            "levels/cooperation/handoff.csv",
            "--goal",
            "cellnot:10,5,web",
            "--depth",
            "50",
            "--secs",
            "25",
            "--maxnodes",
            "250000",
            "--results",
            "8",
            "--min-rats",
            "5",
            "--min-explosives",
            "20",
            "--min-triggers",
            "4",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=40,
    ),
    PredicateJob(
        name="ai_takeover_trigger7_only",
        level="levels/cyborg_rats/ai_takeover.csv",
        args=(
            "branchdump",
            "levels/cyborg_rats/ai_takeover.csv",
            "--prefix",
            "v<vv^^^",
            "--goal",
            "triggeronly:7",
            "--depth",
            "80",
            "--secs",
            "40",
            "--maxnodes",
            "500000",
            "--results",
            "8",
            "--min-rats",
            "22",
            "--min-reachable-rats",
            "20",
            "--min-explosives",
            "9",
            "--min-triggers",
            "12",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=55,
    ),
    PredicateJob(
        name="ai_takeover_open_16_8",
        level="levels/cyborg_rats/ai_takeover.csv",
        args=(
            "branchdump",
            "levels/cyborg_rats/ai_takeover.csv",
            "--prefix",
            "v<vv^^^",
            "--goal",
            "cellnot:16,8,web",
            "--depth",
            "80",
            "--secs",
            "40",
            "--maxnodes",
            "500000",
            "--results",
            "8",
            "--min-rats",
            "22",
            "--min-reachable-rats",
            "20",
            "--min-explosives",
            "9",
            "--min-triggers",
            "12",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=55,
    ),
    PredicateJob(
        name="on_the_clock_reachable_5",
        level="levels/old_levels/on_the_clock.csv",
        args=(
            "branchdump",
            "levels/old_levels/on_the_clock.csv",
            "--prefix",
            ">>>^^>>>vvv><vvvvv",
            "--goal",
            "reachablege:5",
            "--depth",
            "80",
            "--secs",
            "35",
            "--maxnodes",
            "400000",
            "--results",
            "8",
            "--min-rats",
            "8",
            "--min-explosives",
            "3",
            "--min-triggers",
            "13",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=50,
    ),
    PredicateJob(
        name="on_the_clock_trigger8_open_1_18",
        level="levels/old_levels/on_the_clock.csv",
        args=(
            "branchdump",
            "levels/old_levels/on_the_clock.csv",
            "--prefix",
            ">>>^^>>>vvv><vvvvv",
            "--goal",
            "triggeronlycellnot:8,1,18,web",
            "--depth",
            "80",
            "--secs",
            "35",
            "--maxnodes",
            "400000",
            "--results",
            "8",
            "--min-rats",
            "8",
            "--min-explosives",
            "3",
            "--min-triggers",
            "13",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=50,
    ),
    PredicateJob(
        name="release_open_18_5_left_route",
        level="levels/release.csv",
        args=(
            "branchdump",
            "levels/release.csv",
            "--prefix",
            "v<vv^^>",
            "--goal",
            "cellnotratrect:18,5,web,16,4,19,8",
            "--depth",
            "80",
            "--secs",
            "40",
            "--maxnodes",
            "500000",
            "--results",
            "8",
            "--min-rats",
            "23",
            "--min-reachable-rats",
            "20",
            "--max-trapped-rats",
            "1",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=55,
    ),
    PredicateJob(
        name="release_open_18_5_right_route",
        level="levels/release.csv",
        args=(
            "branchdump",
            "levels/release.csv",
            "--prefix",
            "v>vv^^^",
            "--goal",
            "cellnotratrect:18,5,web,16,4,19,8",
            "--depth",
            "80",
            "--secs",
            "40",
            "--maxnodes",
            "500000",
            "--results",
            "8",
            "--min-rats",
            "23",
            "--min-reachable-rats",
            "20",
            "--max-trapped-rats",
            "1",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=55,
    ),
    PredicateJob(
        name="reload_v3_station_open_1_21",
        level="levels/reload_v3.csv",
        args=(
            "branchdump",
            "levels/reload_v3.csv",
            "--prefix",
            ">>>^>>>>vvv.v<^<<<v<<<<<<<^^<^",
            "--goal",
            "cellnotratrectplayerrect:1,21,web,1,20,8,22,0,17,8,22",
            "--depth",
            "85",
            "--secs",
            "45",
            "--maxnodes",
            "600000",
            "--results",
            "8",
            "--min-rats",
            "3",
            "--min-reachable-rats",
            "1",
            "--max-trapped-rats",
            "1",
            "--min-triggers",
            "14",
            "--require-reachable-trigger",
            "2",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=60,
    ),
    PredicateJob(
        name="reload_v3_alt_open_1_21",
        level="levels/reload_v3.csv",
        args=(
            "branchdump",
            "levels/reload_v3.csv",
            "--prefix",
            ">>>^>>>>.>>.<.<<<<",
            "--goal",
            "cellnot:1,21,web",
            "--depth",
            "85",
            "--secs",
            "45",
            "--maxnodes",
            "600000",
            "--results",
            "8",
            "--min-rats",
            "3",
            "--min-reachable-rats",
            "1",
            "--min-triggers",
            "14",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=60,
    ),
    PredicateJob(
        name="tinderrectangle_separate_lower_rat",
        level="levels/tinderrectangle.csv",
        args=(
            "branchdump",
            "levels/tinderrectangle.csv",
            "--prefix",
            "<^^<v<^^>vvv<<^^^>>>>>>vvv>>^^^>>vvv",
            "--goal",
            "ratrectplayerrectcellnot:1,4,1,4,13,6,15,8,2,4,web",
            "--depth",
            "90",
            "--secs",
            "45",
            "--maxnodes",
            "500000",
            "--results",
            "8",
            "--min-rats",
            "16",
            "--states",
            "--no-canonical",
        ),
        timeout_sec=60,
    ),
    PredicateJob(
        name="tinderrectangle_geomlure_lower",
        level="levels/tinderrectangle.csv",
        args=(
            "geomlure",
            "levels/tinderrectangle.csv",
            "--prefix",
            "<^^<v<^^>vvv<<^^^>>>>>>vvv>>^^^>>vvv",
            "--rats",
            "1,4;1,3;0,0",
            "--safe",
            "13,6;14,7;15,8",
            "--preserve-rats",
            "--depth",
            "120",
            "--secs",
            "45",
            "--maxnodes",
            "500000",
            "--strategy",
            "astar",
            "--weight",
            "2",
        ),
        timeout_sec=60,
    ),
)


def set_memory_limit(mem_mb: int) -> None:
    if mem_mb <= 0:
        return
    limit = mem_mb * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (limit, limit))


def selected_jobs(only: list[str]) -> list[PredicateJob]:
    jobs = list(PREDICATES)
    if only:
        jobs = [
            job
            for job in jobs
            if any(token in job.name or token in job.level for token in only)
        ]
    return jobs


def log_name(job: PredicateJob) -> str:
    clean = re.sub(r"[^a-zA-Z0-9]+", "_", job.name).strip("_").lower()
    return f"{clean}.log"


def run_job(job: PredicateJob, out_dir: pathlib.Path, mem_mb: int) -> tuple[str, str, int]:
    cmd = [str(SOLVER), *job.args]
    try:
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=job.timeout_sec,
            check=False,
            preexec_fn=(lambda: set_memory_limit(mem_mb)) if mem_mb > 0 else None,
        )
        status = "BRANCH" if "BRANCH " in proc.stdout else "NO_BRANCH"
        if "SOLVED" in proc.stdout or "result=Won" in proc.stdout:
            status = "WIN_MARKER"
        if proc.returncode != 0:
            status = f"EXIT_{proc.returncode}_{status}"
        text = "\n".join(
            [
                "$ " + " ".join(cmd),
                "--- stderr ---",
                proc.stderr.rstrip(),
                "--- stdout ---",
                proc.stdout.rstrip(),
                f"STATUS {status}",
            ]
        )
        (out_dir / log_name(job)).write_text(text + "\n", encoding="utf-8")
        return job.name, status, proc.returncode
    except subprocess.TimeoutExpired as error:
        text = "\n".join(
            [
                "$ " + " ".join(cmd),
                "--- stderr ---",
                (error.stderr or "").rstrip() if isinstance(error.stderr, str) else "",
                "--- stdout ---",
                (error.stdout or "").rstrip() if isinstance(error.stdout, str) else "",
                "STATUS TIMEOUT",
            ]
        )
        (out_dir / log_name(job)).write_text(text + "\n", encoding="utf-8")
        return job.name, "TIMEOUT", 124


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out-dir", type=pathlib.Path)
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--mem-mb", type=int, default=1200)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--list", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not SOLVER.exists():
        raise SystemExit(f"missing solver binary: {SOLVER}")
    jobs = selected_jobs(args.only)
    if args.list:
        for job in jobs:
            print(f"{job.name}\t{job.level}\t{' '.join(job.args)}")
        return 0
    if not jobs:
        raise SystemExit("no predicate jobs selected")
    out_dir = args.out_dir
    if out_dir is None:
        stamp = dt.datetime.now(dt.UTC).strftime("%Y%m%dT%H%M%SZ")
        out_dir = pathlib.Path("/tmp/infestation-runs") / f"{stamp}_obligation_predicates"
    out_dir.mkdir(parents=True, exist_ok=True)
    workers = max(1, min(args.jobs, len(jobs)))
    print(f"out_dir={out_dir} jobs={len(jobs)} workers={workers} mem_mb={args.mem_mb}")
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        futures = [pool.submit(run_job, job, out_dir, args.mem_mb) for job in jobs]
        for future in concurrent.futures.as_completed(futures):
            name, status, returncode = future.result()
            print(f"{status}\t{returncode}\t{name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
