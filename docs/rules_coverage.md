# Rules Coverage Matrix (v2.10)

This document maps the official rules text (`weiss_rules_v2_10_llm.md`) to **current engine behavior**.
Code is the source of truth. If code and docs disagree, update the docs.

Legend:
- **Implemented**: matches v2.10 behavior.
- **Implemented (local policy)**: implemented in code but intentionally deviates from v2.10; see `docs/rules_policy.md`.
- **Local policy**: intentional deviation; documented in `docs/rules_policy.md`.
- **Partial**: subset implemented or edge cases missing.
- **Not implemented**: missing.

## Section 1 — Overview of the Game

- **1.1 Number of players (2 only)**: Implemented.
  - `weiss_core/src/state.rs` (two-player state layout)
- **1.2.1 Win/lose immediately on loss**: Implemented (terminal set on loss). `weiss_core/src/env/phases.rs`.
- **1.2.2.1 Level 4 loss**: Implemented. `weiss_core/src/env/phases.rs`.
- **1.2.2.2 Deck + waiting room empty loss**: Implemented.
  - `weiss_core/src/env/phases.rs` (`resolve_rule_actions_until_stable`)
  - `weiss_core/src/env/movement.rs` (`refresh_deck`)
- **1.2.3 Simultaneous loss => draw**: **Local policy** (configurable via `EndConditionPolicy`).
  - `weiss_core/src/env/phases.rs` (`resolve_pending_losses`)
- **1.2.4 Concede**: Implemented (gated by `allow_concede`, immediate terminal, no check timing).
- **1.2.5 Win/lose by effect**: **Not implemented** (no effect kind yet).

## Section 5 — Setting Up the Game

- **Deck size (50)**: Implemented (`MAX_DECK`). `weiss_core/src/encode.rs`.
- **Shuffle and draw starting hand**: Implemented. `weiss_core/src/env.rs`.
- **Mulligan**: Implemented. `weiss_core/src/legal.rs`, `weiss_core/src/env.rs`.

## Section 6 — Game Procedure (Phases)

- **Phase order (Stand → Draw → Clock → Main → Climax → Attack → End)**: Implemented.
  - `weiss_core/src/env.rs`
- **No attacks on starting player's first turn**: Implemented.
  - `weiss_core/src/legal.rs`

## Section 7 — Attack and Battle

- **Attack declaration rules**: Implemented (slot constraints, frontal/side/direct legality).
  - `weiss_core/src/legal.rs`, `weiss_core/src/env/movement.rs`
- **Trigger check timing**: Implemented (uses `Resolution` zone).
  - `weiss_core/src/env/phases.rs`
- **Counter window**: Implemented (frontal only, once per attack).
  - `weiss_core/src/env/interaction.rs`
- **Damage cancellation (climax)**: Implemented.
  - `weiss_core/src/env/phases.rs` (`resolve_effect_damage`)
- **Battle resolution / power comparison**: Implemented.
  - `weiss_core/src/env/phases.rs`
- **Encore step**: Implemented (if enabled).
  - `weiss_core/src/env.rs`
- **Keywords beyond basic counters/encore**: **Partial** (see Section 10).

## Section 8 — Play and Resolve Cards and Abilities

- **Play character/event/climax with cost/level/color**: Implemented.
  - `weiss_core/src/env/movement.rs`
- **Play event/card goes to resolution**: Implemented.
  - `weiss_core/src/env/movement.rs`
- **Activated abilities**: Implemented (if enabled).
  - `weiss_core/src/env/interaction.rs`
- **Paid activated abilities**: **Implemented (local policy)**.
  - `weiss_core/src/db.rs` (AbilityDef `cost` and paid templates)
  - `weiss_core/src/env/interaction.rs` (cost gating + ordered pay)
- **Auto abilities at check timing**: Implemented.
  - `weiss_core/src/env/phases.rs` (`run_check_timing`)
- **Continuous modifiers**: **Partial** (recomputed deterministically; conditional text not fully modeled).
  - `weiss_core/src/env/modifiers.rs`

## Section 9 — Rule Actions

- **Check timing**: Implemented (explicit checkpoints).
  - `weiss_core/src/env/phases.rs`
- **Loss checks at rule action**: **Partial** (see 1.2.2.2).

## Section 10 — Keywords and Keyword Abilities

- **Encore (basic)**: Implemented (if enabled).
- **Backup / Counter**: Implemented (if enabled).
- **Trigger icons (Soul/Draw/Shot/Bounce/Treasure/Gate/Standby)**: **Local policy** (see `docs/rules_policy.md`).
  - `weiss_core/src/env/phases.rs` (`resolve_trigger_step`, `compile_trigger_icon_effects`)
  - Known deviations vs v2.10:
    - **Optionality**: “may” is modeled for Gate/Bounce/Standby via an explicit skip option; Draw remains mandatory.
    - **Draw**: implemented as mandatory `Draw { count: 1 }` (v2.10: may draw).
    - **Return/Bounce**: targets the controller’s own stage (`TargetSide::SelfSide`) (v2.10: may choose an opponent’s character on stage).
    - **Shot**: implemented as immediate 1 damage (v2.10: delayed “when next damage is canceled, deal 1”).
    - **Treasure**: effect is modeled as “move trigger card to hand” + optional stock; trigger card remains in resolution until moved to hand.
- **Simple search/salvage/reveal templates**: **Implemented (local policy)**.
  - `AutoOnPlaySalvage` (waiting room → hand)
  - `AutoOnPlaySearchDeckTop` (top‑N search to hand, controller reveal)
  - `AutoOnPlayRevealDeckTop` (top‑N reveal to controller)
  - `AutoOnPlayStockCharge` (top‑N deck → stock)
  - `AutoOnPlayMillTop` (top‑N deck → waiting room)
  - `AutoOnPlayHeal` (clock → waiting room)
  - `AutoOnReverseDraw` (draw on being reversed)
  - `AutoOnReverseSalvage` (waiting room → hand on being reversed)
- **AbilityDef‑driven effects (P0 set)**: **Implemented (local policy)**.
  - Effect verbs: MoveToWaitingRoom / MoveToStock / MoveToClock / Heal / RestTarget / StandTarget / StockCharge / MillTop / MoveStageSlot / SwapStageSlots / RandomDiscardFromHand / RandomMill / RevealZoneTop
  - Targeting: Opponent stage targeting, BackRow and SpecificSlot filters, `This` (source‑only)
  - Filters: trait, level_max, cost_max
  - `weiss_core/src/db.rs` (AbilityDef, EffectTemplate, TargetTemplate)
  - `weiss_core/src/env/interaction.rs` (target enumeration + resolution)
- **Other keyword abilities**: **Not implemented** (requires card text ingestion).

## Section 11 — Miscellaneous

- **Concession**: Implemented (see 1.2.4).
- **Effects that override rules**: Not implemented beyond current AbilityTemplate coverage.
