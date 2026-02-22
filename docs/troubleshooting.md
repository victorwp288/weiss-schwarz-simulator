# Troubleshooting

Common issues and direct fixes.

## Build/import issues

### `ModuleNotFoundError: weiss_sim`

Install module/wheel into the active environment:

```bash
python -m pip install -U weiss-sim
```

or local build:

```bash
python -m pip install -U maturin
maturin develop --release --manifest-path weiss_py/Cargo.toml
```

### `maturin`/Rust toolchain failures

```bash
rustup default stable
python -m pip install -U maturin
maturin build --release --manifest-path weiss_py/Cargo.toml --out dist --interpreter python
```

## Test failures

### Python tests fail after Rust changes

Rebuild and reinstall wheel before running tests:

```bash
maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter python
python -m pip install --force-reinstall --no-deps /tmp/wss_dist/*.whl
pytest -q python/tests
```

### Docs checks fail

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
```

Fix reported link/checksum mismatches, then rerun.

## Runtime issues

### Non-zero `engine_status`

Treat as fault, not warning:

- inspect `engine_status`, `decision_id`, `actor`
- reset faulted envs via `WeissEnv.auto_reset_on_engine_errors(...)` in high-level loops

High-level example:

```python
step = sim.step(actions)
if (step.engine_status != 0).any():
    reset_count, _ = sim.auto_reset_on_engine_errors(step.engine_status)
    if reset_count:
        # faulted envs were reset in-place; continue loop with updated state
        pass
```

Low-level fallback (buffer/pool workflow):

```python
codes = out.engine_status
if (codes != 0).any():
    pool.auto_reset_on_error_codes_into(codes, out)
```

### Unexpected illegal actions / empty legal sets

Checklist:

1. verify you use legal mask or packed legal ids from current boundary row
2. fallback to `PASS_ACTION_ID` when allowed
3. verify action id shape is `(num_envs,)` and dtype `uint32`

### Determinism drift

Checklist:

1. same seed path + deck config
2. same action sequence
3. same curriculum/reward/end-condition settings
4. same package/build version and `SPEC_HASH`
5. compare replay metadata (`spec_hash`, `config_hash`, seeds)

## Performance regressions

Run canonical snapshot commands:

```bash
scripts/run_perf_snapshot.sh /tmp/wss_perf_after
python scripts/check_perf_budget.py \
  --baseline-benches benchmark/benches.txt \
  --current-benches /tmp/wss_perf_after/benches.txt \
  --baseline-python benchmark/python_bench.txt \
  --current-python /tmp/wss_perf_after/python_bench.txt \
  --max-core-regression-pct 15 \
  --max-python-regression-pct 10 \
  --require-zero-alloc
```

## Full local parity

```bash
scripts/run_local_ci_parity.sh
```

## Related

- [Quickstart](quickstart.md)
- [Python API](python_api.md)
- [RL Contract](rl_contract.md)
- [Performance & Benchmarks](performance_benchmarks.md)
