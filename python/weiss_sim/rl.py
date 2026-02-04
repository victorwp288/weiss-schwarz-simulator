"""Lightweight RL helpers for the Weiss simulator.

Spec exports:
- `weiss_sim.observation_spec_json()` and `weiss_sim.action_spec_json()` for
  the stable layout and versioned encoding contract.
- `weiss_sim.spec_bundle()` for a combined, self-describing spec payload.

Episode metadata:
- `EnvPool.episode_seed_batch()`, `episode_index_batch()`, `env_index_batch()`,
  `starting_player_batch()` for deterministic bookkeeping.

Replay controls:
- `EnvPool.enable_replay_sampling(..., visibility_mode="public"|"full")`
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .weiss_sim import (
    PASS_ACTION_ID,
    BatchOutMinimal,
    BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask,
    EnvPool,
)


@dataclass(frozen=True)
class RlStep:
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


@dataclass(frozen=True)
class RlStepNoMask:
    obs: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray


@dataclass(frozen=True)
class RlStepI16LegalIds:
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


def pass_action_id_for_decision_kind(decision_kind):
    return PASS_ACTION_ID


def reset_rl(pool: EnvPool) -> RlStep:
    """Reset the pool and return a minimal step bundle."""
    out = BatchOutMinimal(pool.envs_len)
    return reset_rl_into(pool, out)


def step_rl(pool: EnvPool, actions) -> RlStep:
    """Step the pool once with one action per env and return a minimal bundle."""
    out = BatchOutMinimal(pool.envs_len)
    return step_rl_into(pool, actions, out)


def reset_rl_into(pool: EnvPool, out: BatchOutMinimal) -> RlStep:
    """Reset the pool into a preallocated BatchOutMinimal."""
    pool.reset_into(out)
    return RlStep(
        obs=out.obs,
        masks=out.masks,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def step_rl_into(pool: EnvPool, actions, out: BatchOutMinimal) -> RlStep:
    """Step the pool into a preallocated BatchOutMinimal."""
    pool.step_into(actions, out)
    return RlStep(
        obs=out.obs,
        masks=out.masks,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def reset_rl_nomask(pool: EnvPool) -> RlStepNoMask:
    """Reset the pool and return a minimal bundle without dense masks."""
    out = BatchOutMinimalNoMask(pool.envs_len)
    return reset_rl_nomask_into(pool, out)


def step_rl_nomask(pool: EnvPool, actions) -> RlStepNoMask:
    """Step the pool once without dense masks."""
    out = BatchOutMinimalNoMask(pool.envs_len)
    return step_rl_nomask_into(pool, actions, out)


def reset_rl_nomask_into(pool: EnvPool, out: BatchOutMinimalNoMask) -> RlStepNoMask:
    """Reset the pool into a preallocated BatchOutMinimalNoMask."""
    pool.reset_into_nomask(out)
    return RlStepNoMask(
        obs=out.obs,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def step_rl_nomask_into(pool: EnvPool, actions, out: BatchOutMinimalNoMask) -> RlStepNoMask:
    """Step the pool into a preallocated BatchOutMinimalNoMask."""
    pool.step_into_nomask(actions, out)
    return RlStepNoMask(
        obs=out.obs,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def reset_rl_i16_legal_ids(pool: EnvPool) -> RlStepI16LegalIds:
    """Reset the pool and return an i16+legal-ids step bundle."""
    out = BatchOutMinimalI16LegalIds(pool.envs_len)
    return reset_rl_i16_legal_ids_into(pool, out)


def step_rl_i16_legal_ids(pool: EnvPool, actions) -> RlStepI16LegalIds:
    """Step the pool once with one action per env and return i16+legal-ids bundle."""
    out = BatchOutMinimalI16LegalIds(pool.envs_len)
    return step_rl_i16_legal_ids_into(pool, actions, out)


def reset_rl_i16_legal_ids_into(
    pool: EnvPool, out: BatchOutMinimalI16LegalIds
) -> RlStepI16LegalIds:
    """Reset the pool into a preallocated BatchOutMinimalI16LegalIds."""
    pool.reset_into_i16_legal_ids(out)
    return RlStepI16LegalIds(
        obs=out.obs,
        legal_ids=out.legal_ids,
        legal_offsets=out.legal_offsets,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def step_rl_i16_legal_ids_into(
    pool: EnvPool, actions, out: BatchOutMinimalI16LegalIds
) -> RlStepI16LegalIds:
    """Step the pool into a preallocated BatchOutMinimalI16LegalIds."""
    pool.step_into_i16_legal_ids(actions, out)
    return RlStepI16LegalIds(
        obs=out.obs,
        legal_ids=out.legal_ids,
        legal_offsets=out.legal_offsets,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def step_rl_select_from_logits_i16_legal_ids(pool: EnvPool, logits):
    """Select actions from logits in Rust, step envs, return i16+legal-ids bundle."""
    out = BatchOutMinimalI16LegalIds(pool.envs_len)
    actions = np.empty(pool.envs_len, dtype=np.uint32)
    return step_rl_select_from_logits_i16_legal_ids_into(pool, logits, actions, out)


def step_rl_sample_from_logits_i16_legal_ids(pool: EnvPool, logits, seeds):
    """Sample actions from logits in Rust, step envs, return i16+legal-ids bundle."""
    out = BatchOutMinimalI16LegalIds(pool.envs_len)
    actions = np.empty(pool.envs_len, dtype=np.uint32)
    return step_rl_sample_from_logits_i16_legal_ids_into(pool, logits, seeds, actions, out)


def step_rl_select_from_logits_i16_legal_ids_into(
    pool: EnvPool, logits, actions, out: BatchOutMinimalI16LegalIds
):
    """Select actions from logits into preallocated buffers."""
    logits = np.ascontiguousarray(logits, dtype=np.float32)
    pool.step_select_from_logits_into_i16_legal_ids(logits, actions, out)
    return (
        RlStepI16LegalIds(
            obs=out.obs,
            legal_ids=out.legal_ids,
            legal_offsets=out.legal_offsets,
            rewards=out.rewards,
            terminated=out.terminated,
            truncated=out.truncated,
            actor=out.actor,
            decision_kind=out.decision_kind,
            decision_id=out.decision_id,
            engine_status=out.engine_status,
            spec_hash=out.spec_hash,
        ),
        actions,
    )


def step_rl_sample_from_logits_i16_legal_ids_into(
    pool: EnvPool, logits, seeds, actions, out: BatchOutMinimalI16LegalIds
):
    """Sample actions from logits into preallocated buffers."""
    logits = np.ascontiguousarray(logits, dtype=np.float32)
    seeds = np.asarray(seeds, dtype=np.uint64).ravel()
    pool.step_sample_from_logits_into_i16_legal_ids(logits, seeds, actions, out)
    return (
        RlStepI16LegalIds(
            obs=out.obs,
            legal_ids=out.legal_ids,
            legal_offsets=out.legal_offsets,
            rewards=out.rewards,
            terminated=out.terminated,
            truncated=out.truncated,
            actor=out.actor,
            decision_kind=out.decision_kind,
            decision_id=out.decision_id,
            engine_status=out.engine_status,
            spec_hash=out.spec_hash,
        ),
        actions,
    )
