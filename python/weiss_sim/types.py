"""Typed batch and legality helpers for the high-level `weiss_sim` API."""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field
from typing import Literal, Protocol

import numpy as np

from ._coerce import coerce_logits, coerce_seeds
from .weiss_sim import PASS_ACTION_ID


def _coerce_logits_with_utils(logits: np.ndarray, *, num_envs: int) -> np.ndarray:
    return coerce_logits(logits, num_envs=num_envs)


def _coerce_seed_vector_with_utils(seeds, *, num_envs: int) -> np.ndarray:
    seeds_arr = np.asarray(seeds)
    if seeds_arr.ndim == 0 and num_envs != 1:
        raise ValueError(f"seed array length must equal num_envs ({num_envs}), got 1")
    return coerce_seeds(seeds, num_envs=num_envs)


class _LegalBatchProtocol(Protocol):
    legal_mask: np.ndarray | None
    legal_ids: np.ndarray | None
    legal_offsets: np.ndarray | None
    legal_action_meta: np.ndarray | None
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
                legal_action_meta=self.legal_action_meta,
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


@dataclass(slots=True, frozen=True)
class DeckValidationIssue:
    """Structured deck validation finding."""

    code: str
    message: str
    severity: Literal["error", "warning"]
    card_id: int | None = None
    card_no: str | None = None
    got: int | None = None
    max_allowed: int | None = None
    suggestions: list[str] = field(default_factory=list)


@dataclass(slots=True)
class DeckValidationReport:
    """Non-throwing deck validation result."""

    ok: bool
    deck_size: int
    resolved_ids: list[int]
    errors: list[DeckValidationIssue]
    warnings: list[DeckValidationIssue]
    summary: dict[str, int]


