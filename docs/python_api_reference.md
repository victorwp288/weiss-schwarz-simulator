# Python API Reference (generated)

This page is an **auto-generated reference** for the public `weiss_sim` Python surface.

Goals:

- keep “what is exported and how do I call it?” always in sync with code
- give a stable, linkable reference for names, signatures, and short docstrings

If this page and code disagree, the code is authoritative. Regenerate this file with:

```bash
python scripts/gen_docs_snippets.py --write
```

Behavior notes:

- `decode_action_id(...)` and `decode_factorized_action_id(...)` return `None`
  for out-of-range or otherwise unknown action ids.
- `BatchOutTrajectoryI16LegalIds` is the only public trajectory type that exposes
  per-step `episode_seed`.

<!-- GENERATED:PYTHON_API_REFERENCE:START -->
## Constants & versions

These values are compatibility boundaries; see [RL Contract](rl_contract.md) for the checksum table.

- `ACTION_SPACE_SIZE: int`
- `ACTION_META_WIDTH: int`
- `ACTION_META_UNUSED: int`
- `OBS_LEN: int`
- `SPEC_HASH: int`
- `POLICY_VERSION: int`
- `PASS_ACTION_ID: int`
- `ACTOR_NONE: int`
- `DECISION_KIND_NONE: int`
- `__version__: str`

## Specs & metadata

### `observation_spec_json`
```python
def observation_spec_json() -> str: ...
```

### `action_spec_json`
```python
def action_spec_json() -> str: ...
```

### `decode_action_id`
```python
def decode_action_id(action_id: int) -> dict[str, object] | None: ...
```

### `decode_factorized_action_id`
```python
def decode_factorized_action_id(action_id: int) -> dict[str, object] | None: ...
```

### `encode_factorized_action`
```python
def encode_factorized_action(
    family: str,
    arg0: int | None = ...,
    arg1: int | None = ...,
    arg2: int | None = ...,
) -> int | None: ...
```

### `build_info`
```python
def build_info() -> dict[str, object]: ...
```

### `spec_bundle`
Return the current observation/action spec bundle as a Python dict.

```python
def spec_bundle() -> dict[str, object]: ...
```

### `export_spec_bundle`
Export the current observation/action specs and compatibility hashes.

```python
def export_spec_bundle() -> dict[str, object]: ...
```

### `export_card_table`
Export static per-card features for structured policy encoders.

```python
def export_card_table(db_path: str | Path | None = None) -> dict[str, object]: ...
```

### `db_info`
Return hash/compatibility metadata for the selected card database.

```python
def db_info(db_path: str | Path | None = None) -> dict[str, object]: ...
```

## High-level API

### `make`
Create a high-level `WeissEnv` for batched reset/step loops.

```python
def make(
    *,
    mode: Literal["fast", "inspect"] = "fast",
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | Mapping[str, object] | None = None,
    reward_json: str | Mapping[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = 1,
    num_threads: ThreadsLike = "auto",
    seed: int | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: Literal["raise", "replace", "terminate"] = "replace",
    control_seat: Literal[0, 1] | None = None,
) -> WeissEnv: ...
```

### `fast`
Shortcut for `make(mode="fast", ...)`.

```python
def fast(
    *,
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | Mapping[str, object] | None = None,
    reward_json: str | Mapping[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = 1,
    num_threads: ThreadsLike = "auto",
    seed: int | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: Literal["raise", "replace", "terminate"] = "replace",
    control_seat: Literal[0, 1] | None = None,
) -> WeissEnv: ...
```

### `inspect`
Shortcut for `make(mode="inspect", ...)`.

```python
def inspect(
    *,
    deck: DeckInput | None = None,
    opponent_deck: DeckInput | None = None,
    db_path: str | None = None,
    rules_profile: RulesProfile = "strict",
    card_pool: CardPoolMode = "parsed_only",
    curriculum: CurriculumOverrides | Mapping[str, object] | None = None,
    reward_json: str | Mapping[str, object] | None = None,
    end_condition_policy: EndConditionOverrides | Mapping[str, object] | str | None = None,
    observation_visibility: ObservationVisibility = "public",
    reveal_opponent_hand_stock_counts: bool | None = None,
    legal_repr: LegalRepr | None = None,
    obs_dtype: ObsDType | None = None,
    ids_safety: IdsSafety | None = None,
    num_envs: NumLike = 1,
    num_threads: ThreadsLike = "auto",
    seed: int | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    error_policy: Literal["raise", "replace", "terminate"] = "replace",
    control_seat: Literal[0, 1] | None = None,
) -> WeissEnv: ...
```

### `WeissEnv`
<details>
<summary><code>WeissEnv</code></summary>

High-level wrapper around `EnvPool` for batched RL-style stepping.

