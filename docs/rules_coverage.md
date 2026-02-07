# Rules Coverage & Local Policy (v2.10)

**TL;DR**
- This document maps official rules to current engine behavior.
- The engine is deterministic but includes explicit local policy choices.
- If code and docs disagree, update docs to match code.

[Overview](README.md) | [Quickstart](quickstart.md) | [Engine](engine_architecture.md) | [RL Contract](rl_contract.md) | [Encodings](encodings.md) | [Performance](performance_benchmarks.md) | [Replays](replays_determinism.md) | Rules | [Invariants](invariants_validation.md) | [Contributing](contributing.md)

---

## On this page

- Coverage matrix
- Local policy notes

---

## Coverage matrix (v2.10)

Legend:
- **Implemented**: matches v2.10 behavior.
- **Implemented (local policy)**: implemented in code but intentionally deviates from v2.10.
- **Local policy (docs-only)**: intentional deviation documented below that may not be fully implemented yet.
- **Partial**: subset implemented or edge cases missing.
- **Not implemented**: missing.

### Section 1 — Overview of the Game

- **1.1 Number of players (2 only)**: Implemented.
  - `weiss_core::state` (two-player state layout)
- **1.2.1 Win/lose immediately on loss**: Implemented (terminal set on loss). `weiss_core::env::phases`.
- **1.2.2.1 Level 4 loss**: Implemented. `weiss_core::env::phases`.
- **1.2.2.2 Deck + waiting room empty loss**: Implemented.
  - `weiss_core::env::phases` (`resolve_rule_actions_until_stable`)
  - `weiss_core::env::movement` (`refresh_deck`)
- **1.2.3 Simultaneous loss => draw**: **Local policy** (configurable via `EndConditionPolicy`).
  - `weiss_core::env::phases` (`resolve_pending_losses`)
- **1.2.4 Concede**: Implemented (gated by `allow_concede`, immediate terminal, no check timing).
- **1.2.5 Win/lose by effect**: **Not implemented** (no effect kind yet).

### Section 5 — Setting Up the Game

- **Deck size (50)**: Implemented (`MAX_DECK`). `weiss_core::encode`.
- **Shuffle and draw starting hand**: Implemented. `weiss_core::env`.
- **Mulligan**: Implemented. `weiss_core::legal`, `weiss_core::env`.

### Section 6 — Game Procedure (Phases)

- **Phase order (Stand → Draw → Clock → Main → Climax → Attack → End)**: Implemented.
  - `weiss_core::env`
- **No attacks on starting player's first turn**: Implemented.
  - `weiss_core::legal`

### Section 7 — Attack and Battle

- **Attack declaration rules**: Implemented (slot constraints, frontal/side/direct legality).
  - `weiss_core::legal`, `weiss_core::env::movement`
- **Trigger check timing**: Implemented (uses `Resolution` zone).
  - `weiss_core::env::phases`
- **Counter window**: Implemented (frontal only, once per attack).
  - `weiss_core::env::interaction`
- **Damage cancellation (climax)**: Implemented.
  - `weiss_core::env::phases` (`resolve_effect_damage`)
- **Battle resolution / power comparison**: Implemented.
  - `weiss_core::env::phases`
- **Encore step**: Implemented (if enabled).
  - `weiss_core::env`
- **Keywords beyond basic counters/encore**: **Partial** (see Section 10).

### Section 8 — Play and Resolve Cards and Abilities

- **Play character/event/climax with cost/level/color**: Implemented.
  - `weiss_core::env::movement`
- **Play event/card goes to resolution**: Implemented.
  - `weiss_core::env::movement`
- **Activated abilities**: Implemented (if enabled).
  - `weiss_core::env::interaction`
- **Paid activated abilities**: **Implemented (local policy)**.
  - `weiss_core::db` (AbilityDef `cost` and paid templates)
  - `weiss_core::env::interaction` (cost gating + ordered pay)
- **Auto abilities at check timing**: Implemented.
  - `weiss_core::env::phases` (`run_check_timing`)
