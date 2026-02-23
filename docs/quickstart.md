# Quickstart

Use this page to go from install to a verified reset/step loop with the current high-level API (`weiss-sim 0.7.x`).

Next read: [RL Contract](rl_contract.md)

## Prerequisites

- Python 3.10+
- Rust stable (`rustup default stable`) for local source builds
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

## First successful reset + step (recommended path)

```python
import weiss_sim

with weiss_sim.make(mode="inspect", num_envs=32, seed=0, card_pool="all") as sim:
    batch = sim.reset()
    actions = batch.legal.sample_uniform(seed=123)
    step = sim.step(actions)
    print(step.obs.shape, step.reward.shape, step.engine_status[:4])
```

Expected result:

- no exception
- stable output shapes
- `engine_status == 0` for healthy envs

## Which API should you use?

- `weiss_sim.make(...)`, `weiss_sim.fast(...)`, `weiss_sim.inspect(...)`: recommended for most users
- low-level canonical surface: `make_pool(...)`, `EnvPoolBuffers(..., layout=...)`, `EnvPoolTrajectoryBuffers(..., layout=...)`, `reset_rl(...)`, `step_rl(...)`

## High-level defaults (`make`/`fast`/`inspect`)

Common defaults:

- `rules_profile="strict"`
- `card_pool="parsed_only"`
- `observation_visibility="public"`
- `error_policy="replace"`
- `max_decisions=2000`, `max_ticks=100000`

Mode defaults:

| mode | internal runtime_mode | legal_repr | obs_dtype | ids_safety |
| --- | --- | --- | --- | --- |
| `fast` | `speed` | `ids_u16` | `i16` | `checked` |
| `inspect` | `eval_debug` | `both` | `i32` | n/a |

`runtime_mode=` is rejected on the high-level API.

## 0.7 migration notes

Breaking changes in `0.7.0`:

- `error_policy` accepts only `raise | replace | terminate`.
- legacy `error_policy` aliases are removed.
- high-level `make(...)` no longer accepts deprecated compatibility kwargs such as `runtime_mode=...`.
- `batch.legal.select_from_logits(...)` / `batch.legal.sample_from_logits(...)` were renamed to `batch.legal.argmax_logits(...)` / `batch.legal.sample_logits(...)`.
- coverage tooling accepts only profile names `strict | approx` (legacy `none` / `rl_v1` aliases removed).

## Seed behavior and determinism

- `seed=None` (default) uses entropy.
- `seed=<int>` uses a deterministic user seed.
- For reproducible trajectories, hold fixed:

1. seed
2. deck lists/ids
3. action sequence
4. curriculum/reward/end-condition settings
5. compatibility constants (`OBS_ENCODING_VERSION`, `ACTION_ENCODING_VERSION`, `SPEC_HASH`)

## Legal actions: use `batch.legal`

Preferred:

- `batch.legal.ids(i)`
- `batch.legal.contains(i, action_id)`
- `batch.legal.mask`
- `batch.legal.sample_uniform(seed=...)`
- `batch.legal.argmax_logits(logits)`
- `batch.legal.sample_logits(logits, seed=...)`

Raw properties are still available for advanced integrations:

- `batch.legal_ids`
- `batch.legal_offsets`
- `batch.legal_mask`

## Fast and inspect shortcuts

```python
import weiss_sim

with weiss_sim.fast(num_envs=32, seed=0) as sim:
    batch = sim.reset()

with weiss_sim.inspect(num_envs=32, seed=0) as sim:
    batch = sim.reset()
```

## Deck inputs in high-level API

`deck` / `opponent_deck` accept:

- `Sequence[int]`
- `Mapping[int|str, int]`
- preset string (`"preset:starter_v1"`)
- path string (`"file:..."` or path-like string)

`card_pool="parsed_only"` enforces catalog/db hash compatibility and raises `DbMismatchError` on mismatch.

## Low-level canonical API (layout-based)

Use `make_pool` as the single constructor for low-level RL loops:

```python
import numpy as np
import weiss_sim

pool, buffers = weiss_sim.make_pool(
    mode="train",                # "train" or "eval"
    num_envs=8,
    deck_lists=[deck_a, deck_b],
    seed=7,
    profile="fast",              # optional: fast / balanced / eval / debug
    layout="mask",               # mask / nomask / i16 / i16_legal_ids
)

step = weiss_sim.reset_rl(pool, layout="mask")
actions = np.array([int(np.flatnonzero(step.masks[i])[0]) for i in range(pool.envs_len)], dtype=np.uint32)
step = weiss_sim.step_rl(pool, actions, layout="mask")
```

Logit helpers are canonical and layout-aware:

- `step_rl_select_from_logits(pool, logits, layout=..., actions=None, out=None)`
- `step_rl_sample_from_logits(pool, logits, seeds, layout=..., actions=None, out=None)`

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
