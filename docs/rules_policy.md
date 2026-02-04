# Local Rules & Policy Spec

This document describes the **local** rules implemented by `weiss_core` and their ordering. It is **not** an official Weiss Schwarz rules text.

## Core Turn Ordering (Implemented)

- Mulligan → Stand → Draw → Clock → Main → Climax → Attack → End
- Attack pipeline order:
  - Attack declaration
  - Trigger check
  - Counter window (if allowed)
  - Damage resolution
  - Battle resolution
  - On‑reverse auto triggers (if any) are queued immediately after reversal
  - Encore handling
- Starting player cannot declare attacks on their first turn.

## Implemented Loss/Terminal Conditions

Default (current behavior):

- Deck+waiting-room empty: if a player has **zero cards in both deck and waiting room** at rule action, they lose.
- Level loss: a player at level 4 loses.
- Concession: when `allow_concede=true` (disabled by default), a player may concede at any time and immediately loses (no check timing).
- Timeout: when `max_ticks` reached.

## Deck Legality (Implemented)

- Max 8 climax cards per deck.
- Max 4 copies of any card id per deck.

Optional policies (config-gated):

- `use_alternate_end_conditions` enables simultaneous-loss handling (see below).

## Simultaneous Loss Policy

When `use_alternate_end_conditions` is enabled, losses are tracked in `turn.pending_losses` and resolved by `EndConditionPolicy`:

- `simultaneous_loss`:
  - `Draw` (default)
  - `ActivePlayerWins`
  - `NonActivePlayerWins`
- `allow_draw_on_simultaneous_loss` (default true)

If draw is disallowed and `Draw` is selected, the engine falls back to **ActivePlayerWins**.

## Config / Policy Flags

These flags gate behavior for training curricula and optional systems:

- **Core phase/attack toggles**
  - `enable_clock_phase` (default true)
  - `enable_climax_phase` (default true)
  - `enable_side_attacks` (default true)
  - `enable_direct_attacks` (default true)
  - `enable_counters` (default true)
  - `enable_triggers` (default true)
  - `enable_trigger_soul/draw/shot/bounce/treasure/gate/standby` (default true)
  - `enable_on_reverse_triggers` (default true): enables auto abilities that trigger when a character is reversed.
  - `enable_backup` (default true)
  - `enable_encore` (default true)
  - `enable_refresh_penalty` (default true)
  - `enable_level_up_choice` (default true)
  - `enable_activated_abilities` (default true)
  - `enable_continuous_modifiers` (default true)

- **Optional systems**
  - `enable_priority_windows` (default false): enables additional timing windows beyond Main/Counter.
  - `enable_visibility_policies` (default false): masks hidden info in choice events/replays and action logs under public visibility.
  - `use_alternate_end_conditions` (default false): uses the simultaneous loss policy described above.

- **Training / policy knobs**
  - `priority_autopick_single_action` (default true): auto-executes the only priority action.
  - `priority_allow_pass` (default true): pass is always available in priority windows.
  - `strict_priority_mode` (default false): disallow pass when actions exist (legacy/diagnostic mode).
  - `reduced_stage_mode` (default false): stage size reduced to 1 slot.
  - `allowed_card_sets` (default empty): optional whitelist (cached internally).
  - `enforce_color_requirement` (default true)
  - `enforce_cost_requirement` (default true)
  - `allow_concede` (default false): include concede in the legal action mask.
  - `memory_is_public` (default true): treat Memory as public for replay/choice masking.

Observation visibility is controlled by `observation_visibility` in `EnvConfig` (`Public` | `Full`). Public observations are always masked; visibility policies only affect replay/action sanitization.

## Ability Costs (Local Policy)

Activated abilities are legal **only if all costs are payable up front**. Costs are then paid in a fixed canonical order:

1. Rest self
2. Rest other character(s)
3. Pay stock
4. Discard from hand
5. Clock from hand
6. Clock from deck top
7. Reveal from hand

If any cost step cannot be paid when it is required, the action is illegal and cannot be selected.

## Notes on Rule Uncertainty

When official rule ordering is unclear, the engine documents a local policy choice instead of silently changing behavior. If ambiguity is discovered, consult the local rules PDF (`WSE-Comprehensive-Rules-v2.10.pdf`) and record the decision.

## On‑Reverse Trigger Caution (Local)

Auto abilities that trigger when a character is reversed can **increase decision frequency**. For RL throughput, the engine currently limits on‑reverse templates to low‑branching effects (e.g., draw/salvage). If decision counts spike, consider batching auto‑resolves or keeping on‑reverse effects simple.

## Refresh + Penalty (Local Event Semantics)

