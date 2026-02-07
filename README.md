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

Use a real `.wsdb` path on your machine. The `db_path` below is a placeholder and the
`legal_deck` ids must exist in that database.

If you are working from source, you can use the fixture DB:

```bash
db_path="python/tests/fixtures/cards.wsdb"
```

```python
from pathlib import Path
import numpy as np
import weiss_sim

db_path = Path("/path/to/your/cards.wsdb")
legal_deck = (list(range(1, 14)) * 4)[:50]

pool = weiss_sim.EnvPool.new_rl_train(
    32,
    str(db_path),
    deck_lists=[legal_deck, legal_deck],
    deck_ids=[1, 2],
    seed=0,
)
buf = weiss_sim.EnvPoolBuffers(pool)
out = buf.reset()

actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buf.step(actions)
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

## Repository layout

- `weiss_core/` Rust engine core
- `weiss_py/` PyO3 bindings
- `python/weiss_sim/` Python API helpers and buffer wrappers
- `python/examples/` benchmark and integration examples
- `python/tests/` Python contract and smoke tests
- `docs/` user/developer documentation hub

## Local quality checks

Rust:

```bash
scripts/check_env_layering.sh
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --features test-harness
```

Python:

```bash
ruff format --check python scraper scripts
ruff check python scraper scripts
pytest -q python/tests
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
