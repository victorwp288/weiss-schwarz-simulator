# Training Contract (Deterministic Hidden-Info RL)

This document defines the stable environment contract for ML training and evaluation.

## Step Semantics (Decision-Based)
- Each `step` applies exactly one **player decision**.
- After the action, the simulator advances internally until the **next decision** or **terminal**.
- If `terminated` or `truncated` is `true`, the caller must reset that env before using it again.
- If `terminated` or `truncated` is `false`, `actor` is `0` or `1` and **at least one legal action exists**.
- `decision_id` resets to `0` on reset and increments by exactly `+1` per decision within the episode.

## Output Fields (Minimal Batch)
The minimal RL output is a fixed, contiguous set of arrays:
- `obs`: `int32` observation vector (length `OBS_LEN`)
- `masks`: `uint8` legality mask for each action id (length `ACTION_SPACE_SIZE`)
- `rewards`: `float32` reward from the acting player perspective
- `terminated` / `truncated`: `bool` terminal vs timeout
- `actor`: `int8` acting player (`0`/`1`), or `ACTOR_NONE` when done
- `decision_kind`: `int8` decision kind enum (`DECISION_KIND_NONE` when no decision)
- `decision_id`: `uint32` per-episode decision counter (0-based)
- `engine_status`: `uint8` error code (0 means OK)
- `spec_hash`: `uint64` stable hash for obs/action encoding + policy version

### Optional Fast Output (i16 + Legal Ids)
For high-throughput training, you can use the i16 + legal-ids variant:
- `obs`: `int16` observation vector (length `OBS_LEN`)
- `legal_ids`: packed `uint16` legal action ids
- `legal_offsets`: `uint32` offsets into `legal_ids` per env (length `num_envs + 1`)

Use cases:
- Fast action selection in Rust (`step_select_from_logits_into_i16_legal_ids`)
- Python V-trace by reconstructing masked log-probs from `legal_ids`

## Observation Encoding
- The observation is encoded from the **acting player perspective**.
- The **self** block appears first; opponent block second (`self_first = true`).
- Hidden values are masked with `sentinel_hidden = -1`.
- Empty card slots use `sentinel_empty_card = 0`.

Use:
- `weiss_sim.observation_spec_json()` for the full layout spec.
- `weiss_sim.spec_bundle()` for both observation and action specs.

### Versioning
- `OBS_ENCODING_VERSION = 1`
- `ACTION_ENCODING_VERSION = 1`
- `POLICY_VERSION = 2` (contract bump for RL-facing changes)

## Action Encoding
- Action ids are stable within `ACTION_ENCODING_VERSION`.
- `PASS_ACTION_ID` is a valid action id and contextually interpreted.
- Use the legality mask or `legal_action_ids_into` to choose valid actions.

Use:
- `weiss_sim.action_spec_json()` for the full action layout spec.
- `weiss_sim.decode_action_id(id)` for human-readable decoding.

## Reward Convention (Zero-Sum)
- Rewards are from the **acting player perspective**.
- Reward perspective is fixed per step (it does not switch to a fixed seat mid-episode).
- Terminal rewards are zero-sum:
  - `terminal_win + terminal_loss = 0`
  - `terminal_draw = 0`
- If shaping is enabled, it is **antisymmetric** (damage to opponent adds, damage to self subtracts).

Tests:
- `weiss_core/src/env/tests.rs` enforces terminal zero-sum and shaping antisymmetry.

## Termination vs Truncation
- `terminated` means **true terminal**, no bootstrap.
- `truncated` means **timeout**, bootstrap allowed.
- Timeouts map to `truncated`.

## Determinism & Episode Metadata
Per-episode metadata exposed to Python:
- `episode_seed`
- `episode_index`
- `env_index`
- `starting_player`

Determinism helpers:
- `reset_indices_with_episode_seeds_into(indices, episode_seeds, out)`
- `state_fingerprint_batch()`, `events_fingerprint_batch()`, `obs_fingerprint_batch()`
- `max_card_id()` for embedding table sizing

### Episode Key
Use these fields to identify a trajectory:
- `spec_hash`
- `config_hash`
- `episode_seed`
- `starting_player`

Python access:
- `EnvPool.config_hash()`
- `EnvPool.episode_seed_batch()`
- `EnvPool.episode_index_batch()`
- `EnvPool.env_index_batch()`
- `EnvPool.starting_player_batch()`

## Replay Modes
Two replay modes:
- **Full**: raw actions + canonical events for deterministic replay
- **Public**: sanitized actions + public events for sharing

Replay headers include:
- `obs_version`, `action_version`, `replay_version`
- `seed`, `base_seed`, `episode_seed`
- `spec_hash`, `config_hash`
- `deck_ids`, `curriculum_id`, `starting_player`

Python control:
```python
pool.enable_replay_sampling(
    sample_rate=1.0,
    out_dir="replays",
    compress=False,
    include_trigger_card_id=True,
    visibility_mode="full",  # or "public"
)
```

## Evaluation Hygiene
- Use `error_policy="strict"` in `EnvPool.new_rl_eval(...)` for strict evaluation.
- Do **not** auto-reset on errors during evaluation (prevents corrupted win-rate matrices).
- Training can use auto-reset, but should track it via `EnvPool.engine_error_reset_count()`.
- Reset the counter with `EnvPool.reset_engine_error_reset_count()` when starting a new run.

## Legality Consistency
- Dense masks and `legal_action_ids_into` are required to agree.
- Debug output includes `mask_fingerprint` to detect divergence across replays.
- You can disable dense mask output via `EnvPool.set_output_mask_enabled(false)` to save bandwidth; masks are then undefined (buffers are zeroed on disable) and `action_masks_batch()` is disabled.
- For zero mask allocation/copy, use `BatchOutMinimalNoMask` / `EnvPoolBuffersNoMask` and the `*_nomask` stepping APIs.
- Packed masks are available via `EnvPool.action_mask_bits_batch()` when you need compact legality output.

## Main Seat Wrapper (Future)
If you later train only on a “main” seat:
- Use `decision_id` deltas to compute `k` for discounting (`gamma^k`).
- The simulator can be extended to auto-advance to the main seat if needed.
