# Glossary

This glossary defines contract-facing terms used throughout the documentation.

## Decision boundary

A point where the engine stops internal progression and requires a caller-selected action to continue. `reset()` returns at a decision boundary, and `step()` returns at the next one (or a terminal/truncated boundary).

## `to_play_seat` / `actor`

Which seat must act at the current boundary.

- High-level: `ResetBatch.to_play_seat`, `StepBatch.to_play_seat`
- Low-level: `BatchOut*.actor`

Values:

- `0` or `1`: seat id
- `-1`: no actor (for example, terminal rows)

## Action id

An integer in a fixed action space (`ACTION_SPACE_SIZE`) that fully identifies an action the caller can take at a boundary. The engine maps internal canonical actions into this id space.

## Legal actions

The set of action ids permitted at the current boundary.

The API exposes legality as:

- a dense mask (`legal_mask` / `masks`), and/or
- packed ids + offsets (`legal_ids`, `legal_offsets`)

## Legal mask

A dense `(N, ACTION_SPACE_SIZE)` array where non-zero entries indicate legal actions. This is the most convenient representation for masked-policy RL implementations.

## Packed legal ids / offsets

A compact representation where:

- `legal_ids` is a flat vector of ids across all envs
- `legal_offsets` is a `(N+1,)` vector of slice boundaries into `legal_ids`

This is typically the lowest-overhead path for high-throughput stepping and legality iteration.

## `PASS_ACTION_ID`

The action id for “pass”, used when an environment has no legal choices (for example, after it is done) or as a safe default action.

## `terminated` vs `truncated`

Episode end signals:

- `terminated=True`: a terminal game result was reached (win/loss/draw)
- `truncated=True`: the episode ended due to limits or faults (for example, max decisions/ticks or a latched engine fault)

They are mutually exclusive per env index.

## `engine_status`

Per-env engine fault code.

- `0` indicates healthy execution
- non-zero values indicate a fault condition; the fault is latched until reset

See [RL Contract](rl_contract.md) for the stable encoding table.

## `spec_hash`

A stable compatibility hash over the observation/action spec bundle. Persist this with checkpoints and fail fast on mismatches unless you have an explicit migration plan.

## Encoding version

An explicit compatibility boundary for the meaning/layout of an encoded payload:

- observation encoding: `OBS_ENCODING_VERSION`
- action encoding: `ACTION_ENCODING_VERSION`

Semantic/layout changes must bump the corresponding version and update the changelog.

## `episode_seed`, `episode_index`, `episode_key`

Determinism metadata for batched environments:

- `episode_seed`: the per-episode seed used by the engine
- `episode_index`: per-env episode counter
- `episode_key`: a stable derived identifier (useful for logging and dedup)

## `rules_profile`

Rules implementation profile:

- `strict`: avoid deterministic approximations
- `approx`: enable approved deterministic approximations for coverage/perf

See [Approximation Policy](approximation_policy.md).

## `card_pool`

Catalog/DB validation mode:

- `parsed_only`: require packaged-catalog compatibility (fail fast on DB mismatch)
- `all`: allow full DB usage even if the packaged catalog hash differs

## `error_policy`

How illegal actions and engine errors are handled:

- `raise`: raise an exception to the caller
- `replace`: convert errors into a no-op outcome (lenient)
- `terminate`: convert errors into a terminal loss for the acting player (lenient)

## Replay

A structured payload capturing enough information to reproduce or debug execution, typically including config/spec metadata and optional events/actions depending on visibility.

## Fingerprint

A stable hash over state and/or event streams used to detect drift and aid deterministic debugging.

## Related

- [How it works](how_it_works.md)
- [RL Contract](rl_contract.md)
- [Encodings](encodings.md)

