# Weiss Schwarz Simulator (Rust + PyO3)

## Overview
A deterministic Weiss Schwarz simulator core in Rust with a thin PyO3 binding for RL training. The Rust core advances internally until the next decision point, returns canonical legal actions, and provides fixed-size action masks for Maskable PPO or other algorithms.

## Observation format
Observations are fixed-length integer arrays. The format is versioned by `OBS_ENCODING_VERSION`.

### Visibility
`EnvConfig.observation_visibility` controls hidden information exposure:
- `Public` (default): opponent hand and deck contents are hidden (`-1`).
- `Full`: opponent hand and deck contents are fully revealed.

### Header fields
Indices in the observation array:
- `0`: active player
- `1`: phase
- `2`: decision kind (`-1` if none)
- `3`: decision player (`-1` if none)
- `4`: terminal code (`0` none, `1` win P0, `2` win P1, `3` draw, `4` timeout)
- `5..7`: last action fields (kind, param1, param2)
- `8`: attack attacker slot (`-1` if none)
- `9`: attack defender slot (`-1` if none)
- `10`: attack type (`0` frontal, `1` side, `2` direct, `-1` none)
- `11`: pending attack damage
- `12`: counter power bonus
- `13`: decision focus slot (`-1` if none)

Per-player blocks follow for the observation perspective player first, then the opponent. See `weiss_core/src/encode.rs` for exact constants.
Each player block includes counts, per-slot stage fields (card id, status, has-attacked, power, soul), top public zones (level cards, clock top N, waiting room top N), stock top N (full visibility only), and hand/deck contents with hidden information masking.

## Curriculum flags implemented
- `allow_character` / `allow_event` / `allow_climax`
- `enable_clock_phase` / `enable_climax_phase`
- `enable_side_attacks` / `enable_direct_attacks`
- `enable_counters` / `enable_encore`
- `enable_triggers` + per-icon toggles
- `enable_refresh_penalty`
- `enable_level_up_choice`
- `enable_activated_abilities` / `enable_continuous_modifiers`
- `enforce_color_requirement` / `enforce_cost_requirement`
- `reduced_stage_mode`
- `allowed_card_sets` (filters legality by `card_set` in `CardStatic`)

## Replay format
Replays are binary `WSR1` files written via `ReplayWriter` and serialized with postcard.
- Header includes obs/action versions and replay schema version.
- Body includes actions, step metadata, event stream, and a final snapshot with terminal + state hash.
`ReplayConfig.include_trigger_card_id` controls whether trigger events include the revealed attacker deck card id (default false).

## Python API notes
`EnvPool.step_batch_fast` returns only arrays for throughput:
```
(obs, rewards, terminated, truncated, current_player, decision_kind, actor, illegal_action, engine_error)
```
`actor` is the observation/reward perspective for that transition.

## Card DB format (WSDB)
Card DB files start with:
- Magic: `WSDB`
- Schema version: u32 little-endian
- Postcard-encoded payload

Use the packer:
```
cargo run -p weiss_core --bin carddb_pack -- cards.json cards.wsdb
```
