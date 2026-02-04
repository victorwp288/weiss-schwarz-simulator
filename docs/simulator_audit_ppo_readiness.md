# Simulator Audit for Weiss Schwarz RL Environment
## PPO Readiness, Safety, and Next Fixes

Date: 2025-12-25

This document is a high level scan of the current simulator with concrete code references.
Code is the source of truth. This doc describes what the engine actually does today and what will likely cause issues for PPO scale training or long term maintainability.

---

## Executive Summary

Not ready for PPO — P0 blockers remain: Python boundary correctness contract and Python boundary visibility test.

### Previously Resolved Blockers (Implemented)

1. Public observations are always public (visibility policies only affect replay/action masking)
   • Default Python usage no longer leaks hidden information in observations  
   • Code: `weiss_core/src/encode.rs` (`encode_observation`), `weiss_core/src/env.rs` (`build_outcome_with_obs`)  
   • Python defaults: `weiss_py/src/lib.rs` (`EnvPool.new_rl_train` / `EnvPool.new_rl_eval`)

2. Priority windows now always include pass (strict mode restores forced-selection behavior)
   • In `CounterWindow`, pass is available even when counters are legal  
   • In `MainWindow`, pass is available even when activated abilities are legal  
   • Code: `weiss_core/src/env/interaction.rs` (`collect_priority_actions`, `start_priority_choice`, `apply_priority_action`)

3. Concede is gated by `allow_concede` (default false)
   • Prevents the classic PPO exploration escape hatch  
   • Code: `weiss_core/src/legal.rs` (gated by curriculum)  
   • Python tests now cover both modes (`allow_concede` true/false)

---

## PPO Readiness Checklist

### What is already strong

• Advance until decision loop  
  `weiss_core/src/env.rs` (`advance_until_decision`)

• Fixed action space plus legality mask  
  `weiss_core/src/encode.rs` (`ACTION_SPACE_SIZE`, `fill_action_mask`)  
  `weiss_core/src/env.rs` (`update_action_cache`)

• Fixed length observation tensor  
  `weiss_core/src/encode.rs` (`encode_observation`, `OBS_LEN`)

• Parallel batched stepping  
  `weiss_core/src/pool.rs` (Rayon stepping)  
  `weiss_py/src/lib.rs` (`step_into` with `BatchOutMinimal`)

• Deterministic replay artifacts when enabled  
  `weiss_core/src/replay.rs`  
  `weiss_core/src/env/visibility.rs` (`log_event`, `ReplayWriter`)

• Determinism posture is good  
  `weiss_core/src/db.rs` (ability sort key)  
  `weiss_core/src/env/interaction.rs` (stack ordering)  
  `weiss_core/tests/determinism_tests.rs`  
  `weiss_core/tests/property_invariants_tests.rs`

---

## Training Correctness at the Python Boundary
## This is now treated as P0, not a wrapper afterthought

Your core is multi agent and turn based. PPO training becomes invalid if transitions are attributed incorrectly, or if truncation is treated as termination.

### Default training policy selection
Pick a single default and bake it into docs and examples.

Chosen default for this project  
Symmetric training on the acting player  
• Every decision point produces a transition for the player who acts  
• One shared policy is trained for both players  
• The observation includes a player identity signal so the policy can condition on perspective  
• This doubles usable data relative to training only player 0  
• This matches your engine behavior where reward is computed from the acting player perspective  
  Code: `weiss_core/src/env.rs` (reward computed at the acting player call site)

Wrapper requirements for this default  
• The wrapper stores transitions keyed by actor  
• The wrapper uses `actor` returned by `BatchOutMinimal` to attribute the transition  
• The wrapper ensures the observation is from the actor’s perspective  
  If the core currently encodes from a fixed perspective, add an explicit perspective parameter to the encoder call path or provide two encodings

### Truncation versus termination contract
If you cap episodes by decision count or time budget, those are truncations.

Requirements  
• When time budget triggers, set truncated true and terminated false  
• PPO must bootstrap value from the last state when truncated  
• The environment should not silently end with terminated true for time limits  
• Any max decision guard should be explicitly surfaced to Python as truncation metadata

---

## Hidden Information Safety
## Make it impossible to misuse

### Current State
• `encode_observation` respects `ObservationVisibility` directly; public observations are derived from `ObservationVisibility::Public`.  
• `build_outcome_with_obs` passes `observation_visibility` directly into `encode_observation`.  
• `enable_visibility_policies` only affects replay sanitization and action parameter masking, not observation meaning.  
• Python RL defaults enable visibility policies by default (`EnvPool.new_rl_train` / `EnvPool.new_rl_eval`).

### Required Changes
1. Make `ObservationVisibility::Public` a hard invariant that always produces public‑safe features (no hidden card ids or hidden ordering).  
2. Clarify in docs/config that `enable_visibility_policies` only affects replay sanitization and action parameter masking.

### Verification
• Rust: `weiss_core/tests/public_obs_invariance_tests.rs` should continue to pass; it mutates hidden zones and asserts public observation invariance.  
• Python boundary: add a default‑constructor test that steps into hidden zones and asserts no hidden card ids appear in observations.  
• Call path check: `build_outcome_with_obs` → `encode_observation` passes `ObservationVisibility` through unchanged.

