# Weiss Schwarz Simulator (Rust core + Python bindings)

Deterministic, RL-first Weiss Schwarz simulation: **Rust runs the hot loop**, advances until a **decision point**, and exposes a **fixed action space + mask** for Maskable PPO (and friends). Python gets a thin, fast `EnvPool` wrapper for batched stepping.

---

## Why this exists

Weiss Schwarz has a lot of hidden information and branching. For RL, you typically want:

- **Determinism**: reproduce episodes from a seed, keep training debuggable.
- **Few boundary crossings**: don’t bounce Python↔Rust every micro-step.
- **A stable action space**: fixed-size \(N\) actions + legal-action mask.
- **Introspectability**: canonical action descriptions, replays, event logs.

This repo is built around those constraints.

---

## Highlights

- **Advance-until-decision loop**: the engine runs internally until a player must act (mulligan/clock/main/attacks/etc.).
- **Canonical legal actions**: truth source is a list of `ActionDesc` (debuggable, human-readable).
- **Fixed action id space + mask**: derived from canonical actions and **versioned** by `ACTION_ENCODING_VERSION`.
- **Fixed-length observations**: int32 arrays, **versioned** by `OBS_ENCODING_VERSION`.
- **Multicore stepping**: `EnvPool` steps many envs in parallel via `rayon`; Python binding releases the GIL.
- **Replays**: record seeds, actions, step metadata, optional event stream, final snapshot hash (sanitized only when visibility policies are enabled in public mode).
- **Curriculum switches**: selectively enable/disable chunks of the rules for training curricula.

Each environment is deterministic given its seed and action sequence. Parallel batch stepping does not change outcomes because environments have no shared state.

---

## Repo layout

- `weiss_core/`: Rust simulator core (state machine, legality, encoding, replay, pool)
- `weiss_py/`: PyO3 extension module (`weiss_sim`) exposing `EnvPool`
- `python/weiss_sim/`: Python package wrapper that re-exports the extension
- `python/tests/`: pytest smoke tests + fixture card DB

---

## Installation

### Python (local build via `maturin`)

Prerequisites:
- **Python**: ≥ 3.10
- **Rust toolchain**: stable (`cargo`, `rustc`)

Install (editable):

```bash
python -m pip install -U pip
python -m pip install -U maturin
python -m pip install -e .
```

Sanity check:

```bash
python -c "import weiss_sim; print(weiss_sim.__version__, weiss_sim.EnvPool)"
```

### Rust (core only)

```bash
cargo build -p weiss_core
cargo test -p weiss_core
```

---

## Development & tests

Python tests:

```bash
python -m pip install -e .
python -m pip install -U pytest numpy
pytest -q
```

Benchmarks (Criterion):

```bash
cargo bench -p weiss_core
```

---

## Quickstart (Python): step with a trivial policy

The environment exposes a **fixed action space** and an **action mask**. Pick any index where mask==1.

```python
from pathlib import Path
import numpy as np
import weiss_sim

fixture_dir = Path("python/tests/fixtures")
db_path = fixture_dir / "cards.wsdb"

pool = weiss_sim.EnvPool(
    1,
    str(db_path),
    deck_lists=[[1] * 50, [2] * 50],
    deck_ids=[1, 2],
    max_decisions=200,
    max_ticks=10_000,
    seed=123,
    observation_visibility="public",  # "public" | "full"
    error_policy="lenient_terminate", # "strict" | "lenient_terminate" | "lenient_noop"
)

obs = pool.reset_all()

def first_legal(mask_row: np.ndarray) -> int:
    idxs = np.flatnonzero(mask_row)
    assert idxs.size > 0
    return int(idxs[0])

for _ in range(10):
    masks = pool.action_masks_batch()  # shape: (num_envs, action_space), dtype: uint8
    actions = [first_legal(masks[0])]
    (obs, rewards, terminated, truncated,
     current_player, decision_kind, actor,
     illegal_action, engine_error) = pool.step_batch_fast(actions)
```

Debug print (very lightweight):

```python
print(pool.render_ansi(env_index=0, perspective=0))
```

---

## Python API (what you get)

The extension module is `weiss_sim` and the package re-exports it as `import weiss_sim`.

### `weiss_sim.EnvPool`

Constructor:

```python
EnvPool(
    num_envs: int,
    db_path: str,
    deck_lists: list[list[int]],          # length 2
    deck_ids: list[int] | None = None,    # length 2; defaults to [0, 1]
    max_decisions: int = 10_000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,   # JSON for weiss_core::CurriculumConfig
    reward_json: str | None = None,       # JSON for weiss_core::RewardConfig
    error_policy: str | None = None,      # "strict" | "lenient_terminate" | "lenient_noop"
    observation_visibility: str | None = None,  # "public" | "full"
)
```

Core methods:
- `reset_all() -> np.ndarray[int32]`: shape `(num_envs, obs_len)`
- `reset_indices(indices: list[int]) -> np.ndarray[int32]`: resets subset, returns full batch obs
- `action_masks_batch() -> np.ndarray[uint8]`: shape `(num_envs, action_space)`
- `step_batch(actions: list[int]) -> (obs, rewards, terminated, truncated, infos)`
  - `infos` is a list of dicts including `actor`, versions, terminal, etc.
