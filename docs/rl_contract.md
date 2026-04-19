# RL Contract

This is the integration contract for stepping semantics, output payloads, and compatibility checksums.

If this page and code disagree, code is authoritative and this page must be updated in the same PR.

## Contract scope

This contract covers:

- step/reset boundary semantics
- output fields and meanings
- legal action surfaces
- compatibility constants and `SPEC_HASH`

This contract does not describe full rules coverage. See [Rules Coverage](rules_coverage.md).

## Step semantics

One `step` call per env:

1. applies exactly one caller-selected action id
2. advances internal runtime until next decision or terminal/truncated boundary
3. emits one row for that boundary

```mermaid
flowchart LR
  A["reset"] --> B["decision boundary"]
  B --> C["choose legal action id"]
  C --> D["step"]
  D --> B
  D --> E["terminated or truncated"]
```

## Reward semantics

Reward values are reported per-step and are computed from the **acting player’s perspective** for that boundary.

Intuition:

- the engine identifies the current actor (`to_play_seat` / `actor`)
- the caller supplies an action id for that actor
- the returned reward is computed relative to that actor (the seat that took the action)

This alignment is intentional: it supports self-play style training/evaluation where a single policy can act for both seats while still receiving correctly-signed rewards for whichever seat acted.

Non-terminal behavior:

- if shaping rewards are disabled, non-terminal rewards are `0.0`
- if shaping rewards are enabled, the current shaping term is applied from the actor’s perspective

Terminal behavior:

- win/loss rewards use the configured `terminal_win` / `terminal_loss`
- draws use `terminal_draw`
- timeouts/truncations use `terminal_timeout`

Fault behavior (engine errors):

- faults latch `engine_status != 0` and emit `truncated=True`
- a latched fault emits a terminal-loss reward at most once (for the known actor); subsequent fault rows emit `0.0` until reset

Reward configuration defaults (`RewardConfig`):

- `terminal_win = 1.0`
- `terminal_loss = -1.0`
- `terminal_draw = 0.0`
- `terminal_timeout = 0.0`
- `enable_shaping = false`
- `damage_reward = 0.1`

`reward_json` schema keys match `RewardConfig` field names:

- `terminal_win` (float)
- `terminal_loss` (float)
- `terminal_draw` (float)
- `terminal_timeout` (float)
- `enable_shaping` (bool)
- `damage_reward` (float)
- `level_reward` (float)
- `board_reward` (float)
- `no_progress_penalty` (float)

## Core output schema

Typical low-level outputs (`BatchOut*` / buffer wrappers):

| Field | Shape | Meaning |
| --- | --- | --- |
| `obs` | `(N, OBS_LEN)` | observation vector |
| `rewards` | `(N,)` | per-step reward |
| `terminated` | `(N,)` | terminal result reached |
| `truncated` | `(N,)` | limit/fault truncation |
| `actor` | `(N,)` | acting player for boundary (`-1` sentinel when none) |
| `decision_kind` | `(N,)` | encoded decision kind (`-1` sentinel when none) |
| `decision_id` | `(N,)` | per-env monotonic decision index |
| `engine_status` | `(N,)` | engine fault code (`0` healthy) |
| `spec_hash` | `(N,)` | compatibility hash |
| `main_move_action` | `(N,)` | whether the last transition consumed the once-per-turn main move |
| `main_pass_action` | `(N,)` | whether the last transition passed main |

Low-level legal action surfaces:

- dense mask (`masks`) when enabled
- packed ids (`legal_ids`, `legal_offsets`) depending on buffer/API mode
- aligned packed metadata (`legal_action_meta`) when legal ids are present

## High-level batch schema (`ResetBatch` / `StepBatch`)

Common fields:

- `obs`, `to_play_seat`, `starting_seat`
- `episode_seed`, `episode_index`, `env_index`, `episode_key`
- `decision_id`, `engine_status`, `spec_hash`
- `main_move_action`, `main_pass_action`
- optional legal surfaces: `legal_mask`, `legal_ids`, `legal_offsets`, `legal_action_meta`

Preferred high-level legal API:

- use `batch.legal` for ids/mask/logit helpers
- `legal_ids`, `legal_offsets`, and `legal_action_meta` remain available as raw properties for advanced integration

`StepBatch` adds:

- `reward`, `terminated`, `truncated`
- `terminal_during_internal_opponent`
- `decision_count`, `tick_count`, `no_progress_count`

Invariants enforced in high-level path:

