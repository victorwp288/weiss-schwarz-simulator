# Rules Coverage & Local Policy

This page describes implemented rule areas and intentional simulator-local behavior.

For machine-checkable integration behavior, see [RL Contract](rl_contract.md).

## Coverage summary by subsystem

| Area | Status | Notes |
| --- | --- | --- |
| Core game flow (mulligan -> stand/draw/clock/main/climax/attack/end) | Implemented | driven by advance loop + phase modules |
| Attack declaration, trigger checks, counter window, damage/cancel, battle, encore | Implemented | strict ordering in phase/interaction logic |
| Rule-action loss checks (level/deck+waiting room) | Implemented | enforced in rule-action/lifecycle paths |
| Simultaneous-loss resolution policy | Implemented | configurable via `EndConditionPolicy` |
| Activated abilities / cost payment / target choices | Implemented (broad) | parser/template coverage still expanding |
| Continuous modifiers / replacements | Implemented (broad) | deterministic recompute + explicit ordering |
| Card-text ingestion for all effects | Partial | parser/template coverage is ongoing |
| Win/lose-by-effect (rule 1.2.5 style direct terminal effects) | Partial/pending | tracked in [Project State](../PROJECT_STATE.md) |

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

- [Engine Architecture](engine_architecture.md)
- [Approximation Policy](approximation_policy.md)
- [Project State](../PROJECT_STATE.md)
