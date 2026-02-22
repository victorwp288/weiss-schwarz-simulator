"""Typed batch and legality helpers for the high-level `weiss_sim` API."""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field
from typing import Protocol

import numpy as np

from ._logits_utils import prepare_logits, prepare_seeds
from .weiss_sim import PASS_ACTION_ID


def _coerce_logits_with_utils(logits: np.ndarray, *, num_envs: int) -> np.ndarray:
    return prepare_logits(logits, num_envs=num_envs)


def _coerce_seed_vector_with_utils(seeds, *, num_envs: int) -> np.ndarray:
    seeds_arr = np.asarray(seeds)
    if seeds_arr.ndim == 0 and num_envs != 1:
        raise ValueError(f"seed array length must equal num_envs ({num_envs}), got 1")
    return prepare_seeds(seeds, num_envs=num_envs)


class _LegalBatchProtocol(Protocol):
    legal_mask: np.ndarray | None
    legal_ids: np.ndarray | None
    legal_offsets: np.ndarray | None
    _legal_cache: LegalActions | None


class _LegalBatchMixin:
    __slots__ = ()

    @property
    def legal(self: _LegalBatchProtocol) -> LegalActions:
        legal = self._legal_cache
        if legal is None:
            legal = LegalActions(
                legal_ids=self.legal_ids,
                legal_offsets=self.legal_offsets,
                legal_mask_raw=self.legal_mask,
            )
            self._legal_cache = legal
        return legal


@dataclass(slots=True, frozen=True)
class CardRef:
    """Lightweight card metadata reference from the packaged catalog."""

    id: int
    card_no: str
    name: str
    card_type: str
    card_set: str | None
    strict_ok: bool
    approx_ok: bool


