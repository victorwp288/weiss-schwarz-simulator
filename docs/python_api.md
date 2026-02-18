# Python API Guide

This page is a practical reference for the Python-facing simulator API (`weiss_sim`).

Use this guide when integrating training loops, choosing buffer variants, or handling runtime errors.

## Core module

```python
import weiss_sim
```

## High-level API (v1)

Recommended entrypoints:

- `weiss_sim.create(...)`
- `weiss_sim.train(...)`
- `weiss_sim.evaluate(...)`

These return a `SimRunner` with:

- `reset() -> ResetBatch`
- `step(actions) -> StepBatch`
- `effective_config() -> dict`
- `spec() -> dict`
- context manager support

`ResetBatch` / `StepBatch` always include:

- `obs`, `to_play_seat`, `starting_seat`
- `episode_seed`, `episode_index`, `env_index`, `episode_key`
- `decision_id`, `engine_status`, `spec_hash`

`StepBatch` additionally includes:

- `reward`, `terminated`, `truncated`
- `terminal_during_internal_opponent`
- `decision_count`, `tick_count`

Legal action representation is controlled by `legal_repr`:

- `"ids_u16"`, `"ids_u32"`, `"mask_u8"`, `"both"`

DB/catalog compatibility helpers:

- `weiss_sim.db_info(...)`
- `weiss_sim.cards.search(...)`
- `weiss_sim.cards.get(...)`
- `weiss_sim.cards.resolve_deck(...)`

Key exported constants:

- `OBS_LEN`
- `ACTION_SPACE_SIZE`
- `PASS_ACTION_ID`
- `SPEC_HASH`
- `POLICY_VERSION`

Key exported spec helpers:

- `observation_spec_json()`
- `action_spec_json()`
- `spec_bundle()`

### `create()` surface

