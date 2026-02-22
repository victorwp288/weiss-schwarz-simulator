from __future__ import annotations

from typing import Any

import numpy as np

from ._logits_utils import prepare_logits, prepare_seeds


def coerce_logits(logits: object, *, num_envs: int, action_space: int | None = None) -> np.ndarray:
    return prepare_logits(logits, num_envs=num_envs, action_space=action_space)


def coerce_seeds(seeds: object, *, num_envs: int) -> np.ndarray:
    return prepare_seeds(seeds, num_envs)


def coerce_actions_u32(
    actions: Any,
    *,
    num_envs: int,
    name: str = "actions",
    error_cls: type[Exception] = ValueError,
    allow_none: bool = False,
) -> np.ndarray:
    if actions is None:
        if allow_none:
            return np.empty(int(num_envs), dtype=np.uint32)
        raise error_cls(f"{name} must not be None")
    arr = np.asarray(actions, dtype=np.uint32)
    arr = np.ravel(arr)
    if arr.shape[0] != int(num_envs):
        raise error_cls(f"{name} length must equal num_envs ({num_envs}), got {arr.shape[0]}")
    return np.ascontiguousarray(arr, dtype=np.uint32)


def expand_sample_seeds(seed: int | np.ndarray | None, *, num_envs: int) -> np.ndarray:
    if seed is None:
        rng = np.random.default_rng()
        return rng.integers(0, np.iinfo(np.uint64).max, size=num_envs, dtype=np.uint64)
    if np.isscalar(seed):
        rng = np.random.default_rng(int(seed))
        return rng.integers(0, np.iinfo(np.uint64).max, size=num_envs, dtype=np.uint64)
    return coerce_seeds(seed, num_envs=num_envs)