### Memory zone semantics
Current behavior treats memory as public for replay/choice sanitization by default.

• Code: `weiss_core/src/env/visibility.rs` (`hidden_event_zone`)

Chosen policy  
• Keep memory as public in replay sanitization for rules fidelity  
• If you want memory hidden for curriculum simplicity, make it an explicit curriculum switch  
  Example: `memory_is_public`, default true for fidelity, optional false for curriculum

---

## Priority Windows and Pass Semantics
## Stop forced decisions

### Current behavior
Priority choices include a pass option by default when priority windows are enabled.

• Code: `weiss_core/src/env/interaction.rs` (priority choice construction + pass handling)  
• Configuration: `CurriculumConfig.priority_allow_pass` (default true) and `CurriculumConfig.strict_priority_mode` (default false).  
  When `strict_priority_mode` is true or `priority_allow_pass` is false, pass can be suppressed for debugging or curriculum experiments.

### Recommended defaults
Pass should remain available in priority windows for RL.

Rules  
• In `CounterWindow`, include pass even when counters are legal  
• In `MainWindow`, include pass even when activated abilities are legal  
• Pass ends the priority opportunity and continues engine advance

Verification  
• Counter window pass is legal when a counter exists  
• Main window pass is legal when an activation exists  
• Pass produces a deterministic continuation, with no forced selection

---

## Concede Control
## Remove the escape hatch by default

### Current behavior
Concede is gated by `CurriculumConfig.allow_concede` (default false in RL presets).

• Code: `weiss_core/src/legal.rs` gates `ActionDesc::Concede` insertion behind this flag  
• Python tests cover both modes (concede on/off)

### Recommended defaults
• Keep `allow_concede=false` for training presets to avoid the escape hatch.  
• Provide a debug preset with `allow_concede=true` for human testing or safety experiments.

---

## Observation Encoding Stability and Versioning

### Why this matters
PPO and offline model reuse break silently if observation indices change meaning without a version bump.

Current observation encoding version: `OBS_ENCODING_VERSION = 1`. Any change to `OBS_LEN` or index semantics must bump this value and add a changelog entry.

### Hard contract
Any change to any observation index meaning requires a version bump.

Add a single authoritative index map test  
• A unit test that asserts  
  • `OBS_LEN`  
  • reserved sentinel fields  
  • key index ranges  
  • encoding version value  
• This test should fail loudly if the layout drifts

Add a changelog rule  
• When bumping encoding version, record the diff summary in a short markdown file  
  Example: `docs/obs_encoding_changelog.md`

---

## Action Mask Learning Aids
## Optional learning speed wins, not correctness blockers

Problem  
Fixed action masks can be hard for PPO to learn when the reason is not inferable.

Minimal addition option  
Add small binary signals to observation, not to the mask.

Examples  
• insufficient stock for play  
• insufficient color  
• phase mismatch  
• target required but none valid  
• cost payment blocked

Placement  
• Treat as P2 or P3 since this affects learning speed, not correctness

---

## Performance and Throughput
## Reduce boundary crossings and allocations

### Step returns should include masks
Current loop requires a second call to get masks.

Chosen change  
Implemented: `step_into` fills obs/rewards/dones/masks in one Rust→Python crossing (`BatchOutMinimal`).

Code points  
• `weiss_core/src/pool.rs`  
• `weiss_py/src/lib.rs`

### Pool output buffers
Step outputs are filled into caller-provided buffers.

• Code: `weiss_core/src/pool.rs` (`BatchOutMinimal`, `fill_minimal_out`, `ensure_outcomes_scratch`)

Chosen optimization  
Reuse internal scratch (`outcomes_scratch`) and write directly into `BatchOutMinimal` / `BatchOutDebug`.

### Observation encoding hot spots
Encoding recomputes derived power by scanning modifiers per slot.

• Code: `weiss_core/src/encode.rs`  
• Related: `weiss_core/src/env/movement.rs` (`compute_slot_power`)

Chosen optimization  
Maintain cached derived power per slot and update on modifier changes and slot movement.

### Rayon configurability
Implemented: `EnvPool::new_rl_train/new_rl_eval/new_debug` accept `num_threads` to pin a dedicated Rayon pool.

Reason  
Multiple pools in one process can oversubscribe.

---

## Python Debuggability Features
## These become first class, not nice to have

### Fingerprints in Python
Expose batch fingerprints for fast drift detection.

• Rust already has: `weiss_core/src/fingerprint.rs`

Implemented  
• `state_fingerprint_batch()`  
• `events_fingerprint_batch()`

### Stable action descriptions
Expose action id to canonical description mapping.

Reason  
When PPO picks something odd, you need instant human readable explanations.

Implementation hook  
• Rust has `last_action_lookup: Vec<Option<ActionDesc>>` per env in `weiss_core/src/env.rs`

Implemented  
• `describe_action_ids(action_ids)`

### Decision metadata
Expose more decision context than just `decision_kind` and `current_player`.

Implemented fields  
• `ChoiceReason`  
• focus slot  
• decision origin, trigger, priority, target selection

