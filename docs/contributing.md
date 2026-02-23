# Contributing

This project prioritizes deterministic behavior, stable contracts, and reproducible performance.

A change is not complete until code, tests, and docs agree.

## Workflow

1. keep PR scope focused
2. implement with tests (or test+code in lock-step)
3. update docs in the same PR
4. run local quality gates
5. include determinism/contract impact in PR description

## Repository map

- `weiss_core/`: engine runtime and contracts
- `weiss_py/`: PyO3 boundary
- `python/weiss_sim/`: Python API and helpers
- `python/tests/`: Python tests
- `scripts/`: parity/coverage/perf/docs checks
- `docs/`: docs hub and reference pages

## Required local checks

### Full parity (recommended)

```bash
scripts/run_local_ci_parity.sh
```

During iteration:

```bash
SKIP_BENCHMARKS=1 scripts/run_local_ci_parity.sh
```

### Docs checks

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
python scripts/gen_docs_snippets.py --check
```

### Wheel-install pytest path

```bash
maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter python
python -m pip install --force-reinstall --no-deps /tmp/wss_dist/*.whl
pytest -q python/tests
```

## Contract-sensitive changes

If encoding/action/replay behavior changes:

1. update constants/logic in code
2. update [RL Contract](rl_contract.md)
3. append [Encodings Changelog](encodings_changelog.md)
4. update tests

Do not bump compatibility versions casually.

## Coverage baseline updates

When parser/rule changes intentionally move coverage baselines:

```bash
python scripts/ability_coverage_report.py --output /tmp/ability_coverage_report.json
python scripts/ability_coverage_targets.py --report /tmp/ability_coverage_report.json --output /tmp/ability_coverage_targets.json
cp /tmp/ability_coverage_targets.json scripts/ability_coverage_baseline.json
```

## Perf baseline updates

```bash
mkdir -p /tmp/wss_perf_after
scripts/run_perf_snapshot.sh /tmp/wss_perf_after
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
cp /tmp/wss_perf_after/benches.txt benchmark/benches.txt
cp /tmp/wss_perf_after/python_bench.txt benchmark/python_bench.txt
```

## PR checklist

Include in PR description:

- behavior change summary
- determinism impact (`none` if unchanged)
- contract/version impact (`none` if unchanged)
- commands run + key results
- docs files updated

## Related

- [Docs Hub](README.md)
- [Engine Architecture](engine_architecture.md)
- [RL Contract](rl_contract.md)
- [Project State](../PROJECT_STATE.md)