- `step_batch_fast(actions: list[int]) -> (obs, rewards, terminated, truncated, current_player, decision_kind, actor, illegal_action, engine_error)`
  - pure arrays for throughput; `actor` is the observation/reward perspective for that transition
- `legal_actions_batch() -> list[list[dict]]`: canonical structured actions (for debugging/human play)
- `get_current_player_batch() -> np.ndarray[int8]`: player who must act next, or `-1`
- `render_ansi(env_index: int, perspective: int) -> str`
- `set_curriculum(curriculum_json: str) -> None`
- `enable_replay_sampling(enabled: bool, sample_rate: float, out_dir: str, compress: bool = False, include_trigger_card_id: bool = False) -> None`

Convenience properties:
- `action_space: int`
- `obs_len: int`

Module constants:
- `weiss_sim.OBS_ENCODING_VERSION`
- `weiss_sim.ACTION_ENCODING_VERSION`

---

## CurriculumConfig flags (defaults)

Curriculum flags gate rules/complexity. For the canonical default list and behavior notes,
see `PROJECT_STATE.md` (single source of truth) and `docs/invariants.md` (machine-checked).

Notes:
- `enable_priority_windows` gates **additional** windows beyond the base phase decisions. MainWindow opens on `MainPass`, and CounterWindow opens during the attack counter step when counters are allowed. Priority actions (activated abilities, counters) are surfaced as `ChoiceSelect` decisions.
- `enable_visibility_policies` masks hidden-zone choice info in replays/labels when `observation_visibility="public"`. When disabled, replays can include hidden info even in public mode.

---

## Rust API (core crate)

`weiss_core` re-exports the main types for embedding the simulator in Rust:

- `EnvPool`, `GameEnv`
- `EnvConfig`, `CurriculumConfig`, `RewardConfig`, `ErrorPolicy`, `ObservationVisibility`
- `ActionDesc`, `Decision`, `DecisionKind`
- `CardDb`, `CardId`

Minimal sketch:

```rust
use std::sync::Arc;
use weiss_core::{CardDb, CurriculumConfig, EnvConfig, EnvPool, RewardConfig, ErrorPolicy, ObservationVisibility};

let db = Arc::new(CardDb::load("cards.wsdb")?);
let config = EnvConfig {
    deck_lists: [vec![1; 50], vec![2; 50]],
    deck_ids: [1, 2],
    max_decisions: 10_000,
    max_ticks: 100_000,
    reward: RewardConfig::default(),
    error_policy: ErrorPolicy::LenientTerminate,
    observation_visibility: ObservationVisibility::Public,
};
let curriculum = CurriculumConfig::default();
let mut pool = EnvPool::new(8, db, config, curriculum, 123);
let batch = pool.reset_all();
```

---

## Replay schema policy

- Replay binary format is versioned by `REPLAY_SCHEMA_VERSION` in `weiss_core/src/replay.rs` (pinned to `1`).
- If the serialized structure changes, update code/tests/docs **without** bumping the version; old artifacts are deleted.
- Event **content** changes (e.g., masked labels under visibility policies) do **not** change the schema version.
- Dump tooling should check the header and reject unsupported versions.

---

## Encodings (stable + versioned)

Encodings are **deterministic** and **explicitly versioned** (pinned to `1`). Query current versions at runtime via:

- `weiss_sim.OBS_ENCODING_VERSION`
- `weiss_sim.ACTION_ENCODING_VERSION`

### Observation tensor

Observations are fixed-length `int32` arrays. Query the current length via:

- `weiss_sim.OBS_LEN` or `pool.obs_len`

Visibility modes (`observation_visibility`):
- `"public"`: opponent hand/deck are hidden (filled with `-1`)
- `"full"`: opponent hand/deck are revealed

Header indices in the observation array:
- `0`: active player
- `1`: phase (`0..7` = Mulligan, Stand, Draw, Clock, Main, Climax, Attack, End)
- `2`: decision kind (`-1` none; `0..8` = Mulligan, Clock, Main, Climax, AttackDeclaration, LevelUp, Encore, TriggerOrder, Choice)
- `3`: decision player (`-1` if none)
- `4`: terminal code (`0` none, `1` win P0, `2` win P1, `3` draw, `4` timeout)
- `5..7`: last action fields (kind, param1, param2)
- `8`: attack attacker slot (`-1` if none)
- `9`: attack defender slot (`-1` if none)
- `10`: attack type (`0` frontal, `1` side, `2` direct, `-1` none)
- `11`: pending attack damage
- `12`: counter power bonus
- `13`: decision focus slot (`-1` if none)
- `14`: choice page start (`-1` if not in a choice decision)
- `15`: choice total candidates (`-1` if not in a choice decision)

Per-player blocks follow for the **perspective player first**, then the opponent. The exact layout is defined in `weiss_core/src/encode.rs` and versioned by `OBS_ENCODING_VERSION`.

### Action space

