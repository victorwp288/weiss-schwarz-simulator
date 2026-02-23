# Weiss Schwarz Simulator

[![CI](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/ci.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/ci.yml)
[![Wheels](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/wheels.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/wheels.yml)
[![Benchmarks](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/benchmarks.yml)
[![Security](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/security.yml)
[![PyPI](https://img.shields.io/pypi/v/weiss-sim?logo=pypi&cacheSeconds=300)](https://pypi.org/project/weiss-sim/)
[![Rustdoc](https://img.shields.io/badge/rustdoc-online-blue)](https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/)
[![Docs Hub](https://img.shields.io/badge/docs-hub-informational)](docs/README.md)

Deterministic Weiss Schwarz simulation for RL and engine research.

## What you get

- Rust engine (`weiss_core`) with deterministic advance-until-decision stepping
- PyO3 bindings (`weiss_py`) and Python API (`python/weiss_sim`) for batched training/eval loops
- Stable observation/action contracts (`OBS_LEN=378`, `ACTION_SPACE_SIZE=527`, `SPEC_HASH=8590000130`)
- Replay and fingerprint surfaces for drift detection and reproducibility

## 5-minute start

### Option A: install from PyPI

```bash
python -m pip install -U weiss-sim numpy
```

### Option B: local build from source

```bash
python -m pip install -U maturin numpy
maturin develop --release --manifest-path weiss_py/Cargo.toml
```

### Option C: contributor setup (dev extras + local extension)

```bash
python -m venv .venv
source .venv/bin/activate
./scripts/setup_dev_env.sh
```

This installs `.[dev]` extras (`ruff`, `pytest`, `maturin`, `pip-audit`) and builds the local extension in-place.

### Minimal high-level loop

```python
import numpy as np
import weiss_sim

with weiss_sim.fast(num_envs=32, seed=0) as sim:
    reset = sim.reset()
    actions = reset.legal.sample_uniform(seed=123)
    step = sim.step(actions)
```

### Minimal low-level loop

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

### Deck authoring flow (Python)

```python
import weiss_sim

builder = weiss_sim.cards.builder(initial="starter_v1")
builder.set_count("CARD-1", 4)
report = builder.validate(rules_profile="approx", card_pool="all")
if report.ok:
    deck_ids = builder.build(rules_profile="approx", card_pool="all")
```

## Architecture at a glance

```mermaid
flowchart LR
  A["Python API\npython/weiss_sim"] --> B["PyO3 bindings\nweiss_py"]
  B --> C["Engine core\nweiss_core"]
  C --> D["Deterministic outputs\nobs/masks-or-ids/reward/status"]
  C --> E["Replay + fingerprint\nrepro & drift debugging"]
```

## Documentation map

Start in [`docs/README.md`](docs/README.md).

Recommended paths:

- New users: [`docs/beginner_happy_path.md`](docs/beginner_happy_path.md) -> [`docs/quickstart.md`](docs/quickstart.md) -> [`docs/how_it_works.md`](docs/how_it_works.md)
- RL users: [`docs/quickstart.md`](docs/quickstart.md) -> [`docs/tutorials/ppo.md`](docs/tutorials/ppo.md) -> [`docs/rl_contract.md`](docs/rl_contract.md) -> [`docs/encodings.md`](docs/encodings.md)
- Off-policy users: [`docs/tutorials/impala_vtrace.md`](docs/tutorials/impala_vtrace.md) -> [`docs/rl_contract.md`](docs/rl_contract.md)
- Python integrators: [`docs/python_api.md`](docs/python_api.md) -> [`docs/python_api_reference.md`](docs/python_api_reference.md) -> [`docs/troubleshooting.md`](docs/troubleshooting.md)
- Engine contributors: [`docs/engine_architecture.md`](docs/engine_architecture.md) -> [`docs/rules_coverage.md`](docs/rules_coverage.md) -> [`PROJECT_STATE.md`](PROJECT_STATE.md)
- Performance work: [`docs/performance_benchmarks.md`](docs/performance_benchmarks.md)

## Repository layout

- `weiss_core/`: Rust engine and deterministic rule runtime
- `weiss_py/`: PyO3 extension layer
- `python/weiss_sim/`: high-level and low-level Python interfaces
- `python/tests/`: Python API/contract tests
- `scripts/`: CI parity, coverage, perf, and docs checks
- `docs/`: user + contributor documentation hub

## Local quality checks

Full local CI parity:

```bash
scripts/run_local_ci_parity.sh
```

Skip benchmark gate during iteration:

```bash
SKIP_BENCHMARKS=1 scripts/run_local_ci_parity.sh
```

Docs-only checks:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
python scripts/gen_docs_snippets.py --check
```

## Benchmark snapshot (main)

<!-- BENCHMARKS:START -->
_Last updated: 2026-02-23 01:29 UTC_

| Benchmark | Time |
| --- | --- |
| rust/advance_until_decision | 49148 ns/iter |
| rust/step_batch_64 | 15240 ns/iter |
| rust/reset_batch_256 | 803322 ns/iter |
| rust/step_batch_fast_256_priority_off | 67411 ns/iter |
| rust/step_batch_fast_256_priority_on | 68008 ns/iter |
| rust/legal_actions | 12 ns/iter |
| rust/legal_actions_forced | 11 ns/iter |
| rust/on_reverse_decision_frequency_on | 1253 ns/iter |
| rust/on_reverse_decision_frequency_off | 1290 ns/iter |
| rust/observation_encode | 186 ns/iter |
| rust/observation_encode_forced | 185 ns/iter |
| rust/mask_construction | 300 ns/iter |
<!-- BENCHMARKS:END -->

Long-form benchmark docs: [`docs/performance_benchmarks.md`](docs/performance_benchmarks.md)

## Compatibility policy

Contract constants are explicit compatibility boundaries:

- `OBS_ENCODING_VERSION=2`
- `ACTION_ENCODING_VERSION=1`
- `POLICY_VERSION=2`
- `REPLAY_SCHEMA_VERSION=2`
- `WSDB_SCHEMA_VERSION=2`

If encoding/layout semantics change, update code + docs in the same PR:

1. constants/encode implementation
2. [`docs/rl_contract.md`](docs/rl_contract.md) checksum table
3. [`docs/encodings_changelog.md`](docs/encodings_changelog.md)

## License

MIT OR Apache-2.0