```python
weiss_sim.create(
    *,
    deck=None,
    opponent_deck=None,
    db_path=None,
    rules_profile="approx",
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

Defaults by `runtime_mode`:

| runtime_mode | legal_repr | obs_dtype | ids_safety |
| --- | --- | --- | --- |
| `speed` | `ids_u16` | `i16` | `checked` |
| `eval_debug` | `both` | `i32` | n/a (not used unless `legal_repr="ids_u16"`) |

Override/conflict rules:

- explicit args override mode defaults
- `rules_profile="strict"` + `curriculum.enable_approx_effects=True` raises `ConfigConflictError`
- `ids_safety` is only valid with `legal_repr="ids_u16"`; otherwise `ConfigConflictError`
- `observation_visibility` defaults to `public` (hidden opponent private zones); use `full` only for debug/eval workflows where information leakage is acceptable
- with `observation_visibility="public"`, `memory_is_public` defaults to `False` unless explicitly overridden in `curriculum`
- `reveal_opponent_hand_stock_counts` defaults to hidden (`False`) and can be set to `True` as a top-level override without editing the full curriculum payload
- `end_condition_policy` accepts dict/JSON and can override simultaneous-loss handling (`draw`, `active_player_wins`, `non_active_player_wins`) and `allow_draw_on_simultaneous_loss` (default `True`)

Auto sizing is deterministic:

- `num_threads="auto"` -> `min(16, cpu_count)`
- if `num_envs` is explicit, resolved threads are capped at `num_envs`
- `num_envs="auto"` -> `min(128, max(32, 4 * resolved_threads))`

### Reset/step contract

`ResetBatch` required fields:

- `obs` (`(N, OBS_LEN)`, `int16` or `int32`)
- `to_play_seat` (`(N,)`, `int8`, values in `{0, 1, -1}`)
- `starting_seat` (`(N,)`, `uint8`, values in `{0, 1}`)
- `episode_seed` (`(N,)`, `uint64`)
- `episode_index` (`(N,)`, `uint32`)
- `env_index` (`(N,)`, `uint32`)
- `episode_key` (`(N,)`, `uint64`)
- `decision_id` (`(N,)`, `uint32`)
- `engine_status` (`(N,)`, `uint8`)
- `spec_hash` (`(N,)`, `uint64`)
- plus `legal_mask` and/or (`legal_ids`, `legal_offsets`) based on `legal_repr`

`episode_key` formula:

```text
episode_key = mix64(episode_seed ^ mix64((episode_index << 32) ^ env_index))
```

where `mix64` is fixed SplitMix64-style mixing (stable in `python/weiss_sim/runner.py`).

`StepBatch` adds:

- `reward` (`(N,)`, `float32`)
- `terminated` (`(N,)`, `bool`)
- `truncated` (`(N,)`, `bool`)
- `terminal_during_internal_opponent` (`(N,)`, `bool`)
- `decision_count` (`(N,)`, `uint32`)
- `tick_count` (`(N,)`, `uint32`)

Seat-control helpers on `SimRunner`:

- `current_to_play_seat()` returns latest actor-seat vector
- `merge_actions_by_seat(seat0_actions, seat1_actions, default_action=...)` merges two action vectors using current actor seat
- `step_by_seat(seat0_actions, seat1_actions, default_action=...)` convenience wrapper for merged stepping

Termination/truncation taxonomy:

- `terminated=True` only for win/loss/draw outcomes
- `truncated=True` only for timeout/limit/fault truncation
- both are never `True` at the same index
- timeout truncation uses terminal draw reward semantics (`0.0` under zero-sum validation)

Legal-id ordering contract:

- legal ids are strictly ascending and unique per env slice
- `eval_debug`: check every reset/step (fail fast)
- `speed`: deterministic spot-check every 4096 batch steps (plus reset)
- no automatic sorting is performed

### Deck and card resolution

`DeckInput` accepted forms:

- `Sequence[int]`
- `Mapping[int | str, int]`
- preset string
- file path (`str`/`Path`)

String resolution is deterministic:

- `preset:<name>` -> preset
- `file:<path>` -> file
- otherwise, strings with `/`, `\`, or `.json` suffix -> file
- all other strings -> preset name

Deck validation behavior:

- high-level prevalidation enforces total card count `50` and known card identifiers
- engine remains source of truth for full deck legality (copy count, climax limits, etc.)
- with `card_pool="parsed_only"`, profile-specific support gate is enforced before pool creation

Card helpers:

- `weiss_sim.cards.search(query, limit=20)`
- `weiss_sim.cards.get(identifier)`
- `weiss_sim.cards.presets()`
- `weiss_sim.cards.resolve_deck(...)`

### DB/catalog compatibility

Packaged data:

- `python/weiss_sim/data/card_catalog.json.gz`
- `python/weiss_sim/data/deck_presets.json`
- `python/weiss_sim/data/catalog_meta.json`

`card_pool="parsed_only"` requires DB hash match against `catalog_meta.json`.

- match -> parsed-only validation is allowed
- mismatch -> raises `DbMismatchError` (`expected_db_sha256`, `actual_db_sha256`, remediation text)

`card_pool="all"` does not block on mismatch; status is surfaced in `effective_config()["db"]`.

### Migration (low-level remains stable)

| Goal | New API | Existing low-level |
| --- | --- | --- |
| Zero-config train loop | `weiss_sim.train(...)` | `make_train_pool(...)` / `EnvPool.new_rl_train(...)` |
| Eval/debug visibility | `weiss_sim.evaluate(...)` | `make_eval_pool(...)` / `EnvPool.new_rl_eval(...)` |
| Deck resolution by preset/path/map | `deck=...` in `create()` | caller builds `deck_lists` manually |
| Parsed-only support gate | `card_pool="parsed_only"` | custom user-side checks |
| Stable typed batch outputs | `ResetBatch`, `StepBatch` | raw `BatchOut*` buffers |

Low-level constructors and buffer APIs are unchanged for backward compatibility.

### League / population helpers

Use the `weiss_sim.league` helpers to avoid writing scheduling/aggregation boilerplate:

- `round_robin_schedule(agent_ids, double_round=True)`
- `sample_population_schedule(agent_ids, num_matches, seed=0, allow_mirror=False)`
- `records_from_step(step_batch, seat0_agents=..., seat1_agents=...)`
- `summarize_records(records)`
- `summarize_first_player_bias(records)`
- `summarize_clock_greed_from_replay(replay_data, actor=None, draw_window_events=8)`
- `rank_agents(summary)`

`sample_population_schedule(...)` uses an internal deterministic RNG path keyed only by `seed`, so schedule generation is reproducible across runs.

`records_from_step(...)` includes `starting_seat` in each `MatchRecord`, so first-player bias can be computed directly from batched evaluation logs with `summarize_first_player_bias(...)`.

`summarize_clock_greed_from_replay(...)` consumes a replay JSON payload (for example output from `replay_dump`) and reports:
- optional clock decision rate (`Clock` vs `Pass` on `DecisionKind::Clock`)
- clock events followed by draw events
- self-effect damage intents/commits followed by draws

This helper only needs replay metadata/actions/events and does not require private observation tensors.

## Creating environment pools

Primary constructors on `EnvPool`:

- `EnvPool.new_rl_train(...)`
- `EnvPool.new_rl_eval(...)`
- `EnvPool.new_debug(...)`

Common constructor parameters:

- `num_envs`
- `db_path` (optional override; defaults to bundled package DB when omitted/`None`)
- `deck_lists`
- `deck_ids`
- `max_decisions`
- `max_ticks`
- `seed`
- `curriculum_json`
- `reward_json`
- `end_condition_policy_json`
- `error_policy`
- `observation_visibility` (`"public"` default; `"full"` optional override)
- `num_threads`

`new_rl_train` and `new_rl_eval` default to public observation visibility and support `output_masks` toggling. Public visibility means hidden opponent zones (hand/deck/stock) are masked in observations; pass `observation_visibility="full"` only for debugging/evaluation workflows where hidden info leakage is acceptable.

Threading behavior:

- `new_rl_train` / `new_rl_eval` with `num_threads=None` now auto-select thread count from CPU parallelism (capped by `num_envs`).
- pass `num_threads=1` to force serial execution.
- `EnvPool.num_threads` exposes the effective runtime thread count.

## Buffer classes and when to use them

- `EnvPoolBuffers`: standard i32 observations + masks
- `EnvPoolBuffersNoMask`: i32 observations, no dense masks
- `EnvPoolBuffersI16`: i16 observations + masks
- `EnvPoolBuffersI16LegalIds`: i16 observations + packed legal-id outputs

Trajectory variants exist for rollout collection:

- `EnvPoolTrajectoryBuffers`
- `EnvPoolTrajectoryBuffersNoMask`
- `EnvPoolTrajectoryBuffersI16`
- `EnvPoolTrajectoryBuffersI16LegalIds`

## Minimal stepping pattern

```python
import numpy as np
import weiss_sim

