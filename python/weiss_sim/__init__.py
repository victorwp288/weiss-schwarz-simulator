from __future__ import annotations

import numpy as np

from .weiss_sim import (
    ACTION_SPACE_SIZE,
    OBS_LEN,
    PASS_ACTION_ID,
    SPEC_HASH,
    BatchOutDebug,
    BatchOutMinimal,
    EnvPool,
    __version__,
)
from .rl import RlStep, pass_action_id_for_decision_kind, reset_rl, step_rl


class EnvPoolBuffers:
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
        self.decision_id = self.out.decision_id
        self.engine_status = self.out.engine_status
        self.spec_hash = self.out.spec_hash
        self.legal_ids = np.empty(num_envs * pool.action_space, dtype=np.uint16)
        self.legal_offsets = np.zeros(num_envs + 1, dtype=np.uint32)

    def reset(self):
        self.pool.reset_into(self.out)
        return self.out

    def reset_indices(self, indices):
        self.pool.reset_indices_into(list(indices), self.out)
        return self.out

    def reset_done(self, done_mask):
        self.pool.reset_done_into(done_mask, self.out)
        return self.out

    def step(self, actions):
        self.pool.step_into(actions, self.out)
        return self.out

    def legal_action_ids(self):
        count = self.pool.legal_action_ids_into(self.legal_ids, self.legal_offsets)
        return self.legal_ids[:count], self.legal_offsets


__all__ = [
    "EnvPool",
    "EnvPoolBuffers",
    "BatchOutMinimal",
    "BatchOutDebug",
    "ACTION_SPACE_SIZE",
    "OBS_LEN",
    "SPEC_HASH",
    "RlStep",
    "reset_rl",
    "step_rl",
    "pass_action_id_for_decision_kind",
    "PASS_ACTION_ID",
    "__version__",
]
