# Freeze Preflight (Items 2, 3, 5)

This runbook targets the freeze tasks:

- `2` lock perf + coverage gates
- `3` freeze environment/tooling versions
- `5` freeze RL contract consistency

## One-command preflight

```bash
scripts/freeze_preflight_235.sh
```

Optional output directory:

```bash
scripts/freeze_preflight_235.sh /tmp/wss_freeze_candidate
```

Quick dry run (skip heavy checks):

```bash
SKIP_COVERAGE=1 SKIP_PERF=1 scripts/freeze_preflight_235.sh
```

## What it runs

1. Environment snapshot
2. `scripts/check_docs_constants.py`
3. `scripts/ability_coverage_report.py`
4. `scripts/ability_coverage_targets.py`
5. `scripts/check_coverage_budget.py` with CI thresholds
6. `python -m maturin develop --release`
7. `scripts/run_perf_snapshot.sh`
8. `scripts/check_perf_budget.py` with CI thresholds

## Artifacts

The output directory contains:

- `environment_snapshot.txt` (Python/Rust/maturin versions + git head)
- `pip_freeze.txt` (Python package lock snapshot)
- `git_status_short.txt` (working tree state at capture time)
- `input_sha256.txt` (hashes of lock/baseline/contract inputs)
- `ability_coverage_report.json`
- `ability_coverage_targets.json`
- `perf_snapshot/benches.txt`
- `perf_snapshot/python_bench.txt`

Keep this folder with your freeze tag/commit as thesis reproducibility evidence.