- **Continuous modifiers**: **Partial** (recomputed deterministically; conditional text not fully modeled).
  - `weiss_core::env::modifiers`

### Section 9 — Rule Actions

- **Check timing**: Implemented (explicit checkpoints).
  - `weiss_core::env::phases`
- **Loss checks at rule action**: **Partial** (see 1.2.2.2).

### Section 10 — Keywords and Keyword Abilities

- **Encore (basic)**: Implemented (if enabled).
- **Backup / Counter**: Implemented (if enabled).
- **Trigger icons (Soul/Draw/Shot/Bounce/Treasure/Gate/Standby)**: **Local policy** (see below).
- **Simple search/salvage/reveal templates**: **Implemented (local policy)**.
- **AbilityDef-driven effects (P0 set)**: **Implemented (local policy)**.
- **Other keyword abilities**: **Not implemented** (requires card text ingestion).

### Section 11 — Miscellaneous

- **Concession**: Implemented (see 1.2.4).
- **Effects that override rules**: Not implemented beyond current AbilityTemplate coverage.

---

## Local policy notes (authoritative)

### Core turn ordering (implemented)

- Mulligan → Stand → Draw → Clock → Main → Climax → Attack → End
- Attack pipeline order:
  - Attack declaration
  - Trigger check
  - Counter window (if allowed)
  - Damage resolution
  - Battle resolution
  - On-reverse auto triggers (if any) are queued immediately after reversal
  - Encore handling
- Starting player cannot declare attacks on their first turn.

### Implemented loss / terminal conditions

Default behavior:
- Deck + waiting room empty loss at rule action.
- Level 4 loss.
- Concession (if enabled) ends the episode immediately.
- Timeout if `max_ticks` is reached.

### Simultaneous loss policy

When alternate end conditions are enabled, losses are resolved by `EndConditionPolicy`:
- `Draw` (default)
- `ActivePlayerWins`
- `NonActivePlayerWins`

### Config / policy flags

Key flags that gate behavior:
- `enable_clock_phase`, `enable_climax_phase`, `enable_side_attacks`, `enable_direct_attacks`
- `enable_counters`, `enable_triggers` (and per-trigger toggles)
- `enable_backup`, `enable_encore`, `enable_refresh_penalty`
- `enable_activated_abilities`, `enable_continuous_modifiers`
- `enable_priority_windows`, `enable_visibility_policies`, `use_alternate_end_conditions`
- `priority_autopick_single_action`, `priority_allow_pass`, `strict_priority_mode`
- `reduced_stage_mode`, `allowed_card_sets`, `enforce_color_requirement`, `enforce_cost_requirement`
- `allow_concede`, `memory_is_public`

### Ability costs (local policy)

Activated abilities are legal only if all costs are payable up front. Costs are paid in this order:
1. Rest self
2. Rest other character(s)
3. Pay stock
4. Discard from hand
5. Clock from hand
6. Clock from deck top
7. Reveal from hand

### Trigger semantics (local policy)

Fixed choices:
- **Draw** is mandatory (no “may”).
- **Shot** resolves as immediate 1 damage (cancelable).
- **Bounce/Return** targets the controller’s own stage.

### Refresh + penalty (local policy)

- Refresh is bulk: waiting room → deck, then shuffle.
- Refresh penalty (if enabled) is immediate: reveal top card, move to clock, then level-up check.
- During damage resolution, if a climax is already in resolution zone, refresh loss is skipped.

### Hidden info policy (local)

When `observation_visibility` is `Public` and `enable_visibility_policies` is true:
- Hidden-zone ids and instance ids are masked in public outputs.
- Replays strip instance ids even for public zones.
- Target ordering is deterministic by slot index.
- Reveals are tracked per viewer and invalidated on hidden-zone shuffle.

### Targeting & randomness

- Target candidates are snapshotted at choice creation.
- Random selection is uniform and deterministic under seed.
- Opponent hidden zones are not direct choice targets.

---

## Related

- [Engine architecture](engine_architecture.md)
- [RL contract](rl_contract.md)
