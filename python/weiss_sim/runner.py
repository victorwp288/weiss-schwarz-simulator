from __future__ import annotations

import copy
from typing import Callable

import numpy as np

from .config_types import IdsSafety, LegalRepr, RuntimeMode
from .errors import ConfigConflictError, WeissSimError
from .types import ResetBatch, StepBatch
from .weiss_sim import PASS_ACTION_ID

_U64_MASK = np.uint64(0xFFFFFFFFFFFFFFFF)
_LEGAL_SPOTCHECK_INTERVAL = 4096


def _mix_u64(values: np.ndarray) -> np.ndarray:
    x = values.astype(np.uint64, copy=False)
    x = (x + np.uint64(0x9E3779B97F4A7C15)) & _U64_MASK
    x = (x ^ (x >> np.uint64(30))) & _U64_MASK
    x = (x * np.uint64(0xBF58476D1CE4E5B9)) & _U64_MASK
    x = (x ^ (x >> np.uint64(27))) & _U64_MASK
    x = (x * np.uint64(0x94D049BB133111EB)) & _U64_MASK
    x = (x ^ (x >> np.uint64(31))) & _U64_MASK
    return x


def _episode_key(
    episode_seed: np.ndarray, episode_index: np.ndarray, env_index: np.ndarray
) -> np.ndarray:
    combo = (episode_index.astype(np.uint64) << np.uint64(32)) ^ env_index.astype(np.uint64)
    return _mix_u64(episode_seed.astype(np.uint64) ^ _mix_u64(combo))


def _validate_legal_ids_contract(
    ids: np.ndarray, offsets: np.ndarray, num_envs: int, action_space: int
) -> None:
    if offsets.shape[0] != num_envs + 1:
        raise WeissSimError(f"legal_offsets must have shape ({num_envs + 1},), got {offsets.shape}")
    if int(offsets[0]) != 0:
        raise WeissSimError("legal_offsets[0] must be 0")
    if np.any(offsets[1:] < offsets[:-1]):
        raise WeissSimError("legal_offsets must be nondecreasing")
    last = int(offsets[-1])
    if last > ids.shape[0]:
        raise WeissSimError(f"legal_offsets[-1] ({last}) exceeds legal_ids length ({ids.shape[0]})")
    for env_index in range(num_envs):
        start = int(offsets[env_index])
        end = int(offsets[env_index + 1])
        if end <= start:
            continue
        env_ids = ids[start:end]
        if np.any(env_ids >= action_space):
            raise WeissSimError(
                f"legal ids for env {env_index} contain values outside action space {action_space}"
            )
        if env_ids.shape[0] > 1 and np.any(env_ids[1:] <= env_ids[:-1]):
            raise WeissSimError(
                f"legal ids for env {env_index} must be strictly ascending with no duplicates"
            )