Methods:
- `def action_space(self) -> int: ...`
- `def action_space_n(self) -> int: ...`
- `def as_gym(self) -> GymVectorEnvAdapter: ...`
- `def as_single_env(self) -> SingleEnvAdapter: ...`
- `def auto_reset_on_engine_errors(
    self,
    codes: np.ndarray | None = None,
) -> tuple[int, ResetBatch | None]: ...`
- `def close(self) -> None: ...`
- `def current_to_play_seat(self) -> np.ndarray: ...`
- `def decode_action(self, action_id: int) -> dict[str, object] | None: ...`
- `def effective_config(self) -> dict[str, object]: ...`
- `def enable_replay_sampling(
    self,
    sample_rate: float,
    out_dir: str | None = None,
    compress: bool = False,
    include_trigger_card_id: bool = False,
    visibility_mode: str | None = None,
    store_actions: bool = True,
) -> None: ...`
- `def latest_batch(self) -> ResetBatch | StepBatch | None: ...`
- `def legal(self) -> LegalActions: ...`
- `def merge_actions_by_seat(
    self,
    seat0_actions,
    seat1_actions,
    *,
    default_action: int | None = None,
) -> np.ndarray: ...`
- `def num_envs(self) -> int: ...`
- `def obs_shape(self) -> tuple[int, ...]: ...`
- `def render(self, env_i: int = 0, mode: Literal["ansi"] = "ansi") -> str: ...`
- `def reset(self, *, seed: int | None = None, indices: object | None = None) -> ResetBatch: ...`
- `def reset_done(self, done_mask: object) -> ResetBatch: ...`
- `def reset_indices(self, indices: object) -> ResetBatch: ...`
- `def rollout(
    self,
    steps: int,
    *,
    policy: Literal["first", "uniform", "random"]
        | Callable[[ResetBatch | StepBatch], object] = "uniform",
    seed: int | np.ndarray | None = None,
    auto_reset: bool = False,
    reset_done: bool = True,
    reset_engine_errors: bool = True,
) -> list[StepBatch]: ...`
- `def spec(self) -> dict[str, object]: ...`
- `def step(self, actions: object) -> StepBatch: ...`
- `def step_argmax_logits(
    self,
    logits: np.ndarray,
    illegal_value: float = -1e9,
) -> tuple[StepBatch, np.ndarray]: ...`
- `def step_auto(
    self,
    actions: object | None = None,
    *,
    policy: Literal["first", "uniform", "random"] = "first",
    seed: int | np.ndarray | None = None,
    reset_done: bool = True,
    reset_engine_errors: bool = True,
) -> tuple[StepBatch, np.ndarray, ResetBatch | None]: ...`
- `def step_by_seat(
    self,
    seat0_actions,
    seat1_actions,
    *,
    default_action: int | None = None,
) -> StepBatch: ...`
- `def step_first_legal(self) -> tuple[StepBatch, np.ndarray]: ...`
- `def step_sample_logits(
    self,
    logits: np.ndarray,
    seed: int | np.ndarray | None = None,
    temperature: float = 1.0,
    illegal_value: float = -1e9,
) -> tuple[StepBatch, np.ndarray]: ...`
- `def step_uniform_legal(
    self,
    seed: int | np.ndarray | None = None,
) -> tuple[StepBatch, np.ndarray]: ...`

</details>

### `ResetBatch`
<details>
<summary><code>ResetBatch</code></summary>

