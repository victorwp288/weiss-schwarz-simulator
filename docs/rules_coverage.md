# Rules Coverage & Local Policy

This page shows what is implemented today, what is partial/pending, and where simulator-local policy differs from tabletop expectations.

For machine-checkable integration behavior, see [RL Contract](rl_contract.md).

## Coverage snapshot

| Area | Status | Notes |
| --- | --- | --- |
| Core game flow (`mulligan -> stand/draw/clock/main/climax/attack/end`) | Implemented | phase progression and decision boundaries are in the runtime loop |
| Attack declaration, trigger checks, counter window, damage/cancel, battle, encore | Implemented | deterministic ordering and sequencing |
| Rule-action loss checks (level/deck+waiting room) | Implemented | enforced as rule/lifecycle checks |
| Simultaneous-loss resolution policy | Implemented | configurable via `EndConditionPolicy` |
| Activated abilities / costs / target choices | Implemented (broad) | parser/template expansion continues to grow |
| Continuous modifiers / replacements | Implemented (broad) | deterministic recompute/order; advanced corner layering is still incremental |
| Card-text ingestion for all effects | Partial | not all card text patterns map to implemented templates yet |
| Direct win/lose-by-effect handling (rule 1.2.5 style) | Partial/pending | tracked as a known gap in [Project State](../PROJECT_STATE.md) |

## Implemented today (clear list)

The following are implemented in runtime behavior today:

- deterministic decision-boundary stepping (`advance_until_decision` model)
- full turn-phase cycle with mulligan and per-phase timing windows
- attack pipeline: declaration, trigger pipeline, counter window, damage/cancel, battle, encore
- level/deck loss checks and terminal handling
- simultaneous-loss resolution policy (`Draw`, `ActivePlayerWins`, `NonActivePlayerWins`)
- legal-action generation in fixed action-id space with mask/id views
- public visibility sanitization and replay visibility modes

Runtime and test surfaces covering these areas include:

- runtime modules under `weiss_core/src/env/advance/`, `weiss_core/src/env/phases/`, `weiss_core/src/env/interaction/`
- contract/behavior tests such as:
  - `weiss_core/tests/turn_cycle_tests.rs`
  - `weiss_core/tests/combat_basic_tests.rs`
  - `weiss_core/tests/combat_damage_tests.rs`
  - `weiss_core/tests/trigger_resolution_tests.rs`
  - `weiss_core/tests/priority_window_tests.rs`
  - `weiss_core/tests/rl_contract_tests.rs`

## Partial / not yet implemented (clear list)

These areas are not fully complete yet:

- full card-text effect coverage across the entire card corpus
- direct win/lose-by-effect handling in the rule 1.2.5 style
- advanced replacement/prevention corner layering remains incremental

These are tracked in [Project State](../PROJECT_STATE.md) and should be treated as active engineering areas, not silent behavior.

## Local policy choices

### End-condition policy

`EndConditionPolicy` supports:

- `Draw` (default)
- `ActivePlayerWins`
- `NonActivePlayerWins`

`allow_draw_on_simultaneous_loss` defaults to `true`.

### RL constructor safety overrides

`EnvPool.new_rl_train/new_rl_eval` enforce RL-safe defaults such as:

- public visibility mode
- visibility policies enabled
- concede disabled

### Visibility policy

In public mode:

- hidden-zone identities are masked in outputs
- replay public mode serializes sanitized actions/events
- hidden info sanitization happens at output/serialization boundaries

### Approximation gating

Approx-only abilities are tagged with `conditions.requires_approx_effects=true`.

Runtime ignores them unless `CurriculumConfig.enable_approx_effects=true`.

See [Approximation Policy](approximation_policy.md).

## Coverage tooling and source of truth

Use scripts, not stale prose numbers, for current coverage values:

```bash
python scripts/ability_coverage_report.py --output /tmp/ability_coverage_report.json
python scripts/ability_coverage_targets.py --report /tmp/ability_coverage_report.json --output /tmp/ability_coverage_targets.json
python scripts/check_coverage_budget.py \
  --report /tmp/ability_coverage_report.json \
  --baseline scripts/ability_coverage_baseline.json
```

Baseline file: `scripts/ability_coverage_baseline.json`.

Supported coverage profile tokens:

- `strict`
- `approx`

Legacy profile aliases (`none`, `rl_v1`) are removed.

## Guidance for contributors

When adding or changing rule behavior:

1. update tests first or in lock-step
2. update this page + [Project State](../PROJECT_STATE.md)
3. if encoding behavior changed, update:
   - [RL Contract](rl_contract.md)
   - [Encodings Changelog](encodings_changelog.md)

## Related

- [Beginner happy path](beginner_happy_path.md)
- [Engine Architecture](engine_architecture.md)
- [Approximation Policy](approximation_policy.md)
- [Project State](../PROJECT_STATE.md)
