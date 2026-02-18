from __future__ import annotations

import numpy as np

from .weiss_sim import (
    ACTION_SPACE_SIZE,
    ACTOR_NONE,
    DECISION_KIND_NONE,
    POLICY_VERSION,
    OBS_LEN,
    PASS_ACTION_ID,
    SPEC_HASH,
    BatchOutMinimalI16,
    BatchOutMinimalI16LegalIds,
    BatchOutDebug,
    BatchOutMinimal,
    BatchOutMinimalNoMask,
    BatchOutTrajectory,
    BatchOutTrajectoryI16,
    BatchOutTrajectoryI16LegalIds,
    BatchOutTrajectoryNoMask,
    EnvPool,
    action_spec_json,
    build_info,
    decode_action_id,
    observation_spec_json,
    __version__,
)
from .rl import (
    RlStepI16LegalIds,
    RlStep,
    RlStepNoMask,
    pass_action_id_for_decision_kind,
    reset_rl,
    reset_rl_into,
    reset_rl_nomask,
    reset_rl_nomask_into,
    reset_rl_i16_legal_ids,
    reset_rl_i16_legal_ids_into,
    step_rl,
    step_rl_into,
    step_rl_nomask,
    step_rl_nomask_into,
    step_rl_i16_legal_ids,
    step_rl_i16_legal_ids_into,
    step_rl_select_from_logits_i16_legal_ids,
    step_rl_select_from_logits_i16_legal_ids_into,
    step_rl_sample_from_logits_i16_legal_ids,
    step_rl_sample_from_logits_i16_legal_ids_into,
)
from .api import create, db_info, evaluate, export_spec_bundle, train
from .catalog import cards
from .config_types import CurriculumOverrides, DeckInput, EndConditionOverrides
from .league import (
    AgentSummary,
    ClockGreedSummary,
    FirstPlayerBiasSummary,
    MatchRecord,
    rank_agents,
    records_from_step,
    round_robin_schedule,
    sample_population_schedule,
    summarize_clock_greed_from_replay,
    summarize_first_player_bias,
    summarize_records,
)
from .errors import (
    CardLookupError,
    ConfigConflictError,
    DbMismatchError,
    DeckSpecError,
    DeckValidationError,
    WeissSimError,
)
from .runner import SimRunner
from .types import CardRef, ResetBatch, StepBatch

_PROFILE_FAST = "fast"
_PROFILE_BALANCED = "balanced"
_PROFILE_EVAL = "eval"
_PROFILE_DEBUG = "debug"


def _resolve_profile(profile: str):
    profile_norm = profile.lower().strip()
    if profile_norm == _PROFILE_FAST:
        return profile_norm, False, True, True, True
    if profile_norm in (_PROFILE_BALANCED, _PROFILE_EVAL, _PROFILE_DEBUG):
        return profile_norm, True, False, False, False
    raise ValueError(f"unknown profile '{profile}' (expected fast, balanced, eval, debug)")


def _prepare_pool_for_legal_ids(pool: EnvPool) -> None:
    """Legal-id outputs require dense output masks to be disabled."""
    pool.set_output_mask_enabled(False)
    pool.set_output_mask_bits_enabled(False)


def _resolve_pool_options(
    *,
    profile: str,
    output_masks: bool | None,
    use_i16: bool | None,
    legal_ids: bool | None,
    unsafe_i16: bool | None,
) -> tuple[bool, bool, bool, bool]:
    _, profile_masks, profile_i16, profile_legal_ids, profile_unsafe_i16 = _resolve_profile(profile)
    resolved_output_masks = profile_masks if output_masks is None else bool(output_masks)
    resolved_use_i16 = profile_i16 if use_i16 is None else bool(use_i16)
    resolved_legal_ids = profile_legal_ids if legal_ids is None else bool(legal_ids)
    resolved_unsafe_i16 = profile_unsafe_i16 if unsafe_i16 is None else bool(unsafe_i16)
    if resolved_legal_ids:
        if resolved_output_masks:
            raise ValueError("legal_ids requires output_masks=False")
        if resolved_use_i16 is False:
            raise ValueError("legal_ids currently requires use_i16=True")
        resolved_output_masks = False
        resolved_use_i16 = True
    if resolved_unsafe_i16 and not resolved_use_i16:
        raise ValueError("unsafe_i16 requires use_i16=True")
    return resolved_output_masks, resolved_use_i16, resolved_legal_ids, resolved_unsafe_i16


