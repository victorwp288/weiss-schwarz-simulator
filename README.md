# Weiss Schwarz Simulator

[![CI](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/ci.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/ci.yml)
[![Wheels](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/wheels.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/wheels.yml)
[![Benchmarks](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/benchmarks.yml)
[![Security](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/security.yml)
[![Docs](https://img.shields.io/badge/docs-rustdoc-blue)](https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/)
[![PyPI](https://img.shields.io/pypi/v/weiss-sim.svg)](https://pypi.org/project/weiss-sim/)
[![Changelog](https://img.shields.io/badge/changelog-view-blue)](https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/CHANGELOG.md)

Deterministic Weiss Schwarz simulation for RL and engine research.

- Rust handles the hot loop (`weiss_core`)
- Python provides batched stepping (`weiss_sim`)
- The engine advances internally until a decision point, then exposes a stable action-space contract

## Why this project

- Deterministic episodes from seed + action sequence
- Fixed, versioned observation/action encodings for training pipelines
- High-throughput `EnvPool` stepping for large batched RL workloads
- Replay/fingerprint metadata for drift detection and debugging

## 5-minute start (Python)

### Option A: install from PyPI

```bash
python -m pip install -U weiss-sim numpy
```

### Option B: local dev install (Rust + Python)

```bash
python -m pip install -U maturin numpy
maturin develop --release --manifest-path weiss_py/Cargo.toml
```

### Minimal step loop

`EnvPool.new_rl_train/new_rl_eval/new_debug` now default to the bundled `.wsdb` shipped with
the package. Pass `db_path=...` only when you want to override with your own database.

```python
import numpy as np
import weiss_sim

legal_deck = (list(range(1, 14)) * 4)[:50]

pool = weiss_sim.EnvPool.new_rl_train(
    32,
    deck_lists=[legal_deck, legal_deck],
    deck_ids=[1, 2],
    seed=0,
)
buf = weiss_sim.EnvPoolBuffers(pool)
out = buf.reset()

actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buf.step(actions)
```

### New High-Level API (v1)

For a zero-config surface with strict contracts and optional overrides:

```python
import numpy as np
import weiss_sim

sim = weiss_sim.train(num_envs=32, seed=0)
reset = sim.reset()
actions = np.full((32,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
step = sim.step(actions)

# Keep hidden info masked by default; opt into debugging visibility/metadata when needed.
debug_sim = weiss_sim.evaluate(
    num_envs=4,
    observation_visibility="full",
    reveal_opponent_hand_stock_counts=True,
)
```

Key helpers:

- `weiss_sim.create(...)`
- `weiss_sim.train(...)`
- `weiss_sim.evaluate(...)`
- `weiss_sim.cards.search(...)`
- `weiss_sim.cards.get(...)`
- `weiss_sim.cards.resolve_deck(...)`
- `weiss_sim.db_info(...)`

`card_pool="parsed_only"` validates card support against packaged catalog data and requires DB hash
compatibility. If you override `db_path` with a different DB, `create()/train()/evaluate()` fail fast
with `DbMismatchError` instead of silently applying mismatched parsed-only rules.

Optional override:

```python
pool = weiss_sim.EnvPool.new_rl_train(
    32,
    db_path="/path/to/your/cards.wsdb",
    deck_lists=[legal_deck, legal_deck],
)
```

For training-safe loop semantics and contract details, read [`docs/rl_contract.md`](docs/rl_contract.md).

## Documentation

Primary docs entrypoint: [`docs/README.md`](docs/README.md)

Recommended reading paths:

- RL users: `docs/quickstart.md` -> `docs/rl_contract.md` -> `docs/encodings.md`
- Python integrators: `docs/python_api.md` -> `docs/rl_contract.md`
- Engine contributors: `docs/engine_architecture.md` -> `PROJECT_STATE.md` -> `docs/rules_coverage.md`
- Performance work: `docs/performance_benchmarks.md` -> benchmark workflow in `.github/workflows/benchmarks.yml`

Reference links:

- Rust API docs: <https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/>
- Benchmark charts: <https://victorwp288.github.io/weiss-schwarz-simulator/benchmarks>
- Changelog: [`CHANGELOG.md`](CHANGELOG.md)

### Benchmark Snapshot (main, top 12)

<!-- BENCHMARKS:START -->
_Last updated: 2026-02-16 08:36 UTC_

| Benchmark | Time |
| --- | --- |
| rust/advance_until_decision | 35677 ns/iter |
| rust/step_batch_64 | 15265 ns/iter |
| rust/reset_batch_256 | 867903 ns/iter |
| rust/step_batch_fast_256_priority_off | 74124 ns/iter |
| rust/step_batch_fast_256_priority_on | 80038 ns/iter |
| rust/legal_actions | 12 ns/iter |
| rust/legal_actions_forced | 10 ns/iter |
| rust/on_reverse_decision_frequency_on | 1162 ns/iter |
| rust/on_reverse_decision_frequency_off | 1169 ns/iter |
| rust/observation_encode | 178 ns/iter |
| rust/observation_encode_forced | 183 ns/iter |
| rust/mask_construction | 394 ns/iter |
<!-- BENCHMARKS:END -->

## Repository layout

- `weiss_core/` Rust engine core
- `weiss_py/` PyO3 bindings
- `python/weiss_sim/` Python API helpers and buffer wrappers
- `python/examples/` benchmark and integration examples
- `python/tests/` Python contract and smoke tests
- `docs/` user/developer documentation hub

## Local quality checks

Full CI-equivalent local parity:

```bash
scripts/run_local_ci_parity.sh
# Optional during iterative work on thermally constrained machines:
SKIP_BENCHMARKS=1 scripts/run_local_ci_parity.sh
```

Non-benchmark subset (useful on thermally constrained laptops):

```bash
scripts/check_env_layering.sh
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features test-harness
RUSTDOCFLAGS="-D missing-docs" cargo doc --workspace --no-deps
ruff format --check python scraper scripts
ruff check python scraper scripts
python scripts/ability_coverage_report.py --output /tmp/ability_coverage_report.json
python scripts/ability_coverage_targets.py --report /tmp/ability_coverage_report.json --output /tmp/ability_coverage_targets.json
python scripts/check_coverage_budget.py --report /tmp/ability_coverage_report.json --baseline scripts/ability_coverage_baseline.json --min-parse-line-coverage-strict 0.52 --max-unsupported-lines-strict 14200 --min-card-coverage-approx 0.99
maturin build --release --manifest-path weiss_py/Cargo.toml --out /tmp/wss_dist --interpreter .venv/bin/python
.venv/bin/python -m pip install --force-reinstall --no-deps /tmp/wss_dist/*.whl
.venv/bin/python -m pytest -q python/tests
cargo audit
pip-audit .
pip-audit -r scraper/requirements.txt
```

## Compatibility and versioning

Encoding and schema values are explicit and versioned:

- `OBS_ENCODING_VERSION`
- `ACTION_ENCODING_VERSION`
- `REPLAY_SCHEMA_VERSION`
- `WSDB_SCHEMA_VERSION`

If any encoding layout changes, update:

1. source constants
2. [`docs/rl_contract.md`](docs/rl_contract.md) checksum table
3. [`docs/encodings_changelog.md`](docs/encodings_changelog.md)

## License

MIT OR Apache-2.0
