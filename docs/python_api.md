# Python API Guide

This page documents the current `weiss_sim` Python API.

- High-level API: `make()`, `fast()`, `inspect()` -> returns `WeissEnv`
- Low-level API: `EnvPool` + buffer classes for direct batched stepping

## Import

```python
import weiss_sim
```

## High-level API (`WeissEnv`)

### Entry points

- `weiss_sim.make(...)`
- `weiss_sim.fast(...)` (`mode="fast"` fixed)
- `weiss_sim.inspect(...)` (`mode="inspect"` fixed)

Legacy entry points `create()`, `train()`, and `evaluate()` are removed.

### `make()` parameters

```python
weiss_sim.make(
    mode="fast",
    deck=None,
    opponent_deck=None,
    db_path=None,
    rules_profile="strict",
    card_pool="parsed_only",
    curriculum=None,
    reward_json=None,
    end_condition_policy=None,
    observation_visibility="public",
    reveal_opponent_hand_stock_counts=None,
    legal_repr=None,
    obs_dtype=None,
    ids_safety=None,
    num_envs=1,
    num_threads="auto",
    seed=None,
    max_decisions=2000,
    max_ticks=100_000,
    error_policy="replace",
    control_seat=None,
)
```

### Mode defaults

| mode | internal runtime_mode | legal_repr | obs_dtype | ids_safety |
| --- | --- | --- | --- | --- |
| `fast` | `speed` | `ids_u16` | `i16` | `checked` |
| `inspect` | `eval_debug` | `both` | `i32` | n/a |

`runtime_mode=` is rejected at the high-level API; use `mode="fast"` or `mode="inspect"`.

### Seed behavior

- `seed=None` (default): seed comes from entropy (`seed_source="entropy"`).
- `seed=<int>`: deterministic start seed (`seed_source="user"`).
- Deterministic replay requires fixed seed, fixed decks/config, and fixed action sequence.

### Error policy tokens

`make()` accepts exactly:

- `"raise"` -> backend `strict`
- `"replace"` -> backend `lenient_noop`
- `"terminate"` -> backend `lenient_terminate`

### Deck input forms

Accepted for `deck` and `opponent_deck`:

- `Sequence[int]`
- `Mapping[int|str, int]`
- preset string (for example `"preset:starter_v1"`)
- path-like string / `Path`

`card_pool="parsed_only"` enforces packaged catalog compatibility and may raise `DbMismatchError`.

### `WeissEnv` methods

- `reset(seed=None, indices=None) -> ResetBatch`
- `step(actions) -> StepBatch`
- `spec() -> dict`
- `effective_config() -> dict`
- `current_to_play_seat() -> np.ndarray`
- `merge_actions_by_seat(seat0_actions, seat1_actions, default_action=...)`
- `step_by_seat(seat0_actions, seat1_actions, default_action=...)`
- `step_select_from_logits(logits, illegal_value=-1e9)`
- `step_sample_from_logits(logits, seed=None, temperature=1.0, illegal_value=-1e9)`

### Legal actions: primary path (`batch.legal`)

Use `batch.legal` as the main integration surface.

Common helpers:

- `batch.legal.ids(env_i)`
- `batch.legal.contains(env_i, action_id)`
- `batch.legal.mask` (dense mask view)
- `batch.legal.select_from_logits(logits)`
- `batch.legal.sample_from_logits(logits, seed=...)`
- `batch.legal.sample_uniform(seed=...)`

`ResetBatch` and `StepBatch` still expose `legal_ids` and `legal_offsets` as properties for interoperability, but most code should consume `batch.legal`.

### Minimal high-level loop (preferred)

```python
import numpy as np
import weiss_sim

with weiss_sim.make(mode="inspect", num_envs=8, seed=7, card_pool="all") as sim:
    batch = sim.reset()
    actions = batch.legal.sample_uniform(seed=123)
    step = sim.step(actions)
    print(step.obs.shape, step.reward.dtype, step.engine_status[:4])
```

### `effective_config()` keys worth using

The returned dict includes:

