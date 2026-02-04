# Timing Windows & Priority (Local Spec)

This document defines the timing windows and priority model implemented in `weiss_core`. It is **not** an official Weiss Schwarz rules reference. It describes the deterministic engine behavior as implemented here.

## Timing Windows

The engine recognizes the following windows (see `TimingWindow` enum):

- `MainWindow`
- `ClimaxWindow`
- `AttackDeclarationWindow`
- `TriggerResolutionWindow`
- `CounterWindow`
- `DamageResolutionWindow`
- `EncoreWindow`
- `EndPhaseWindow`

## RL Impact Notes (per window)

- **MainWindow**: adds a decision point only when `enable_priority_windows` is true; action set is activated abilities + pass.
- **ClimaxWindow**: adds a decision point only when `enable_priority_windows` is true; action set is pass only (no new actions).
- **AttackDeclarationWindow**: adds a decision point only when `enable_priority_windows` is true; action set is pass only.
- **TriggerResolutionWindow**: adds a decision point only when `enable_priority_windows` is true; action set is pass only.
- **CounterWindow**: adds a decision point when counters are legal; action set is counter + pass (pass always available).
- **DamageResolutionWindow**: adds a decision point only when `enable_priority_windows` is true; action set is pass only.
- **EncoreWindow**: adds a decision point only when `enable_priority_windows` is true; action set is pass only.
- **EndPhaseWindow**: adds a decision point only when `enable_priority_windows` is true; action set is pass only.

### Window Entry Points

Current engine order (per turn):

1. **Main phase**: after the active player chooses `Pass` during the main decision, `MainWindow` opens.
2. **Climax phase**: after `Pass` or `ClimaxPlay`, `ClimaxWindow` opens if `enable_priority_windows` is true; otherwise the phase advances directly to Attack.
3. **Attack declaration**: after an attack is declared, `AttackDeclarationWindow` opens once per attack if `enable_priority_windows` is true.
4. **Trigger resolution**: after the trigger check queues trigger effects, `TriggerResolutionWindow` opens once per attack if `enable_priority_windows` is true.
5. **Counter**: `CounterWindow` opens for the defending player if counters are allowed (regardless of `enable_priority_windows`).
6. **Damage resolution**: after damage intent resolution and before battle step, `DamageResolutionWindow` opens once per attack if `enable_priority_windows` is true.
7. **Encore**: if the encore queue is non-empty, `EncoreWindow` opens once per end phase if `enable_priority_windows` is true.
8. **End phase**: after end-of-turn effects/triggers are processed, `EndPhaseWindow` opens once if `enable_priority_windows` is true.

### Window Advancement

- The engine tracks the active window in `turn.active_window`.
- When a window closes, the engine logs `WindowAdvanced { from, to }`. `to` is currently `None` because the next window is determined by the main phase/attack pipelines.

## Priority Model

Priority is tracked in `turn.priority` as `PriorityState`:

- **Priority holder**: which player currently has priority.
- **Pass count**: how many consecutive passes have occurred.
- **Window**: which timing window is active.
- **Used activated-ability mask**: prevents repeated activation in the same window.

### Priority Rules

- The **active player** gets priority first in all windows except `CounterWindow`, where the **defender** gets priority.
- Priority passes are logged as `PriorityPassed`.
- When a player receives priority (on window entry, after a pass, or after stack resolution), `PriorityGranted` is logged.

### Pass Handling

- If `priority_allow_pass` is enabled, a **pass** action is always available in priority windows.
- If the only available action is **pass**, the engine auto-passes when `priority_autopick_single_action` is true (default).
- If there are **multiple actions** (including pass), a `Choice` is presented and `ChoiceMade` selects the action.

### Stack Resolution

- When both players pass consecutively, the engine checks the stack:
  - If the stack is **non-empty**, the **top item** resolves, `StackResolved` is logged, and priority returns to the active player.
  - If the stack is **empty**, the window closes and control returns to the phase/attack pipeline.
- If `enable_priority_windows` is **false**, the stack still exists; items are auto-resolved in the main loop without opening additional windows.
- Auto-resolution is bounded by `STACK_AUTO_RESOLVE_CAP`. Exceeding the cap emits `AutoResolveCapExceeded` and triggers deterministic engine error handling.

## Stack Model

A `StackItem` includes:

- `id`: stable stack/effect instance id
- `controller`: player controlling the effect
- `source_id`: source card id
- `effect_id`: stable effect definition id
- `payload`: `EffectPayload` containing the `EffectSpec` and resolved targets

### Deterministic Ordering

When multiple stack items are queued simultaneously:

1. `source_id` ascending
2. `effect_kind` stable key (see `stack_effect_key`)
3. `stack_id` ascending

If multiple items belong to the same controller in a group, a stack ordering choice is presented and logged via `StackGroupPresented` and `StackOrderChosen`.

## State-Based Checks

The engine checks state-based outcomes at the following points:

- At the top of the `advance_until_decision` loop (`resolve_pending_losses`).
- On each **check timing** boundary (`run_check_timing`), which also queues auto abilities for:
  - Begin/After Stand
  - Begin/After Draw
  - Begin/After Clock
  - Begin/After Climax
  - Begin Main
  - Begin Attack Phase
  - Begin Attack Declaration Step
  - End of Attack
  - Begin Encore Step
  - End Phase + End Phase Cleanup
- As part of rule actions, deck refresh is performed when the deck is empty and the waiting room is non-empty; if both deck and waiting room are empty, a loss is registered.
- After damage resolution (`resolve_damage_step`) for pending level-up.

## Check Timing Batching

Check-timing triggers are queued deterministically and resolved by the core loop until quiescence.
`advance_until_decision` only stops when a player choice is required (e.g., trigger order, target selection, priority action, or phase decision).
This prevents “micro windows” from exploding decision counts while preserving rules order.

Quiescence is bounded by `CHECK_TIMING_QUIESCENCE_CAP`. If exceeded, the engine records
`AutoResolveCapExceeded`, sets `terminal=Timeout` (truncated), and reports `engine_error_code`
for deterministic RL-safe handling.

## Trigger Queue Ordering

- Trigger groups are sorted by a stable key `(player, source card id, trigger effect kind, ability index)`.
- Each group emits `TriggerQueued` for each trigger and a single `TriggerGrouped` event with ordered ids.

## Policy Flags

The following config flags gate behavior:

- `enable_priority_windows` (default **false**): enables additional timing windows beyond Main/Counter.
- `priority_autopick_single_action` (default **true**): auto-executes the only priority action without a choice.
- `priority_allow_pass` (default **true**): when true, a **pass** action is available in priority windows; when false, pass is suppressed.
