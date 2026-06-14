# Tooling (session scripts)

These drove the solving/verification effort. Newer tools resolve the repository
root from their own path and invoke `target/release/solver` plus level CSVs under
`levels/`. A few older one-off scripts still contain stale `/tmp/infestation`
paths; check before reusing those.

- `build_final.py` — re-verify every solution against the oracle, emit `final_solutions.json`.
- `verify.py`      — spot-verify a list of solutions.
- `gen_docs.py`    — generate `SOLUTIONS.md` + the autoplay data from the verified set.
- `run_all.py`, `run_failures.py`, `run_pass3.py` — parallel solver orchestration
  (escalating strategies / progress-heuristic) across many levels.
- `bounded_portfolio.py` — current hard-level mechanism portfolio runner. It
  caps concurrency, wall time, and virtual memory per solver process, and
  records one log per job under `/tmp/infestation-runs/`. Jobs are submitted
  round-robin by level and set `PRUNE_DEAD=1` by default.
- `archive_logs.py` — parse bounded-run logs into JSONL mechanism/frontier
  records, so follow-up passes can dedupe event families and mark dead basins
  without rereading every raw log.
- `go_explore_portfolio.py` — archive-seeded portfolio runner. It treats saved
  frontier prefixes as return cells, then launches capped `fess`, `dropchain`,
  `lookup win`, and `novelty` probes from the best diverse prefixes per level.
  It also interleaves levels and enables `PRUNE_DEAD=1` by default.
  Use `--max-jobs` to run short, inspectable waves instead of the whole archive
  queue.
- `frontier_triage.py` — replays archive/static prefixes with the Rust oracle's
  `diag` mode, attaches structural warning flags, and emits compact seed JSONL.
  Use this before learned ranking so known dead basins become labeled examples.
- `transfer_ranker.py` — transfer-guided seed ranker. It loads the public
  pretrained Sokoban checkpoint
  `AlignmentResearch/learned-planner/drc11/eue6pax7/cp_2002944000`, decodes the
  Flax/msgpack conv filters, freezes them as a spatial prior, trains a small
  Infestation-specific head on `trajectory-json` oracle snapshots, and writes
  Go-Explore-compatible ranked seeds. This is not a second game engine and does
  not prove a solution.
- `learned_macro_ranker.py` — older NumPy-only macro-feature ranker. Keep it as
  a low-dependency fallback, but prefer `transfer_ranker.py` when the `.venv-ml`
  PyTorch environment is available.

Current transfer workflow:

```bash
# One-time local environment; keep .venv-ml untracked.
python3 -m venv .venv-ml
.venv-ml/bin/python -m pip install --index-url https://download.pytorch.org/whl/cpu \
  torch torchvision --extra-index-url https://pypi.org/simple \
  huggingface_hub safetensors msgpack

# Convert logs to archive records, triage them, then rank return cells.
.venv-ml/bin/python solver/solutions/tools/archive_logs.py /tmp/infestation-runs/<run-dir> \
  --include-best --out /tmp/infestation-runs/archive_recent.jsonl
.venv-ml/bin/python solver/solutions/tools/frontier_triage.py \
  /tmp/infestation-runs/archive_recent.jsonl --include-static \
  --jsonl-out /tmp/infestation-runs/triage_recent.jsonl \
  --seeds-out /tmp/infestation-runs/seeds_recent.jsonl
OMP_NUM_THREADS=4 MKL_NUM_THREADS=4 .venv-ml/bin/python solver/solutions/tools/transfer_ranker.py \
  --archive /tmp/infestation-runs/archive_recent.jsonl \
  --seed-file /tmp/infestation-runs/seeds_recent.jsonl \
  --triage /tmp/infestation-runs/triage_recent.jsonl \
  --include-static --jsonl-out /tmp/infestation-runs/transfer_rank_recent.jsonl

# Spend exact oracle search on the ranked prefixes.
PRUNE_DEAD=1 .venv-ml/bin/python solver/solutions/tools/go_explore_portfolio.py \
  --archive /tmp/infestation-runs/empty.jsonl \
  --seed-file /tmp/infestation-runs/transfer_rank_recent.jsonl \
  --no-static --rank-key score --jobs 6 --mem-mb 1600 --timeout-sec 240
```

The solver binary also exposes JSON oracle modes for ML/data work:

```bash
target/release/solver trajectory-json levels/rats.csv 'v<<^^^>>>><>^' --no-csv
target/release/solver stepjson levels/rats.csv --prefix 'v<<' --action '^' --no-csv
```
