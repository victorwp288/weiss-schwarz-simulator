# PPO Training Guide (Local)

This guide defines the **safe default contract** for PPO training with this simulator.
Code is the source of truth; this document records the intended training semantics.

## Primary RL Entry Point (Python)

Use the RL-safe constructor as the default training path:

- `EnvPool.new_rl_train(...)` enforces public observations, visibility policies, and concede disabled.
- `EnvPool.new_rl_eval(...)` keeps the same safety semantics with eval-friendly error handling.
- `EnvPool.new_debug(...)` is for debug/legacy usage only; it can override safety settings.

## Safe Defaults (Recommended)

- `observation_visibility = Public`
- `enable_visibility_policies = true` (replay/action masking)
- `allow_concede = false`
- `priority_allow_pass = true`
- `strict_priority_mode = false`
- `enable_priority_windows = false` (unless you explicitly train with priority systems)
  - When disabled, activated abilities are not presented and a main `Pass` advances directly to Climax.

## Multi-Agent Training Contract

Default policy: **symmetric training on the acting player**.

- Each decision point produces a transition for the player who acts.
- One shared policy is trained for both players.
- The observation is already from the actor’s perspective.
- Use `actor` from `BatchOutMinimal` to attribute transitions correctly.

## Truncation Semantics

Time limits are **truncations**, not terminations.

- `truncated = true` when `max_ticks` or `max_decisions` is hit.
- `terminated = false` for timeouts.
- PPO should bootstrap value from the final state when truncated.

## Recommended Step Flow

Use the minimal batch output and capture masks in the same call:

1) Allocate `BatchOutMinimal(num_envs)`.
2) `reset_into(out)` to get the first `(obs, masks, ...)` together.
3) `step_into(actions, out)` for subsequent transitions.
4) Use `actor` to route the transition to the correct policy buffer.
5) Use `truncated` to decide whether to bootstrap.

Optional throughput helpers:

- `EnvPoolBuffers(pool).reset()` / `.step(actions)` wraps the into-buffer calls.
- `reset_indices_into(indices, out)` resets subsets while keeping masks aligned.
- `reset_done_into(done_mask, out)` resets envs where done_mask is true.
- `auto_reset_on_error_codes_into(engine_status, out)` resets envs with non-zero error codes.
- `reset_rl(pool)` / `step_rl(pool, actions)` return a `RlStep` dataclass with named fields.
- `reset_rl_i16_legal_ids(pool)` / `step_rl_i16_legal_ids(pool, actions)` return `RlStepI16LegalIds`.
- `step_rl_select_from_logits_i16_legal_ids(pool, logits)` selects actions in Rust (fast path).
- `pass_action_id_for_decision_kind(decision_kind)` returns `PASS_ACTION_ID` for convenience.

Fast-path note:
- When using legal ids instead of masks, compute `behavior_logp` from `logits` and the per-env legal id slices to keep V-trace correct.

## Observation Reason Bits

Observation encoding version 1 includes the 8-length reason bit block after the per-player blocks.
These bits are public-safe and only populated for the acting player’s decision:

- Phase gating: main, climax, attack, counter window
- Resource blocks: no stock, no color, no hand candidates
- Target absence: no valid targets for the current choice

## Reveal History Buffer

Observation encoding version 1 appends a reveal history buffer (length `REVEAL_HISTORY_LEN`) for the observing player:

- Stores the last N card ids revealed to that player (oldest → newest).
- Uses public-safe card ids only (no instance ids, no hidden ordering).
- Does not include opponent-only reveals.

## Context Bits

Observation encoding version 1 appends a small context bit block (length 4):

- priority window open
- active choice present
- non-empty stack
- encore queue pending

## Known RL Limitations (Current)

- None known for reveal/search visibility; revealed cards to the acting player are now represented.

## Debug & Drift Detection

Use these Python helpers for debugging:

- `state_fingerprint_batch()`
- `events_fingerprint_batch()` (requires replay recording)
- `describe_action_ids(action_ids)`
- `decision_info_batch()`
- `engine_error_reset_count()` tracks auto-resets triggered from Python.

Engine error codes (`engine_status` in `BatchOutMinimal`):
- `0`: none
- `1`: stack auto-resolve cap
- `2`: trigger quiescence cap
- `3`: panic in step (caught)
- `4`: action error (lenient policies)

## Regression Tests Worth Keeping Green

- `weiss_core/tests/public_obs_invariance_tests.rs`
- `weiss_core/tests/determinism_tests.rs`
- `python/tests/test_rl_defaults.py`
- `python/tests/test_priority_pass.py`
- `python/tests/test_step_into_buffers.py`

## Replay Safety

Replays and action logs are sanitized **only** when:

- `enable_visibility_policies = true`
- `observation_visibility = Public`

Keep these enabled for any public or shared dataset generation.
