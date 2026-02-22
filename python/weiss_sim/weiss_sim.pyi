from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np

__version__: str

OBS_LEN: int
ACTION_SPACE_SIZE: int
SPEC_HASH: int
POLICY_VERSION: int
PASS_ACTION_ID: int
ACTOR_NONE: int
DECISION_KIND_NONE: int

def action_spec_json() -> str: ...
def observation_spec_json() -> str: ...
def decode_action_id(action_id: int) -> dict[str, object]: ...
def build_info() -> dict[str, object]: ...

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

    def __init__(self, num_envs: int) -> None: ...

class BatchOutMinimalI16LegalIds:
    obs: np.ndarray
    legal_ids: np.ndarray
    legal_offsets: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray

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
    actions: np.ndarray

    def __init__(self, steps: int, num_envs: int) -> None: ...

class BatchOutTrajectoryI16LegalIds:
    steps: int
    obs: np.ndarray
    legal_ids: np.ndarray
    legal_offsets: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
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

    def __init__(self, num_envs: int) -> None: ...

class EnvPool:
    envs_len: int
    action_space: int

    @classmethod
    def new_rl_train(
        cls,
        num_envs: int,
        db_path: str | None = ...,
        deck_lists: list[list[int]] | None = ...,
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
        deck_lists: list[list[int]] | None = ...,
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
        deck_lists: list[list[int]] | None = ...,
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
    def set_output_mask_enabled(self, enabled: bool) -> None: ...
    def set_output_mask_bits_enabled(self, enabled: bool) -> None: ...
    def set_i16_clamp_enabled(self, enabled: bool) -> None: ...
    def set_i16_overflow_counter_enabled(self, enabled: bool) -> None: ...
    def i16_overflow_count(self) -> int: ...
    def reset_i16_overflow_count(self) -> None: ...
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
        self, logits: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimal
    ) -> None: ...
    def step_select_from_logits_into_i16(
        self, logits: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimalI16
    ) -> None: ...
    def step_select_from_logits_into_nomask(
        self, logits: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimalNoMask
    ) -> None: ...
    def step_select_from_logits_into_i16_legal_ids(
        self, logits: np.ndarray, actions_out: np.ndarray, out: BatchOutMinimalI16LegalIds
    ) -> None: ...
    def step_sample_from_logits_into(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions_out: np.ndarray,
        out: BatchOutMinimal,
    ) -> None: ...
    def step_sample_from_logits_into_i16(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions_out: np.ndarray,
        out: BatchOutMinimalI16,
    ) -> None: ...
    def step_sample_from_logits_into_nomask(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions_out: np.ndarray,
        out: BatchOutMinimalNoMask,
    ) -> None: ...
    def step_sample_from_logits_into_i16_legal_ids(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions_out: np.ndarray,
        out: BatchOutMinimalI16LegalIds,
    ) -> None: ...
    def rollout_first_legal_into(self, steps: int, out: BatchOutTrajectory) -> None: ...
    def rollout_first_legal_into_i16(self, steps: int, out: BatchOutTrajectoryI16) -> None: ...
    def rollout_first_legal_into_i16_legal_ids(
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
    def episode_seed_batch(self) -> np.ndarray: ...
    def episode_index_batch(self) -> np.ndarray: ...
    def env_index_batch(self) -> np.ndarray: ...
    def starting_player_batch(self) -> np.ndarray: ...
    def decision_count_batch(self) -> np.ndarray: ...
    def tick_count_batch(self) -> np.ndarray: ...
    def __getattr__(self, name: str) -> Any: ...