def _select_pool_buffers(
    pool: EnvPool,
    *,
    output_masks: bool,
    use_i16: bool,
    legal_ids: bool,
    rollout_steps: int | None,
):
    if rollout_steps is not None:
        if legal_ids:
            return pool, EnvPoolTrajectoryBuffersI16LegalIds(pool, rollout_steps)
        if use_i16:
            return pool, EnvPoolTrajectoryBuffersI16(pool, rollout_steps)
        if output_masks:
            return pool, EnvPoolTrajectoryBuffers(pool, rollout_steps)
        return pool, EnvPoolTrajectoryBuffersNoMask(pool, rollout_steps)
    if legal_ids:
        return pool, EnvPoolBuffersI16LegalIds(pool)
    if use_i16:
        return pool, EnvPoolBuffersI16(pool)
    if output_masks:
        return pool, EnvPoolBuffers(pool)
    return pool, EnvPoolBuffersNoMask(pool)


def _make_pool(
    *,
    constructor,
    num_envs: int,
    db_path: str | None,
    deck_lists,
    deck_ids,
    max_decisions: int,
    max_ticks: int,
    seed: int,
    curriculum_json: str | None,
    reward_json: str | None,
    error_policy: str | None,
    num_threads: int | None,
    debug_fingerprint_every_n: int,
    debug_event_ring_capacity: int,
    profile: str,
    output_masks: bool | None,
    use_i16: bool | None,
    legal_ids: bool | None,
    unsafe_i16: bool | None,
    rollout_steps: int | None,
):
    if deck_lists is None:
        raise ValueError("deck_lists is required")
    output_masks_v, use_i16_v, legal_ids_v, unsafe_i16_v = _resolve_pool_options(
        profile=profile,
        output_masks=output_masks,
        use_i16=use_i16,
        legal_ids=legal_ids,
        unsafe_i16=unsafe_i16,
    )
    pool = constructor(
        num_envs,
        db_path,
        deck_lists,
        deck_ids=deck_ids,
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        seed=seed,
        curriculum_json=curriculum_json,
        reward_json=reward_json,
        error_policy=error_policy,
        num_threads=num_threads,
        output_masks=output_masks_v,
        debug_fingerprint_every_n=debug_fingerprint_every_n,
        debug_event_ring_capacity=debug_event_ring_capacity,
    )
    if legal_ids_v:
        _prepare_pool_for_legal_ids(pool)
    if unsafe_i16_v:
        pool.set_i16_clamp_enabled(False)
    return _select_pool_buffers(
        pool,
        output_masks=output_masks_v,
        use_i16=use_i16_v,
        legal_ids=legal_ids_v,
        rollout_steps=rollout_steps,
    )


def make_train_pool(
    num_envs: int,
    db_path: str | None = None,
    deck_lists=None,
    deck_ids=None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    error_policy: str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: str = _PROFILE_FAST,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: int | None = None,
):
    """Create an RL training pool plus preallocated buffers with sensible defaults.

    Profiles:
    - fast: masks off + i16 obs + legal ids (highest throughput)
    - balanced/eval/debug: masks on + i32 obs (easier debugging)
    - num_threads=None uses RL auto-threading (CPU parallelism capped by num_envs)
      from EnvPool.new_rl_train; pass num_threads=1 to force serial execution.

    Returns: (pool, buffers)
    """
    return _make_pool(
        constructor=EnvPool.new_rl_train,
        num_envs=num_envs,
        db_path=db_path,
        deck_lists=deck_lists,
        deck_ids=deck_ids,
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        seed=seed,
        curriculum_json=curriculum_json,
        reward_json=reward_json,
        error_policy=error_policy,
        num_threads=num_threads,
        debug_fingerprint_every_n=debug_fingerprint_every_n,
        debug_event_ring_capacity=debug_event_ring_capacity,
        profile=profile,
        output_masks=output_masks,
        use_i16=use_i16,
        legal_ids=legal_ids,
        unsafe_i16=unsafe_i16,
        rollout_steps=rollout_steps,
    )


