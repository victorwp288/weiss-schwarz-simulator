from __future__ import annotations

import numpy as np


def prepare_logits(logits, num_envs: int, action_space: int | None = None) -> np.ndarray:
    """Normalize logits to a contiguous float32 array shaped (num_envs, action_space)."""
    arr = np.asarray(logits, dtype=np.float32)
    expected_envs = int(num_envs)

    if arr.ndim == 1:
        if expected_envs != 1:
            raise ValueError(
                f"logits must have shape ({expected_envs}, action_space), got {arr.shape}"
            )
        arr = arr.reshape(1, -1)

    if arr.ndim != 2 or arr.shape[0] != expected_envs:
        raise ValueError(f"logits must have shape ({expected_envs}, action_space), got {arr.shape}")

    if action_space is not None and arr.shape[1] != int(action_space):
        raise ValueError(
            "logits action dimension does not match action_space "
            f"({arr.shape[1]} != {int(action_space)})"
        )

    return np.ascontiguousarray(arr, dtype=np.float32)


def prepare_seeds(seeds, num_envs: int) -> np.ndarray:
    """Normalize seeds to a contiguous uint64 vector of length num_envs."""
    expected_envs = int(num_envs)
    arr = np.asarray(seeds, dtype=np.uint64)

    if arr.ndim == 0:
        return np.full(expected_envs, arr.item(), dtype=np.uint64)

    arr = np.ravel(arr)
    if arr.shape[0] != expected_envs:
        raise ValueError(
            f"seed array length must equal num_envs ({expected_envs}), got {arr.shape[0]}"
        )

    return np.ascontiguousarray(arr, dtype=np.uint64)
