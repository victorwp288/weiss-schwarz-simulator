# Architecture

`weiss-schwarz-simulator` is a deterministic Rust simulator with a PyO3 extension
and a Python API for batched RL workloads. The important design boundary is the
decision boundary: reset and step advance until the next legal action must be chosen.

## Layer Model

```text
python/weiss_sim public API
  -> Python buffer/layout helpers
    -> weiss_py PyO3 extension
      -> weiss_core::pool batched runtime
        -> weiss_core::env single-game runtime
          -> state/db/effects/legal/encode/replay/fingerprint
```

## Rust Core

- `weiss_core/src/state/`: game-state data, cards, stage, turn, attacks, choices,
  targets, stack, modifiers, players, and reveal history.
- `weiss_core/src/db/`: card database schemas, WSDB serialization
  (`WSDB_SCHEMA_VERSION=2`), ability models, and effect compilation.
- `weiss_core/src/effects.rs` and `weiss_core/src/effects/`: effect identifiers,
  replacement specs, payloads, and executable effect kinds.
- `weiss_core/src/legal/`: action descriptors, legal action id generation, attack
  legality, hand-play checks, and shared phase helpers.
- `weiss_core/src/env/`: single-game runtime, phase progression, movement, priority,
  visibility, choices, combat, trigger resolution, and fault handling.
- `weiss_core/src/pool/`: batched reset/step surfaces, rollout helpers, output
  writers, buffer validation, legal-id/logit fast paths, and pool-level fault recovery.
- `weiss_core/src/encode/`: fixed RL observation/action contracts.
- `weiss_core/src/replay.rs`: replay bundles (`REPLAY_SCHEMA_VERSION=3`) and
  visibility-safe replay output.
- `weiss_core/src/fingerprint.rs`: deterministic drift-debugging fingerprints.

## Python Boundary

- `weiss_py/` exposes the Rust pool as the compiled `weiss_sim` extension.
- `weiss_py/src/lib_parts/batch_types/` contains Python-visible batch output classes
  grouped by minimal, trajectory, debug, and dimension validation concerns.
- `weiss_py/src/lib_parts/env_pool/*.rs` contains small generated `EnvPool` method
  fragments for reset, step, logits, legal sampling, rollout, debug, and config APIs.
- `python/weiss_sim/api.py` provides `make`, `fast`, and `inspect`.
- `python/weiss_sim/runner.py` materializes high-level `ResetBatch` and `StepBatch`.
- `python/weiss_sim/_buffers.py` exposes reusable low-level buffers for hot loops.
- `python/weiss_sim/rl.py` exposes lightweight functional helpers for integration code.

Keep binding code explicit. Abstractions are welcome when they remove repeated NumPy view
validation or error mapping, but not when they hide output shape, ownership, or layout rules.

## Rules And Card Data

The scraper/parser pipeline is intentionally deterministic:

- `scraper/convert.py`: card conversion CLI/facade and card-field normalization.
- `scraper/ability_common.py`: shared parser constants/helpers.
- `scraper/ability_cost.py`: cost parsing.
- `scraper/ability_rules.py`: small rule tables.
- `scraper/convert_abilities.py`: high-coverage ability parser.
- `scraper/out/`: large generated conversion artifacts. Treat these as versioned data;
  do not regenerate them casually without recording the source command and expected diff.

Rules coverage is approximate by design in places where full Weiss Schwarz card text
support would be larger than the research simulator needs. Unsupported or approximated
ability families should be deterministic, documented in code/tests, and covered by the
coverage-budget scripts.

## Compatibility Boundaries

Public compatibility means:

- serialized field names and enum variants remain stable
- action ids, observation layout, and legal-id ordering remain stable
- replay and fingerprint behavior remains deterministic
- Python class names, method names, dtypes, and output shapes remain stable

Current compatibility constants:

| Boundary | Value |
| --- | ---: |
| `OBS_LEN` | 378 |
| `ACTION_SPACE_SIZE` | 527 |
| `OBS_ENCODING_VERSION` | 2 |
| `ACTION_ENCODING_VERSION` | 1 |
| `POLICY_VERSION` | 2 |
| `REPLAY_SCHEMA_VERSION` | 3 |
| `WSDB_SCHEMA_VERSION` | 2 |
| `SPEC_HASH` | 8590000130 |

If one of these semantics changes, update code, tests, [RL Contract](rl_contract.md),
and release notes together.

## Determinism And Fault Handling

Reproducibility depends on fixed seed/config/decks/action sequence and matching
compatibility constants. Replay payloads include seeds, config/spec hashes, schema
versions, env/episode ids, and visibility policy metadata.

Faults are isolated per environment in `EnvPool`. A faulted env latches
`engine_status != 0`, emits a truncated row, and can be reset without aborting the rest
of the batch.

## Extension Rules

- Prefer behavior-preserving refactors with characterization tests.
- Keep imports stable through `pub use` when moving Rust modules.
- Add or update Python stubs when a PyO3 method/class/constant changes.
- Update docs in the same PR when public behavior or compatibility changes.
- Run the docs, Rust, Python, scraper, and perf gates listed in [Contributing](../CONTRIBUTING.md).
