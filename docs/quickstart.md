# Quickstart

Use this page to go from clone/install to a verified deterministic step loop.

Next read: [RL Contract](rl_contract.md)

## Prerequisites

- Python 3.10+
- Rust stable (`rustup default stable`)
- `pip`

Recommended:

- virtual environment (`python -m venv .venv`)
- `maturin` for local wheel/module builds

## Install

### Fastest: PyPI

```bash
python -m pip install -U weiss-sim numpy
```

### Local source build

```bash
python -m pip install -U maturin numpy
maturin develop --release --manifest-path weiss_py/Cargo.toml
```

## First successful reset + step

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

print(out.obs.shape, out.rewards.shape, out.engine_status[:4])
```

Expected result:

- no exception
- stable shapes across runs
- `engine_status == 0` for healthy envs

## Which API should you use?

- `weiss_sim.train(...)` / `weiss_sim.evaluate(...)`: recommended for most users
- `weiss_sim.EnvPool.*`: lower-level control for custom pipelines

## High-level API defaults (`create/train/evaluate`)

From `python/weiss_sim/api.py`:

- `rules_profile="strict"`
- `runtime_mode="speed"` for `train()`, `"eval_debug"` for `evaluate()`
- `card_pool="parsed_only"`
- `observation_visibility="public"`
- `error_policy="lenient_terminate"`
- `max_decisions=2000`, `max_ticks=100000`

Runtime-mode defaults:

| runtime_mode | legal_repr | obs_dtype | ids_safety |
| --- | --- | --- | --- |
| `speed` | `ids_u16` | `i16` | `checked` |
| `eval_debug` | `both` | `i32` | n/a |

Auto sizing rules:

- `num_threads="auto"` -> `min(16, cpu_count)` then capped at `num_envs`
- `num_envs="auto"` -> `min(128, max(32, 4 * resolved_threads))`

Public-visibility behavior in high-level API:

- opponent private zones stay masked
- `memory_is_public` is forced to `False` unless explicitly overridden
- `reveal_opponent_hand_stock_counts` defaults to `False`

## High-level minimal loop

```python
import numpy as np
import weiss_sim

sim = weiss_sim.train(num_envs=32, seed=0)
reset = sim.reset()
actions = np.full((32,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
step = sim.step(actions)
```

Seat-aware helpers for two-policy play:

- `sim.current_to_play_seat()`
- `sim.merge_actions_by_seat(seat0_actions, seat1_actions, default_action=...)`
- `sim.step_by_seat(seat0_actions, seat1_actions, default_action=...)`

## Deck inputs in high-level API

`deck` / `opponent_deck` accept:

- `Sequence[int]`
- `Mapping[int|str, int]`
- preset string (`"preset:starter_v1"`)
- path string (`"file:..."` or path-like string)

Examples:

```python
import weiss_sim

sim = weiss_sim.create(
    deck="preset:starter_v1",
    opponent_deck="preset:starter_v1",
    card_pool="parsed_only",
)
```

`card_pool="parsed_only"` enforces catalog/db hash compatibility and raises `DbMismatchError` on mismatch.

## Low-level constructor behavior

- `EnvPool.new_rl_train(...)` and `EnvPool.new_rl_eval(...)`:
  - force public observation visibility
  - force `enable_visibility_policies=true`
  - force `allow_concede=false`
- `EnvPool.new_debug(...)`:
  - no RL policy overrides; use when you need exact curriculum/control

## Determinism checklist

For reproducible episodes, keep all of these fixed:

1. seed
2. deck lists/ids
3. action sequence
4. config/curriculum/reward/end-condition settings
5. contract versions (`OBS_ENCODING_VERSION`, `ACTION_ENCODING_VERSION`, `SPEC_HASH`)

Useful metadata surfaces:

- `episode_seed_batch()`
- `episode_index_batch()`
- `env_index_batch()`
- `starting_player_batch()`

## Sanity checks before long runs

```bash
python scripts/check_docs_constants.py
python scripts/check_docs_links.py
pytest -q python/tests
cargo test --workspace --features test-harness
```

## Common setup issues

- `ModuleNotFoundError: weiss_sim`: package/module not installed in active env.
- maturin build errors: verify active Python + Rust toolchain.
- deck validation failures: deck must be legal and 50 cards.

## Related

- [Python API](python_api.md)
- [RL Contract](rl_contract.md)
- [Encodings](encodings.md)
- [Troubleshooting](troubleshooting.md)
