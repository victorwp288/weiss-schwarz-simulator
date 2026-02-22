# Python API Guide

This page documents the current `weiss_sim` Python API.

- High-level API: `make()`, `fast()`, `inspect()` -> returns `WeissEnv`
- Low-level API: `EnvPool` + buffer classes for direct batched stepping

## Import

```python
import weiss_sim
```

## High-level API (`WeissEnv`)

### Entry points

- `weiss_sim.make(...)`
- `weiss_sim.fast(...)` (`mode="fast"` fixed)
- `weiss_sim.inspect(...)` (`mode="inspect"` fixed)

Legacy entry points `create()`, `train()`, and `evaluate()` are removed.

### Python API reference (generated)

For an exhaustive, always-in-sync list of exported names and signatures, see
[Python API Reference (generated)](python_api_reference.md).

### `make()` signature (generated)

Regenerate this snippet with:

```bash
python scripts/gen_docs_snippets.py --write
```

<!-- GENERATED:MAKE_SIGNATURE:START -->
```python
weiss_sim.make(
    mode="fast",
    deck=None,
    opponent_deck=None,
    db_path=None,
    rules_profile="strict",
    card_pool="parsed_only",
    curriculum=None,
    reward_json=None,
    end_condition_policy=None,
    observation_visibility="public",
    reveal_opponent_hand_stock_counts=None,
    legal_repr=None,
    obs_dtype=None,
    ids_safety=None,
    num_envs=1,
    num_threads="auto",
    seed=None,
    max_decisions=2000,
    max_ticks=100_000,
    error_policy="replace",
    control_seat=None,
)
```
<!-- GENERATED:MAKE_SIGNATURE:END -->

### Mode defaults

| mode | internal runtime_mode | legal_repr | obs_dtype | ids_safety |
| --- | --- | --- | --- | --- |
| `fast` | `speed` | `ids_u16` | `i16` | `checked` |
| `inspect` | `eval_debug` | `both` | `i32` | n/a |

`runtime_mode=` is rejected at the high-level API; use `mode="fast"` or `mode="inspect"`.

### Seed behavior

- `seed=None` (default): seed comes from entropy (`seed_source="entropy"`).
- `seed=<int>`: deterministic start seed (`seed_source="user"`).
- Deterministic replay requires fixed seed, fixed decks/config, and fixed action sequence.

### Error policy tokens

`make()` accepts exactly:

- `"raise"`
- `"replace"`
- `"terminate"`

Legacy policy aliases are removed.

## 0.6 migration notes

Breaking API updates in `0.6.0`:

- `make(..., error_policy=...)` now accepts only `raise | replace | terminate`.
- deprecated `**kwargs` compatibility shim in `make(...)` was removed; unknown kwargs now raise native `TypeError`.
- high-level `runtime_mode=...` is no longer accepted.
- coverage/script profile aliases `none` and `rl_v1` were removed; use `strict` or `approx`.

### High-level method rename policy

Canonical names now use clearer intent-focused verbs:

- `LegalActions.first()` -> `LegalActions.first_legal()`
- `LegalActions.select_from_logits()` -> `LegalActions.argmax_logits()`
- `LegalActions.sample_from_logits()` -> `LegalActions.sample_logits()`
- `WeissEnv.step_random_legal()` -> `WeissEnv.step_uniform_legal()`
- `WeissEnv.step_select_from_logits()` -> `WeissEnv.step_argmax_logits()`
- `WeissEnv.step_sample_from_logits()` -> `WeissEnv.step_sample_logits()`

Old names are intentionally removed in 0.6; update callsites to the canonical names above.

### Deck input forms

Accepted for `deck` and `opponent_deck`:

- `Sequence[int]`
- `Mapping[int|str, int]`
- preset string (for example `"preset:starter_v1"`)
- path-like string / `Path`

`card_pool="parsed_only"` enforces packaged catalog compatibility and may raise `DbMismatchError`.

### Deck authoring helpers

The `cards` namespace exposes higher-level authoring and validation helpers:

- `weiss_sim.cards.builder(initial=None) -> DeckBuilder`
- `weiss_sim.cards.validate_deck(...) -> DeckValidationReport`
- `weiss_sim.cards.suggest(query, limit=5)`
- `weiss_sim.cards.export_deck(...)`
- `weiss_sim.cards.save_deck(path, ...)`
- `weiss_sim.cards.load_deck(path)`

Minimal builder flow:

```python
import weiss_sim

b = weiss_sim.cards.builder()
b.add("CARD-1", 4).add("CARD-2", 4)
report = b.validate(rules_profile="approx", card_pool="all")
if report.ok:
    ids = b.build(rules_profile="approx", card_pool="all")
```

`DeckValidationReport` includes `ok`, `resolved_ids`, and structured `errors`/`warnings`
where each issue has a stable `code` (for example `deck_length`, `unknown_card`,
`copy_count_exceeded`, `climax_count_at_limit`).

### `WeissEnv` methods

