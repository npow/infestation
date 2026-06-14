# Tooling (session scripts)

These drove the solving/verification effort. They invoke the `solver` binary
(`../../target/release/solver`) and the level CSVs under `../../../levels/`.
Paths are hard-coded to the session workspace (`/tmp/infestation`) — adjust if reusing.

- `build_final.py` — re-verify every solution against the oracle, emit `final_solutions.json`.
- `verify.py`      — spot-verify a list of solutions.
- `gen_docs.py`    — generate `SOLUTIONS.md` + the autoplay data from the verified set.
- `run_all.py`, `run_failures.py`, `run_pass3.py` — parallel solver orchestration
  (escalating strategies / progress-heuristic) across many levels.
- `bounded_portfolio.py` — current hard-level mechanism portfolio runner. It
  caps concurrency, wall time, and virtual memory per solver process, and
  records one log per job under `/tmp/infestation-runs/`.