Actions are fixed to `ACTION_SPACE_SIZE` (`pool.action_space`). The exact id layout is **derived from constants in `weiss_core/src/encode.rs`** and is versioned by `ACTION_ENCODING_VERSION`. Action families include:

- mulligan keep/all
- clock pass / clock(hand_index)
- main pass / play_character(hand_index, stage_slot) / play_event(hand_index) / move(from_slot, to_slot)
- climax pass / play(hand_index)
- attack pass / attack(slot, attack_type)
- choice_select(index) (also used for priority-window actions like activated abilities and counters)
- choice_prev_page / choice_next_page
- level_up(index)
- encore yes/no
- trigger_order(index)
- concede (always legal; id is `ACTION_SPACE_SIZE - 1`)

The legal-action **mask** is derived from the canonical `ActionDesc` list, and the mapping is versioned by `ACTION_ENCODING_VERSION`.

---

## Curriculum flags (training switches)

Curricula are configured via JSON (passed as `curriculum_json`) and mapped to `weiss_core::CurriculumConfig`.

Implemented flags include:
- `allow_character`, `allow_event`, `allow_climax`
- `enable_clock_phase`, `enable_climax_phase`
- `enable_side_attacks`, `enable_direct_attacks`
- `enable_counters`, `enable_backup`, `enable_encore`
- `enable_triggers` plus per-icon toggles (`enable_trigger_soul`, `enable_trigger_draw`, etc.)
- `enable_refresh_penalty`
- `enable_level_up_choice`
- `enable_activated_abilities`, `enable_continuous_modifiers`
- `enforce_color_requirement`, `enforce_cost_requirement`
- `reduced_stage_mode`
- `allowed_card_sets` (filters legality by `card_set` on `CardStatic`)

See `weiss_core/src/config.rs` for the full struct and defaults.

---

## Errors, illegal actions, and truncation

Each step returns flags per env:
- **`illegal_action`**: action id was not legal under the current decision
- **`engine_error`**: simulator hit an internal error while applying an action

Error policy (`error_policy`):
- `"strict"`: `step_batch*` raises on engine errors
- `"lenient_terminate"`: terminates the episode on engine error (marks `engine_error=True`)
- `"lenient_noop"`: (implemented in core policy enum; behavior is still evolving)

Truncation limits:
- `max_decisions`: cap number of decision applications per episode
- `max_ticks`: cap internal advancement ticks per episode

---

## Replays (WSR1)

Replays are binary `WSR1` files written via `ReplayWriter` and serialized with `postcard`.

What gets recorded:
- **Header**: obs/action encoding versions, replay schema version, seed, starting player, deck ids, curriculum id, config hash
- **Body**: action sequence, per-step metadata (actor/decision kind/flags), optional event stream, final snapshot (terminal + state hash)

Python: enable sampled replay recording per pool:

```python
pool.enable_replay_sampling(
    enabled=True,
    sample_rate=0.05,
    out_dir="replays",
    compress=False,
    include_trigger_card_id=False,
)
```

Notes:
- Files are named like `episode_00000000.wsr`, `episode_00000001.wsr`, ...
- Compression is implemented in the Rust core behind the `weiss_core` feature `replay-zstd`.
  - The current Python build does **not** enable that feature by default; keep `compress=False` unless you’ve wired it through.

Tooling:

```bash
cargo run -p weiss_core --bin replay_dump -- path/to/episode_00000000.wsr
```

---

## Card database (WSDB)

The simulator loads a binary card DB:

- Magic: `WSDB`
- Schema version: `u32` little-endian (`WSDB_SCHEMA_VERSION = 1`)
- Payload: `postcard`-encoded `CardDb`

Pack JSON → WSDB:

```bash
cargo run -p weiss_core --bin carddb_pack -- cards.json cards.wsdb
```

Input JSON can be either:
- a full `CardDb` object, or
- a flat list of `CardStatic`

`CardStatic` fields (see `weiss_core/src/db.rs`):
- `id: u32` (**0 is reserved and invalid in DB**)
- `card_set: string | null` (optional; used by `allowed_card_sets`)
- `card_type: "Character" | "Event" | "Climax"`
- `color: "Yellow" | "Green" | "Red" | "Blue" | "Colorless"`
- `level: u8`, `cost: u8`, `power: i32`, `soul: u8`
- `triggers: list[TriggerIcon]` (`Soul`, `Shot`, `Bounce`, `Draw`, `Treasure`, `Gate`, `Standby`)
- `traits: list[u16]`
- `abilities: list[AbilityTemplate]` (see enum in `db.rs`)
- `counter_timing: bool` (optional; only meaningful for Character/Event)
- `raw_text: string | null` (optional)

---

## Project status (implemented vs stubbed)

This is a simulator core built for RL training and determinism first. Some rule systems are intentionally stubbed/simplified right now.


Examples of current known simplifications include:
- no full priority/stack model (beyond current phase/trigger ordering)
- limited card text modeling beyond the included `AbilityTemplate` variants
- simplified Treasure trigger, simplified Standby targeting constraints

---

## License

Dual-licensed under **MIT OR Apache-2.0** (see workspace metadata in `Cargo.toml`).
