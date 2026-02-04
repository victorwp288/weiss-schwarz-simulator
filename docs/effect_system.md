# Unified Effect System (Mostly Single Path)

This document describes the unified effect representation and resolution pipeline implemented in `weiss_core`. It is **not** an official rules reference.

## Core Types

- **EffectId**: stable identifier for an effect definition (`EffectSourceKind`, `source_card`, `ability_index`, `effect_index`).
- **EffectSpec**: pure data description of an effect.
- **EffectPayload**: an `EffectSpec` plus resolved targets.
- **StackItem**: carries `effect_id` + `EffectPayload` when the effect is placed on the stack.

### EffectSourceKind

`EffectSourceKind` indicates the origin of the effect:

- `Trigger`, `Auto`, `Activated`, `Continuous`, `EventPlay`, `Counter`, `Replacement`, `System`

### EffectKind (current)

- `Draw { count }`
- `Damage { amount, cancelable, damage_type }`
- `AddModifier { kind, magnitude, duration }`
- `MoveToHand`
- `MoveToWaitingRoom`
- `MoveToStock`
- `MoveToClock`
- `Heal`
- `RestTarget`
- `StandTarget`
- `StockCharge { count }`
- `MillTop { target, count }`
- `MoveStageSlot { slot }`
- `SwapStageSlots`
- `RandomDiscardFromHand { target, count }`
- `RandomMill { target, count }`
- `MoveTriggerCardToHand`
- `ChangeController { new_controller }`
- `Standby { target_slot }`
- `TreasureStock { take_stock }`
- `ModifyPendingAttackDamage { delta }`
- `RevealDeckTop { count, audience }`
- `RevealZoneTop { target, zone, count, audience }`
- `TriggerIcon { icon }` (parsed/diagnostic marker that is compiled into concrete `EffectSpec` entries, e.g., Soul → `ModifyPendingAttackDamage`, Treasure/Standby → choice/targeting effects)
- `CounterBackup { power }`
- `CounterDamageReduce { amount }`
- `CounterDamageCancel`

## Resolution Pipeline

Most effects flow through the same pipeline, with a few explicit exceptions noted below.

1. **EffectSpec creation**
   - Trigger icons, abilities, events, counters, and system effects compile into `EffectSpec`.
   - Counter card movement and continuous modifier application are still handled directly.

2. **Targeting (if required)**
   - If `EffectKind::expects_target()` and a `TargetSpec` is present, the engine creates a `TargetSelectionState`.
   - Target selection produces deterministic **snapshotted** candidate lists and a `Choice` decision.
   - Once selection is complete, the `EffectPayload` is created and enqueued.
   - `MoveToHand` currently accepts targets from **Stage**, **WaitingRoom**, and **DeckTop** (top‑N limited via `TargetSpec.limit`).
   - `TargetSpec.source_only` (“this”) bypasses target selection and binds directly to the source card.

3. **Replacement / prevention layer**
   - Before applying `Damage`, the engine gathers `ReplacementSpec` entries with `ReplacementHook::Damage`.
   - Replacements are applied deterministically by `(priority, insertion, source)`.

4. **Stack + priority (or auto-resolve)**
   - `EffectPayload`s are pushed onto the stack.
   - With priority windows enabled, stack resolution follows the priority model.
   - Priority actions (activated abilities, counters) are selected via `Choice` decisions and enqueue stack items.
   - With priority windows disabled, the engine auto-resolves stack items in the main loop.

5. **Apply handler**
   - `resolve_effect_payload` is the apply path for queued effects.
   - Continuous modifiers are recomputed deterministically when state changes (stage moves, stand/rest, phase transitions).

6. **Replay events + visibility masking**
   - All effect-driven state changes emit canonical `Event` records (serialized to replays).
   - Hidden information is masked at the serialization boundary when visibility policies are enabled in public mode.
   - Choice option snapshots are numeric only (`option_id` + `ChoiceOptionRef`); no labels are built in the hot path.
   - Choice decisions are paged (page size = 16); page changes emit `ChoicePageChanged`.

7. **State-based checks + terminal resolution**
   - Loss conditions and end-of-turn cleanup run after effect resolution at defined checkpoints.

## Stack Integration

- Most effects are placed on the stack via `enqueue_effect_spec` or `enqueue_effect_with_targets`.
- Exceptions: continuous modifiers resolve immediately; counter card movement and refresh reshuffles are direct zone operations.
- When priority windows are disabled, the stack is drained by an auto-resolve loop with a hard cap:
  - `STACK_AUTO_RESOLVE_CAP` bounds auto-resolution steps.
  - Exceeding the cap emits `AutoResolveCapExceeded` and triggers deterministic engine error handling.

## Ability Compilation & Ordering

- Abilities are compiled at DB load time into `EffectSpec` lists.
- Canonical ability ordering is computed in `CardDb::build_index` by sorting the combined ability list (both `abilities` and `ability_defs`) by a stable key:
  - `AbilityTemplateTag` (enum discriminant order in `weiss_core/src/db.rs`)
  - then an explicit per-variant field key (no serialization-based ordering)
- All ability indexing (action encoding, replays, caches) uses `iter_card_abilities_in_canonical_order`.

## Trigger Icons

- All trigger icons compile into `EffectSpec` through `compile_trigger_icon_effects`.
- `Soul` uses `ModifyPendingAttackDamage` to preserve local timing.
- `Treasure` and `Standby` are represented as `EffectSpec` + choice/targeting.
- Refresh penalty is applied as a direct zone move immediately after the refresh shuffle (no damage intent).
- Trigger queueing is deterministic: trigger groups are sorted by a stable key and emit
  `TriggerQueued` followed by a single `TriggerGrouped` event per group.

## Determinism Rules

- No hash-map iteration or pointer ordering is used for externally observable ordering.
- Simultaneous stack items are ordered deterministically by `(source_id, effect_kind, stack_id)`.
- Effect instance ids are deterministic via `next_effect_instance_id`.
- Target candidate enumeration and replacement application are sorted by explicit stable keys.
