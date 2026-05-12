# Contributing

This project prioritizes deterministic behavior, stable contracts, and reproducible
performance. A change is not complete until code, tests, and docs agree.

## Workflow

1. keep PR scope focused
2. add or update tests with the behavior change
3. update docs in the same PR
4. run local quality gates
5. include behavior, determinism, compatibility, and perf impact in the PR

## Repository Map

- `weiss_core/`: Rust engine runtime, rules, encodings, replay, and benchmarks.
- `weiss_py/`: PyO3 extension layer.
- `python/weiss_sim/`: Python API and buffer helpers.
- `python/tests/`: Python API, parity, and contract tests.
- `scraper/`: card conversion, ability parsing, and coverage tooling.
- `scripts/`: docs, CI parity, coverage, release, and perf checks.
- `docs/`: compact human-facing documentation.

## Required Checks

Fast local checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
python -m ruff format --check python scraper scripts
python -m ruff check python scraper scripts
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
python scripts/gen_docs_snippets.py --check
python -m pytest -q python/tests
python -m unittest scraper.test_convert
```

Full local parity:

```bash
bash scripts/run_local_ci_parity.sh
```

During iteration:

```bash
SKIP_BENCHMARKS=1 bash scripts/run_local_ci_parity.sh
```

Wheel-install path:

```bash
python -m maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter python
python -m pip install --force-reinstall --no-deps /tmp/wss_dist/*.whl
python -m pytest -q python/tests
```

## Contract-Sensitive Changes

If observation/action encoding, legal payloads, replay, WSDB, or public Python names
change:

1. update implementation and compatibility constants
2. update [RL Contract](docs/rl_contract.md)
3. update stubs/generated docs when Python exports change
4. add regression tests
5. mention the compatibility impact in the PR

Do not bump compatibility versions casually.

## Coverage And Performance Baselines

Coverage:

```bash
python scripts/ability_coverage_report.py --output /tmp/ability_coverage_report.json
python scripts/ability_coverage_targets.py --report /tmp/ability_coverage_report.json --output /tmp/ability_coverage_targets.json
python scripts/check_coverage_budget.py \
  --report /tmp/ability_coverage_report.json \
  --baseline scripts/ability_coverage_baseline.json \
  --min-parse-line-coverage-strict 0.52 \
  --max-unsupported-lines-strict 14200 \
  --min-card-coverage-approx 0.99
```

Performance:

```bash
mkdir -p /tmp/wss_perf_after
bash scripts/run_perf_snapshot.sh /tmp/wss_perf_after
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --core-budget-override reset_batch_256=25 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
```

Only copy new baselines into `benchmark/` after confirming the change is intentional
and measured on a comparable setup.

## Release Flow

Versioned releases are managed by Release Please. For manual release-prep checks,
keep these aligned:

- `pyproject.toml`
- `weiss_core/Cargo.toml`
- `weiss_py/Cargo.toml`
- `weiss_py -> weiss_core` dependency version
- `Cargo.lock`
- `.release-please-manifest.json`
- `CHANGELOG.md`

Before tagging, verify the repo secret `RELEASE_PLEASE_TOKEN` is configured if release
tags must trigger downstream wheel publishing.

## PR Checklist

Include:

- behavior summary
- determinism impact (`none` if unchanged)
- compatibility/version impact (`none` if unchanged)
- performance impact (`not measured` only when irrelevant)
- commands run and key results
- docs updated

Related docs: [Quickstart](docs/quickstart.md), [Python API](docs/python_api.md),
[RL Contract](docs/rl_contract.md), [Architecture](docs/architecture.md), and
[Performance](docs/performance_benchmarks.md).
