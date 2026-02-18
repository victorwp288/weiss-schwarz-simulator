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

`EnvPool.new_rl_train/new_rl_eval/new_debug` default to the bundled `.wsdb` shipped with
the package. Pass `db_path=...` only when you need to override with your own database.

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
buffers = weiss_sim.EnvPoolBuffers(pool)
out = buffers.reset()

actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buffers.step(actions)

print(out.obs.shape, out.rewards.shape)
```

## High-Level API (recommended)

Use the high-level runner when you want minimal arguments with deterministic defaults:

```python
import numpy as np
import weiss_sim

sim = weiss_sim.train(num_envs=32, seed=0)
reset = sim.reset()
actions = np.full((32,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
step = sim.step(actions)
```

`weiss_sim.evaluate(...)` defaults to eval/debug outputs (legal masks + legal ids).
By default, opponent private zones stay hidden (`observation_visibility="public"`). Override with
`observation_visibility="full"` only for trusted debug/eval runs. If you only need counts, set
`reveal_opponent_hand_stock_counts=True`.
Memory zone is treated as private by default under public visibility; override with
`curriculum={"memory_is_public": True}` only when you intentionally want that information exposed.
For two-policy or human-vs-AI loops, use `sim.current_to_play_seat()`, `sim.merge_actions_by_seat(...)`,
or `sim.step_by_seat(...)` so you can provide separate seat-0/seat-1 action vectors.
For league/population runs, use `weiss_sim.round_robin_schedule(...)` or
`weiss_sim.sample_population_schedule(...)`, then aggregate with `weiss_sim.summarize_records(...)`.

Deck inputs can be presets, paths, card-id lists, or count maps:

```python
import weiss_sim

deck = weiss_sim.cards.resolve_deck(
    "preset:starter_v1",
    rules_profile="approx",
    card_pool="parsed_only",
)
sim = weiss_sim.create(deck=deck, opponent_deck="preset:starter_v1", card_pool="parsed_only")
```

When `card_pool="parsed_only"`, external `db_path` must hash-match the packaged catalog metadata.
If it does not, creation fails with `DbMismatchError` so parsed-only filtering is never silently wrong.

Optional override:

```python
pool = weiss_sim.EnvPool.new_rl_train(
    32,
    db_path="/path/to/your/cards.wsdb",
    deck_lists=[legal_deck, legal_deck],
)
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
