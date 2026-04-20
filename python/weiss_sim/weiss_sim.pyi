from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np

__version__: str

OBS_LEN: int
ACTION_SPACE_SIZE: int
ACTION_META_WIDTH: int
ACTION_META_UNUSED: int
SPEC_HASH: int
POLICY_VERSION: int
PASS_ACTION_ID: int
ACTOR_NONE: int
DECISION_KIND_NONE: int

def action_spec_json() -> str: ...
def observation_spec_json() -> str: ...
def decode_action_id(action_id: int) -> dict[str, object] | None: ...
def decode_factorized_action_id(action_id: int) -> dict[str, object] | None: ...
def encode_factorized_action(
    family: str, arg0: int | None = ..., arg1: int | None = ..., arg2: int | None = ...
) -> int | None: ...
def build_info() -> dict[str, object]: ...
def export_card_table_json(db_path: str | None = ...) -> str: ...

class BatchOutMinimal:
    obs: np.ndarray
    masks: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray

    def __init__(self, num_envs: int) -> None: ...

class BatchOutMinimalI16:
    obs: np.ndarray
    masks: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray

    def __init__(self, num_envs: int) -> None: ...

class BatchOutMinimalI16LegalIds:
    obs: np.ndarray
    legal_ids: np.ndarray
    legal_action_meta: np.ndarray
    legal_offsets: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray

    def __init__(self, num_envs: int) -> None: ...

class BatchOutMinimalNoMask:
    obs: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray

    def __init__(self, num_envs: int) -> None: ...

class BatchOutTrajectory:
    steps: int
    obs: np.ndarray
    masks: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray
    actions: np.ndarray

    def __init__(self, steps: int, num_envs: int) -> None: ...

class BatchOutTrajectoryI16:
    steps: int
    obs: np.ndarray
    masks: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray
    actions: np.ndarray

    def __init__(self, steps: int, num_envs: int) -> None: ...

class BatchOutTrajectoryI16LegalIds:
    steps: int
    obs: np.ndarray
    legal_ids: np.ndarray
    legal_action_meta: np.ndarray
    legal_offsets: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    episode_seed: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray
    actions: np.ndarray

    def __init__(self, steps: int, num_envs: int) -> None: ...

class BatchOutTrajectoryNoMask:
    steps: int
    obs: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray
    actions: np.ndarray

    def __init__(self, steps: int, num_envs: int) -> None: ...

class BatchOutDebug:
    obs: np.ndarray
    masks: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    main_move_action: np.ndarray
    main_pass_action: np.ndarray
    state_fingerprint: np.ndarray
    events_fingerprint: np.ndarray
    mask_fingerprint: np.ndarray
    event_counts: np.ndarray
    event_codes: np.ndarray

    def __init__(self, num_envs: int, event_capacity: int) -> None: ...