- Refresh is a bulk operation: waiting room → deck, then a shuffle. The event stream emits `Refresh` plus the `Shuffle` boundary (no per-card `ZoneMove` by default).
- Refresh does not resolve during cost payment.
- **Deviation vs v2.10:** the engine applies a simplified exception during damage resolution: if the player is in a damage process and a climax is already in their resolution zone, refresh loss is skipped; otherwise a loss is registered immediately when a refresh would occur. Full 9.2.2.1 handling is not implemented.
- Refresh penalty (when enabled) is applied immediately after refresh: reveal the new top card, move it to clock, then emit `RefreshPenalty`. This is a direct zone move (not a damage intent) and is followed by a level-up check.
- Local visibility policy: clock is public, so refresh penalty reveals use `RevealAudience::Public`. Public replay masking still applies based on visibility policies.

## Resolution Zone (Local)

- Trigger checks, damage checks, counters, and events place cards into the `Resolution` zone while resolving.
- Resolution cards are public; reveal events are emitted before any public disclosure.

## Hidden Info Policy (Local)

When `observation_visibility` is `Public` and `enable_visibility_policies` is true:

- Public outputs (observations, replays, action logs, render output) are sanitized by viewer context at the serialization boundary.
- Hidden-zone card ids, indices, and instance ids are masked in public outputs; revealed instances may surface CardId but not instance id.
- Public replays strip instance ids even for public zones (stage), retaining only stable public identifiers (card id + slot/index).
- Targeting order is deterministic by slot index (front row 0..2, back row 3..4). Opponent stage targeting follows the same ordering and never exposes hidden identifiers.
- Masked choice option identifiers avoid stable handles (use choice id + global index).
- Reveal tracking is per viewer and per instance; reveals are invalidated when a card re-enters a hidden zone or when a hidden zone is shuffled.
- Observation encoding appends a reveal history buffer (card ids only, no instance ids) for the observing player.

When `enable_visibility_policies` is false, replay/action masking is disabled but observation encoding still honors the configured visibility mode.

Visibility is driven by a centralized zone table (`weiss_core::visibility_policy`), which is used by event emission, replay sanitization, and observation encoding.

## Card Database Strictness (Local)

Unsupported ability templates are rejected at card‑db load time. This prevents silent “no‑op” abilities in training data. Use explicit `AbilityDef` entries or supported templates only.

## Continuous Modifiers (Local)

Continuous modifiers are recomputed deterministically whenever relevant state changes occur (stage movement, stand/rest, phase transitions, and other zone moves). Continuous modifier contributions are derived from active continuous abilities on stage and applied as modifier instances; this keeps derived power and attack legality consistent even as state changes.

## Rule Action Quiescence (Local)

Rule actions are enforced whenever the state changes in ways that could invalidate legality:
- after zone moves (including effects)
- after phase transitions
- before presenting a decision to a player

If a quiescence cap is exceeded, the engine emits an error status and terminates the episode with a timeout (truncated), rather than panicking.

## Fixed Trigger Semantics (Engine Policy)

The following trigger icon behaviors are **fixed local policy** and not curriculum flags:

- **Draw** is mandatory (no “may”).
- **Shot** resolves as immediate 1 damage (cancelable), not delayed on cancel.
- **Bounce/Return** targets the controller’s own stage, not the opponent’s.

## Deck-Top Search / Reveal (Local Policy)

- Deck-top search effects are modeled as **top‑N reveal to the controller**, then choose one to move to hand.
- The remaining cards stay in deck order (no shuffle).

## Target Snapshotting (Local Policy)

- Target candidates are **snapshotted when the choice is created**.
- The candidate list does not change if underlying zones change before the choice is resolved.
- Snapshotting is used to preserve determinism and stable choice ordering.

## Targeting & Filters (Local Policy)

- Targeting supports **Self or Opponent stage** selection; stage is public so this does not introduce hidden info.
- Slot filters include **FrontRow**, **BackRow**, and **SpecificSlot**.
- Target filters include **trait**, **level_max**, and **cost_max**. Filters must not leak hidden identities under public visibility.
- AbilityDef targets can specify **top‑N deck limits** via `target_limit` (DeckTop only).
- `TargetTemplate::This` binds to the **source card on stage** (source-only). If the source is missing, the effect is skipped.

## Random Selection & Reveal (Local Policy)

- **RandomDiscardFromHand** selects uniformly from the target hand and discards to waiting room.
- **RandomMill** mills from the **top of deck** (deck order is already randomized by shuffle).
- **RevealZoneTop** reveals the top‑N or first‑N cards of a zone in a deterministic order:
  - DeckTop: top of deck.
  - Other zones: index‑ascending order in that zone.
- Opponent hidden zones are **not** direct choice targets; use random selection or explicit reveal effects instead.