@dataclass(slots=True)
class LegalActions:
    """Convenience helpers for consuming legal actions.

    Instances are constructed lazily from `ResetBatch` / `StepBatch` via `batch.legal`.
    The underlying representation may be a dense mask, packed ids/offsets, or both.
    """

    legal_ids: np.ndarray | None
    legal_offsets: np.ndarray | None
    legal_mask_raw: np.ndarray | None
    _mask_cache: np.ndarray | None = field(default=None, init=False, repr=False, compare=False)

    @property
    def num_envs(self) -> int:
        if self.legal_offsets is not None:
            return int(self.legal_offsets.shape[0]) - 1
        if self.legal_mask_raw is not None:
            return int(self.legal_mask_raw.shape[0])
        return 0

    @property
    def action_space(self) -> int:
        if self.legal_mask_raw is not None:
            return int(self.legal_mask_raw.shape[1])
        if self.legal_ids is None or self.legal_ids.size == 0:
            return 0
        return int(np.max(self.legal_ids)) + 1

    def _coerce_env_index(self, i: int) -> int:
        env_i = int(i)
        if env_i < 0 or env_i >= self.num_envs:
            raise IndexError(f"env index {env_i} out of range for {self.num_envs} envs")
        return env_i

    def _coerce_logits(self, logits: np.ndarray) -> np.ndarray:
        return _coerce_logits_with_utils(logits, num_envs=self.num_envs)

    def _dense_mask(self, action_space: int | None = None) -> np.ndarray:
        if self.legal_mask_raw is not None:
            if action_space is not None and int(action_space) != int(self.legal_mask_raw.shape[1]):
                raise ValueError(
                    "logits action dimension does not match legal mask width "
                    f"({action_space} != {self.legal_mask_raw.shape[1]})"
                )
            return self.legal_mask_raw

        if self.legal_ids is None or self.legal_offsets is None:
            raise ValueError("legal ids/offsets or legal mask are required")

        if action_space is None:
            action_space = self.action_space
            if self._mask_cache is not None:
                return self._mask_cache

        mask = np.zeros((self.num_envs, int(action_space)), dtype=np.uint8)
        for i in range(self.num_envs):
            ids_i = self.ids(i)
            if ids_i.size == 0:
                continue
            if int(ids_i[-1]) >= int(action_space):
                raise ValueError(
                    f"action id {int(ids_i[-1])} is out of bounds for action space {action_space}"
                )
            mask[i, ids_i] = 1

        if action_space == self.action_space:
            self._mask_cache = mask
        return mask

    @property
    def mask(self) -> np.ndarray | None:
        if self.legal_mask_raw is not None:
            return self.legal_mask_raw
        if self.legal_ids is None or self.legal_offsets is None:
            return None
        return self._dense_mask()

    def ids(self, i: int) -> np.ndarray:
        env_i = self._coerce_env_index(i)
        if self.legal_ids is not None and self.legal_offsets is not None:
            start = int(self.legal_offsets[env_i])
            end = int(self.legal_offsets[env_i + 1])
            return self.legal_ids[start:end]
        mask_row = self._dense_mask()[env_i]
        return np.flatnonzero(mask_row).astype(np.uint32, copy=False)

    def __getitem__(self, i: int) -> np.ndarray:
        return self.ids(i)

    def iter_ids(self) -> Iterator[np.ndarray]:
        """Yield the legal action id vector for each environment."""
        for i in range(self.num_envs):
            yield self.ids(i)

    def contains(self, i: int, action_id: int) -> bool:
        env_i = self._coerce_env_index(i)
        action = int(action_id)
        if action < 0:
            return False
        if self.legal_mask_raw is not None:
            if action >= self.legal_mask_raw.shape[1]:
                return False
            return bool(self.legal_mask_raw[env_i, action] != 0)
        ids_i = self.ids(env_i)
        if ids_i.size == 0:
            return False
        pos = int(np.searchsorted(ids_i, action))
        return pos < ids_i.shape[0] and int(ids_i[pos]) == action

    def sample_uniform(self, seed: int | np.ndarray | None = None) -> np.ndarray:
        out = np.full(self.num_envs, np.uint32(PASS_ACTION_ID), dtype=np.uint32)
        if seed is None:
            rng = np.random.default_rng()
            for i in range(self.num_envs):
                ids_i = self.ids(i)
                if ids_i.size:
                    out[i] = np.uint32(ids_i[int(rng.integers(ids_i.size))])
            return out

        if np.isscalar(seed):
            rng = np.random.default_rng(int(seed))
            for i in range(self.num_envs):
                ids_i = self.ids(i)
                if ids_i.size:
                    out[i] = np.uint32(ids_i[int(rng.integers(ids_i.size))])
            return out

        seeds = _coerce_seed_vector_with_utils(seed, num_envs=self.num_envs)
        for i in range(self.num_envs):
            ids_i = self.ids(i)
            if ids_i.size == 0:
                continue
            rng = np.random.default_rng(int(seeds[i]))
            out[i] = np.uint32(ids_i[int(rng.integers(ids_i.size))])
        return out

    def mask_logits(self, logits: np.ndarray, illegal_value: float = -1e9) -> np.ndarray:
        arr = self._coerce_logits(logits)
        mask = self._dense_mask(action_space=arr.shape[1])
        return np.where(mask != 0, arr, np.float32(illegal_value))

    def select_from_logits(self, logits: np.ndarray, illegal_value: float = -1e9) -> np.ndarray:
        masked = self.mask_logits(logits, illegal_value=illegal_value)
        actions = np.argmax(masked, axis=1).astype(np.uint32, copy=False)
        mask = self._dense_mask(action_space=masked.shape[1])
        has_legal = np.any(mask != 0, axis=1)
        if not np.all(has_legal):
            actions = actions.copy()
            actions[np.logical_not(has_legal)] = np.uint32(PASS_ACTION_ID)
        return actions

    def sample_from_logits(
        self,
        logits: np.ndarray,
        seed: int | np.ndarray | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
    ) -> np.ndarray:
        if temperature <= 0:
            raise ValueError("temperature must be > 0")
        masked = self.mask_logits(logits, illegal_value=illegal_value)
        if temperature != 1.0:
            masked = masked / np.float32(temperature)

        actions = np.full(self.num_envs, np.uint32(PASS_ACTION_ID), dtype=np.uint32)
        if seed is None:
            global_rng = np.random.default_rng()
            per_env_seeds = None
        elif np.isscalar(seed):
            global_rng = np.random.default_rng(int(seed))
            per_env_seeds = None
        else:
            global_rng = None
            per_env_seeds = _coerce_seed_vector_with_utils(seed, num_envs=self.num_envs)

        for i in range(self.num_envs):
            legal_ids_i = self.ids(i)
            if legal_ids_i.size == 0:
                continue
            row_logits = masked[i, legal_ids_i]
            row_max = float(np.max(row_logits))
            probs = np.exp(row_logits - row_max)
            probs_sum = float(np.sum(probs))
            if not np.isfinite(probs_sum) or probs_sum <= 0.0:
                if per_env_seeds is None:
                    pick = int(global_rng.integers(legal_ids_i.size))
                else:
                    rng_i = np.random.default_rng(int(per_env_seeds[i]))
                    pick = int(rng_i.integers(legal_ids_i.size))
            else:
                probs = probs / probs_sum
                if per_env_seeds is None:
                    pick = int(global_rng.choice(legal_ids_i.size, p=probs))
                else:
                    rng_i = np.random.default_rng(int(per_env_seeds[i]))
                    pick = int(rng_i.choice(legal_ids_i.size, p=probs))
            actions[i] = np.uint32(legal_ids_i[pick])
        return actions


@dataclass(slots=True)
class ResetBatch(_LegalBatchMixin):
    obs: np.ndarray
    to_play_seat: np.ndarray
    starting_seat: np.ndarray
    episode_seed: np.ndarray
    episode_index: np.ndarray
    env_index: np.ndarray
    episode_key: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    legal_mask: np.ndarray | None = None
    legal_ids: np.ndarray | None = None
    legal_offsets: np.ndarray | None = None
    _legal_cache: LegalActions | None = field(default=None, init=False, repr=False, compare=False)


@dataclass(slots=True)
class StepBatch(_LegalBatchMixin):
    obs: np.ndarray
    to_play_seat: np.ndarray
    starting_seat: np.ndarray
    episode_seed: np.ndarray
    episode_index: np.ndarray
    env_index: np.ndarray
    episode_key: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    reward: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    terminal_during_internal_opponent: np.ndarray
    decision_count: np.ndarray
    tick_count: np.ndarray
    legal_mask: np.ndarray | None = None
    legal_ids: np.ndarray | None = None
    legal_offsets: np.ndarray | None = None
    _legal_cache: LegalActions | None = field(default=None, init=False, repr=False, compare=False)

    @property
    def done(self) -> np.ndarray:
        return np.logical_or(self.terminated, self.truncated)
