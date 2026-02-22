from __future__ import annotations

from typing import TYPE_CHECKING, Any

import numpy as np

from .errors import WeissSimError
from .types import LegalActions, StepBatch

if TYPE_CHECKING:
    from .runner import WeissEnv


_BATCH_INFO_FIELDS = ("to_play_seat", "starting_seat", "decision_id", "engine_status", "spec_hash")


def _batch_info(
    batch, *, env_index: int | None = None, mask_key: str | None = None
) -> dict[str, Any]:
    info: dict[str, Any] = {}
    for field in _BATCH_INFO_FIELDS:
        value = getattr(batch, field)
        info[field] = value if env_index is None else int(value[env_index])
    if mask_key is not None and batch.legal_mask is not None:
        info[mask_key] = batch.legal_mask if env_index is None else batch.legal_mask[env_index]
    return info


def _import_gym_module():
    try:
        import gymnasium as gym  # type: ignore
    except ImportError:
        try:
            import gym  # type: ignore
        except ImportError as exc:
            raise WeissSimError("as_gym() requires gymnasium or gym to be installed") from exc
    return gym


class SingleEnvLegalActions:
    """Scalar-friendly legal action helpers for `num_envs == 1`."""

    def __init__(self, legal: LegalActions) -> None:
        self._legal = legal

    @property
    def mask(self) -> np.ndarray | None:
        mask = self._legal.mask
        if mask is None:
            return None
        return mask[0]

    def ids(self) -> np.ndarray:
        return self._legal.ids(0)

    def contains(self, action_id: int) -> bool:
        return self._legal.contains(0, action_id)

    def sample_uniform(self, *, seed: int | None = None) -> int:
        return int(self._legal.sample_uniform(seed=seed)[0])

    def select_from_logits(self, logits, *, illegal_value: float = -1e9) -> int:
        try:
            return int(self._legal.argmax_logits(logits, illegal_value=illegal_value)[0])
        except ValueError as exc:
            if str(exc).startswith("logits must have shape"):
                raise WeissSimError("single-env logits must have shape (A,) or (1, A)") from exc
            raise

    def sample_from_logits(
        self,
        logits,
        *,
        seed: int | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
    ) -> int:
        try:
            return int(
                self._legal.sample_logits(
                    logits,
                    seed=seed,
                    temperature=temperature,
                    illegal_value=illegal_value,
                )[0]
            )
        except ValueError as exc:
            if str(exc).startswith("logits must have shape"):
                raise WeissSimError("single-env logits must have shape (A,) or (1, A)") from exc
            raise


class SingleEnvAdapter:
    """Scalar API wrapper for debugging when `num_envs == 1`."""

    def __init__(self, env: WeissEnv) -> None:
        self.env = env
        if self.env.num_envs != 1:
            raise WeissSimError("as_single_env() requires num_envs == 1")
        self._action_buf = np.empty(1, dtype=np.uint32)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        self.close()
        return False

    @property
    def legal(self) -> SingleEnvLegalActions:
        return SingleEnvLegalActions(self.env.legal)

    def _build_info(self, batch) -> dict[str, Any]:
        info = _batch_info(batch, env_index=0, mask_key="legal_mask")
        info["legal_ids"] = batch.legal.ids(0)
        if isinstance(batch, StepBatch):
            info["done"] = bool(batch.done[0])
        return info

    def reset(self, *, seed: int | None = None):
        batch = self.env.reset(seed=seed)
        return batch.obs[0]

    def step(self, action: int):
        self._action_buf[0] = np.uint32(int(action))
        batch = self.env.step(self._action_buf)
        return (
            batch.obs[0],
            float(batch.reward[0]),
            bool(batch.terminated[0]),
            bool(batch.truncated[0]),
            self._build_info(batch),
        )

    def render(self, mode: str = "ansi"):
        return self.env.render(env_i=0, mode=mode)

    def close(self) -> None:
        self.env.close()

    def decode_action(self, action_id: int):
        return self.env.decode_action(action_id)


class GymVectorEnvAdapter:
    """Gym-compatible vector-style wrapper around `WeissEnv`."""

    metadata = {"render_modes": ["ansi"]}

    def __init__(self, env: WeissEnv) -> None:
        self.env = env
        self.num_envs = env.num_envs

        gym = _import_gym_module()
        obs_dtype = np.asarray(self.env._out.obs).dtype
        obs_shape = self.env.obs_shape
        if np.issubdtype(obs_dtype, np.integer):
            obs_info = np.iinfo(obs_dtype)
            low = obs_info.min
            high = obs_info.max
        else:
            low = -np.inf
            high = np.inf

        self.single_observation_space = gym.spaces.Box(
            low=low,
            high=high,
            shape=obs_shape,
            dtype=obs_dtype,
        )
        self.single_action_space = gym.spaces.Discrete(int(self.env.action_space_n))
        self.observation_space = self.single_observation_space
        self.action_space = self.single_action_space
        self.reward_range = (-float("inf"), float("inf"))

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        self.close()
        return False

    def _batch_info(self, batch) -> dict[str, Any]:
        return _batch_info(batch, mask_key="action_mask")

    def reset(self, *, seed: int | None = None, options: dict[str, object] | None = None):
        _ = options
        batch = self.env.reset(seed=seed)
        return batch.obs, self._batch_info(batch)

    def step(self, actions):
        batch = self.env.step(actions)
        return (
            batch.obs,
            batch.reward,
            batch.terminated,
            batch.truncated,
            self._batch_info(batch),
        )

    def action_masks(self) -> np.ndarray | None:
        return self.env.legal.mask_for_action_space(self.env.action_space_n)

    def render(self, mode: str = "ansi"):
        return self.env.render(env_i=0, mode=mode)

    def close(self) -> None:
        self.env.close()