- `reset(seed=None, indices=None) -> ResetBatch`
- `step(actions) -> StepBatch`
- `step_first_legal() -> (StepBatch, np.ndarray actions)`
- `step_uniform_legal(seed=None) -> (StepBatch, np.ndarray actions)`
- `step_auto(actions=None, policy="first", seed=None, reset_done=True, reset_engine_errors=True) -> (StepBatch, np.ndarray actions, ResetBatch | None)`
- `rollout(steps, policy="uniform"|callable, seed=None, auto_reset=False) -> list[StepBatch]`
- `spec() -> dict`
- `effective_config() -> dict`
- `current_to_play_seat() -> np.ndarray`
- `merge_actions_by_seat(seat0_actions, seat1_actions, default_action=...)`
- `step_by_seat(seat0_actions, seat1_actions, default_action=...)`
- `step_argmax_logits(logits, illegal_value=-1e9) -> (StepBatch, np.ndarray actions)`
- `step_sample_logits(logits, seed=None, temperature=1.0, illegal_value=-1e9) -> (StepBatch, np.ndarray actions)`
- `auto_reset_on_engine_errors(codes=None) -> (int reset_count, ResetBatch | None)`
- `enable_replay_sampling(sample_rate, out_dir=None, compress=False, include_trigger_card_id=False, visibility_mode=None, store_actions=True) -> None`

Both logits helpers return `(step, actions)` where `actions` is a `np.ndarray` of shape `(num_envs,)` with `dtype=uint32`.

### Logits helper semantics

- Default path: `illegal_value=-1e9` keeps the Rust logits fast path enabled for both select and sample helpers.
- Compatibility path: non-default `illegal_value` keeps legacy masking semantics (materialized compatibility mode) but is slower and can add Python-side allocation overhead.
- Sampling at zero temperature: `temperature=0.0` is deterministic argmax (equivalent action selection behavior to `step_argmax_logits`).

For throughput-sensitive training loops, keep `illegal_value` at the default unless you explicitly need compatibility masking behavior.

### Legal actions: primary path (`batch.legal`)

Use `batch.legal` as the main integration surface.

Common helpers:

- `batch.legal.ids(env_i)`
- `batch.legal.contains(env_i, action_id)`
- `batch.legal.first_legal(default_action=...)`
- `batch.legal.choose(strategy, logits=..., seed=..., default_action=...)`
- `batch.legal.mask` (dense mask view)
- `batch.legal.argmax_logits(logits)`
- `batch.legal.sample_logits(logits, seed=...)`
- `batch.legal.sample_uniform(seed=...)`

`ResetBatch` and `StepBatch` still expose `legal_ids` and `legal_offsets` as properties for interoperability, but most code should consume `batch.legal`.

### Minimal high-level loop (preferred)

```python
import weiss_sim

with weiss_sim.make(mode="inspect", num_envs=8, seed=7, card_pool="all") as sim:
    batch = sim.reset()
    actions = batch.legal.sample_uniform(seed=123)
    step = sim.step(actions)
    print(step.obs.shape, step.reward.dtype, step.engine_status[:4])
```

### `effective_config()` keys worth using

The returned dict includes:

- mode/runtime knobs (`mode`, `runtime_mode`, `legal_repr`, `obs_dtype`, `ids_safety`)
- sizing (`num_envs`, `num_threads`)
- determinism (`seed`, `seed_source`)
- policy/config (`rules_profile`, `card_pool`, `curriculum`, `error_policy`, `end_condition_policy`)
- resolved deck visibility (`resolved_decks.player` / `resolved_decks.opponent` with `ids`, per-slot `cards`, and aggregated `counts`)
- db compatibility (`db` object with hashes and match status)
- compatibility/runtime metadata (`spec_hash`, `action_space`, `reward_timeout_policy`)

### Advanced: raw packed legality

If you need zero-overhead packed legality arrays, consume batch properties directly:

- `batch.legal_ids`
- `batch.legal_offsets`
- optional `batch.legal_mask`

Example packed-id first-legal selection:

```python
num_envs = int(len(batch.legal_offsets) - 1)
actions = np.full((num_envs,), weiss_sim.PASS_ACTION_ID, dtype=np.uint32)
for i in range(num_envs):
    start = int(batch.legal_offsets[i])
    end = int(batch.legal_offsets[i + 1])
    if end > start:
        actions[i] = int(batch.legal_ids[start])
```

### Debug helpers

- `render(env_i=0) -> str`: compact single-env debug view (mode `"ansi"` only)
- `decode_action(action_id) -> dict`: decode an action id into a structured dict (family + params)

### Runtime error auto-reset helper

Use `WeissEnv.auto_reset_on_engine_errors(...)` to reset only faulted envs (`engine_status != 0`) after a step, instead of wiring low-level pool helpers yourself.
`StepBatch.needs_reset` is available as a convenience boolean vector (`done | (engine_status != 0)`).
Index helpers are also available: `step.done_indices`, `step.error_indices`, and `step.needs_reset_indices`.