class EnvPool:
    envs_len: int
    num_envs: int
    obs_len: int
    action_space: int
    num_threads: int

    @classmethod
    def new_rl_train(
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
    ) -> EnvPool: ...
    @classmethod
    def new_rl_eval(
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
    ) -> EnvPool: ...
    @classmethod
    def new_debug(
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
    ) -> EnvPool: ...
    @staticmethod
    def validate_deck_issues(
        deck_lists: list[list[int]],
        db_path: str | None = ...,
        deck_ids: list[int] | None = ...,
    ) -> list[dict[str, object]]: ...
    def engine_error_reset_count(self) -> int: ...
    def reset_engine_error_reset_count(self) -> None: ...
    def set_error_policy(self, error_policy: str) -> None: ...
    def set_output_mask_enabled(self, enabled: bool) -> None: ...
    def set_output_mask_bits_enabled(self, enabled: bool) -> None: ...
    def set_i16_clamp_enabled(self, enabled: bool) -> None: ...
    def set_i16_overflow_counter_enabled(self, enabled: bool) -> None: ...
    def i16_overflow_count(self) -> int: ...
    def reset_i16_overflow_count(self) -> None: ...
    def set_timing_enabled(self, enabled: bool) -> None: ...
    def reset_timing_counters(self) -> None: ...
    def timing_counters(self) -> dict[str, int]: ...
    def action_mask_bits_batch(self) -> np.ndarray: ...
    def sample_legal_actions_uniform(self, seeds: np.ndarray) -> np.ndarray: ...
    def config_hash(self) -> int: ...
    def debug_event_ring_capacity(self) -> int: ...
    def max_card_id(self) -> int: ...
    def reset_into(self, out: BatchOutMinimal) -> None: ...
    def reset_into_i16(self, out: BatchOutMinimalI16) -> None: ...
    def reset_into_i16_legal_ids(self, out: BatchOutMinimalI16LegalIds) -> None: ...
    def reset_into_nomask(self, out: BatchOutMinimalNoMask) -> None: ...
    def reset_indices_into(self, indices: Sequence[int], out: BatchOutMinimal) -> None: ...
    def reset_indices_into_i16(self, indices: Sequence[int], out: BatchOutMinimalI16) -> None: ...
    def reset_indices_into_i16_legal_ids(
        self, indices: Sequence[int], out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def reset_indices_into_nomask(
        self, indices: Sequence[int], out: BatchOutMinimalNoMask
    ) -> None: ...
    def reset_done_into(self, done_mask: np.ndarray, out: BatchOutMinimal) -> None: ...
    def reset_done_into_i16(self, done_mask: np.ndarray, out: BatchOutMinimalI16) -> None: ...
    def reset_done_into_i16_legal_ids(
        self, done_mask: np.ndarray, out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def reset_done_into_nomask(self, done_mask: np.ndarray, out: BatchOutMinimalNoMask) -> None: ...
    def reset_indices_with_episode_seeds_into(
        self, indices: Sequence[int], episode_seeds: Sequence[int], out: BatchOutMinimal
    ) -> None: ...
    def reset_indices_with_episode_seeds_into_i16(
        self, indices: Sequence[int], episode_seeds: Sequence[int], out: BatchOutMinimalI16
    ) -> None: ...
    def reset_indices_with_episode_seeds_into_i16_legal_ids(
        self, indices: Sequence[int], episode_seeds: Sequence[int], out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def reset_indices_with_episode_seeds_into_nomask(
        self, indices: Sequence[int], episode_seeds: Sequence[int], out: BatchOutMinimalNoMask
    ) -> None: ...
    def step_into(self, actions: np.ndarray, out: BatchOutMinimal) -> None: ...
    def step_into_i16(self, actions: np.ndarray, out: BatchOutMinimalI16) -> None: ...
    def step_into_i16_legal_ids(
        self, actions: np.ndarray, out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def step_into_nomask(self, actions: np.ndarray, out: BatchOutMinimalNoMask) -> None: ...
    def step_debug_into(self, actions: np.ndarray, out: BatchOutDebug) -> None: ...
    def reset_debug_into(self, out: BatchOutDebug) -> None: ...
    def step_first_legal_into(self, actions_out: np.ndarray, out: BatchOutMinimal) -> None: ...
    def step_first_legal_into_i16(
        self, actions_out: np.ndarray, out: BatchOutMinimalI16
    ) -> None: ...
    def step_first_legal_into_i16_legal_ids(
        self, actions_out: np.ndarray, out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def step_first_legal_into_nomask(
        self, actions_out: np.ndarray, out: BatchOutMinimalNoMask
    ) -> None: ...
    def step_sample_legal_action_ids_uniform_into(
        self, seeds: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimal
    ) -> None: ...
    def step_sample_legal_action_ids_uniform_into_i16(
        self, seeds: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimalI16
    ) -> None: ...
    def step_sample_legal_action_ids_uniform_into_i16_legal_ids(
        self, seeds: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def step_sample_legal_action_ids_uniform_into_nomask(
        self, seeds: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimalNoMask
    ) -> None: ...
    def legal_action_ids_into(self, legal_ids: np.ndarray, legal_offsets: np.ndarray) -> int: ...
    def legal_action_meta_into(self, legal_action_meta: np.ndarray) -> int: ...
    def select_actions_from_logits_into(
        self, logits: np.ndarray, actions_out: np.ndarray
    ) -> None: ...
    def sample_actions_from_logits_into(
        self, logits: np.ndarray, seeds: np.ndarray, actions_out: np.ndarray
    ) -> None: ...
    def legal_action_ids_and_sample_uniform_into(
        self,
        legal_ids: np.ndarray,
        legal_offsets: np.ndarray,
        seeds: np.ndarray,
        actions_out: np.ndarray,
    ) -> int: ...
    def step_select_from_logits_into(
        self, logits: np.ndarray, actions: np.ndarray, out: BatchOutMinimal
    ) -> None: ...
    def step_select_from_logits_into_i16(
        self, logits: np.ndarray, actions: np.ndarray, out: BatchOutMinimalI16
    ) -> None: ...
    def step_select_from_logits_into_nomask(
        self, logits: np.ndarray, actions: np.ndarray, out: BatchOutMinimalNoMask
    ) -> None: ...
    def step_select_from_logits_into_i16_legal_ids(
        self, logits: np.ndarray, actions: np.ndarray, out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def step_sample_from_logits_into(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions: np.ndarray,
        out: BatchOutMinimal,
    ) -> None: ...
    def step_sample_from_logits_into_i16(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions: np.ndarray,
        out: BatchOutMinimalI16,
    ) -> None: ...
    def step_sample_from_logits_into_nomask(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions: np.ndarray,
        out: BatchOutMinimalNoMask,
    ) -> None: ...
    def step_sample_from_logits_into_i16_legal_ids(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions: np.ndarray,
        out: BatchOutMinimalI16LegalIds,
    ) -> None: ...
    def step_sample_from_logits_with_logp_into_i16_legal_ids(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions: np.ndarray,
        action_logp: np.ndarray,
        out: BatchOutMinimalI16LegalIds,
    ) -> None: ...
    def rollout_first_legal_into(self, steps: int, out: BatchOutTrajectory) -> None: ...
    def rollout_first_legal_into_i16(self, steps: int, out: BatchOutTrajectoryI16) -> None: ...
    def rollout_first_legal_into_i16_legal_ids(
        self, steps: int, out: BatchOutTrajectoryI16LegalIds
    ) -> None: ...
    def rollout_heuristic_public_into_i16_legal_ids(
        self, steps: int, out: BatchOutTrajectoryI16LegalIds
    ) -> None: ...
    def rollout_first_legal_into_nomask(
        self, steps: int, out: BatchOutTrajectoryNoMask
    ) -> None: ...
    def rollout_sample_legal_action_ids_uniform_into(
        self, steps: int, seeds: np.ndarray, out: BatchOutTrajectory
    ) -> None: ...
    def rollout_sample_legal_action_ids_uniform_into_i16(
        self, steps: int, seeds: np.ndarray, out: BatchOutTrajectoryI16
    ) -> None: ...
    def rollout_sample_legal_action_ids_uniform_into_i16_legal_ids(
        self, steps: int, seeds: np.ndarray, out: BatchOutTrajectoryI16LegalIds
    ) -> None: ...
    def rollout_sample_legal_action_ids_uniform_into_nomask(
        self, steps: int, seeds: np.ndarray, out: BatchOutTrajectoryNoMask
    ) -> None: ...
    def auto_reset_on_error_codes_into(self, codes: np.ndarray, out: BatchOutMinimal) -> int: ...
    def auto_reset_on_error_codes_into_nomask(
        self, codes: np.ndarray, out: BatchOutMinimalNoMask
    ) -> int: ...
    def episode_seed_batch(self) -> np.ndarray: ...
    def episode_index_batch(self) -> np.ndarray: ...
    def env_index_batch(self) -> np.ndarray: ...
    def starting_player_batch(self) -> np.ndarray: ...
    def decision_count_batch(self) -> np.ndarray: ...
    def tick_count_batch(self) -> np.ndarray: ...
    def no_progress_count_batch(self) -> np.ndarray: ...
    def obs_fingerprint_batch(self) -> np.ndarray: ...
    def state_fingerprint_batch(self) -> np.ndarray: ...
    def events_fingerprint_batch(self) -> np.ndarray: ...
    def enable_replay_sampling(
        self,
        sample_rate: float,
        out_dir: str | None = ...,
        compress: bool = ...,
        include_trigger_card_id: bool = ...,
        visibility_mode: str | None = ...,
        store_actions: bool = ...,
    ) -> None: ...
    def action_lookup_batch(self) -> list[list[dict[str, Any] | None]]: ...
    def describe_action_ids(self, action_ids: Sequence[int]) -> list[dict[str, Any] | None]: ...
    def decision_info_batch(self) -> list[dict[str, Any]]: ...
    def choose_heuristic_public_actions_into(
        self, env_indices: np.ndarray, actions_out: np.ndarray
    ) -> None: ...
    def sample_legal_action_ids_uniform_into(
        self, seeds: np.ndarray, actions_out: np.ndarray
    ) -> None: ...
    def render_ansi(self, env_index: int, perspective: int) -> str: ...
    def __getattr__(self, name: str) -> Any: ...