Fields:
- `obs: np.ndarray`
- `to_play_seat: np.ndarray`
- `starting_seat: np.ndarray`
- `episode_seed: np.ndarray`
- `episode_index: np.ndarray`
- `env_index: np.ndarray`
- `episode_key: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `legal_mask: np.ndarray | None = None`
- `legal_ids: np.ndarray | None = None`
- `legal_offsets: np.ndarray | None = None`
- `legal_action_meta: np.ndarray | None = None`
- `_legal_cache: LegalActions | None = field(default=None, init=False, repr=False, compare=False)`

</details>

### `StepBatch`
<details>
<summary><code>StepBatch</code></summary>

Fields:
- `obs: np.ndarray`
- `to_play_seat: np.ndarray`
- `starting_seat: np.ndarray`
- `episode_seed: np.ndarray`
- `episode_index: np.ndarray`
- `env_index: np.ndarray`
- `episode_key: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `reward: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `terminal_during_internal_opponent: np.ndarray`
- `decision_count: np.ndarray`
- `tick_count: np.ndarray`
- `no_progress_count: np.ndarray = field(default_factory=lambda: np.zeros((0,), dtype=np.uint32))`
- `main_move_action: np.ndarray = field(default_factory=lambda: np.zeros((0,), dtype=np.bool_))`
- `main_pass_action: np.ndarray = field(default_factory=lambda: np.zeros((0,), dtype=np.bool_))`
- `legal_mask: np.ndarray | None = None`
- `legal_ids: np.ndarray | None = None`
- `legal_offsets: np.ndarray | None = None`
- `legal_action_meta: np.ndarray | None = None`
- `_legal_cache: LegalActions | None = field(default=None, init=False, repr=False, compare=False)`

Methods:
- `def done(self) -> np.ndarray: ...`
- `def done_indices(self) -> np.ndarray: ...`
- `def error_indices(self) -> np.ndarray: ...`
- `def needs_reset(self) -> np.ndarray: ...`
- `def needs_reset_indices(self) -> np.ndarray: ...`

</details>

### `LegalActions`
<details>
<summary><code>LegalActions</code></summary>

Convenience helpers for consuming legal actions.

Fields:
- `legal_ids: np.ndarray | None`
- `legal_offsets: np.ndarray | None`
- `legal_mask_raw: np.ndarray | None`
- `legal_action_meta: np.ndarray | None = None`

Methods:
- `def action_space(self) -> int: ...`
- `def argmax_logits(self, logits: np.ndarray, illegal_value: float = -1e9) -> np.ndarray: ...`
- `def choose(
    self,
    strategy: Literal["first", "uniform", "random", "argmax", "select", "sample"] = "first",
    *,
    logits: np.ndarray | None = None,
    seed: int | np.ndarray | None = None,
    temperature: float = 1.0,
    illegal_value: float = -1e9,
    default_action: int | None = None,
) -> np.ndarray: ...`
- `def contains(self, i: int, action_id: int) -> bool: ...`
- `def first_legal(self, default_action: int | None = None) -> np.ndarray: ...`
- `def ids(self, i: int) -> np.ndarray: ...`
- `def iter_ids(self) -> Iterator[np.ndarray]: ...`
- `def mask(self) -> np.ndarray | None: ...`
- `def mask_for_action_space(self, action_space: int) -> np.ndarray: ...`
- `def mask_logits(self, logits: np.ndarray, illegal_value: float = -1e9) -> np.ndarray: ...`
- `def meta(self, i: int) -> np.ndarray: ...`
- `def num_envs(self) -> int: ...`
- `def sample_logits(
    self,
    logits: np.ndarray,
    seed: int | np.ndarray | None = None,
    temperature: float = 1.0,
    illegal_value: float = -1e9,
) -> np.ndarray: ...`
- `def sample_uniform(self, seed: int | np.ndarray | None = None) -> np.ndarray: ...`

</details>

## Low-level API

### `EnvPool`
<details>
<summary><code>EnvPool</code></summary>

Fields:
- `envs_len: int`
- `num_envs: int`
- `obs_len: int`
- `action_space: int`
- `num_threads: int`

Methods:
- `def action_lookup_batch(self) -> list[list[dict[str, Any] | None]]: ...`
- `def action_mask_bits_batch(self) -> np.ndarray: ...`
- `def auto_reset_on_error_codes_into(self, codes: np.ndarray, out: BatchOutMinimal) -> int: ...`
- `def auto_reset_on_error_codes_into_nomask(
    self,
    codes: np.ndarray,
    out: BatchOutMinimalNoMask,
) -> int: ...`
- `def choose_heuristic_public_actions_into(
    self,
    env_indices: np.ndarray,
    actions_out: np.ndarray,
) -> None: ...`
- `def config_hash(self) -> int: ...`
- `def debug_event_ring_capacity(self) -> int: ...`
- `def decision_count_batch(self) -> np.ndarray: ...`
- `def decision_info_batch(self) -> list[dict[str, Any]]: ...`
- `def describe_action_ids(self, action_ids: Sequence[int]) -> list[dict[str, Any] | None]: ...`
- `def enable_replay_sampling(
    self,
    sample_rate: float,
    out_dir: str | None = ...,
    compress: bool = ...,
    include_trigger_card_id: bool = ...,
    visibility_mode: str | None = ...,
    store_actions: bool = ...,
) -> None: ...`
- `def engine_error_reset_count(self) -> int: ...`
- `def env_index_batch(self) -> np.ndarray: ...`
- `def episode_index_batch(self) -> np.ndarray: ...`
- `def episode_seed_batch(self) -> np.ndarray: ...`
- `def events_fingerprint_batch(self) -> np.ndarray: ...`
- `def i16_overflow_count(self) -> int: ...`
- `def legal_action_ids_and_sample_uniform_into(
    self,
    legal_ids: np.ndarray,
    legal_offsets: np.ndarray,
    seeds: np.ndarray,
    actions_out: np.ndarray,
) -> int: ...`
- `def legal_action_ids_into(self, legal_ids: np.ndarray, legal_offsets: np.ndarray) -> int: ...`
- `def legal_action_meta_into(self, legal_action_meta: np.ndarray) -> int: ...`
- `def max_card_id(self) -> int: ...`
- `def new_debug(
    cls,
    num_envs: int,
    db_path: str | None = ...,
    *,
    deck_lists: list[list[int]],
    deck_ids: list[int] | None = ...,
    max_decisions: int = ...,
    max_ticks: int = ...,
    seed: int = ...,
    curriculum_json: str | None = ...,
    reward_json: str | None = ...,
    end_condition_policy_json: str | None = ...,
    error_policy: str | None = ...,
    observation_visibility: str | None = ...,
    num_threads: int | None = ...,
    debug_fingerprint_every_n: int = ...,
    debug_event_ring_capacity: int = ...,
) -> EnvPool: ...`
- `def new_rl_eval(
    cls,
    num_envs: int,
    db_path: str | None = ...,
    *,
    deck_lists: list[list[int]],
    deck_ids: list[int] | None = ...,
    max_decisions: int = ...,
    max_ticks: int = ...,
    seed: int = ...,
    curriculum_json: str | None = ...,
    reward_json: str | None = ...,
    end_condition_policy_json: str | None = ...,
    error_policy: str | None = ...,
    observation_visibility: str | None = ...,
    num_threads: int | None = ...,
    output_masks: bool = ...,
    debug_fingerprint_every_n: int = ...,
    debug_event_ring_capacity: int = ...,
) -> EnvPool: ...`
- `def new_rl_train(
    cls,
    num_envs: int,
    db_path: str | None = ...,
    *,
    deck_lists: list[list[int]],
    deck_ids: list[int] | None = ...,
    max_decisions: int = ...,
    max_ticks: int = ...,
    seed: int = ...,
    curriculum_json: str | None = ...,
    reward_json: str | None = ...,
    end_condition_policy_json: str | None = ...,
    error_policy: str | None = ...,
    observation_visibility: str | None = ...,
    num_threads: int | None = ...,
    output_masks: bool = ...,
    debug_fingerprint_every_n: int = ...,
    debug_event_ring_capacity: int = ...,
) -> EnvPool: ...`
- `def no_progress_count_batch(self) -> np.ndarray: ...`
- `def obs_fingerprint_batch(self) -> np.ndarray: ...`
- `def render_ansi(self, env_index: int, perspective: int) -> str: ...`
- `def reset_debug_into(self, out: BatchOutDebug) -> None: ...`
- `def reset_done_into(self, done_mask: np.ndarray, out: BatchOutMinimal) -> None: ...`
- `def reset_done_into_i16(self, done_mask: np.ndarray, out: BatchOutMinimalI16) -> None: ...`
- `def reset_done_into_i16_legal_ids(
    self,
    done_mask: np.ndarray,
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def reset_done_into_nomask(self, done_mask: np.ndarray, out: BatchOutMinimalNoMask) -> None: ...`
- `def reset_engine_error_reset_count(self) -> None: ...`
- `def reset_i16_overflow_count(self) -> None: ...`
- `def reset_indices_into(self, indices: Sequence[int], out: BatchOutMinimal) -> None: ...`
- `def reset_indices_into_i16(self, indices: Sequence[int], out: BatchOutMinimalI16) -> None: ...`
- `def reset_indices_into_i16_legal_ids(
    self,
    indices: Sequence[int],
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def reset_indices_into_nomask(self, indices: Sequence[int], out: BatchOutMinimalNoMask) -> None: ...`
- `def reset_indices_with_episode_seeds_into(
    self,
    indices: Sequence[int],
    episode_seeds: Sequence[int],
    out: BatchOutMinimal,
) -> None: ...`
- `def reset_indices_with_episode_seeds_into_i16(
    self,
    indices: Sequence[int],
    episode_seeds: Sequence[int],
    out: BatchOutMinimalI16,
) -> None: ...`
- `def reset_indices_with_episode_seeds_into_i16_legal_ids(
    self,
    indices: Sequence[int],
    episode_seeds: Sequence[int],
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def reset_indices_with_episode_seeds_into_nomask(
    self,
    indices: Sequence[int],
    episode_seeds: Sequence[int],
    out: BatchOutMinimalNoMask,
) -> None: ...`
- `def reset_into(self, out: BatchOutMinimal) -> None: ...`
- `def reset_into_i16(self, out: BatchOutMinimalI16) -> None: ...`
- `def reset_into_i16_legal_ids(self, out: BatchOutMinimalI16LegalIds) -> None: ...`
- `def reset_into_nomask(self, out: BatchOutMinimalNoMask) -> None: ...`
- `def reset_timing_counters(self) -> None: ...`
- `def rollout_first_legal_into(self, steps: int, out: BatchOutTrajectory) -> None: ...`
- `def rollout_first_legal_into_i16(self, steps: int, out: BatchOutTrajectoryI16) -> None: ...`
- `def rollout_first_legal_into_i16_legal_ids(
    self,
    steps: int,
    out: BatchOutTrajectoryI16LegalIds,
) -> None: ...`
- `def rollout_first_legal_into_nomask(self, steps: int, out: BatchOutTrajectoryNoMask) -> None: ...`
- `def rollout_heuristic_public_into_i16_legal_ids(
    self,
    steps: int,
    out: BatchOutTrajectoryI16LegalIds,
) -> None: ...`
- `def rollout_sample_legal_action_ids_uniform_into(
    self,
    steps: int,
    seeds: np.ndarray,
    out: BatchOutTrajectory,
) -> None: ...`
- `def rollout_sample_legal_action_ids_uniform_into_i16(
    self,
    steps: int,
    seeds: np.ndarray,
    out: BatchOutTrajectoryI16,
) -> None: ...`
- `def rollout_sample_legal_action_ids_uniform_into_i16_legal_ids(
    self,
    steps: int,
    seeds: np.ndarray,
    out: BatchOutTrajectoryI16LegalIds,
) -> None: ...`
- `def rollout_sample_legal_action_ids_uniform_into_nomask(
    self,
    steps: int,
    seeds: np.ndarray,
    out: BatchOutTrajectoryNoMask,
) -> None: ...`
- `def sample_actions_from_logits_into(
    self,
    logits: np.ndarray,
    seeds: np.ndarray,
    actions_out: np.ndarray,
) -> None: ...`
- `def sample_legal_action_ids_uniform_into(
    self,
    seeds: np.ndarray,
    actions_out: np.ndarray,
) -> None: ...`
- `def sample_legal_actions_uniform(self, seeds: np.ndarray) -> np.ndarray: ...`
- `def select_actions_from_logits_into(self, logits: np.ndarray, actions_out: np.ndarray) -> None: ...`
- `def set_error_policy(self, error_policy: str) -> None: ...`
- `def set_i16_clamp_enabled(self, enabled: bool) -> None: ...`
- `def set_i16_overflow_counter_enabled(self, enabled: bool) -> None: ...`
- `def set_output_mask_bits_enabled(self, enabled: bool) -> None: ...`
- `def set_output_mask_enabled(self, enabled: bool) -> None: ...`
- `def set_timing_enabled(self, enabled: bool) -> None: ...`
- `def starting_player_batch(self) -> np.ndarray: ...`
- `def state_fingerprint_batch(self) -> np.ndarray: ...`
- `def step_debug_into(self, actions: np.ndarray, out: BatchOutDebug) -> None: ...`
- `def step_first_legal_into(self, actions_out: np.ndarray, out: BatchOutMinimal) -> None: ...`
- `def step_first_legal_into_i16(self, actions_out: np.ndarray, out: BatchOutMinimalI16) -> None: ...`
- `def step_first_legal_into_i16_legal_ids(
    self,
    actions_out: np.ndarray,
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def step_first_legal_into_nomask(
    self,
    actions_out: np.ndarray,
    out: BatchOutMinimalNoMask,
) -> None: ...`
- `def step_into(self, actions: np.ndarray, out: BatchOutMinimal) -> None: ...`
- `def step_into_i16(self, actions: np.ndarray, out: BatchOutMinimalI16) -> None: ...`
- `def step_into_i16_legal_ids(self, actions: np.ndarray, out: BatchOutMinimalI16LegalIds) -> None: ...`
- `def step_into_nomask(self, actions: np.ndarray, out: BatchOutMinimalNoMask) -> None: ...`
- `def step_sample_from_logits_into(
    self,
    logits: np.ndarray,
    seeds: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimal,
) -> None: ...`
- `def step_sample_from_logits_into_i16(
    self,
    logits: np.ndarray,
    seeds: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimalI16,
) -> None: ...`
- `def step_sample_from_logits_into_i16_legal_ids(
    self,
    logits: np.ndarray,
    seeds: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def step_sample_from_logits_into_nomask(
    self,
    logits: np.ndarray,
    seeds: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimalNoMask,
) -> None: ...`
- `def step_sample_from_logits_with_logp_into_i16_legal_ids(
    self,
    logits: np.ndarray,
    seeds: np.ndarray,
    actions: np.ndarray,
    action_logp: np.ndarray,
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def step_sample_legal_action_ids_uniform_into(
    self,
    seeds: np.ndarray,
    actions_out: np.ndarray,
    out: BatchOutMinimal,
) -> None: ...`
- `def step_sample_legal_action_ids_uniform_into_i16(
    self,
    seeds: np.ndarray,
    actions_out: np.ndarray,
    out: BatchOutMinimalI16,
) -> None: ...`
- `def step_sample_legal_action_ids_uniform_into_i16_legal_ids(
    self,
    seeds: np.ndarray,
    actions_out: np.ndarray,
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def step_sample_legal_action_ids_uniform_into_nomask(
    self,
    seeds: np.ndarray,
    actions_out: np.ndarray,
    out: BatchOutMinimalNoMask,
) -> None: ...`
- `def step_select_from_logits_into(
    self,
    logits: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimal,
) -> None: ...`
- `def step_select_from_logits_into_i16(
    self,
    logits: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimalI16,
) -> None: ...`
- `def step_select_from_logits_into_i16_legal_ids(
    self,
    logits: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimalI16LegalIds,
) -> None: ...`
- `def step_select_from_logits_into_nomask(
    self,
    logits: np.ndarray,
    actions: np.ndarray,
    out: BatchOutMinimalNoMask,
) -> None: ...`
- `def tick_count_batch(self) -> np.ndarray: ...`
- `def timing_counters(self) -> dict[str, int]: ...`
- `def validate_deck_issues(
    deck_lists: list[list[int]],
    db_path: str | None = ...,
    deck_ids: list[int] | None = ...,
) -> list[dict[str, object]]: ...`

</details>

### `EnvPoolBuffers`
<details>
<summary><code>EnvPoolBuffers</code></summary>

Preallocated numpy buffers for high-throughput stepping.

Methods:
- `def i16_overflow_count(self) -> int: ...`
- `def legal_action_data(self) -> tuple[np.ndarray, np.ndarray | None, np.ndarray]: ...`
- `def legal_action_ids(self) -> tuple[np.ndarray, np.ndarray]: ...`
- `def legal_action_ids_and_sample_uniform(
    self,
    seeds: int | Sequence[int] | np.ndarray,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]: ...`
- `def reset(self) -> MinimalOut: ...`
- `def reset_done(self, done_mask: Sequence[bool] | np.ndarray) -> MinimalOut: ...`
- `def reset_i16_overflow_count(self) -> None: ...`
- `def reset_indices(self, indices: Sequence[int] | np.ndarray) -> MinimalOut: ...`
- `def reset_indices_with_episode_seeds(
    self,
    indices: Sequence[int] | np.ndarray,
    episode_seeds: Sequence[int] | np.ndarray,
) -> MinimalOut: ...`
- `def reset_timing_counters(self) -> None: ...`
- `def sample_actions_from_logits(
    self,
    logits: object,
    seeds: int | Sequence[int] | np.ndarray,
) -> np.ndarray: ...`
- `def select_actions_from_logits(self, logits: object) -> np.ndarray: ...`
- `def set_i16_clamp_enabled(self, enabled: bool) -> None: ...`
- `def set_i16_overflow_counter_enabled(self, enabled: bool) -> None: ...`
- `def set_output_mask_bits_enabled(self, enabled: bool) -> None: ...`
- `def set_output_mask_enabled(self, enabled: bool) -> None: ...`
- `def set_timing_enabled(self, enabled: bool) -> None: ...`
- `def step(self, actions: Sequence[int] | np.ndarray) -> MinimalOut: ...`
- `def step_first_legal(self) -> tuple[MinimalOut, np.ndarray]: ...`
- `def step_random_legal(
    self,
    seeds: int | Sequence[int] | np.ndarray,
) -> tuple[MinimalOut, np.ndarray]: ...`
- `def step_sample_from_logits(
    self,
    logits: object,
    seeds: int | Sequence[int] | np.ndarray,
) -> tuple[MinimalOut, np.ndarray]: ...`
- `def step_select_from_logits(self, logits: object) -> tuple[MinimalOut, np.ndarray]: ...`
- `def timing_counters(self) -> dict[str, int]: ...`

</details>

### `EnvPoolTrajectoryBuffers`
<details>
<summary><code>EnvPoolTrajectoryBuffers</code></summary>

Preallocated numpy buffers for multi-step rollouts.

Methods:
- `def rollout_first_legal(self) -> TrajectoryOut: ...`
- `def rollout_random_legal(self, seeds: int | Sequence[int] | np.ndarray) -> TrajectoryOut: ...`

</details>

### `BatchOutMinimal`
<details>
<summary><code>BatchOutMinimal</code></summary>

Fields:
- `obs: np.ndarray`
- `masks: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`

</details>

### `BatchOutMinimalI16`
<details>
<summary><code>BatchOutMinimalI16</code></summary>

Fields:
- `obs: np.ndarray`
- `masks: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`

</details>

### `BatchOutMinimalI16LegalIds`
<details>
<summary><code>BatchOutMinimalI16LegalIds</code></summary>

Fields:
- `obs: np.ndarray`
- `legal_ids: np.ndarray`
- `legal_action_meta: np.ndarray`
- `legal_offsets: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`

</details>

### `BatchOutMinimalNoMask`
<details>
<summary><code>BatchOutMinimalNoMask</code></summary>

Fields:
- `obs: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`

</details>

### `BatchOutTrajectory`
<details>
<summary><code>BatchOutTrajectory</code></summary>

Fields:
- `steps: int`
- `obs: np.ndarray`
- `masks: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `actions: np.ndarray`

</details>

### `BatchOutTrajectoryI16`
<details>
<summary><code>BatchOutTrajectoryI16</code></summary>

Fields:
- `steps: int`
- `obs: np.ndarray`
- `masks: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `actions: np.ndarray`

</details>

### `BatchOutTrajectoryI16LegalIds`
<details>
<summary><code>BatchOutTrajectoryI16LegalIds</code></summary>

Fields:
- `steps: int`
- `obs: np.ndarray`
- `legal_ids: np.ndarray`
- `legal_action_meta: np.ndarray`
- `legal_offsets: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `episode_seed: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `actions: np.ndarray`

</details>

### `BatchOutTrajectoryNoMask`
<details>
<summary><code>BatchOutTrajectoryNoMask</code></summary>

Fields:
- `steps: int`
- `obs: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `actions: np.ndarray`

</details>

### `BatchOutDebug`
<details>
<summary><code>BatchOutDebug</code></summary>

Fields:
- `obs: np.ndarray`
- `masks: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `state_fingerprint: np.ndarray`
- `events_fingerprint: np.ndarray`
- `mask_fingerprint: np.ndarray`
- `event_counts: np.ndarray`
- `event_codes: np.ndarray`

</details>

### `make_pool`
Create an `EnvPool` plus canonical preallocated numpy buffers.

```python
def make_pool(
    mode: ModeName | str,
    num_envs: int,
    db_path: str | None = None,
    deck_lists: DeckLists | None = None,
    deck_ids: DeckIds | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    end_condition_policy_json: str | None = None,
    error_policy: str | None = None,
    observation_visibility: str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: ProfileName | str | None = None,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: None = None,
    layout: LayoutName | str | None = None,
) -> tuple[EnvPool, EnvPoolBuffers]: ...

def make_pool(
    mode: ModeName | str,
    num_envs: int,
    db_path: str | None = None,
    deck_lists: DeckLists | None = None,
    deck_ids: DeckIds | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    end_condition_policy_json: str | None = None,
    error_policy: str | None = None,
    observation_visibility: str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: ProfileName | str | None = None,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: int,
    layout: LayoutName | str | None = None,
) -> tuple[EnvPool, EnvPoolTrajectoryBuffers]: ...
```

### `make_batch_out_debug`
Allocate a `BatchOutDebug` buffer with safe defaults for an existing pool.

```python
def make_batch_out_debug(pool: EnvPool, *, event_capacity: int | None = None) -> BatchOutDebug: ...
```

### `RlStep`
<details>
<summary><code>RlStep</code></summary>

Fields:
- `obs: np.ndarray`
- `rewards: np.ndarray`
- `terminated: np.ndarray`
- `truncated: np.ndarray`
- `actor: np.ndarray`
- `decision_kind: np.ndarray`
- `decision_id: np.ndarray`
- `engine_status: np.ndarray`
- `spec_hash: np.ndarray`
- `decision_count: np.ndarray`
- `tick_count: np.ndarray`
- `no_progress_count: np.ndarray`
- `main_move_action: np.ndarray`
- `main_pass_action: np.ndarray`
- `masks: np.ndarray | None = None`
- `legal_ids: np.ndarray | None = None`
- `legal_offsets: np.ndarray | None = None`
- `legal_action_meta: np.ndarray | None = None`

Methods:
- `def actor_known(self) -> np.ndarray: ...`
- `def engine_error(self) -> np.ndarray: ...`
- `def reset_recommended(self) -> np.ndarray: ...`

</details>

### `reset_rl`
Reset the pool and return an `RlStep` view over the output buffers.

```python
def reset_rl(pool: EnvPool, *, layout: Layout = "mask", out: object | None = None) -> RlStep: ...
```

### `step_rl`
Step the pool once and return an `RlStep` view over the output buffers.

```python
def step_rl(
    pool: EnvPool,
    actions: Sequence[int] | np.ndarray,
    *,
    layout: Layout = "mask",
    out: object | None = None,
) -> RlStep: ...
```

### `step_rl_select_from_logits`
Select argmax actions from `logits` (respecting legality) and step the pool.

```python
def step_rl_select_from_logits(
    pool: EnvPool,
    logits: object,
    *,
    layout: Layout = "i16_legal_ids",
    actions: Sequence[int] | np.ndarray | None = None,
    out: object | None = None,
): ...
```

### `step_rl_sample_from_logits`
Sample actions from `logits` (respecting legality) and step the pool.

```python
def step_rl_sample_from_logits(
    pool: EnvPool,
    logits: object,
    seeds: int | Sequence[int] | np.ndarray,
    *,
    layout: Layout = "i16_legal_ids",
    actions: Sequence[int] | np.ndarray | None = None,
    out: object | None = None,
): ...
```

### `step_rl_sample_from_logits_with_logp`
Sample actions from `logits`, return sampled-action log-probs, and step the pool.

```python
def step_rl_sample_from_logits_with_logp(
    pool: EnvPool,
    logits: object,
    seeds: int | Sequence[int] | np.ndarray,
    *,
    layout: Layout = "i16_legal_ids",
    actions: Sequence[int] | np.ndarray | None = None,
    action_logp: np.ndarray | None = None,
    out: object | None = None,
): ...
```

### `pass_action_id_for_decision_kind`
Return the action id corresponding to "pass" for a decision kind.

```python
def pass_action_id_for_decision_kind(decision_kind: object) -> int: ...
```

## Cards & decks

### `cards`
`cards` is a namespace object exposed as `weiss_sim.cards`.

Methods:
- `def builder(self, initial: DeckInput | None = None) -> DeckBuilder: ...`
- `def describe_deck(
    self,
    deck_input: DeckInput,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
) -> dict[str, object]: ...`
- `def export_deck(
    self,
    deck_input: DeckInput,
    *,
    format: str = "card_no_map",
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    include_meta: bool = True,
) -> DeckExportPayload: ...`
- `def get(self, identifier: int | str) -> CardRef: ...`
- `def load_deck(self, path: str | Path) -> DeckRawPayload: ...`
- `def presets(self) -> list[str]: ...`
- `def resolve_deck(
    self,
    deck_input: DeckInput,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
) -> list[int]: ...`
- `def save_deck(
    self,
    path: str | Path,
    deck_input: DeckInput,
    *,
    format: str = "card_no_map",
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    include_meta: bool = True,
    indent: int = 2,
) -> str: ...`
- `def search(self, query: str, *, limit: int = 20) -> list[CardRef]: ...`
- `def suggest(self, query: str, *, limit: int = 5) -> list[CardRef]: ...`
- `def validate_deck(
    self,
    deck_input: DeckInput,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    deck_size: int = 50,
) -> DeckValidationReport: ...`

### `DeckInput`
`DeckInput = Sequence[int] | Mapping[int | str, int] | str | Path`

### `DeckBuilder`
<details>
<summary><code>DeckBuilder</code></summary>

Fluent deck authoring helper backed by card-id counts.

Methods:
- `def add(self, card: int | str, count: int = 1) -> DeckBuilder: ...`
- `def build(
    self,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    deck_size: int = 50,
) -> list[int]: ...`
- `def count(self, card: int | str) -> int: ...`
- `def describe(
    self,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    deck_size: int = 50,
) -> dict[str, object]: ...`
- `def remaining_slots(self, deck_size: int = 50) -> int: ...`
- `def remove(self, card: int | str, count: int = 1) -> DeckBuilder: ...`
- `def set_count(self, card: int | str, count: int) -> DeckBuilder: ...`
- `def to_card_no_map(self) -> dict[str, int]: ...`
- `def to_id_list(self, order: Literal["id_asc"] = "id_asc") -> list[int]: ...`
- `def to_id_map(self) -> dict[int, int]: ...`
- `def total_cards(self) -> int: ...`
- `def validate(
    self,
    *,
    rules_profile: RulesProfile,
    card_pool: CardPoolMode,
    db_path: str | Path | None = None,
    deck_size: int = 50,
) -> DeckValidationReport: ...`

</details>

### `CurriculumOverrides`
<details>
<summary><code>CurriculumOverrides</code></summary>

Fields:
- `allowed_card_sets: list[str] | None = None`
- `allow_character: bool | None = None`
- `allow_event: bool | None = None`
- `allow_climax: bool | None = None`
- `enable_clock_phase: bool | None = None`
- `enable_climax_phase: bool | None = None`
- `enable_side_attacks: bool | None = None`
- `enable_direct_attacks: bool | None = None`
- `enable_counters: bool | None = None`
- `enable_triggers: bool | None = None`
- `enable_trigger_soul: bool | None = None`
- `enable_trigger_draw: bool | None = None`
- `enable_trigger_shot: bool | None = None`
- `enable_trigger_bounce: bool | None = None`
- `enable_trigger_treasure: bool | None = None`
- `enable_trigger_gate: bool | None = None`
- `enable_trigger_standby: bool | None = None`
- `enable_on_reverse_triggers: bool | None = None`
- `enable_backup: bool | None = None`
- `enable_encore: bool | None = None`
- `enable_refresh_penalty: bool | None = None`
- `enable_level_up_choice: bool | None = None`
- `enable_activated_abilities: bool | None = None`
- `enable_continuous_modifiers: bool | None = None`
- `enable_approx_effects: bool | None = None`
- `enable_priority_windows: bool | None = None`
- `enable_visibility_policies: bool | None = None`
- `use_alternate_end_conditions: bool | None = None`
- `priority_autopick_single_action: bool | None = None`
- `priority_allow_pass: bool | None = None`
- `strict_priority_mode: bool | None = None`
- `enable_legacy_cost_order: bool | None = None`
- `enable_legacy_shot_damage_step_only: bool | None = None`
- `reduced_stage_mode: bool | None = None`
- `enforce_color_requirement: bool | None = None`
- `enforce_cost_requirement: bool | None = None`
- `allow_concede: bool | None = None`
- `reveal_opponent_hand_stock_counts: bool | None = None`
- `memory_is_public: bool | None = None`
- `max_no_progress_decisions: int | None = None`

Methods:
- `def to_dict(self) -> dict[str, object]: ...`

</details>

### `EndConditionOverrides`
<details>
<summary><code>EndConditionOverrides</code></summary>

Fields:
- `simultaneous_loss: SimultaneousLossPolicy | None = None`
- `allow_draw_on_simultaneous_loss: bool | None = None`

Methods:
- `def to_dict(self) -> dict[str, object]: ...`

</details>

### `CardRef`
<details>
<summary><code>CardRef</code></summary>

Lightweight card metadata reference from the packaged catalog.

Fields:
- `id: int`
- `card_no: str`
- `name: str`
- `card_type: str`
- `card_set: str | None`
- `strict_ok: bool`
- `approx_ok: bool`

</details>

### `DeckValidationIssue`
<details>
<summary><code>DeckValidationIssue</code></summary>

Structured deck validation finding.

Fields:
- `code: str`
- `message: str`
- `severity: Literal["error", "warning"]`
- `card_id: int | None = None`
- `card_no: str | None = None`
- `got: int | None = None`
- `max_allowed: int | None = None`
- `suggestions: list[str] = field(default_factory=list)`

</details>

### `DeckValidationReport`
<details>
<summary><code>DeckValidationReport</code></summary>

Non-throwing deck validation result.

Fields:
- `ok: bool`
- `deck_size: int`
- `resolved_ids: list[int]`
- `errors: list[DeckValidationIssue]`
- `warnings: list[DeckValidationIssue]`
- `summary: dict[str, int]`

</details>

## League utilities

### `MatchRecord`
<details>
<summary><code>MatchRecord</code></summary>

Fields:
- `seat0_agent: str`
- `seat1_agent: str`
- `winner: int | None`
- `terminated: bool`
- `truncated: bool`
- `reward_seat0: float`
- `decision_count: int`
- `tick_count: int`
- `episode_seed: int`
- `episode_key: int`
- `starting_seat: int = 0`

</details>

### `AgentSummary`
<details>
<summary><code>AgentSummary</code></summary>

Fields:
- `matches: int`
- `wins: int`
- `losses: int`
- `draws: int`
- `truncated: int`
- `win_rate: float`

</details>

### `FirstPlayerBiasSummary`
<details>
<summary><code>FirstPlayerBiasSummary</code></summary>

Fields:
- `matches: int`
- `decided: int`
- `first_player_wins: int`
- `second_player_wins: int`
- `draws: int`
- `truncated: int`
- `first_player_win_rate: float`

</details>

### `ClockGreedSummary`
<details>
<summary><code>ClockGreedSummary</code></summary>

Fields:
- `decision_samples: int`
- `clock_decisions: int`
- `clock_actions_taken: int`
- `clock_passes: int`
- `clock_action_rate: float`
- `clock_events: int`
- `clock_events_followed_by_draw: int`
- `clock_followed_by_draw_rate: float`
- `self_effect_damage_intents: int`
- `self_effect_damage_committed: int`
- `self_effect_damage_followed_by_draw: int`
- `self_effect_damage_followed_by_draw_rate: float`

</details>

### `round_robin_schedule`
```python
def round_robin_schedule(
    agent_ids: Sequence[str],
    *,
    double_round: bool = True,
) -> list[tuple[str, str]]: ...
```

### `sample_population_schedule`
```python
def sample_population_schedule(
    agent_ids: Sequence[str],
    num_matches: int,
    *,
    seed: int = 0,
    allow_mirror: bool = False,
) -> list[tuple[str, str]]: ...
```

### `records_from_step`
```python
def records_from_step(
    step: StepBatch,
    *,
    seat0_agents: str | Sequence[str],
    seat1_agents: str | Sequence[str],
) -> list[MatchRecord]: ...
```

### `summarize_records`
```python
def summarize_records(records: Iterable[MatchRecord]) -> dict[str, AgentSummary]: ...
```

### `summarize_first_player_bias`
```python
def summarize_first_player_bias(records: Iterable[MatchRecord]) -> FirstPlayerBiasSummary: ...
```

### `summarize_clock_greed_from_replay`
```python
def summarize_clock_greed_from_replay(
    replay_data: Mapping[str, Any],
    *,
    actor: int | None = None,
    draw_window_events: int = 8,
) -> ClockGreedSummary: ...
```

### `rank_agents`
```python
def rank_agents(summary: dict[str, AgentSummary]) -> list[tuple[str, AgentSummary]]: ...
```

## Errors

### `WeissSimError`
<details>
<summary><code>WeissSimError</code></summary>

Base error for high-level weiss_sim API failures.

</details>

### `DeckSpecError`
<details>
<summary><code>DeckSpecError</code></summary>

Deck input format is invalid or cannot be resolved.

</details>

### `CardLookupError`
<details>
<summary><code>CardLookupError</code></summary>

Card identifier could not be resolved in the packaged catalog.

</details>

### `DeckValidationError`
<details>
<summary><code>DeckValidationError</code></summary>

Resolved deck violates high-level validation rules.

</details>

### `ConfigConflictError`
<details>
<summary><code>ConfigConflictError</code></summary>

Requested high-level options are mutually incompatible.

</details>

### `DbMismatchError`
<details>
<summary><code>DbMismatchError</code></summary>

`card_pool="parsed_only"` was requested against a mismatched DB hash.

</details>
<!-- GENERATED:PYTHON_API_REFERENCE:END -->

