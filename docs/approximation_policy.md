# Approximation Policy

This page tracks intentional deterministic approximations used for parser/runtime practicality.

Default runtime behavior keeps approximations disabled.

## Gate rules

Approximation is allowed only when all are true:

1. effect is non-combat utility or similarly low-risk for RL objectives
2. behavior is deterministic
3. approximation is documented here
4. runtime opt-in is explicit (`enable_approx_effects=true`)

Approximation is not allowed for core timing/legality primitives without explicit design review.

## Runtime gate

- curriculum flag: `enable_approx_effects`
- default: `false`
- approx-only emitted abilities include `conditions.requires_approx_effects=true`

## Documentation template for new approximations

For each approximation, record:

1. signature
2. exact intended behavior
3. implemented deterministic approximation
4. RL-safety rationale
5. exit criteria for exact implementation

## Current approximation families

Representative families currently emitted by parser-v2/rule-pack conversion under approx profiles:

- brainstorm custom-action fallbacks (`Activated.Brainstorm.*.Approx*`)
- nested granted-ability placeholders (`Continuous.FollowingAbility*`, `Auto.FollowingAbility*`)
- unresolved on-play utility fallbacks (`Auto.SearchSalvage.Generic.OnPlay*.ApproxNoop`)
- all-players nested action placeholders (`Auto.OnPlayAllPlayersAction.ApproxNoop`)
- use-this-card-ability trigger placeholders (`Auto.UseThisCardAbility.ApproxNoop`)

Most placeholder forms resolve to deterministic no-op-compatible effects (`Draw 0`) with preserved timing/cost metadata where available.

## Converter/runtime notes

- emitted by parser-v2 rule-pack converter in approx profile mode
- profile aliases: `approx` (`rl_v1` alias)
- strict profiles (`strict` / `none`) avoid emitting approx-only effects
- optional provenance field may be emitted as `conditions.source_rule_id`

## Exit strategy

Approximation entries should be removed only when exact behavior ships with:

1. deterministic ordering and bounded execution
2. tests covering correctness + determinism
3. updated docs in:
   - [Rules Coverage](rules_coverage.md)
   - [Project State](../PROJECT_STATE.md)

## Related

- [Rules Coverage](rules_coverage.md)
- [Engine Architecture](engine_architecture.md)
- [Project State](../PROJECT_STATE.md)
