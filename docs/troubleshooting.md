# Troubleshooting

This page covers frequent local issues when building, testing, or stepping the simulator.

## Build and Import Failures

### `ModuleNotFoundError: weiss_sim`

Cause:

- Python environment does not contain built extension.

Fix:

```bash
python -m pip install -U maturin
maturin develop --release --manifest-path weiss_py/Cargo.toml
```

Or install published package:

```bash
python -m pip install -U weiss-sim
```

### `maturin` build fails with Rust/toolchain errors

Cause:

- missing or non-stable Rust toolchain
- interpreter mismatch

Fix:

```bash
rustup default stable
python -m pip install -U maturin
maturin build --release --manifest-path weiss_py/Cargo.toml --out dist --interpreter python
```

## Test Failures

### Python tests fail after Rust changes

Cause:

- stale wheel/binary from previous build.

Fix:

```bash
maturin build --release --manifest-path weiss_py/Cargo.toml --out dist --interpreter python
python -m pip install --force-reinstall dist/weiss_sim-*.whl
pytest -q python/tests
```

If multiple wheels exist in `dist/`, remove old files or install the exact wheel you just built.

### Docs checks fail in CI

Cause:

- broken Markdown links
- contract checksum table drift

Fix:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
```

Follow the reported file/anchor mismatch and re-run.

## Runtime and Contract Issues

### Illegal actions or empty legal sets unexpectedly

Checklist:

1. confirm you are using `PASS_ACTION_ID` fallback where appropriate
2. verify decision/mask interpretation matches [RL Contract](rl_contract.md)
3. log `decision_kind`, `decision_id`, and `engine_status` per step

### Determinism drift between runs

Checklist:

1. hold seed, deck lists/ids, and action sequence constant
2. verify config flags match exactly (`CurriculumConfig`, reward/error policy)
3. compare replay/fingerprint metadata from both runs
4. ensure you are not mixing wheel versions with local source versions

## Performance Regressions

If throughput suddenly drops:

1. confirm you are using buffer reuse APIs (`*_into` patterns)
2. prefer legal ids over dense mask scans in hot loops
3. rerun benchmark baselines:

```bash
cargo bench -p weiss_core --bench core_benches
cargo bench -p weiss_core --bench alloc_benches
python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
```

## Related

- [Quickstart](quickstart.md)
- [RL Contract](rl_contract.md)
- [Performance & Benchmarks](performance_benchmarks.md)
- [Contributing](contributing.md)