---

## Determinism Notes

### Strong points
• Stable ability canonical ordering  
  `weiss_core/src/db.rs` (`ability_sort_key`, `AbilityTemplateTag`)

• Stable ordering keys for trigger pipeline and stack grouping  
  `weiss_core/src/env/phases.rs` (`handle_trigger_pipeline`)  
  `weiss_core/src/env/interaction.rs` (`enqueue_stack_items`)  
  `weiss_core/src/env/phases.rs` (`resolve_damage_intent`)

• Determinism tests exist and are meaningful  
  `weiss_core/tests/determinism_tests.rs`  
  `weiss_core/tests/property_invariants_tests.rs`  
  `weiss_core/src/env/tests.rs` (`deterministic_replay_from_seed_and_actions`)

### Remaining watch list
• Parallel stepping is deterministic per env, not globally ordered by time  
  `weiss_core/src/pool.rs`

• Future hazard, introducing `HashMap` iteration in hot path enumerations  
Keep a policy rule, never iterate `HashMap` for action enumeration or stack ordering

• Hidden zone option ids rely on enumeration order  
  `weiss_core/src/env/interaction.rs` (`choice_option_id`)  
Treat candidate ordering as externally observable for replay stability

---

## Docs Deliverables
## Make training safe by default

Add a single training guide file  
`docs/ppo_guide.md`

Owner: vwp  
Target date: 2026-02-18

Contents  
• Safe defaults for RL training presets  
  • public observations are truly public  
  • visibility policies enabled by default for replay and action sanitization  
  • concede disabled by default  
  • pass always enabled in priority windows  
• Multi agent training contract  
  • symmetric acting player transitions  
  • actor based attribution  
• Truncation semantics  
  • truncated true, terminated false for time limits  
  • PPO bootstrapping requirements  
• Recommended API calls for a training step  
  • prefer `BatchOutMinimal` + `step_into` (or `EnvPoolBuffers`)  
• Debug workflow  
  • fingerprints  
  • action descriptions  
  • replay artifact usage

---

## Recommended Fixes, Prioritized

### P0 (Blocking)
1. Establish Python boundary correctness contract (TODO / blocking)
   • Symmetric actor based transition attribution as the default training mode  
   • Proper truncation handling for time limits
   • Owner: vwp  
   • Target date: 2026-02-12  
   • Acceptance criteria:  
     • Unit tests prove actor-based attribution is symmetric across both players  
     • Integration test covers time-limit truncation (terminated=false, truncated=true) and PPO bootstrapping contract  
     • Tests fail if attribution flips or truncation is reported as termination  
   • Test plan:  
     • Add `weiss_py/tests/test_actor_attribution.py::test_actor_attribution_symmetric`  
     • Add `weiss_py/tests/test_truncation_contract.py::test_time_limit_truncation`  
   • CI gating: `python -m pytest weiss_py/tests/test_actor_attribution.py weiss_py/tests/test_truncation_contract.py`

2. Add Python boundary visibility test (TODO / blocking)
   • Fail if public mode leaks any hidden information in default constructor paths
   • Owner: vwp  
   • Target date: 2026-02-12  
   • Acceptance criteria:  
     • Test fails on any constructor path that yields hidden ids or ordering in public observations  
     • Public constructor paths are asserted to return only public-safe features  
   • Test plan:  
     • Add `weiss_py/tests/test_public_visibility.py::test_public_mode_no_hidden_leak`  
     • Exercise `EnvPool.new_rl_train` and `EnvPool.new_rl_eval` with hidden-zone mutations and verify no leaks  
   • CI gating: `python -m pytest weiss_py/tests/test_public_visibility.py`

3. Enforce public observation semantics (done)
   • `encode_observation` respects `ObservationVisibility`  
   • `build_outcome_with_obs` passes `observation_visibility` directly

4. Add `allow_concede` to curriculum (done)
   • `CurriculumConfig.allow_concede` default false  
   • `ActionDesc::Concede` gated in `weiss_core/src/legal.rs`

### P1 (Non-blocking)
1. Add `docs/ppo_guide.md` (TODO)
   • Owner: vwp  
   • Target date: 2026-02-18  
   • Justification: safe defaults, multi-agent contract, truncation semantics, recommended API calls (`BatchOutMinimal` + `step_into` / `EnvPoolBuffers`), and debug workflow

2. Implement pass semantics in priority windows (done)
   • `priority_allow_pass` default true  
   • `strict_priority_mode` optional false  
   • `CounterWindow` / `MainWindow` include pass

3. `step_into` (done)
   • Removes a Rust→Python boundary crossing per step

4. Expose action descriptions and fingerprints in Python (done)
   • `describe_action_ids`  
   • `state_fingerprint_batch` and `events_fingerprint_batch`

### P2 (Done)
1. Cache derived power per slot (done: slot-level cache with dirty tracking)
2. Reuse pool output buffers (done: scratch buffers in EnvPool)
3. Add Rayon configurability for thread control (done)
4. Add minimal mask reason signals to observation (done; `OBS_ENCODING_VERSION=1` with reason bits + reveal buffer + context bits)

---