@dataclass(slots=True)
class LegalActions:
    """Convenience helpers for consuming legal actions.

    Instances are constructed lazily from `ResetBatch` / `StepBatch` via `batch.legal`.
    The underlying representation may be a dense mask, packed ids/offsets, or both.
    """

    legal_ids: np.ndarray | None
    legal_offsets: np.ndarray | None
    legal_mask_raw: np.ndarray | None
    legal_action_meta: np.ndarray | None = None

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

        return mask

    def mask_for_action_space(self, action_space: int) -> np.ndarray:
        """Return a dense legal mask with exactly `action_space` columns."""
        return self._dense_mask(action_space=int(action_space))

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

    def meta(self, i: int) -> np.ndarray:
        env_i = self._coerce_env_index(i)
        if self.legal_action_meta is None or self.legal_offsets is None:
            raise ValueError("legal action metadata is only available for ids/offsets batches")
        start = int(self.legal_offsets[env_i])
        end = int(self.legal_offsets[env_i + 1])
        return self.legal_action_meta[start:end]

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

    def _resolve_rng_sources(
        self, seed: int | np.ndarray | None
    ) -> tuple[np.random.Generator | None, np.ndarray | None]:
        if seed is None:
            return np.random.default_rng(), None
        if np.isscalar(seed):
            return np.random.default_rng(int(seed)), None
        return None, _coerce_seed_vector_with_utils(seed, num_envs=self.num_envs)

    def _rng_for_env(
        self,
        env_i: int,
        *,
        global_rng: np.random.Generator | None,
        per_env_seeds: np.ndarray | None,
    ) -> np.random.Generator:
        if per_env_seeds is None:
            assert global_rng is not None
            return global_rng
        return np.random.default_rng(int(per_env_seeds[env_i]))

    def _pick_index(
        self,
        size: int,
        env_i: int,
        *,
        probs: np.ndarray | None,
        global_rng: np.random.Generator | None,
        per_env_seeds: np.ndarray | None,
    ) -> int:
        rng = self._rng_for_env(env_i, global_rng=global_rng, per_env_seeds=per_env_seeds)
        if probs is None:
            return int(rng.integers(size))
        return int(rng.choice(size, p=probs))

    @staticmethod
    def _softmax_probs(row_logits: np.ndarray) -> np.ndarray | None:
        row_max = float(np.max(row_logits))
        probs = np.exp(row_logits - row_max)
        probs_sum = float(np.sum(probs))
        if not np.isfinite(probs_sum) or probs_sum <= 0.0:
            return None
        return probs / probs_sum

    def _masked_logits_and_legal_presence(
        self, logits: np.ndarray, *, illegal_value: float
    ) -> tuple[np.ndarray, np.ndarray]:
        arr = self._coerce_logits(logits)
        mask = self._dense_mask(action_space=arr.shape[1])
        masked = np.where(mask != 0, arr, np.float32(illegal_value))
        has_legal = np.any(mask != 0, axis=1)
        return masked, has_legal

    def sample_uniform(self, seed: int | np.ndarray | None = None) -> np.ndarray:
        out = np.full(self.num_envs, np.uint32(PASS_ACTION_ID), dtype=np.uint32)
        global_rng, per_env_seeds = self._resolve_rng_sources(seed)
        for i in range(self.num_envs):
            ids_i = self.ids(i)
            if ids_i.size == 0:
                continue
            pick = self._pick_index(
                ids_i.size,
                i,
                probs=None,
                global_rng=global_rng,
                per_env_seeds=per_env_seeds,
            )
            out[i] = np.uint32(ids_i[pick])
        return out

    def first_legal(self, default_action: int | None = None) -> np.ndarray:
        fallback = PASS_ACTION_ID if default_action is None else int(default_action)
        out = np.full(self.num_envs, np.uint32(fallback), dtype=np.uint32)
        if self.num_envs == 0:
            return out

        if self.legal_ids is not None and self.legal_offsets is not None:
            starts = self.legal_offsets[:-1].astype(np.int64, copy=False)
            ends = self.legal_offsets[1:].astype(np.int64, copy=False)
            nonempty = ends > starts
            if np.any(nonempty):
                out[nonempty] = np.asarray(
                    self.legal_ids[starts[nonempty]], dtype=np.uint32
                ).astype(np.uint32, copy=False)
            return out

        mask = self._dense_mask()
        has_legal = np.any(mask != 0, axis=1)
        if np.any(has_legal):
            first_ids = np.argmax(mask != 0, axis=1).astype(np.uint32, copy=False)
            out[has_legal] = first_ids[has_legal]
        return out

    def _has_legal_per_env(self) -> np.ndarray:
        if self.legal_offsets is not None:
            starts = self.legal_offsets[:-1]
            ends = self.legal_offsets[1:]
            return ends > starts
        mask = self._dense_mask()
        return np.any(mask != 0, axis=1)

    def _apply_default_for_empty_legal(
        self, actions: np.ndarray, *, default_action: int | None
    ) -> np.ndarray:
        if default_action is None:
            return actions
        has_legal = self._has_legal_per_env()
        if np.all(has_legal):
            return actions
        fallback = np.uint32(int(default_action))
        out = actions.copy()
        out[np.logical_not(has_legal)] = fallback
        return out

    def choose(
        self,
        strategy: Literal["first", "uniform", "random", "argmax", "select", "sample"] = "first",
        *,
        logits: np.ndarray | None = None,
        seed: int | np.ndarray | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
        default_action: int | None = None,
    ) -> np.ndarray:
        token = str(strategy).strip().lower()
        if token == "first":
            return self.first_legal(default_action=default_action)
        if token in {"uniform", "random"}:
            actions = self.sample_uniform(seed=seed)
            return self._apply_default_for_empty_legal(actions, default_action=default_action)
        if token in {"argmax", "select"}:
            if logits is None:
                raise ValueError("logits is required when strategy is 'argmax'/'select'")
            actions = self.argmax_logits(logits, illegal_value=illegal_value)
            return self._apply_default_for_empty_legal(actions, default_action=default_action)
        if token == "sample":
            if logits is None:
                raise ValueError("logits is required when strategy is 'sample'")
            actions = self.sample_logits(
                logits,
                seed=seed,
                temperature=temperature,
                illegal_value=illegal_value,
            )
            return self._apply_default_for_empty_legal(actions, default_action=default_action)
        raise ValueError("strategy must be one of: first, uniform, random, argmax, select, sample")

    def mask_logits(self, logits: np.ndarray, illegal_value: float = -1e9) -> np.ndarray:
        masked, _ = self._masked_logits_and_legal_presence(logits, illegal_value=illegal_value)
        return masked

    def argmax_logits(self, logits: np.ndarray, illegal_value: float = -1e9) -> np.ndarray:
        masked, has_legal = self._masked_logits_and_legal_presence(
            logits, illegal_value=illegal_value
        )
        actions = np.argmax(masked, axis=1).astype(np.uint32, copy=False)
        if not np.all(has_legal):
            actions = actions.copy()
            actions[np.logical_not(has_legal)] = np.uint32(PASS_ACTION_ID)
        return actions

    def sample_logits(
        self,
        logits: np.ndarray,
        seed: int | np.ndarray | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
    ) -> np.ndarray:
        temp = float(temperature)
        if temp < 0.0:
            raise ValueError("temperature must be >= 0")
        if temp == 0.0:
            return self.argmax_logits(logits, illegal_value=illegal_value)

        masked, _ = self._masked_logits_and_legal_presence(logits, illegal_value=illegal_value)
        if temp != 1.0:
            masked = masked / np.float32(temp)

        actions = np.full(self.num_envs, np.uint32(PASS_ACTION_ID), dtype=np.uint32)
        global_rng, per_env_seeds = self._resolve_rng_sources(seed)

        for i in range(self.num_envs):
            legal_ids_i = self.ids(i)
            if legal_ids_i.size == 0:
                continue
            row_logits = masked[i, legal_ids_i]
            probs = self._softmax_probs(row_logits)
            pick = self._pick_index(
                legal_ids_i.size,
                i,
                probs=probs,
                global_rng=global_rng,
                per_env_seeds=per_env_seeds,
            )
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
    main_move_action: np.ndarray
    main_pass_action: np.ndarray
    legal_mask: np.ndarray | None = None
    legal_ids: np.ndarray | None = None
    legal_offsets: np.ndarray | None = None
    legal_action_meta: np.ndarray | None = None
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
    no_progress_count: np.ndarray = field(default_factory=lambda: np.zeros((0,), dtype=np.uint32))
    main_move_action: np.ndarray = field(default_factory=lambda: np.zeros((0,), dtype=np.bool_))
    main_pass_action: np.ndarray = field(default_factory=lambda: np.zeros((0,), dtype=np.bool_))
    legal_mask: np.ndarray | None = None
    legal_ids: np.ndarray | None = None
    legal_offsets: np.ndarray | None = None
    legal_action_meta: np.ndarray | None = None
    _legal_cache: LegalActions | None = field(default=None, init=False, repr=False, compare=False)

    @property
    def done(self) -> np.ndarray:
        return np.logical_or(self.terminated, self.truncated)

    @property
    def needs_reset(self) -> np.ndarray:
        return np.logical_or(self.done, self.engine_status != 0)

    @property
    def done_indices(self) -> np.ndarray:
        return np.flatnonzero(self.done).astype(np.int64, copy=False)

    @property
    def error_indices(self) -> np.ndarray:
        return np.flatnonzero(self.engine_status != 0).astype(np.int64, copy=False)

    @property
    def needs_reset_indices(self) -> np.ndarray:
        return np.flatnonzero(self.needs_reset).astype(np.int64, copy=False)