def make_eval_pool(
    num_envs: int,
    db_path: str | None = None,
    deck_lists=None,
    deck_ids=None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    error_policy: str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: str = _PROFILE_BALANCED,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: int | None = None,
):
    """Create an RL eval/debug pool plus preallocated buffers with sensible defaults.

    Profiles:
    - balanced/eval/debug: masks on + i32 obs
    - fast: masks off + i16 obs + legal ids (opt-in)
    - num_threads=None uses RL auto-threading (CPU parallelism capped by num_envs)
      from EnvPool.new_rl_eval; pass num_threads=1 to force serial execution.

    Returns: (pool, buffers)
    """
    return _make_pool(
        constructor=EnvPool.new_rl_eval,
        num_envs=num_envs,
        db_path=db_path,
        deck_lists=deck_lists,
        deck_ids=deck_ids,
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        seed=seed,
        curriculum_json=curriculum_json,
        reward_json=reward_json,
        error_policy=error_policy,
        num_threads=num_threads,
        debug_fingerprint_every_n=debug_fingerprint_every_n,
        debug_event_ring_capacity=debug_event_ring_capacity,
        profile=profile,
        output_masks=output_masks,
        use_i16=use_i16,
        legal_ids=legal_ids,
        unsafe_i16=unsafe_i16,
        rollout_steps=rollout_steps,
    )


class _EngineStatusMixin:
    @property
    def engine_error(self):
        return self.engine_status != 0

    @property
    def reset_recommended(self):
        return self.engine_error

    @property
    def actor_known(self):
        return self.actor != ACTOR_NONE


class EnvPoolBuffers(_EngineStatusMixin):
    """Preallocated numpy buffers for high-throughput stepping."""

    def __init__(self, pool: EnvPool) -> None:
        self.pool = pool
        num_envs = pool.envs_len
        self.out = BatchOutMinimal(num_envs)
        self.obs = self.out.obs
        self.masks = self.out.masks
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.legal_ids = np.empty(num_envs * pool.action_space, dtype=np.uint16)
        self.legal_offsets = np.zeros(num_envs + 1, dtype=np.uint32)
        self.actions = np.empty(num_envs, dtype=np.uint32)

    def reset(self):
        self.pool.reset_into(self.out)
        return self.out

    def reset_indices(self, indices):
        self.pool.reset_indices_into(list(indices), self.out)
        return self.out

    def reset_done(self, done_mask):
        self.pool.reset_done_into(done_mask, self.out)
        return self.out

    def reset_indices_with_episode_seeds(self, indices, episode_seeds):
        self.pool.reset_indices_with_episode_seeds_into(
            list(indices), list(episode_seeds), self.out
        )
        return self.out

    def step(self, actions):
        self.pool.step_into(actions, self.out)
        return self.out

    def step_first_legal(self):
        self.pool.step_first_legal_into(self.actions, self.out)
        return self.out, self.actions

    def step_random_legal(self, seeds):
        self.pool.step_sample_legal_action_ids_uniform_into(seeds, self.actions, self.out)
        return self.out, self.actions

    def step_select_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.step_select_from_logits_into(logits, self.actions, self.out)
        return self.out, self.actions

    def step_sample_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.step_sample_from_logits_into(logits, seeds, self.actions, self.out)
        return self.out, self.actions

    def select_actions_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.select_actions_from_logits_into(logits, self.actions)
        return self.actions

    def sample_actions_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.sample_actions_from_logits_into(logits, seeds, self.actions)
        return self.actions

    def set_output_mask_enabled(self, enabled: bool):
        self.pool.set_output_mask_enabled(enabled)
        if not enabled:
            self.out.masks.fill(0)

    def set_output_mask_bits_enabled(self, enabled: bool):
        self.pool.set_output_mask_bits_enabled(enabled)

    def set_i16_overflow_counter_enabled(self, enabled: bool):
        self.pool.set_i16_overflow_counter_enabled(enabled)

    def i16_overflow_count(self) -> int:
        return int(self.pool.i16_overflow_count())

    def reset_i16_overflow_count(self) -> None:
        self.pool.reset_i16_overflow_count()

    def legal_action_ids(self):
        count = self.pool.legal_action_ids_into(self.legal_ids, self.legal_offsets)
        return self.legal_ids[:count], self.legal_offsets

    def legal_action_ids_and_sample_uniform(self, seeds):
        count = self.pool.legal_action_ids_and_sample_uniform_into(
            self.legal_ids, self.legal_offsets, seeds, self.actions
        )
        return self.legal_ids[:count], self.legal_offsets, self.actions


