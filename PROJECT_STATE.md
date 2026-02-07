# Project State

This file is the implementation-facing source of truth for current simulator behavior and constraints.

Use it to answer: "what is implemented today, what is policy vs official rules, and what must remain stable?"

Machine-checked constants live in [docs/invariants_validation.md](docs/invariants_validation.md).

## Current Posture

- Deterministic, RL-first engine with advance-until-decision semantics.
- Fixed action space and fixed-length observation contract.
- Unified effect pipeline covers most trigger/ability/event flows.
- Priority windows are optional and default to disabled.
- Replay sanitization is active only when visibility policies are enabled in public mode.

## Determinism and Ordering Guarantees

These rules are contract-sensitive and must remain stable unless intentionally versioned:

- Public ordering never depends on hash-map iteration.
- Ability indexing must use `CardDb::iter_card_abilities_in_canonical_order`.
- Canonical ability ordering is generated at DB load by sorting `abilities + ability_defs` by `(AbilityTemplateTag, per-variant key)` in `weiss_core/src/db.rs`.
- Stage order: slot ascending.
- Hand order: index ascending.
- Deck top: index `0` semantic (last element in vector representation).
- Waiting room: stable list order.
- Stock top: last pushed.
- Priority action ordering: stage slot ascending, then ability index.
- Replacement/modifier application ordering uses explicit deterministic keys.

## Contract Versions and Schemas

Current values:

- `OBS_ENCODING_VERSION = 1`
- `ACTION_ENCODING_VERSION = 1`
- `REPLAY_SCHEMA_VERSION = 2`
- `WSDB_SCHEMA_VERSION = 1`

Policy:

- Treat these as compatibility boundaries.
- Any breaking contract shift requires coordinated updates across code, tests, and docs.

## Fingerprints and Drift Detection

- Fingerprint algorithm: `postcard+blake3+u64le v1`
- Config hash: canonical `EnvConfig + CurriculumConfig` snapshot (excluding caches/paths)
- Final state hash: canonical `GameState` snapshot (caches excluded, RNG state included)
- Determinism fingerprint: hash over canonical unsanitized event bytes

## Feature Gate Defaults (`CurriculumConfig`)

Enabled by default:

- `enable_clock_phase`, `enable_climax_phase`
- `enable_side_attacks`, `enable_direct_attacks`
- `enable_counters`, `enable_triggers`
- trigger icons: soul/draw/shot/bounce/treasure/gate/standby
- `enable_backup`, `enable_encore`, `enable_refresh_penalty`, `enable_level_up_choice`
- `enable_activated_abilities`, `enable_continuous_modifiers`
- `priority_autopick_single_action`, `priority_allow_pass`
- `enforce_color_requirement`, `enforce_cost_requirement`
- `memory_is_public`

Disabled by default:

- `enable_priority_windows`
- `enable_visibility_policies`
- `use_alternate_end_conditions`
- `strict_priority_mode`
- `reduced_stage_mode`
- `allow_concede`

## Effect Pipeline Coverage

Mostly centralized in `resolve_effect_payload`, with known direct-path exceptions:

- Counter card movement/cost handling still uses direct movement paths.
- Continuous modifiers apply immediately rather than stack items.
- Refresh/refresh-penalty zone transitions are direct operations with explicit events.

Supported trigger icons:

- Soul, Standby, Treasure, Gate, Bounce, Draw, Shot

Notes:

- `AutoEndPhaseDraw` is modeled as an auto ability, not a trigger icon.

## Priority Windows and Stack Behavior

- Priority windows are gated by `enable_priority_windows`.
- Priority actions are represented as `Choice` decisions.
- With priority windows disabled, stack auto-resolves with bounded loop protection.
- Exceeding `STACK_AUTO_RESOLVE_CAP` emits `AutoResolveCapExceeded` and invokes deterministic error handling.
- Choice paging is deterministic and non-truncating (`page size = 16`).

## Visibility Policy Behavior

Applies only when:

- `enable_visibility_policies = true`
- observation visibility mode is public for relevant outputs

Current behavior:

- Hidden-zone identifiers are masked at output boundaries.
- Replay sanitization is global/viewer-agnostic in public mode.
- Revealed hidden cards may expose `CardId` but not instance id.
- Reveal tracking is per-viewer/per-instance and invalidated by hidden-zone reentry or shuffle.

## Known Gaps / Partial Areas

- Card text ingestion and effect generation beyond current `AbilityTemplate`/`AbilityDef` coverage.
- Advanced replacement/prevention layering beyond current modifier/replacement support.
- Ownership transfer semantics beyond current control-change behavior.

## High-Risk Gotchas

1. Do not bump version constants without explicit migration intent.
2. Do not bypass canonical ability ordering helpers in encodings/replays/legal sets.
3. Do not create unbounded effect generation paths; auto-resolve cap is enforced.
4. Do not leak hidden-zone identity in public outputs.
5. Do not fork replay event schema; `events.rs:Event` is authoritative.

## Near-Term Work

- Expand `AbilityDef` coverage and structured card text parsing.
- Improve replacement/prevention modeling and document local-policy boundaries.
- Profile stack/priority/targeting hot paths under large batch settings.
- Continue tightening hidden-info tracking semantics at instance granularity.

## Maintenance Rules

When behavior changes:

1. update relevant docs (`docs/` + this file)
2. update tests that assert determinism/ordering
3. run docs checks:

```bash
python scripts/check_docs_links.py
python scripts/check_docs_constants.py
```