```python
import weiss_sim

with weiss_sim.fast(num_envs=32, seed=7, card_pool="all") as sim:
    batch = sim.reset()
    actions = batch.legal.sample_uniform(seed=11)
    step = sim.step(actions)
    if (step.engine_status != 0).any():
        reset_count, reset_batch = sim.auto_reset_on_engine_errors(step.engine_status)
        if reset_count:
            batch = reset_batch
```

### Adapters

Optional adapters live in `python/weiss_sim/adapters.py`:

- `WeissEnv.as_single_env()` (single-environment adapter)
- `WeissEnv.as_gym()` / `WeissEnv.as_gym_single()` (Gym/Gymnasium adapters, if installed)

## Low-level API (canonical layout-based surface)

### Constructor

Use `make_pool(...)` as the canonical low-level entrypoint:

```python
pool, buffers = weiss_sim.make_pool(
    mode="train",                 # "train" or "eval"
    num_envs=64,
    db_path=None,
    deck_lists=[deck_a, deck_b],  # required
    deck_ids=[1, 2],
    max_decisions=2000,
    max_ticks=100_000,
    seed=0,
    profile="fast",               # optional: fast / balanced / eval / debug
    rollout_steps=None,           # set int to receive EnvPoolTrajectoryBuffers
    layout="i16_legal_ids",       # mask / nomask / i16 / i16_legal_ids
)
```

`make_pool(...)` returns:

- `(pool, EnvPoolBuffers(...))` when `rollout_steps=None`
- `(pool, EnvPoolTrajectoryBuffers(...))` when `rollout_steps=<int>`

### Canonical buffer classes

- `EnvPoolBuffers(pool, layout=...)`
- `EnvPoolTrajectoryBuffers(pool, steps, layout=...)`

Common methods on `EnvPoolBuffers`:

- `reset()`, `reset_indices(...)`, `reset_done(...)`
- `step(actions)`
- `step_select_from_logits(logits) -> (StepBatch, np.ndarray actions)`
- `step_sample_from_logits(logits, seeds) -> (StepBatch, np.ndarray actions)`
- `legal_action_ids()` / `legal_action_ids_and_sample_uniform(seeds)`

`EnvPoolTrajectoryBuffers` methods:

- `rollout_first_legal()`
- `rollout_random_legal(seeds)`

### Canonical RL helper functions

- `reset_rl(pool, layout=..., out=None)`
- `step_rl(pool, actions, layout=..., out=None)`
- `step_rl_select_from_logits(pool, logits, layout=..., actions=None, out=None)`
- `step_rl_sample_from_logits(pool, logits, seeds, layout=..., actions=None, out=None)`

### Advanced low-level debug output (`BatchOutDebug`)

`BatchOutDebug` remains a public, low-level API for deep engine inspection.

Use it with debug pool entrypoints:

- `pool.reset_debug_into(out_debug)`
- `pool.step_debug_into(actions, out_debug)`
- `weiss_sim.make_batch_out_debug(pool, event_capacity=None)` allocates `BatchOutDebug` with pool-aware defaults.

`BatchOutDebug` includes full debug-facing fields (`obs`, `masks`, `rewards`, `terminated`, `truncated`, `actor`, `decision_kind`, `decision_id`, `engine_status`, `spec_hash`) and is intended for advanced tooling rather than standard RL loops.

### Layouts

| layout | masks | legal ids | obs dtype |
| --- | --- | --- | --- |
| `mask` | yes | no | i32 |
| `nomask` | no | no | i32 |
| `i16` | yes | no | i16 |
| `i16_legal_ids` | no | yes | i16 |

### Legacy cleanup status

The public low-level API is intentionally consolidated around `make_pool`, the two canonical buffer classes, and the four canonical RL helpers.

## Card/catalog helpers

- `weiss_sim.cards.search(query, limit=20)`
- `weiss_sim.cards.suggest(query, limit=5)`
- `weiss_sim.cards.get(identifier)`
- `weiss_sim.cards.presets()`
- `weiss_sim.cards.builder(initial=None)`
- `weiss_sim.cards.resolve_deck(...)`
- `weiss_sim.cards.validate_deck(...)`
- `weiss_sim.cards.describe_deck(...)` (returns resolved ids plus card metadata/counts)
- `weiss_sim.cards.export_deck(...)`
- `weiss_sim.cards.save_deck(path, ...)`
- `weiss_sim.cards.load_deck(path)`
- `weiss_sim.db_info(db_path=None)`

## League/population helpers

- `round_robin_schedule(...)`
- `sample_population_schedule(...)`
- `records_from_step(...)`
- `summarize_records(...)`
- `summarize_first_player_bias(...)`
- `summarize_clock_greed_from_replay(...)`
- `rank_agents(...)`

## Key exported constants/helpers

- `OBS_LEN`
- `ACTION_SPACE_SIZE`
- `PASS_ACTION_ID`
- `SPEC_HASH`
- `POLICY_VERSION`
- `observation_spec_json()`
- `action_spec_json()`
- `spec_bundle()` / `export_spec_bundle()`

## Error types

Important exceptions:

- `ConfigConflictError`
- `DbMismatchError`
- `DeckSpecError`
- `DeckValidationError`
- `CardLookupError`
- `WeissSimError`