class EnvPoolBuffersNoMask(_EngineStatusMixin):
    """Preallocated numpy buffers for stepping without dense masks."""

    def __init__(self, pool: EnvPool) -> None:
        self.pool = pool
        num_envs = pool.envs_len
        self.out = BatchOutMinimalNoMask(num_envs)
        self.obs = self.out.obs
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.legal_ids = np.empty(num_envs * pool.action_space, dtype=np.uint16)
        self.legal_offsets = np.zeros(num_envs + 1, dtype=np.uint32)
        self.actions = np.empty(num_envs, dtype=np.uint32)

    def reset(self):
        self.pool.reset_into_nomask(self.out)
        return self.out

    def reset_indices(self, indices):
        self.pool.reset_indices_into_nomask(list(indices), self.out)
        return self.out

    def reset_done(self, done_mask):
        self.pool.reset_done_into_nomask(done_mask, self.out)
        return self.out

    def reset_indices_with_episode_seeds(self, indices, episode_seeds):
        self.pool.reset_indices_with_episode_seeds_into_nomask(
            list(indices), list(episode_seeds), self.out
        )
        return self.out

    def step(self, actions):
        self.pool.step_into_nomask(actions, self.out)
        return self.out

    def step_first_legal(self):
        self.pool.step_first_legal_into_nomask(self.actions, self.out)
        return self.out, self.actions

    def step_random_legal(self, seeds):
        self.pool.step_sample_legal_action_ids_uniform_into_nomask(seeds, self.actions, self.out)
        return self.out, self.actions

    def step_select_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.step_select_from_logits_into_nomask(logits, self.actions, self.out)
        return self.out, self.actions

    def step_sample_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.step_sample_from_logits_into_nomask(logits, seeds, self.actions, self.out)
        return self.out, self.actions

    def select_actions_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.select_actions_from_logits_into(logits, self.actions)
        return self.actions

    def sample_actions_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.sample_actions_from_logits_into(logits, seeds, self.actions)
        return self.actions

    def set_output_mask_bits_enabled(self, enabled: bool):
        self.pool.set_output_mask_bits_enabled(enabled)

    def set_i16_overflow_counter_enabled(self, enabled: bool):
        self.pool.set_i16_overflow_counter_enabled(enabled)

    def i16_overflow_count(self) -> int:
        return int(self.pool.i16_overflow_count())

    def reset_i16_overflow_count(self) -> None:
        self.pool.reset_i16_overflow_count()

    def legal_action_ids(self):
        count = self.pool.legal_action_ids_into(self.legal_ids, self.legal_offsets)
        return self.legal_ids[:count], self.legal_offsets

    def legal_action_ids_and_sample_uniform(self, seeds):
        count = self.pool.legal_action_ids_and_sample_uniform_into(
            self.legal_ids, self.legal_offsets, seeds, self.actions
        )
        return self.legal_ids[:count], self.legal_offsets, self.actions


