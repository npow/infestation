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
  It also interleaves levels, enables `PRUNE_DEAD=1` by default, and stops
  remaining queued/running probes after a solution marker unless
  `--no-stop-on-solved` is passed.
  Use `--max-jobs` to run short, inspectable waves instead of the whole archive
  queue. Seed-file records with the same structural `event.key` are deduped by
  default before per-level selection; use `--no-dedupe-event-key` to compare
  every route variant. Speculative `lookup win` jobs also stop after
  `--lookup-stagnation-secs` seconds without heuristic improvement; pass `0` to
  restore the older full-budget behavior. By default, expensive `fess` and
  `dropchain` jobs run only for seed records with known clean diagnostics
  (`--expensive-filter known-clean`); pass `--expensive-filter all` to restore
  the older exhaustive queue.
- `frontier_triage.py` — replays archive/static prefixes with the Rust oracle's
  `diag` mode, attaches structural warning flags, and emits rich seed JSONL.
  By default it ranks high-penalty dead-frontier flags below live mechanisms,
  caps repeated diagnostic signatures, and drops high-penalty records when a
  level has lower-penalty alternatives. Use `--per-signature 0` and
  `--max-flag-penalty -1` to preserve the older exhaustive ranking behavior.
  Use this before learned ranking so known dead basins become labeled examples
  and the portfolio runner can avoid expensive probes from already-bad basins.
- `event_seed_builder.py` — macro-expands ranked/static return cells by one
  Rust-oracle `events --families` layer and writes transfer-rankable JSONL
  seeds. With `--transfer-rank`, it lazy-loads the same frozen Sokoban prior and
  scores each irreversible successor before writing it. This is the bridge from
  "score this prefix" to "score the next trigger/web/rat-release event"; it
  keeps level-specific resource/reachability guards and does not reimplement
  game rules. Unscored event successors sort behind transfer-scored successors.
- `transfer_ranker.py` — transfer-guided seed ranker. It loads the public
  pretrained Sokoban checkpoint
  `AlignmentResearch/learned-planner/drc11/eue6pax7/cp_2002944000`, decodes the
  Flax/msgpack conv filters, freezes them as a spatial prior, trains a
  repo-local linear probe by default on `trajectory-json` oracle snapshots plus
  obligation labels, and writes Go-Explore-compatible ranked seeds. Repeating
  `--pretrained-checkpoint` enables an ensemble of frozen Sokoban priors, for
  example DRC11 plus DRC33; JSONL records keep the mean `learned_score` and use
  the best member's penalty-adjusted `rank_score` so a single useful prior can
  propose a branch. This is intentionally transfer learning, not from-scratch
  RL: the expensive representation comes from pretrained board-planning models,
  while the local fit is only a small calibration/ranking head. This is not a
  second game engine and does not prove a solution.
- `obligation_labels.jsonl` — human-style supervision for the hard set. Each
  row labels a prefix as satisfying, staging for, or violating a level-specific
  access/resource obligation such as "open this web before spending that
  trigger". These are transfer-ranking labels only; every move string still
  needs Rust-oracle verification.
- `obligation_predicate_wave.py` — bounded exact-search probes for those same
  obligations. It runs `branchdump`/`geomlure` predicates in parallel with a
  per-child memory cap and writes one log per obligation under
  `/tmp/infestation-runs/`.
- The Rust `lookup`/`branchdump` goals include topology predicates for
  human-style access checks: `ratcomponentge:x,y,min` requires the rat at
  `(x,y)` to sit in a rat-walkable component of at least `min` cells, and
  `ratrectcomponentge:x1,y1,x2,y2,min` applies the same test to any rat in a
  rectangle. Use these to test sealed-rat obligations before launching broad
  cleanup searches.
- `learned_macro_ranker.py` — older NumPy-only macro-feature ranker. Keep it as
  a low-dependency fallback, but prefer `transfer_ranker.py` when the `.venv-ml`
  PyTorch environment is available.

Current transfer workflow:

The intended hard-set path is frozen-prior transfer followed by exact
verification:

1. Collect oracle snapshots from solved trajectories, triage records, and
   hand-labeled obligations.
2. Fit only the small Infestation head (`--head linear` by default) while
   keeping the pretrained spatial prior frozen.
3. Rank archived/static prefixes, then use the Rust oracle to expand structural
   successors.
4. Prefer `event_seed_builder.py --transfer-rank` so the same frozen-prior
   scorer values every irreversible event before expensive children launch.

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
  --obligation-labels solver/solutions/tools/obligation_labels.jsonl \
  --pretrained-checkpoint drc11/eue6pax7/cp_2002944000 \
  --pretrained-checkpoint drc33/bkynosqi/cp_2002944000 \
  --head linear \
  --include-static --jsonl-out /tmp/infestation-runs/transfer_rank_recent.jsonl

# Preferred hard-set path: expand ranked return cells by one structural event
# and transfer-score those event successors in the same pass. This makes the
# pretrained prior choose among trigger/web/rat-release obligations instead of
# only among old raw prefixes.
OMP_NUM_THREADS=4 MKL_NUM_THREADS=4 .venv-ml/bin/python solver/solutions/tools/event_seed_builder.py \
  --seed-file /tmp/infestation-runs/transfer_rank_recent.jsonl \
  --include-static --per-level 3 --rank-key score \
  --transfer-rank \
  --triage /tmp/infestation-runs/triage_recent.jsonl \
  --obligation-labels solver/solutions/tools/obligation_labels.jsonl \
  --pretrained-checkpoint drc11/eue6pax7/cp_2002944000 \
  --pretrained-checkpoint drc33/bkynosqi/cp_2002944000 \
  --head linear \
  --jsonl-out /tmp/infestation-runs/event_seeds_recent.jsonl

# Spend exact oracle search on the ranked prefixes.
PRUNE_DEAD=1 .venv-ml/bin/python solver/solutions/tools/go_explore_portfolio.py \
  --archive /tmp/infestation-runs/empty.jsonl \
  --seed-file /tmp/infestation-runs/event_seeds_recent.jsonl \
  --no-static --rank-key score --jobs 6 --mem-mb 1600 --timeout-sec 240

# Run human-obligation predicate probes directly when transfer-ranked prefixes
# keep falling into the same trap. These are subgoal proofs, not final wins.
python3 solver/solutions/tools/obligation_predicate_wave.py \
  --only release --only reload_v3 --only ai_takeover --only blocked_v2 \
  --jobs 6 --mem-mb 1200 \
  --out-dir /tmp/infestation-runs/obligation_predicates_wave
```

When using transfer-ranked JSONL, `go_explore_portfolio.py --rank-key score`
orders by `rank_score` when present, falling back to `learned_score` and then
the oracle event score. Keep both score fields in generated seeds so model
confidence and hard-obligation penalties can be inspected separately.

The solver binary also exposes JSON oracle modes for ML/data work:

```bash
target/release/solver trajectory-json levels/rats.csv 'v<<^^^>>>><>^' --no-csv
target/release/solver stepjson levels/rats.csv --prefix 'v<<' --action '^' --no-csv
```