- mode/runtime knobs (`mode`, `runtime_mode`, `legal_repr`, `obs_dtype`, `ids_safety`)
- sizing (`num_envs`, `num_threads`)
- determinism (`seed`, `seed_source`)
- policy/config (`rules_profile`, `card_pool`, `curriculum`, `error_policy`, `end_condition_policy`)
- db compatibility (`db` object with hashes and match status)
- compatibility/runtime metadata (`spec_hash`, `action_space`, `reward_timeout_policy`)

### Advanced: raw packed legality

If you need zero-overhead packed legality arrays, consume batch properties directly:

- `batch.legal_ids`
- `batch.legal_offsets`
- optional `batch.legal_mask`

Example packed-id first-legal selection:

```python
actions = np.full((num_envs,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
for i in range(num_envs):
    start = int(batch.legal_offsets[i])
    end = int(batch.legal_offsets[i + 1])
    if end > start:
        actions[i] = int(batch.legal_ids[start])
```

## Low-level API (canonical layout-based surface)

### Constructor

Use `make_pool(...)` as the canonical low-level entrypoint:

```python
pool, buffers = weiss_sim.make_pool(
    mode="train",                 # "train" or "eval"
    num_envs=64,
    db_path=None,
    deck_lists=[deck_a, deck_b],  # required
    deck_ids=[1, 2],
    max_decisions=2000,
    max_ticks=100_000,
    seed=0,
    profile="fast",               # optional: fast / balanced / eval / debug
    rollout_steps=None,           # set int to receive EnvPoolTrajectoryBuffers
    layout="i16_legal_ids",       # mask / nomask / i16 / i16_legal_ids
)
```

`make_pool(...)` returns:

- `(pool, EnvPoolBuffers(...))` when `rollout_steps=None`
- `(pool, EnvPoolTrajectoryBuffers(...))` when `rollout_steps=<int>`

### Canonical buffer classes

- `EnvPoolBuffers(pool, layout=...)`
- `EnvPoolTrajectoryBuffers(pool, steps, layout=...)`

Common methods on `EnvPoolBuffers`:

- `reset()`, `reset_indices(...)`, `reset_done(...)`
- `step(actions)`
- `step_select_from_logits(logits)`
- `step_sample_from_logits(logits, seeds)`
- `legal_action_ids()` / `legal_action_ids_and_sample_uniform(seeds)`

`EnvPoolTrajectoryBuffers` methods:

- `rollout_first_legal()`
- `rollout_random_legal(seeds)`

### Canonical RL helper functions

- `reset_rl(pool, layout=..., out=None)`
- `step_rl(pool, actions, layout=..., out=None)`
- `step_rl_select_from_logits(pool, logits, layout=..., actions=None, out=None)`
- `step_rl_sample_from_logits(pool, logits, seeds, layout=..., actions=None, out=None)`

### Layouts

| layout | masks | legal ids | obs dtype |
| --- | --- | --- | --- |
| `mask` | yes | no | i32 |
| `nomask` | no | no | i32 |
| `i16` | yes | no | i16 |
| `i16_legal_ids` | no | yes | i16 |

### Legacy cleanup status

The public low-level API is intentionally consolidated around `make_pool`, the two canonical buffer classes, and the four canonical RL helpers.

## Card/catalog helpers

- `weiss_sim.cards.search(query, limit=20)`
- `weiss_sim.cards.get(identifier)`
- `weiss_sim.cards.presets()`
- `weiss_sim.cards.resolve_deck(...)`
- `weiss_sim.db_info(db_path=None)`

## League/population helpers

- `round_robin_schedule(...)`
- `sample_population_schedule(...)`
- `records_from_step(...)`
- `summarize_records(...)`
- `summarize_first_player_bias(...)`
- `summarize_clock_greed_from_replay(...)`
- `rank_agents(...)`

## Key exported constants/helpers

- `OBS_LEN`
- `ACTION_SPACE_SIZE`
- `PASS_ACTION_ID`
- `SPEC_HASH`
- `POLICY_VERSION`
- `observation_spec_json()`
- `action_spec_json()`
- `spec_bundle()` / `export_spec_bundle()`

## Error types

Important exceptions:

- `ConfigConflictError`
- `DbMismatchError`
- `DeckSpecError`
- `DeckValidationError`
- `CardLookupError`
- `WeissSimError`