class EnvPoolBuffersI16(_EngineStatusMixin):
    """Preallocated numpy buffers for high-throughput stepping with i16 obs."""

    def __init__(self, pool: EnvPool) -> None:
        self.pool = pool
        num_envs = pool.envs_len
        self.out = BatchOutMinimalI16(num_envs)
        self.obs = self.out.obs
        self.masks = self.out.masks
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.legal_ids = np.empty(num_envs * pool.action_space, dtype=np.uint16)
        self.legal_offsets = np.zeros(num_envs + 1, dtype=np.uint32)
        self.actions = np.empty(num_envs, dtype=np.uint32)

    def reset(self):
        self.pool.reset_into_i16(self.out)
        return self.out

    def reset_indices(self, indices):
        self.pool.reset_indices_into_i16(list(indices), self.out)
        return self.out

    def reset_done(self, done_mask):
        self.pool.reset_done_into_i16(done_mask, self.out)
        return self.out

    def reset_indices_with_episode_seeds(self, indices, episode_seeds):
        self.pool.reset_indices_with_episode_seeds_into_i16(
            list(indices), list(episode_seeds), self.out
        )
        return self.out

    def step(self, actions):
        self.pool.step_into_i16(actions, self.out)
        return self.out

    def step_first_legal(self):
        self.pool.step_first_legal_into_i16(self.actions, self.out)
        return self.out, self.actions

    def step_random_legal(self, seeds):
        self.pool.step_sample_legal_action_ids_uniform_into_i16(seeds, self.actions, self.out)
        return self.out, self.actions

    def step_select_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.step_select_from_logits_into_i16(logits, self.actions, self.out)
        return self.out, self.actions

    def step_sample_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.step_sample_from_logits_into_i16(logits, seeds, self.actions, self.out)
        return self.out, self.actions

    def select_actions_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.select_actions_from_logits_into(logits, self.actions)
        return self.actions

    def sample_actions_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.sample_actions_from_logits_into(logits, seeds, self.actions)
        return self.actions

    def set_output_mask_bits_enabled(self, enabled: bool):
        self.pool.set_output_mask_bits_enabled(enabled)

    def set_i16_clamp_enabled(self, enabled: bool):
        self.pool.set_i16_clamp_enabled(enabled)

    def set_i16_overflow_counter_enabled(self, enabled: bool):
        self.pool.set_i16_overflow_counter_enabled(enabled)

    def i16_overflow_count(self) -> int:
        return int(self.pool.i16_overflow_count())

    def reset_i16_overflow_count(self) -> None:
        self.pool.reset_i16_overflow_count()

    def legal_action_ids(self):
        count = self.pool.legal_action_ids_into(self.legal_ids, self.legal_offsets)
        return self.legal_ids[:count], self.legal_offsets

    def legal_action_ids_and_sample_uniform(self, seeds):
        count = self.pool.legal_action_ids_and_sample_uniform_into(
            self.legal_ids, self.legal_offsets, seeds, self.actions
        )
        return self.legal_ids[:count], self.legal_offsets, self.actions


