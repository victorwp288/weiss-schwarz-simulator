from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .weiss_sim import PASS_ACTION_ID, BatchOutMinimal, EnvPool


@dataclass(frozen=True)
class RlStep:
    obs: np.ndarray
    masks: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray


def pass_action_id_for_decision_kind(decision_kind):
    return PASS_ACTION_ID


def reset_rl(pool: EnvPool) -> RlStep:
    out = BatchOutMinimal(pool.envs_len)
    pool.reset_into(out)
    return RlStep(
        obs=out.obs,
        masks=out.masks,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def step_rl(pool: EnvPool, actions) -> RlStep:
    out = BatchOutMinimal(pool.envs_len)
    pool.step_into(actions, out)
    return RlStep(
        obs=out.obs,
        masks=out.masks,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )
