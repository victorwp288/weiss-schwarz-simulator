# Python API Guide

This page documents the current `weiss_sim` Python surface.

- High-level API: `create()`, `train()`, `evaluate()` -> returns `SimRunner`
- Low-level API: `EnvPool` + buffer classes for direct batched stepping

## Import

```python
import weiss_sim
```

## High-level API (`SimRunner`)

### Entry points

- `weiss_sim.create(...)`
- `weiss_sim.train(...)` (`runtime_mode="speed"` fixed)
- `weiss_sim.evaluate(...)` (`runtime_mode="eval_debug"` fixed)

### `create()` parameters (current)

```python
weiss_sim.create(
    deck=None,
    opponent_deck=None,
    db_path=None,
    rules_profile="strict",
    runtime_mode="speed",
    card_pool="parsed_only",
    curriculum=None,
    reward_json=None,
    end_condition_policy=None,
    observation_visibility="public",
    reveal_opponent_hand_stock_counts=None,
    legal_repr=None,
    obs_dtype=None,
    ids_safety=None,
    num_envs="auto",
    num_threads="auto",
    seed=0,
    max_decisions=2000,
    max_ticks=100_000,
    error_policy="lenient_terminate",
    control_seat=None,
)
```

### Runtime mode defaults

| runtime_mode | legal_repr | obs_dtype | ids_safety |
| --- | --- | --- | --- |
| `speed` | `ids_u16` | `i16` | `checked` |
| `eval_debug` | `both` | `i32` | n/a |

Rules enforced by `create()`:

- `rules_profile="strict"` + `curriculum.enable_approx_effects=True` -> `ConfigConflictError`
- `ids_safety` is only valid with `legal_repr="ids_u16"`
- `reward_json` accepts `dict`, JSON string, or `None`
- `end_condition_policy` accepts dict/dataclass/JSON and normalizes:
  - `draw`
  - `active_player_wins`
  - `non_active_player_wins`

### Deck input forms

Accepted for `deck` and `opponent_deck`:

- `Sequence[int]`
- `Mapping[int|str, int]`
- preset string (for example `"preset:starter_v1"`)
- path-like string / `Path`

`card_pool="parsed_only"` enforces packaged catalog compatibility and may raise `DbMismatchError`.

### `SimRunner` methods

- `reset() -> ResetBatch`
- `step(actions) -> StepBatch`
- `spec() -> dict`
- `effective_config() -> dict`
- `current_to_play_seat() -> np.ndarray`
- `merge_actions_by_seat(seat0_actions, seat1_actions, default_action=...)`
- `step_by_seat(seat0_actions, seat1_actions, default_action=...)`
- context manager support (`with weiss_sim.train(...) as sim:`)

### `ResetBatch` / `StepBatch` fields

Common fields:

- `obs`
- `to_play_seat`
- `starting_seat`
- `episode_seed`
- `episode_index`
- `env_index`
- `episode_key`
- `decision_id`
- `engine_status`
- `spec_hash`
- optional legal surfaces: `legal_mask`, `legal_ids`, `legal_offsets`

`StepBatch` adds:

- `reward`
- `terminated`
- `truncated`
- `terminal_during_internal_opponent`
- `decision_count`
- `tick_count`

Contract rules enforced by `SimRunner`:

- `terminated` and `truncated` cannot both be true at one env index
- legal-id arrays are checked:
  - every batch in `eval_debug`
  - every 4096 steps in `speed`

### `effective_config()` keys worth using

The returned dict includes:

- resolved runtime knobs (`runtime_mode`, `legal_repr`, `obs_dtype`, `ids_safety`)
- sizing (`num_envs`, `num_threads`)
- policy/config (`rules_profile`, `card_pool`, `curriculum`, `error_policy`, `end_condition_policy`)
- db compatibility (`db` object with hashes and match status)
- compatibility/runtime metadata:
  - `spec_hash`
  - `action_space`
  - `reward_timeout_policy`
  - `reveal_opponent_hand_stock_counts`
  - `needs_runtime_legal_ids`

### Minimal high-level example

```python
import numpy as np
import weiss_sim

with weiss_sim.evaluate(num_envs=8, seed=7, card_pool="all") as sim:
    reset = sim.reset()
    actions = np.full((8,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
    step = sim.step(actions)
    print(step.obs.shape, step.reward.dtype, step.engine_status[:4])
```

## Low-level API (`EnvPool`)

### Constructors

- `EnvPool.new_rl_train(...)`
- `EnvPool.new_rl_eval(...)`
- `EnvPool.new_debug(...)`

Behavior notes:

- RL constructors enforce public visibility policies and disable concede.
- `new_debug` is the no-override path for explicit control.

### Common constructor args

- `num_envs`
- `db_path` (optional; default uses bundled `.wsdb`)
- `deck_lists` (required, two decks)
- `deck_ids`
- `max_decisions`, `max_ticks`, `seed`
- `curriculum_json`, `reward_json`, `end_condition_policy_json`
- `error_policy`
- `observation_visibility`
- `num_threads`

### Buffer classes

- `EnvPoolBuffers`
- `EnvPoolBuffersNoMask`
- `EnvPoolBuffersI16`
- `EnvPoolBuffersI16LegalIds`
- rollout variants:
  - `EnvPoolTrajectoryBuffers`
  - `EnvPoolTrajectoryBuffersNoMask`
  - `EnvPoolTrajectoryBuffersI16`
  - `EnvPoolTrajectoryBuffersI16LegalIds`

Typical buffer methods:

- `reset()`, `reset_indices(...)`, `reset_done(...)`
- `step(actions)`
- `step_select_from_logits(logits)`
- `step_sample_from_logits(logits, seeds)`
- `legal_action_ids()` / `legal_action_ids_and_sample_uniform(seeds)`

### Pool-level helpers

- metadata batches:
  - `episode_seed_batch()`
  - `episode_index_batch()`
  - `env_index_batch()`
  - `starting_player_batch()`
- fault handling:
  - `auto_reset_on_error_codes_into(...)`
  - `auto_reset_on_error_codes_into_nomask(...)`
  - `engine_error_reset_count()`
- replay:
  - `enable_replay_sampling(sample_rate, out_dir=None, compress=False, include_trigger_card_id=False, visibility_mode=None, store_actions=True)`

### Low-level minimal example

```python
import numpy as np
import weiss_sim

deck = (list(range(1, 14)) * 4)[:50]
pool = weiss_sim.EnvPool.new_rl_train(16, deck_lists=[deck, deck], deck_ids=[0, 1])
buf = weiss_sim.EnvPoolBuffersI16LegalIds(pool)
out = buf.reset()
actions = np.full((pool.envs_len,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buf.step(actions)
```

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

## Related

- [Quickstart](quickstart.md)
- [RL Contract](rl_contract.md)
- [Encodings](encodings.md)
- [Troubleshooting](troubleshooting.md)