class EnvPoolBuffersI16LegalIds(_EngineStatusMixin):
    """Preallocated numpy buffers for stepping with i16 obs + legal ids."""

    def __init__(self, pool: EnvPool) -> None:
        self.pool = pool
        _prepare_pool_for_legal_ids(self.pool)
        num_envs = pool.envs_len
        self.out = BatchOutMinimalI16LegalIds(num_envs)
        self.obs = self.out.obs
        self.legal_ids = self.out.legal_ids
        self.legal_offsets = self.out.legal_offsets
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.actions = np.empty(num_envs, dtype=np.uint32)

    def reset(self):
        self.pool.reset_into_i16_legal_ids(self.out)
        return self.out

    def reset_indices(self, indices):
        self.pool.reset_indices_into_i16_legal_ids(list(indices), self.out)
        return self.out

    def reset_done(self, done_mask):
        self.pool.reset_done_into_i16_legal_ids(done_mask, self.out)
        return self.out

    def reset_indices_with_episode_seeds(self, indices, episode_seeds):
        self.pool.reset_indices_with_episode_seeds_into_i16_legal_ids(
            list(indices), list(episode_seeds), self.out
        )
        return self.out

    def step(self, actions):
        self.pool.step_into_i16_legal_ids(actions, self.out)
        return self.out

    def step_first_legal(self):
        self.pool.step_first_legal_into_i16_legal_ids(self.actions, self.out)
        return self.out, self.actions

    def step_random_legal(self, seeds):
        self.pool.step_sample_legal_action_ids_uniform_into_i16_legal_ids(
            seeds, self.actions, self.out
        )
        return self.out, self.actions

    def step_select_from_logits(self, logits):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        self.pool.step_select_from_logits_into_i16_legal_ids(logits, self.actions, self.out)
        return self.out, self.actions

    def step_sample_from_logits(self, logits, seeds):
        logits = np.ascontiguousarray(logits, dtype=np.float32)
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.step_sample_from_logits_into_i16_legal_ids(logits, seeds, self.actions, self.out)
        return self.out, self.actions

    def set_i16_overflow_counter_enabled(self, enabled: bool):
        self.pool.set_i16_overflow_counter_enabled(enabled)

    def i16_overflow_count(self) -> int:
        return int(self.pool.i16_overflow_count())

    def reset_i16_overflow_count(self) -> None:
        self.pool.reset_i16_overflow_count()


class EnvPoolTrajectoryBuffers(_EngineStatusMixin):
    """Preallocated numpy buffers for multi-step rollouts with masks."""

    def __init__(self, pool: EnvPool, steps: int) -> None:
        self.pool = pool
        self.out = BatchOutTrajectory(steps, pool.envs_len)
        self.steps = steps
        self.obs = self.out.obs
        self.masks = self.out.masks
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.actions = self.out.actions

    def rollout_first_legal(self):
        self.pool.rollout_first_legal_into(self.steps, self.out)
        return self.out

    def rollout_random_legal(self, seeds):
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.rollout_sample_legal_action_ids_uniform_into(self.steps, seeds, self.out)
        return self.out


class EnvPoolTrajectoryBuffersI16(_EngineStatusMixin):
    """Preallocated numpy buffers for multi-step rollouts with i16 obs."""

    def __init__(self, pool: EnvPool, steps: int) -> None:
        self.pool = pool
        self.out = BatchOutTrajectoryI16(steps, pool.envs_len)
        self.steps = steps
        self.obs = self.out.obs
        self.masks = self.out.masks
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.actions = self.out.actions

    def rollout_first_legal(self):
        self.pool.rollout_first_legal_into_i16(self.steps, self.out)
        return self.out

    def rollout_random_legal(self, seeds):
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.rollout_sample_legal_action_ids_uniform_into_i16(self.steps, seeds, self.out)
        return self.out


class EnvPoolTrajectoryBuffersI16LegalIds(_EngineStatusMixin):
    """Preallocated numpy buffers for multi-step rollouts with i16 obs + legal ids."""

    def __init__(self, pool: EnvPool, steps: int) -> None:
        self.pool = pool
        _prepare_pool_for_legal_ids(self.pool)
        self.out = BatchOutTrajectoryI16LegalIds(steps, pool.envs_len)
        self.steps = steps
        self.obs = self.out.obs
        self.legal_ids = self.out.legal_ids
        self.legal_offsets = self.out.legal_offsets
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.actions = self.out.actions

    def rollout_first_legal(self):
        self.pool.rollout_first_legal_into_i16_legal_ids(self.steps, self.out)
        return self.out

    def rollout_random_legal(self, seeds):
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.rollout_sample_legal_action_ids_uniform_into_i16_legal_ids(
            self.steps, seeds, self.out
        )
        return self.out


