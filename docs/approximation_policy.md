# Approximation Policy

**Purpose**
- Track every non-exact rule implementation that is intentionally approximated for RL practicality.
- Keep approximations deterministic and auditable.

[Overview](README.md) | [Quickstart](quickstart.md) | [Engine](engine_architecture.md) | [RL Contract](rl_contract.md) | [Encodings](encodings.md) | [Performance](performance_benchmarks.md) | [Replays](replays_determinism.md) | [Rules](rules_coverage.md) | [Invariants](invariants_validation.md) | [Contributing](contributing.md)

---

## Gate

Exact implementation is required when the effect impacts:
- combat timing
- damage/cancel resolution
- trigger timing
- action legality
- hidden information guarantees

Approximation is only allowed for non-combat utility effects when:
- behavior is deterministic
- it is documented in this file
- the curriculum flag `enable_approx_effects` is enabled

Default:
- `enable_approx_effects = false`

---

## Template (required for each approximation)

For each approximated signature, document:
1. Signature
2. Exact rule reference
3. Implemented approximation
4. Why it is safe for RL objectives
5. Exit criteria for exact implementation

---

## Current approximations

### 1) `Activated.Brainstorm.CustomAction.ApproxDraw`

1. Signature  
`【ACT】 Brainstorm ... For each climax revealed, perform the following action. "<custom action>"`
2. Exact rule reference  
Resolve the custom per-climax action text exactly.
3. Implemented approximation  
Map custom action to deterministic `Brainstorm` draw mode with `per_climax=1`.
4. Why it is safe for RL objectives  
Non-combat utility substitution; no changes to battle timing/cancel/trigger legality.
5. Exit criteria for exact implementation  
Parse and compile custom action payload into exact effect sequences.

### 2) `Continuous.FollowingAbility.*.ApproxNoop`

1. Signature  
`【CONT】 All of your opponent's characters get "<Q>".`  
`【CONT】 All of your characters get the following ability. "<Q>"`  
`【CONT】 All of your other characters get the following ability. "<Q>"`  
and unresolved conditional variants that grant a following ability.
2. Exact rule reference  
Grant nested continuous/auto abilities to broad target sets.
3. Implemented approximation  
Emit gated no-op effect (`Draw 0`) with `requires_approx_effects=true`.
4. Why it is safe for RL objectives  
Deterministic and timing-safe; avoids introducing illegal timing/combat behavior while making card text parseable.
5. Exit criteria for exact implementation  
Compile nested granted abilities into concrete effect templates and target scopes.

### 3) `Auto.FollowingAbility*.ApproxNoop`

1. Signature  
`【AUTO】 ... choose # character(s) ... gets the following ability ... "<Q>"`  
including end-of-opponent-next-turn variants, multi-target variants, and generic
fallbacks by inferred timing (`OnPlay` / `AttackDeclaration` / `OnReverse` / etc.).
2. Exact rule reference  
Resolve nested granted ability text with correct duration and target cardinality.
3. Implemented approximation  
Emit gated no-op auto ability (`Draw 0`) at parsed/inferred timing, preserving parsed cost when present.
4. Why it is safe for RL objectives  
Deterministic non-combat placeholder; does not alter legality, hidden information, or cancel/timing pipelines.
5. Exit criteria for exact implementation  
Support nested ability compilation with duration windows beyond `UntilEndOfTurn`.

### 4) `Auto.SearchSalvage.Generic.OnPlay*.ApproxNoop`

1. Signature  
On-play search/salvage utility text that remains unresolved after exact handlers, for example:
`search your deck ... put it into your hand ... shuffle` /  
`look at up to X cards ... choose up to # ... put it into your hand` /  
`choose # in your waiting room, and return it to your hand`.
2. Exact rule reference  
Resolve selector constraints and follow-up operations exactly (deck/clock/waiting-room interactions).
3. Implemented approximation  
Emit gated no-op `Draw 0` on `OnPlay`, preserving parsed cost for paid forms.
4. Why it is safe for RL objectives  
Deterministic non-combat placeholder; does not alter combat timing, damage/cancel, legality, or hidden-info policy.
5. Exit criteria for exact implementation  
Unify search/salvage combinator parsing to exact selector + movement templates.

### 5) `Activated.Brainstorm.*.Approx*` (custom utility variants)

1. Signature  
Unresolved brainstorm variants such as:
- per-climax deck search to hand,
- trigger-icon-qualified salvage,
- per-climax team power utility text.
2. Exact rule reference  
Resolve each per-climax branch exactly with full selector constraints.
3. Implemented approximation  
Map to deterministic gated forms (`Brainstorm` with normalized mode where possible, otherwise `Draw 0`).
4. Why it is safe for RL objectives  
Non-combat utility approximation with deterministic output and no timing-legality side effects.
5. Exit criteria for exact implementation  
Compile each brainstorm branch into exact effect sequences and constraints.

### 6) `Continuous.AllCharactersTraitPower.Approx`

1. Signature  
`【CONT】 If all of your characters are 《TraitA》 (or 《TraitB》 ...), this card gets +N power.`
2. Exact rule reference  
Continuously evaluate full-stage trait-uniformity predicates.
3. Implemented approximation  
Emit gated deterministic `AddPower` placeholder under approx gating.
4. Why it is safe for RL objectives  
Static stat-only approximation; avoids introducing timing/combat/hidden-info regressions.
5. Exit criteria for exact implementation  
Add exact condition primitives for all-characters trait-uniformity checks.

### 7) `Auto.UseThisCardAbility.ApproxNoop`

1. Signature  
`【AUTO】 When you use this card's "<Q>", ...`
2. Exact rule reference  
Trigger on usage of a named source ability and apply the described follow-up effect.
3. Implemented approximation  
Emit gated no-op auto ability (`Draw 0`) with parsed cost preserved.
4. Why it is safe for RL objectives  
Deterministic placeholder for a trigger timing class not yet modeled explicitly.
5. Exit criteria for exact implementation  
Add engine event/timing support for “use this card’s named ability” triggers.

### 8) `Auto.OnPlayAllPlayersAction.ApproxNoop`

1. Signature  
`【AUTO】 When this card is placed on the stage from your hand, all players perform the following action. "<Q>"`
2. Exact rule reference  
Apply nested action text to both players at on-play timing.
3. Implemented approximation  
Emit gated no-op on-play effect (`Draw 0`).
4. Why it is safe for RL objectives  
Deterministic and symmetric placeholder with no hidden-info side effects.
5. Exit criteria for exact implementation  
Compile the nested all-players action text into explicit engine effects.

---

Implementation notes:

- Approximation signatures are emitted by the parser-v2 rule-pack converter and serialized into WSDB v2 artifacts.
- These approximations are emitted under converter profile `--approx-profile approx`.
- Legacy alias `--approx-profile rl_v1` remains accepted and maps to `approx`.
- Emitted abilities are marked with `conditions.requires_approx_effects=true`.
- Emitted abilities may include optional provenance `conditions.source_rule_id` (alias `sourceRuleId`) for source-rule traceability.
- Recent strict exactifications (February 15, 2026):
  - named/dual-trait selector constraints now compile into exact `target_card_ids` filters for search/salvage handlers,
  - additional quoted-grant payloads compile exactly (including several battle-opponent reverse branches),
  - granted Encore variants are now surfaced to encore cost resolution through live abilities.
- Runtime ignores these abilities unless `CurriculumConfig.enable_approx_effects=true`.