pool = weiss_sim.EnvPool.new_rl_train(...)
buf = weiss_sim.EnvPoolBuffers(pool)
out = buf.reset()

actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
out = buf.step(actions)
```

## Legal action surfaces

Two common ways to pick valid actions:

1. Dense masks (`out.masks`) from mask-enabled buffers.
2. Packed legal ids via `buf.legal_action_ids()`:

```python
ids, offsets = buf.legal_action_ids()
```

Use `offsets[i]:offsets[i+1]` to slice env `i` legal ids.

## Logit-based action selection helpers

Buffer wrappers provide helpers that choose/sample legal actions in Rust:

- `step_select_from_logits(logits)`
- `step_sample_from_logits(logits, seeds)`
- `select_actions_from_logits(logits)`
- `sample_actions_from_logits(logits, seeds)`

These can reduce Python-side legal-action plumbing in policy loops.

## Reset helpers

Useful reset paths:

- `reset()` all envs
- `reset_indices(indices)` subset reset
- `reset_done(done_mask)` reset only done envs
- `reset_indices_with_episode_seeds(indices, episode_seeds)` deterministic seeded reset

## Runtime metadata helpers

Pool-level batch metadata methods:

- `episode_seed_batch()`
- `episode_index_batch()`
- `env_index_batch()`
- `starting_player_batch()`

Use these for reproducibility logging and replay indexing.

## Engine error handling

Runtime stepping/reset is batch-stable: isolated env faults are surfaced in outputs and do not raise Python exceptions in pool mode.

Per-env output fields:

- `engine_status` (`uint8`): stable engine code (`0` means no fault)
- `truncated` / `terminated`: fault rows are `truncated=True`, `terminated=False`
- `actor`: fault rows keep actor when known (no sentinel overwrite)

Derived/computed signals:

- `engine_error = (out.engine_status != 0)` (there is no `out.engine_error` array field)
- reset recommendation uses the same condition: `(out.engine_status != 0)`

Recommended robust pattern:

```python
engine_error = out.engine_status != 0
if engine_error.any():
    pool.auto_reset_on_error_codes_into(out.engine_status, buf.out)
```

No-mask variant:

- `auto_reset_on_error_codes_into_nomask(...)`

Also available:

- `engine_error_reset_count()`
- `reset_engine_error_reset_count()`

Note: the Python extension requires `panic=unwind` so per-env panic containment can trap unwinds safely.

## Replay sampling controls

Enable replay capture from Python via pool methods (debug/eval workflows):

- `enable_replay_sampling(...)`

For replay semantics and determinism workflow, see [Replays & determinism](replays_determinism.md).

## Helper factory functions

`python/weiss_sim/__init__.py` includes convenience constructors:

- `make_train_pool(...)`
- `make_eval_pool(...)`

Profiles (`fast`, `balanced`, `eval`, `debug`) tune mask/i16/legal-id defaults.

## Integration recommendations

1. Start with `EnvPoolBuffers` for correctness visibility.
2. Move to `EnvPoolBuffersI16LegalIds` for large-scale throughput.
3. Persist `SPEC_HASH` with model artifacts.
4. Keep logs for seed, decision ids, and non-zero engine statuses.

## Related

- [Quickstart](quickstart.md)
- [RL contract](rl_contract.md)
- [Encodings](encodings.md)
- [Troubleshooting](troubleshooting.md)