class EnvPoolTrajectoryBuffersNoMask(_EngineStatusMixin):
    """Preallocated numpy buffers for multi-step rollouts without masks."""

    def __init__(self, pool: EnvPool, steps: int) -> None:
        self.pool = pool
        self.out = BatchOutTrajectoryNoMask(steps, pool.envs_len)
        self.steps = steps
        self.obs = self.out.obs
        self.rewards = self.out.rewards
        self.terminated = self.out.terminated
        self.truncated = self.out.truncated
        self.actor = self.out.actor
        self.decision_kind = self.out.decision_kind
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.actions = self.out.actions

    def rollout_first_legal(self):
        self.pool.rollout_first_legal_into_nomask(self.steps, self.out)
        return self.out

    def rollout_random_legal(self, seeds):
        seeds = np.asarray(seeds, dtype=np.uint64).ravel()
        self.pool.rollout_sample_legal_action_ids_uniform_into_nomask(self.steps, seeds, self.out)
        return self.out


def spec_bundle():
    return export_spec_bundle()


__all__ = [
    "EnvPool",
    "EnvPoolBuffers",
    "EnvPoolBuffersNoMask",
    "EnvPoolBuffersI16",
    "EnvPoolBuffersI16LegalIds",
    "EnvPoolTrajectoryBuffers",
    "EnvPoolTrajectoryBuffersI16",
    "EnvPoolTrajectoryBuffersI16LegalIds",
    "EnvPoolTrajectoryBuffersNoMask",
    "BatchOutMinimal",
    "BatchOutMinimalI16",
    "BatchOutMinimalI16LegalIds",
    "BatchOutMinimalNoMask",
    "BatchOutTrajectory",
    "BatchOutTrajectoryI16",
    "BatchOutTrajectoryI16LegalIds",
    "BatchOutTrajectoryNoMask",
    "BatchOutDebug",
    "ACTION_SPACE_SIZE",
    "ACTOR_NONE",
    "DECISION_KIND_NONE",
    "POLICY_VERSION",
    "OBS_LEN",
    "SPEC_HASH",
    "RlStep",
    "RlStepNoMask",
    "RlStepI16LegalIds",
    "reset_rl",
    "reset_rl_into",
    "reset_rl_nomask",
    "reset_rl_nomask_into",
    "reset_rl_i16_legal_ids",
    "reset_rl_i16_legal_ids_into",
    "step_rl",
    "step_rl_into",
    "step_rl_nomask",
    "step_rl_nomask_into",
    "step_rl_i16_legal_ids",
    "step_rl_i16_legal_ids_into",
    "step_rl_select_from_logits_i16_legal_ids",
    "step_rl_select_from_logits_i16_legal_ids_into",
    "step_rl_sample_from_logits_i16_legal_ids",
    "step_rl_sample_from_logits_i16_legal_ids_into",
    "pass_action_id_for_decision_kind",
    "PASS_ACTION_ID",
    "observation_spec_json",
    "action_spec_json",
    "decode_action_id",
    "build_info",
    "make_train_pool",
    "make_eval_pool",
    "spec_bundle",
    "export_spec_bundle",
    "db_info",
    "create",
    "train",
    "evaluate",
    "cards",
    "SimRunner",
    "CurriculumOverrides",
    "EndConditionOverrides",
    "DeckInput",
    "MatchRecord",
    "AgentSummary",
    "FirstPlayerBiasSummary",
    "ClockGreedSummary",
    "round_robin_schedule",
    "sample_population_schedule",
    "records_from_step",
    "summarize_records",
    "summarize_first_player_bias",
    "summarize_clock_greed_from_replay",
    "rank_agents",
    "CardRef",
    "ResetBatch",
    "StepBatch",
    "WeissSimError",
    "DeckSpecError",
    "CardLookupError",
    "DeckValidationError",
    "ConfigConflictError",
    "DbMismatchError",
    "__version__",
]
