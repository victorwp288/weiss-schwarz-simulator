# Contributing

This project prioritizes deterministic behavior, stable contracts, and reproducible performance.

A contribution is complete only when code, tests, and docs all agree.

## Contribution Workflow

1. Create a focused branch and keep changes scoped.
2. Implement behavior with tests first (or in lock-step).
3. Update docs in the same PR.
4. Run quality gates locally.
5. Open PR with explicit notes on determinism and contract impact.

## Repository Areas

- `weiss_core/`: Rust engine implementation and encode/state contracts.
- `weiss_py/`: PyO3 boundary.
- `python/weiss_sim/`: Python wrappers and ergonomics.
- `python/tests/`: Python API and integration tests.
- `docs/`: user/developer documentation.
- `scripts/`: CI and local quality checks.

## Local Quality Gates

Run these before pushing.

### Full parity run

```bash
scripts/run_local_ci_parity.sh
SKIP_BENCHMARKS=1 scripts/run_local_ci_parity.sh
```

`scripts/run_local_ci_parity.sh` runs checks in CI order and fail-fast mode:

1. docs + layering checks
2. rust fmt/clippy/test/doc
3. ruff format/check
4. coverage report/targets + budget gate
5. wheel build + wheel install + pytest
6. perf capture + perf budget gate
7. security audits (`cargo audit`, `pip-audit`)

### Wheel-install pytest requirement

Always validate Python tests against the built wheel, not source imports:

```bash
maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter .venv/bin/python
.venv/bin/python -m pip install --force-reinstall --no-deps /tmp/wss_dist/*.whl
.venv/bin/python -m pytest -q python/tests
```

## Determinism and Contract Rules

If behavior changes can affect encoding/action semantics, update all of:

1. source constants and implementation
2. [RL Contract](rl_contract.md) checksum table
3. [Encodings Changelog](encodings_changelog.md)
4. relevant tests (Rust and/or Python)

Never change version constants casually. Bump only with an intentional compatibility decision.

## Docs Definition of Done

For each behavior PR:

- Update at least one canonical doc page (not only inline comments).
- Keep `PROJECT_STATE.md` aligned with actual engine behavior.
- Ensure links remain valid (`check_docs_links.py`).
- Ensure checksum constants remain accurate (`check_docs_constants.py`).

## PR Template Expectations

A strong PR description includes:

- behavior change summary
- determinism impact (or explicit "none")
- encoding/schema impact (or explicit "none")
- test evidence (commands + results)
- docs updated (which files)

## Coverage baseline refresh

If parser/rule-pack changes legitimately move coverage floors, refresh baseline and gates:

```bash
python scripts/ability_coverage_report.py --output /tmp/ability_coverage_report.json
python scripts/ability_coverage_targets.py --report /tmp/ability_coverage_report.json --output /tmp/ability_coverage_targets.json
cp /tmp/ability_coverage_targets.json scripts/ability_coverage_baseline.json
```

Then update `.github/workflows/ci.yml` coverage floor/ceiling args to match the new report while keeping regression checks enabled.

## Perf baseline refresh (when relevant)

If core/Python perf behavior legitimately shifts, refresh checked-in baselines:

```bash
mkdir -p /tmp/wss_perf_after
cargo bench -p weiss_core --bench core_benches -- --output-format bencher > /tmp/wss_perf_after/benches.txt
cargo bench -p weiss_core --bench alloc_benches -- --output-format bencher >> /tmp/wss_perf_after/benches.txt
PYTHONPATH=python .venv/bin/python python/examples/bench_python_boundary.py --num-envs 128 --steps 2000 --warmup 200 --reset-reps 200 --mode both > /tmp/wss_perf_after/python_bench.txt
cp /tmp/wss_perf_after/benches.txt benchmark/benches.txt
cp /tmp/wss_perf_after/python_bench.txt benchmark/python_bench.txt
```

Validate with:

```bash
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
```

## Related

- [Docs hub](README.md)
- [Python API guide](python_api.md)
- [Engine architecture](engine_architecture.md)
- [Rules coverage](rules_coverage.md)
- [Project state](../PROJECT_STATE.md)
