# Python API Guide

This page is a practical reference for the Python-facing simulator API (`weiss_sim`).

Use this guide when integrating training loops, choosing buffer variants, or handling runtime errors.

## Core module

```python
import weiss_sim
```

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

## Creating environment pools

Primary constructors on `EnvPool`:

- `EnvPool.new_rl_train(...)`
- `EnvPool.new_rl_eval(...)`
- `EnvPool.new_debug(...)`

Common constructor parameters:

- `num_envs`
- `db_path`
- `deck_lists`
- `deck_ids`
- `max_decisions`
- `max_ticks`
- `seed`
- `curriculum_json`
- `reward_json`
- `error_policy`
- `num_threads`

`new_rl_train` and `new_rl_eval` default to public observation visibility and support `output_masks` toggling. Public visibility means hidden opponent zones (hand/deck/stock) are masked in observations; use full visibility only for debugging/evaluation workflows where hidden info leakage is acceptable.

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