class SimRunner:
    def __init__(
        self,
        *,
        pool,
        out,
        reset_method: str,
        step_method: str,
        has_mask: bool,
        embedded_legal_ids: bool,
        legal_repr: LegalRepr,
        ids_safety: IdsSafety | None,
        runtime_mode: RuntimeMode,
        control_seat: int | None,
        effective: dict[str, object],
        spec_fn: Callable[[], dict[str, object]],
    ) -> None:
        self.pool = pool
        self._out = out
        self._reset_method = reset_method
        self._step_method = step_method
        self._has_mask = has_mask
        self._embedded_legal_ids = embedded_legal_ids
        self._legal_repr = legal_repr
        self._ids_safety = ids_safety
        self._runtime_mode = runtime_mode
        self._control_seat = control_seat
        self._effective = copy.deepcopy(effective)
        self._spec_fn = spec_fn
        self._closed = False
        self._step_count = 0

        self._num_envs = int(self.pool.envs_len)
        self._action_space = int(self.pool.action_space)
        self._last_to_play_seat = np.full((self._num_envs,), -1, dtype=np.int8)
        self._last_done = np.zeros((self._num_envs,), dtype=np.bool_)
        self._legal_ids_buf = np.empty(self._num_envs * self._action_space, dtype=np.uint16)
        self._legal_offsets_buf = np.zeros(self._num_envs + 1, dtype=np.uint32)
        self._u16_max = np.iinfo(np.uint16).max

        if self._legal_repr == "ids_u16" and self._ids_safety == "checked":
            if self._action_space - 1 > self._u16_max:
                raise ConfigConflictError(
                    f"ids_u16 safety check failed: action_space={self._action_space} exceeds uint16 id range"
                )

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        self.close()
        return False

    def close(self) -> None:
        self._closed = True

    def spec(self) -> dict[str, object]:
        return self._spec_fn()

    def effective_config(self) -> dict[str, object]:
        return copy.deepcopy(self._effective)

    def _require_open(self) -> None:
        if self._closed:
            raise WeissSimError("SimRunner is closed")

    def _call_reset(self) -> None:
        getattr(self.pool, self._reset_method)(self._out)

    def _call_step(self, actions: np.ndarray) -> None:
        getattr(self.pool, self._step_method)(actions, self._out)

    def _coerce_actions(self, actions, *, name: str) -> np.ndarray:
        arr = np.asarray(actions, dtype=np.uint32).ravel()
        if arr.shape[0] != self._num_envs:
            raise WeissSimError(
                f"{name} length must equal num_envs ({self._num_envs}), got {arr.shape[0]}"
            )
        return arr

    def _legal_ids_payload(self) -> tuple[np.ndarray | None, np.ndarray | None]:
        if self._legal_repr not in {"ids_u16", "ids_u32", "both"}:
            return None, None
        if self._embedded_legal_ids:
            ids = self._out.legal_ids
            offsets = self._out.legal_offsets
        else:
            count = self.pool.legal_action_ids_into(self._legal_ids_buf, self._legal_offsets_buf)
            ids = self._legal_ids_buf[:count]
            offsets = self._legal_offsets_buf

        ids_payload: np.ndarray
        if self._legal_repr in {"ids_u32", "both"}:
            ids_payload = ids.astype(np.uint32, copy=False)
        else:
            ids_payload = ids.astype(np.uint16, copy=False)
        offsets_payload = offsets.astype(np.uint32, copy=False)
        return ids_payload, offsets_payload

    def _legal_mask_payload(self) -> np.ndarray | None:
        if not self._has_mask:
            return None
        return self._out.masks.astype(np.uint8, copy=False)

    def _common_batch_payload(self) -> dict[str, np.ndarray]:
        to_play = self._out.actor.astype(np.int8, copy=False)
        episode_seed = self.pool.episode_seed_batch()
        episode_index = self.pool.episode_index_batch()
        env_index = self.pool.env_index_batch()
        starting_seat = self.pool.starting_player_batch()
        return {
            "obs": self._out.obs,
            "to_play_seat": to_play,
            "starting_seat": starting_seat.astype(np.uint8, copy=False),
            "episode_seed": episode_seed.astype(np.uint64, copy=False),
            "episode_index": episode_index.astype(np.uint32, copy=False),
            "env_index": env_index.astype(np.uint32, copy=False),
            "decision_id": self._out.decision_id.astype(np.uint32, copy=False),
            "engine_status": self._out.engine_status.astype(np.uint8, copy=False),
            "spec_hash": self._out.spec_hash.astype(np.uint64, copy=False),
        }

    def _should_strict_check_legal_ids(self) -> bool:
        if self._legal_repr not in {"ids_u16", "ids_u32", "both"}:
            return False
        if self._runtime_mode == "eval_debug":
            return True
        return self._step_count % _LEGAL_SPOTCHECK_INTERVAL == 0

    def _validate_ids_safety(self, ids: np.ndarray) -> None:
        if self._legal_repr != "ids_u16" or self._ids_safety != "checked":
            return
        if ids.size and int(ids.max()) > self._u16_max:
            raise WeissSimError("ids_u16 safety check failed: legal id exceeds uint16 max")

    def _collect_batch_materialized(
        self,
    ) -> tuple[
        dict[str, np.ndarray],
        np.ndarray | None,
        np.ndarray | None,
        np.ndarray | None,
        np.ndarray,
    ]:
        payload = self._common_batch_payload()
        ids, offsets = self._legal_ids_payload()
        mask = self._legal_mask_payload()
        if ids is not None:
            self._validate_ids_safety(ids)
            if self._should_strict_check_legal_ids():
                _validate_legal_ids_contract(ids, offsets, self._num_envs, self._action_space)
        episode_key = _episode_key(
            payload["episode_seed"], payload["episode_index"], payload["env_index"]
        )
        return payload, ids, offsets, mask, episode_key

    def current_to_play_seat(self) -> np.ndarray:
        self._require_open()
        return self._last_to_play_seat.copy()

    def merge_actions_by_seat(
        self,
        seat0_actions,
        seat1_actions,
        *,
        default_action: int | None = None,
    ) -> np.ndarray:
        self._require_open()
        seat0 = self._coerce_actions(seat0_actions, name="seat0_actions")
        seat1 = self._coerce_actions(seat1_actions, name="seat1_actions")
        merged = np.where(self._last_to_play_seat == 0, seat0, seat1).astype(np.uint32, copy=False)
        unknown_mask = self._last_to_play_seat < 0
        if np.any(unknown_mask):
            fallback = PASS_ACTION_ID if default_action is None else int(default_action)
            merged = merged.copy()
            merged[unknown_mask] = np.uint32(fallback)
        return merged

    def step_by_seat(
        self,
        seat0_actions,
        seat1_actions,
        *,
        default_action: int | None = None,
    ) -> StepBatch:
        actions = self.merge_actions_by_seat(
            seat0_actions,
            seat1_actions,
            default_action=default_action,
        )
        return self.step(actions)

    def reset(self) -> ResetBatch:
        self._require_open()
        self._call_reset()
        payload, ids, offsets, mask, episode_key = self._collect_batch_materialized()
        batch = ResetBatch(
            obs=payload["obs"],
            to_play_seat=payload["to_play_seat"],
            starting_seat=payload["starting_seat"],
            episode_seed=payload["episode_seed"],
            episode_index=payload["episode_index"],
            env_index=payload["env_index"],
            episode_key=episode_key,
            decision_id=payload["decision_id"],
            engine_status=payload["engine_status"],
            spec_hash=payload["spec_hash"],
            legal_mask=mask,
            legal_ids=ids,
            legal_offsets=offsets,
        )
        self._last_to_play_seat = batch.to_play_seat.copy()
        self._last_done.fill(False)
        return batch

    def step(self, actions) -> StepBatch:
        self._require_open()
        arr = self._coerce_actions(actions, name="actions")
        to_play_before = self._last_to_play_seat.copy()
        self._call_step(arr)
        payload, ids, offsets, mask, episode_key = self._collect_batch_materialized()
        terminated = self._out.terminated.astype(np.bool_, copy=False)
        truncated = self._out.truncated.astype(np.bool_, copy=False)
        if np.any(np.logical_and(terminated, truncated)):
            raise WeissSimError(
                "invalid step output: terminated and truncated cannot both be true for the same env"
            )
        done = np.logical_or(terminated, truncated)
        terminal_transition = np.logical_and(np.logical_not(self._last_done), done)
        if self._control_seat is None:
            terminal_internal = np.zeros(self._num_envs, dtype=np.bool_)
        else:
            terminal_internal = np.logical_and(
                terminal_transition, to_play_before != self._control_seat
            )
        step_batch = StepBatch(
            obs=payload["obs"],
            to_play_seat=payload["to_play_seat"],
            starting_seat=payload["starting_seat"],
            episode_seed=payload["episode_seed"],
            episode_index=payload["episode_index"],
            env_index=payload["env_index"],
            episode_key=episode_key,
            decision_id=payload["decision_id"],
            engine_status=payload["engine_status"],
            spec_hash=payload["spec_hash"],
            reward=self._out.rewards.astype(np.float32, copy=False),
            terminated=terminated,
            truncated=truncated,
            terminal_during_internal_opponent=terminal_internal,
            decision_count=self.pool.decision_count_batch().astype(np.uint32, copy=False),
            tick_count=self.pool.tick_count_batch().astype(np.uint32, copy=False),
            legal_mask=mask,
            legal_ids=ids,
            legal_offsets=offsets,
        )
        self._last_to_play_seat = step_batch.to_play_seat.copy()
        self._last_done = done.copy()
        self._step_count += 1
        return step_batch
