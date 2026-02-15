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
- Large high-churn modules were split for maintainability:
  - `weiss_core/src/env/interaction/effects/{mod,core,resolve,conditions}.rs`
  - `weiss_core/src/db/ability/{mod,models,keys,compile}.rs`
  - `weiss_core/src/env/tests/engine/{mod,targeting_and_stack,core_effects,triggers_and_conditions,reward_and_conditionals,movement_and_reveal,modifiers_and_followups}.rs`

## Determinism and Ordering Guarantees

These rules are contract-sensitive and must remain stable unless intentionally versioned:

- Public ordering never depends on hash-map iteration.
- Ability indexing must use `CardDb::iter_card_abilities_in_canonical_order`.
- Canonical ability ordering is generated at DB load by sorting `abilities + ability_defs` by `(AbilityTemplateTag, per-variant key)` in `weiss_core/src/db/store.rs`.
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
- `WSDB_SCHEMA_VERSION = 2`

Policy:

- Treat these as compatibility boundaries.
- Any breaking contract shift requires coordinated updates across code, tests, and docs.
- WSDB loader behavior is strict; non-v2 DB files must be regenerated with the parser-v2/rule-pack pipeline.
- Migration path for legacy WSDB v1 files is explicit regeneration (no in-place upgrader):
  run parser-v2/rule-pack conversion to JSON and repack via `carddb_pack` to emit WSDB v2 artifacts.

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

## Coverage Tooling

- WSDB build inputs are parser-v2 rule packs; conversion output is versioned as WSDB v2.
- Ability conversion supports approximation profiles:
  - `--approx-profile strict` (strict/default; legacy alias: `none`)
  - `--approx-profile approx` (gated approximation emission; legacy alias: `rl_v1`)
- Coverage reporting scripts:
  - `scripts/ability_coverage_report.py` emits machine-readable profile comparisons.
  - `scripts/check_coverage_budget.py` enforces non-regression against
    `scripts/ability_coverage_baseline.json`.
- Approx-only ability defs are marked with `conditions.requires_approx_effects=true` and are ignored at runtime unless `CurriculumConfig.enable_approx_effects=true`.
- Ability defs may carry optional provenance at `conditions.source_rule_id` (alias `sourceRuleId`) to trace parser-v2 rule-pack origin.
- Ability defs support optional `target_card_ids` selector narrowing for exact named/dual-trait search/salvage selectors.
- Latest coverage snapshot (`2026-02-15`, `scripts/ability_coverage_report.py`):
  - Parse-line coverage:
    - `strict` (alias: `none`): `51.61%` (`15,314 / 29,675`)
    - `approx` (alias: `rl_v1`): `99.77%` (`29,607 / 29,675`)
  - Card-level all-lines-supported coverage:
    - `strict` (alias: `none`): `35.03%` (`6,038 / 17,235`)
    - `approx` (alias: `rl_v1`): `99.61%` (`17,167 / 17,235`)
  - Family clusters (`strict` vs `approx`):
    - `AssistOrScalingPower`: `59.24%` (`6,417 / 10,832`) vs `99.98%` (`10,830 / 10,832`)
    - `FollowingAbilityGrant`: `13.15%` (`205 / 1,559`) vs `100.00%` (`1,559 / 1,559`)
    - `PaidOnPlaySearchSalvage`: `59.12%` (`833 / 1,409`) vs `99.93%` (`1,408 / 1,409`)

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