- `terminated` and `truncated` are never both true at one index
- legal ids (when present) are strictly ascending and unique per env slice
- legal metadata (when present) is aligned 1:1 with the used `legal_ids` prefix

`batch.legal` behavior:

- `batch.legal.ids(i)` returns legal action ids for env `i`
- `batch.legal.meta(i)` returns packed action metadata rows for env `i`
- `batch.legal.mask` returns dense legal mask view
- `batch.legal.argmax_logits(...)` / `sample_logits(...)` enforce legality

## Decision kind encoding

From observation encoding:

| Value | DecisionKind |
| --- | --- |
| `0` | `Mulligan` |
| `1` | `Clock` |
| `2` | `Main` |
| `3` | `Climax` |
| `4` | `AttackDeclaration` |
| `5` | `LevelUp` |
| `6` | `Encore` |
| `7` | `TriggerOrder` |
| `8` | `Choice` |
| `-1` | none |

## Engine status encoding

| Value | Name |
| --- | --- |
| `0` | `None` |
| `1` | `StackAutoResolveCap` |
| `2` | `TriggerQuiescenceCap` |
| `3` | `Panic` |
| `4` | `ActionError` |
| `5` | `InvariantViolation` |
| `6` | `ResetError` |
| `7` | `ResetPanic` |

Fault-row behavior:

- fault rows are emitted with `truncated=True`
- runtime fault state is latched until reset
- batch stepping continues for other envs

## Determinism requirements

Reproducible runs require all of the following to match:

1. seed path (`seed` / derived episode seeds)
2. action sequence
3. config/curriculum/reward/end-condition settings
4. compatibility constants (`OBS/ACTION/POLICY`, replay/wsdb schema)

Useful metadata surfaces:

- `episode_seed_batch()`
- `episode_index_batch()`
- `env_index_batch()`
- `starting_player_batch()`

## Compatibility checksum table

These values are read from `weiss_core/src/encode/constants.rs`.

| Field | Value |
| --- | --- |
| OBS_LEN | 378 |
| ACTION_SPACE_SIZE | 527 |
| OBS_ENCODING_VERSION | 2 |
| ACTION_ENCODING_VERSION | 1 |
| SPEC_HASH | 8590000130 |

## Structured action metadata

Structured-policy integrations can rely on two exported helper blocks from
`weiss_sim.spec_bundle()` / `weiss_sim.export_spec_bundle()`:

- `action_factorization_v1`: stable action-family schema for `family` + `arg0/arg1/arg2`
- `action_meta_v1`: packed legal-row layout mirrored by `legal_action_meta`

## Reference loop patterns

Mask-based baseline:

```python
import numpy as np
import weiss_sim

pool, buf = weiss_sim.make_pool(
    mode="train",
    num_envs=64,
    deck_lists=[deck_a, deck_b],
    layout="mask",
)
out = buf.reset()

for _ in range(1000):
    actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
    for i in range(pool.envs_len):
        legal = np.flatnonzero(out.masks[i])
        if legal.size:
            actions[i] = int(legal[0])
    out = buf.step(actions)
```

RL helper baseline (same contract, no manual buffer wrapper):

```python
pool, _ = weiss_sim.make_pool(
    mode="train",
    num_envs=64,
    deck_lists=[deck_a, deck_b],
    layout="mask",
)
step = weiss_sim.reset_rl(pool, layout="mask")
for _ in range(1000):
    actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
    for i in range(pool.envs_len):
        legal = np.flatnonzero(step.masks[i])
        if legal.size:
            actions[i] = int(legal[0])
    step = weiss_sim.step_rl(pool, actions, layout="mask")
```

## Advanced: raw packed legality arrays

When you need packed legality payloads directly, use:

- `batch.legal_ids`
- `batch.legal_offsets`
- optional `batch.legal_mask`

Packed ids are authoritative for their selected representation and remain part of the stable contract, but higher-level integrations should prefer `batch.legal`.

Packed-id pattern:

```python
ids, offsets = buf.legal_action_ids()
actions = np.full(pool.envs_len, weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
for i in range(pool.envs_len):
    start = int(offsets[i])
    end = int(offsets[i + 1])
    actions[i] = weiss_sim.PASS_ACTION_ID if start == end else int(ids[start])
out = buf.step(actions)
```

## Related

- [Encodings](encodings.md)
- [Encodings Changelog](encodings_changelog.md)
- [Python API](python_api.md)
- [Replays & Determinism](replays_determinism.md)
