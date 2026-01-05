# Weiss Schwarz Simulator (Rust core + Python bindings)

[![CI](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/ci.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/ci.yml)
[![Wheels](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/wheels.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/wheels.yml)
[![Benchmarks](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/benchmarks.yml)
[![Security](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/victorwp288/weiss-schwarz-simulator/actions/workflows/security.yml)
[![Docs](https://img.shields.io/badge/docs-rustdoc-blue)](https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/)
[![PyPI](https://img.shields.io/pypi/v/weiss-sim.svg)](https://pypi.org/project/weiss-sim/)
[![Changelog](https://img.shields.io/badge/changelog-view-blue)](https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/CHANGELOG.md)
[![Last Commit](https://img.shields.io/github/last-commit/victorwp288/weiss-schwarz-simulator.svg)](https://github.com/victorwp288/weiss-schwarz-simulator/commits/main)

Deterministic, RL-first Weiss Schwarz simulation: **Rust runs the hot loop**, advances until a **decision point**, and exposes a **fixed action space + mask** (and legal action ids) for efficient batched training. Python gets a thin `EnvPool` wrapper for stepping many environments in parallel.

---

## Why this exists

Weiss Schwarz has hidden information, branching, and timing windows. For RL you typically want:

- **Determinism**: reproduce episodes from a seed and action sequence.
- **Few boundary crossings**: avoid Python↔Rust overhead for micro-steps.
- **Stable action space**: fixed-size actions with legality masking.
- **Introspectability**: canonical action descriptions, replays, and event logs.

This repo is built around those constraints.

---

## Highlights

- **Advance-until-decision loop**: engine runs internally until a player must act.
- **Canonical legal actions**: `ActionDesc` list is the single truth source.
- **Fixed action id space + mask**: derived from canonical actions and **versioned** (`ACTION_ENCODING_VERSION`).
- **Legal action ids (fast)**: use ids to avoid Python-side mask scans in hot loops.
- **Fixed-length observations**: `int32` arrays, **versioned** (`OBS_ENCODING_VERSION`).
- **Multicore stepping**: `EnvPool` uses Rayon; Python binding releases the GIL.
- **Replays**: deterministic, versioned, optional event stream, public-safe sanitization when enabled.
- **Curriculum switches**: gate rules/features for training curricula.

Each environment is deterministic given its seed and action sequence. Parallel batch stepping does not change outcomes because envs are fully isolated.

---

## Automation & Benchmarks

- **CI** runs on every push/PR: Rust fmt/clippy/tests + Python ruff/pytest.
- **Wheels** build on pushes to `main` (artifacts), and tags `v*` publish to GitHub Releases + PyPI.
- **Benchmarks** run on pushes to `main`; history + charts are published via GitHub Pages.
- **Docs** are published to GitHub Pages on pushes to `main`.

Latest benchmark history and charts:
https://victorwp288.github.io/weiss-schwarz-simulator/benchmarks

Rust API docs:
https://victorwp288.github.io/weiss-schwarz-simulator/rustdoc/

Note: with only 1–2 benchmark runs, charts can look “empty” until more points exist.

### Releases

Release automation is handled by Release Please. To ensure downstream workflows (like `Wheels`) run
automatically when a release tag is created, configure a fine-grained PAT as `RELEASE_PLEASE_TOKEN`
in GitHub Actions secrets; otherwise you can manually run the `Wheels` workflow for the release tag.

### Benchmark Snapshot (main, top 12)

<!-- BENCHMARKS:START -->
_Last updated: 2026-01-05 00:44 UTC_

| Benchmark | Time |
| --- | --- |
| rust/advance_until_decision | 63280 ns/iter |
| rust/step_batch_64 | 26268 ns/iter |
| rust/step_batch_fast_256_priority_off | 111592 ns/iter |
| rust/step_batch_fast_256_priority_on | 109646 ns/iter |
| rust/legal_actions | 44 ns/iter |
| rust/legal_actions_forced | 43 ns/iter |
| rust/on_reverse_decision_frequency_on | 1534 ns/iter |
| rust/on_reverse_decision_frequency_off | 1540 ns/iter |
| rust/observation_encode | 228 ns/iter |
| rust/observation_encode_forced | 233 ns/iter |
| rust/mask_construction | 455 ns/iter |
| rust/mask_construction_forced | 412 ns/iter |
<!-- BENCHMARKS:END -->


---

## Repo layout

- `weiss_core/`: Rust simulator core (state machine, legality, encoding, replay, pool)
- `weiss_py/`: PyO3 extension module (`weiss_sim`) exposing `EnvPool`
- `python/weiss_sim/`: Python wrapper that re-exports the extension
- `python/tests/`: pytest smoke tests + fixture card DB

---

## Installation

### Python (local build via `maturin`)

Prerequisites:
- **Python**: ≥ 3.10
- **Rust toolchain**: stable (`cargo`, `rustc`)
- **Bindings**: built with PyO3 0.24 + numpy 0.24 (Rust side)

Install (editable):

```bash
python -m pip install -U pip
python -m pip install -U maturin
python -m pip install -e .
```

Note (macOS/PyO3): if you build wheels locally, prefer an explicit interpreter to avoid linking errors
and unsupported system Pythons:

```bash
maturin build --release --manifest-path weiss_py/Cargo.toml --interpreter .venv/bin/python -o dist
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

## Quickstart (Python): step with a trivial policy

The environment exposes a **fixed action space** and an **action mask**. You can select any index where mask==1. For speed, use legal action ids instead of scanning the full mask.

```python
from pathlib import Path
import numpy as np
import weiss_sim

fixture_dir = Path("python/tests/fixtures")
db_path = fixture_dir / "cards.wsdb"

pool = weiss_sim.EnvPool.new_rl_train(
    1,
    str(db_path),
    deck_lists=[[1] * 50, [2] * 50],
    deck_ids=[1, 2],
    max_decisions=200,
    max_ticks=10_000,
    seed=123,
    num_threads=None,  # set to an int to pin a dedicated Rayon pool
)

buf = weiss_sim.EnvPoolBuffers(pool)
buf.reset()

for _ in range(10):
    ids_flat, offsets = buf.legal_action_ids()
    start, end = int(offsets[0]), int(offsets[1])
    action = int(ids_flat[start])
    actions = np.array([action], dtype=np.uint32)
    buf.step(actions)
```

Debug print (very lightweight):

```python
print(pool.render_ansi(env_index=0, perspective=0))
```

Examples:

```bash
python python/examples/sb3_maskable_ppo.py
python python/examples/cleanrl_maskable_ppo.py
python python/examples/bench_python_boundary.py --num-envs 256 --steps 5000 --mode both
```

---

## Python API (what you get)

The extension module is `weiss_sim` and the package re-exports it as `import weiss_sim`.

### `weiss_sim.EnvPool`

Constructors (classmethods):

```python
EnvPool.new_rl_train(
    num_envs: int,
    db_path: str,
    deck_lists: list[list[int]],
    deck_ids: list[int] | None = None,
    max_decisions: int = 10_000,
    max_ticks: int = 100_000,
    seed: int = 0,
    reward_json: str | None = None,
    num_threads: int | None = None,
)

EnvPool.new_rl_eval(...)
EnvPool.new_debug(...)
```

Minimal RL stepping uses `BatchOutMinimal`:

```python
out = weiss_sim.BatchOutMinimal(num_envs)
pool.reset_into(out)
pool.step_into(actions, out)
```

Core methods:
- `reset_into(out: BatchOutMinimal) -> None`
- `reset_indices_into(indices: list[int], out: BatchOutMinimal) -> None`
- `reset_done_into(done_mask: np.ndarray[bool], out: BatchOutMinimal) -> None`
- `step_into(actions: np.ndarray[uint32], out: BatchOutMinimal) -> None`
- `step_debug_into(actions: np.ndarray[uint32], out: BatchOutDebug) -> None`
- `reset_debug_into(out: BatchOutDebug) -> None`
- `reset_indices_debug_into(indices: list[int], out: BatchOutDebug) -> None`
- `legal_action_ids_into(ids: np.ndarray[uint16], offsets: np.ndarray[uint32]) -> int`
- `auto_reset_on_error_codes_into(engine_status: np.ndarray[uint8], out: BatchOutMinimal) -> int`
- `engine_error_reset_count() -> int`
- `reset_engine_error_reset_count() -> None`

Debug helpers:
- `action_lookup_batch() -> list[list[dict | None]]`
- `describe_action_ids(action_ids: list[int]) -> list[dict | None]`
- `decision_info_batch() -> list[dict]`
- `state_fingerprint_batch() -> np.ndarray[uint64]`
- `events_fingerprint_batch() -> np.ndarray[uint64]`
- `render_ansi(env_index: int, perspective: int) -> str`

Convenience properties:
- `action_space: int`
- `obs_len: int`
- `num_envs: int`

Python helper:
- `EnvPoolBuffers(pool)` allocates persistent numpy buffers and exposes `reset()`, `step()`, and `legal_action_ids()`.
- `reset_rl(pool)` / `step_rl(pool, actions)` return a `RlStep` dataclass with named fields.
- `pass_action_id_for_decision_kind(decision_kind)` returns `PASS_ACTION_ID` for convenience.

---

## Encodings (stable + versioned)

Encodings are deterministic and **explicitly versioned**:

- `weiss_sim.OBS_ENCODING_VERSION` (currently 1)
- `weiss_sim.ACTION_ENCODING_VERSION` (currently 1)

### Observation tensor

Observations are fixed-length `int32` arrays. Query the current length via:

- `weiss_sim.OBS_LEN` or `pool.obs_len`

Visibility modes (`observation_visibility`):
- `"public"`: opponent hidden zones are masked (filled with `-1`).
- `"full"`: opponent hidden zones are revealed.

The header includes active player, phase, decision kind/player, last action, attack context, and choice pagination metadata. After the per-player blocks (perspective player first), the encoder appends:

- **Reason bits**: public-safe flags for phase/resource/target gating.
- **Reveal history**: recent revealed card ids for the observing player.
- **Context bits**: priority/choice/stack/encore context.

Exact layout is defined in `weiss_core/src/encode.rs` and versioned by `OBS_ENCODING_VERSION`.

### Action space

Actions are fixed to `ACTION_SPACE_SIZE` (`pool.action_space`). Families include:

- pass (contextual; `PASS_ACTION_ID`)
- mulligan confirm / mulligan select
- clock
- main play character/event/climax, moves, activated abilities
- attack / counter
- choice select + pagination
- level up / encore
- trigger order
- concede (only legal when `allow_concede=true`)

The legal-action **mask** is derived from the canonical `ActionDesc` list and mapped to ids in a versioned way (`ACTION_ENCODING_VERSION`).

---


## Performance & throughput

Rust core is already extremely fast; Python-side overhead can dominate. 


Practical performance tips:
- Use into-buffer APIs (`reset_into`, `step_into`) with preallocated buffers.
- Prefer **legal action ids** over mask scans in Python.
- Pin `num_threads` if you want repeatable multicore behavior.

---

## RL-safe defaults (recommended)


- `EnvPool.new_rl_train(...)` for training and `EnvPool.new_rl_eval(...)` for eval.
- `observation_visibility = Public`
- `enable_visibility_policies = true`
- `allow_concede = false`
- `priority_allow_pass = true`
- `strict_priority_mode = false`
- `enable_priority_windows = false` unless you explicitly train with priority timing windows
- Treat timeouts as **truncations** (bootstrap value)

If you need to deviate, document the assumption and update the PPO guide.

---

## Replays (WSR1)

Replays are binary `WSR1` files written via `ReplayWriter` and serialized with `postcard`.

What gets recorded:
- **Header**: obs/action encoding versions, replay schema version, seed, starting player, deck ids, curriculum id, config hash
- **Body**: action sequence, per-step metadata (actor/decision kind/flags), optional event stream, final snapshot (terminal + state hash)

Replay sampling can be enabled from Rust (`EnvPool::enable_replay_sampling`). The Python
binding does not currently expose replay sampling toggles.

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

See `weiss_core/src/db.rs` for the `CardStatic` schema and supported ability templates.

### Scraper + converter pipeline (full card set)

The full EN card set is produced by the scraper + converter pipeline:

- Scrape: `scraper/scrape.py` → `scraper/out/cards.jsonl`
- Convert: `scraper/convert.py` → `scraper/out/cards.json` + `scraper/out/cards_raw.json`
- Pack: `carddb_pack` → `scraper/out/cards.wsdb`

Smoke check with Python:

```bash
PYTHONPATH=python python python/wsdb_smoke.py
```

---

## Project status (implemented vs simplified)

This is a simulator core built for RL training and determinism first. Some rule systems are intentionally simplified or diverge from the physical game.

Examples of current intentional simplifications/deviations:
- local priority/stack model; official check-timing/play-timing subtleties are not fully replicated
- card text is limited to implemented `AbilityTemplate` variants (no free-form text engine)
- trigger icon semantics are simplified :
  - Draw is mandatory (not optional “may”)
  - Shot resolves as immediate 1 damage (not delayed on cancel)
  - Bounce returns a character from **your** stage (not opponent’s)
- deck-top search/reveal modeled as top‑N reveal to controller

---


## License

Dual-licensed under **MIT OR Apache-2.0** (see workspace metadata in `Cargo.toml`).
