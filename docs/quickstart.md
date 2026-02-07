# Quickstart

This guide gets you from clone to a verified `EnvPool` step as fast as possible.

If you are integrating into RL training code, follow this page first, then read [RL Contract](rl_contract.md).

## Prerequisites

- Python 3.10+
- Rust stable toolchain
- `pip`

Optional but recommended:

- virtual environment (`python -m venv .venv`)
- `maturin` for local binding builds

## Installation Paths

### Path A: use published wheel (fastest)

```bash
python -m pip install -U weiss-sim numpy
```

### Path B: build from local source (for contributors)

```bash
python -m pip install -U maturin numpy
maturin develop --release --manifest-path weiss_py/Cargo.toml
```

If you prefer wheel install for parity with CI:

```bash
maturin build --release --manifest-path weiss_py/Cargo.toml --out dist --interpreter python
python -m pip install dist/*.whl
```

## First Successful Reset + Step (Python)

Set `db_path` to a real `.wsdb` file on your machine, and ensure `legal_deck` uses ids
that exist in that database.

If you are running from a source checkout, you can use:

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
buffers = weiss_sim.EnvPoolBuffers(pool)
out = buffers.reset()

actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buffers.step(actions)

print(out.obs.shape, out.rewards.shape)
```

Expected outcome:

- no exceptions
- deterministic results for same seed/action stream
- tensor dimensions match contract constants

## Throughput-Oriented Variant

For large-scale training, avoid dense mask scans and use legal ids:

```python
ids, offsets = buffers.legal_action_ids()
for i in range(pool.envs_len):
    start = int(offsets[i])
    end = int(offsets[i + 1])
    actions[i] = weiss_sim.PASS_ACTION_ID if start == end else int(ids[start])
out = buffers.step(actions)
```

## Sanity Checks Before Real Training

1. Verify contract constants in your runtime:

```python
import weiss_sim
print(weiss_sim.OBS_LEN, weiss_sim.ACTION_SPACE_SIZE, weiss_sim.SPEC_HASH)
```

2. Verify spec JSON is available:

```python
import weiss_sim
print(weiss_sim.observation_spec_json()[:120])
print(weiss_sim.action_spec_json()[:120])
```

3. Run local tests:

```bash
pytest -q python/tests
cargo test --workspace --features test-harness
```

## Common Setup Issues

- `ModuleNotFoundError: weiss_sim`: your environment does not have the built wheel/module. Reinstall with one of the installation paths above.
- Build errors during `maturin`: ensure `rustup default stable` and a compatible Python interpreter are active.
- Runtime deck validation errors: decks must be legal for configured rules and expected deck size.

## Next Reads

- [Python API Guide](python_api.md)
- [RL Contract](rl_contract.md)
- [Encodings](encodings.md)
- [Performance & Benchmarks](performance_benchmarks.md)
- [Troubleshooting](troubleshooting.md)
