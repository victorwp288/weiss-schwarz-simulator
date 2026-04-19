from __future__ import annotations

import copy
import warnings
from dataclasses import dataclass
from types import TracebackType
from typing import TYPE_CHECKING, Callable, Literal

import numpy as np

from ._coerce import coerce_actions_u32, coerce_logits, expand_sample_seeds
from .config_types import IdsSafety, LegalRepr, RuntimeMode
from .errors import ConfigConflictError, WeissSimError
from ._legal_payloads import (
    cast_legal_action_meta,
    cast_legal_ids,
    cast_legal_offsets,
    materialize_legal_action_meta_u16,
    materialize_legal_ids_u16,
)
from .types import LegalActions, ResetBatch, StepBatch
from .weiss_sim import (
    ACTION_META_UNUSED,
    ACTION_META_WIDTH,
    BatchOutMinimal,
    BatchOutMinimalNoMask,
    EnvPool,
    PASS_ACTION_ID,
    decode_action_id,
)

_U64_MASK = np.uint64(0xFFFFFFFFFFFFFFFF)
_LEGAL_SPOTCHECK_INTERVAL = 4096
_DEFAULT_ILLEGAL_VALUE = -1e9
_ILLEGAL_VALUE_COMPAT_WARNING_EMITTED = False
_I16_MIN = int(np.iinfo(np.int16).min)
_I16_MAX = int(np.iinfo(np.int16).max)

if TYPE_CHECKING:
    from .adapters import GymVectorEnvAdapter, SingleEnvAdapter


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


def _episode_seeds_for_indices(seed: int, indices: np.ndarray) -> np.ndarray:
    base = np.uint64(int(seed) & int(_U64_MASK))
    idx = np.asarray(indices, dtype=np.uint64)
    return _mix_u64(np.full(idx.shape, base, dtype=np.uint64) ^ _mix_u64(idx + np.uint64(1)))


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


@dataclass(slots=True, frozen=True)
class _CommonBatchPayload:
    obs: np.ndarray
    to_play_seat: np.ndarray
    starting_seat: np.ndarray
    episode_seed: np.ndarray
    episode_index: np.ndarray
    env_index: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray


@dataclass(slots=True, frozen=True)
class _LegalPayload:
    mask: np.ndarray | None
    ids: np.ndarray | None
    meta: np.ndarray | None
    offsets: np.ndarray | None


@dataclass(slots=True, frozen=True)
class _MaterializedBatchPayload:
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
    legal_mask: np.ndarray | None
    legal_ids: np.ndarray | None
    legal_offsets: np.ndarray | None
    legal_action_meta: np.ndarray | None


class WeissEnv:
    """High-level wrapper around `EnvPool` for batched RL-style stepping.

    `WeissEnv` is intentionally minimal: `reset()` produces a `ResetBatch`, and
    `step()` produces a `StepBatch`. Both batches expose legality via `batch.legal`.

    Use it as a context manager to ensure you don't accidentally keep using a
    closed environment:

    ```python
    with weiss_sim.make(num_envs=32, seed=0) as sim:
        batch = sim.reset()
        step = sim.step(batch.legal.sample_uniform(seed=1))
    ```
    """

    def __init__(
        self,
        *,
        pool: EnvPool,
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
        self._latest_batch: ResetBatch | StepBatch | None = None

        self._num_envs = int(self.pool.envs_len)
        self._action_space = int(self.pool.action_space)
        self._last_to_play_seat = np.full((self._num_envs,), -1, dtype=np.int8)
        self._last_done = np.zeros((self._num_envs,), dtype=np.bool_)
        self._legal_ids_buf = np.empty(self._num_envs * self._action_space, dtype=np.uint16)
        self._legal_action_meta_buf = np.full(
            (self._num_envs * self._action_space, ACTION_META_WIDTH),
            ACTION_META_UNUSED,
            dtype=np.uint16,
        )
        self._legal_offsets_buf = np.zeros(self._num_envs + 1, dtype=np.uint32)
        self._u16_max = np.iinfo(np.uint16).max

        if self._legal_repr == "ids_u16" and self._ids_safety == "checked":
            if self._action_space - 1 > self._u16_max:
                raise ConfigConflictError(
                    f"ids_u16 safety check failed: action_space={self._action_space} exceeds uint16 id range"
                )

    @property
    def num_envs(self) -> int:
        return self._num_envs

    @property
    def action_space(self) -> int:
        return self._action_space

    @property
    def action_space_n(self) -> int:
        return self._action_space

    @property
    def obs_shape(self) -> tuple[int, ...]:
        return tuple(np.asarray(self._out.obs).shape[1:])

    @property
    def latest_batch(self) -> ResetBatch | StepBatch | None:
        return self._latest_batch

    @property
    def legal(self) -> LegalActions:
        """Return legality helpers for the latest reset/step batch."""
        return self._require_latest_batch().legal

    def __enter__(self) -> WeissEnv:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> bool:
        self.close()
        return False

    def close(self) -> None:
        """Mark the environment as closed.

        Note: the underlying Rust objects are reference-counted; closing prevents
        accidental re-use in Python.
        """
        self._closed = True

    def spec(self) -> dict[str, object]:
        """Return the current observation/action spec bundle."""
        return self._spec_fn()

    def effective_config(self) -> dict[str, object]:
        """Return the effective config used to construct this environment."""
        return copy.deepcopy(self._effective)

    def enable_replay_sampling(
        self,
        sample_rate: float,
        out_dir: str | None = None,
        compress: bool = False,
        include_trigger_card_id: bool = False,
        visibility_mode: str | None = None,
        store_actions: bool = True,
    ) -> None:
        """Enable replay sampling on the underlying pool."""
        self._require_open()
        self.pool.enable_replay_sampling(
            sample_rate=sample_rate,
            out_dir=out_dir,
            compress=compress,
            include_trigger_card_id=include_trigger_card_id,
            visibility_mode=visibility_mode,
            store_actions=store_actions,
        )

    def _require_open(self) -> None:
        if self._closed:
            raise WeissSimError("WeissEnv is closed")

    def _require_latest_batch(self) -> ResetBatch | StepBatch:
        batch = self._latest_batch
        if batch is None:
            raise WeissSimError("no batch available; call reset() first")
        return batch

    def _reset_suffix(self) -> str:
        prefix = "reset_into"
        if not self._reset_method.startswith(prefix):
            raise WeissSimError(f"unexpected reset method name: {self._reset_method}")
        return self._reset_method[len(prefix) :]

    def _method_for_reset_suffix(self, prefix: str) -> str:
        return f"{prefix}{self._reset_suffix()}"

    def _call_reset(self) -> None:
        getattr(self.pool, self._reset_method)(self._out)

    def _call_reset_indices(self, indices: np.ndarray) -> None:
        method_name = self._method_for_reset_suffix("reset_indices_into")
        method = getattr(self.pool, method_name)
        method(indices.tolist(), self._out)

    def _call_reset_done(self, done_mask: np.ndarray) -> None:
        method_name = self._method_for_reset_suffix("reset_done_into")
        method = getattr(self.pool, method_name)
        method(done_mask, self._out)

    def _call_reset_indices_with_episode_seeds(
        self, indices: np.ndarray, episode_seeds: np.ndarray
    ) -> bool:
        method_name = self._method_for_reset_suffix("reset_indices_with_episode_seeds_into")
        method = getattr(self.pool, method_name, None)
        if method is None:
            return False
        method(indices.tolist(), episode_seeds.tolist(), self._out)
        return True

    def _call_step(self, actions: np.ndarray) -> None:
        getattr(self.pool, self._step_method)(actions, self._out)

    def _call_step_argmax_logits(self, logits: np.ndarray, actions_out: np.ndarray) -> None:
        method_name = self._method_for_reset_suffix("step_select_from_logits_into")
        method = getattr(self.pool, method_name)
        method(logits, actions_out, self._out)

    def _call_step_sample_logits(
        self,
        logits: np.ndarray,
        seeds: np.ndarray,
        actions_out: np.ndarray,
    ) -> None:
        method_name = self._method_for_reset_suffix("step_sample_from_logits_into")
        method = getattr(self.pool, method_name)
        method(logits, seeds, actions_out, self._out)

    def _call_step_uniform_legal(self, seeds: np.ndarray, actions_out: np.ndarray) -> None:
        method_name = self._method_for_reset_suffix("step_sample_legal_action_ids_uniform_into")
        method = getattr(self.pool, method_name)
        method(seeds, actions_out, self._out)

    def _coerce_logits(self, logits: object) -> np.ndarray:
        return coerce_logits(logits, num_envs=self._num_envs, action_space=self._action_space)

    def _coerce_sample_seeds(self, seed: int | np.ndarray | None) -> np.ndarray:
        return expand_sample_seeds(seed, num_envs=self._num_envs)

    @staticmethod
    def _is_default_illegal_value(illegal_value: float) -> bool:
        return float(illegal_value) == float(_DEFAULT_ILLEGAL_VALUE)

    def _warn_illegal_value_compatibility_once(self) -> None:
        global _ILLEGAL_VALUE_COMPAT_WARNING_EMITTED
        if _ILLEGAL_VALUE_COMPAT_WARNING_EMITTED:
            return
        warnings.warn(
            "non-default illegal_value uses compatibility masking before Rust logits fast-path",
            UserWarning,
            stacklevel=3,
        )
        _ILLEGAL_VALUE_COMPAT_WARNING_EMITTED = True

    def _apply_illegal_value_compatibility_mask(
        self,
        logits: np.ndarray,
        *,
        illegal_value: float,
    ) -> np.ndarray:
        if self._is_default_illegal_value(illegal_value):
            return logits
        self._warn_illegal_value_compatibility_once()
        dense_legal_mask = self._require_latest_batch().legal.mask_for_action_space(
            self._action_space
        )
        masked = np.where(dense_legal_mask != 0, logits, np.float32(illegal_value))
        return np.ascontiguousarray(masked, dtype=np.float32)

    def _coerce_actions(self, actions, *, name: str) -> np.ndarray:
        return coerce_actions_u32(
            actions, num_envs=self._num_envs, name=name, error_cls=WeissSimError
        )

    def _coerce_indices(self, indices, *, name: str) -> np.ndarray:
        arr = np.asarray(indices)
        if arr.dtype == np.bool_:
            mask = arr.ravel()
            if mask.shape[0] != self._num_envs:
                raise WeissSimError(
                    f"{name} boolean mask length must equal num_envs ({self._num_envs}), got {mask.shape[0]}"
                )
            idx = np.flatnonzero(mask).astype(np.int64, copy=False)
        else:
            idx = np.asarray(indices, dtype=np.int64).ravel()
        if idx.size:
            if int(np.min(idx)) < 0 or int(np.max(idx)) >= self._num_envs:
                raise WeissSimError(
                    f"{name} entries must be in [0, {self._num_envs - 1}], got {idx.tolist()}"
                )
        return idx

    def _coerce_done_mask(self, done_mask, *, name: str) -> np.ndarray:
        mask = np.asarray(done_mask, dtype=np.bool_).ravel()
        if mask.shape[0] != self._num_envs:
            raise WeissSimError(
                f"{name} length must equal num_envs ({self._num_envs}), got {mask.shape[0]}"
            )
        return mask

    def _materialize_legal_ids_payload(
        self,
    ) -> tuple[np.ndarray | None, np.ndarray | None, np.ndarray | None]:
        if self._legal_repr not in {"ids_u16", "ids_u32", "both"}:
            return None, None, None
        ids_u16, offsets_u32 = materialize_legal_ids_u16(
            embedded_legal_ids=self._embedded_legal_ids,
            out=self._out,
            legal_ids_buffer=self._legal_ids_buf,
            legal_offsets_buffer=self._legal_offsets_buf,
            legal_action_ids_into=self.pool.legal_action_ids_into,
        )
        used_rows = int(offsets_u32[-1]) if offsets_u32.size else 0
        meta_u16 = materialize_legal_action_meta_u16(
            embedded_legal_ids=self._embedded_legal_ids,
            out=self._out,
            legal_action_meta_buffer=self._legal_action_meta_buf,
            used_rows=used_rows,
            legal_action_meta_into=getattr(self.pool, "legal_action_meta_into", None),
        )
        ids_payload = cast_legal_ids(ids_u16, as_uint32=self._legal_repr in {"ids_u32", "both"})
        offsets_payload = cast_legal_offsets(offsets_u32)
        return ids_payload, cast_legal_action_meta(meta_u16), offsets_payload

    def _materialize_legal_mask_payload(self) -> np.ndarray | None:
        if not self._has_mask:
            return None
        return self._out.masks.astype(np.uint8, copy=False)

    def _extract_legal_payload(self) -> _LegalPayload:
        ids, meta, offsets = self._materialize_legal_ids_payload()
        mask = self._materialize_legal_mask_payload()
        if ids is None:
            return _LegalPayload(mask=mask, ids=None, meta=None, offsets=None)
        self._validate_ids_safety(ids)
        if self._should_strict_check_legal_ids():
            if offsets is None:
                raise WeissSimError("legal offsets are required when legal ids are materialized")
            _validate_legal_ids_contract(ids, offsets, self._num_envs, self._action_space)
        return _LegalPayload(mask=mask, ids=ids, meta=meta, offsets=offsets)

    def _collect_common_batch_payload(self) -> _CommonBatchPayload:
        to_play = self._out.actor.astype(np.int8, copy=False)
        episode_seed = self.pool.episode_seed_batch()
        episode_index = self.pool.episode_index_batch()
        env_index = self.pool.env_index_batch()
        starting_seat = self.pool.starting_player_batch()
        return _CommonBatchPayload(
            obs=self._out.obs,
            to_play_seat=to_play,
            starting_seat=starting_seat.astype(np.uint8, copy=False),
            episode_seed=episode_seed.astype(np.uint64, copy=False),
            episode_index=episode_index.astype(np.uint32, copy=False),
            env_index=env_index.astype(np.uint32, copy=False),
            decision_id=self._out.decision_id.astype(np.uint32, copy=False),
            engine_status=self._out.engine_status.astype(np.uint8, copy=False),
            spec_hash=self._out.spec_hash.astype(np.uint64, copy=False),
        )

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

    def _materialize_batch_payload(self) -> _MaterializedBatchPayload:
        common = self._collect_common_batch_payload()
        legal = self._extract_legal_payload()
        return _MaterializedBatchPayload(
            obs=common.obs,
            to_play_seat=common.to_play_seat,
            starting_seat=common.starting_seat,
            episode_seed=common.episode_seed,
            episode_index=common.episode_index,
            env_index=common.env_index,
            episode_key=_episode_key(common.episode_seed, common.episode_index, common.env_index),
            decision_id=common.decision_id,
            engine_status=common.engine_status,
            spec_hash=common.spec_hash,
            main_move_action=self._out.main_move_action,
            main_pass_action=self._out.main_pass_action,
            legal_mask=legal.mask,
            legal_ids=legal.ids,
            legal_action_meta=legal.meta,
            legal_offsets=legal.offsets,
        )

    def _build_reset_batch(self) -> ResetBatch:
        payload = self._materialize_batch_payload()
        return ResetBatch(
            obs=payload.obs,
            to_play_seat=payload.to_play_seat,
            starting_seat=payload.starting_seat,
            episode_seed=payload.episode_seed,
            episode_index=payload.episode_index,
            env_index=payload.env_index,
            episode_key=payload.episode_key,
            decision_id=payload.decision_id,
            engine_status=payload.engine_status,
            spec_hash=payload.spec_hash,
            main_move_action=payload.main_move_action,
            main_pass_action=payload.main_pass_action,
            legal_mask=payload.legal_mask,
            legal_ids=payload.legal_ids,
            legal_offsets=payload.legal_offsets,
            legal_action_meta=payload.legal_action_meta,
        )

    def _finalize_reset(self, *, reset_indices: np.ndarray | None) -> ResetBatch:
        batch = self._build_reset_batch()
        self._last_to_play_seat = batch.to_play_seat.copy()
        if reset_indices is None:
            self._last_done.fill(False)
        elif reset_indices.size:
            self._last_done[reset_indices] = False
        self._latest_batch = batch
        return batch

    def _finalize_step_from_current_out(self, *, to_play_before: np.ndarray) -> StepBatch:
        payload = self._materialize_batch_payload()
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
            obs=payload.obs,
            to_play_seat=payload.to_play_seat,
            starting_seat=payload.starting_seat,
            episode_seed=payload.episode_seed,
            episode_index=payload.episode_index,
            env_index=payload.env_index,
            episode_key=payload.episode_key,
            decision_id=payload.decision_id,
            engine_status=payload.engine_status,
            spec_hash=payload.spec_hash,
            reward=self._out.rewards.astype(np.float32, copy=False),
            terminated=terminated,
            truncated=truncated,
            terminal_during_internal_opponent=terminal_internal,
            decision_count=self.pool.decision_count_batch().astype(np.uint32, copy=False),
            tick_count=self.pool.tick_count_batch().astype(np.uint32, copy=False),
            no_progress_count=self.pool.no_progress_count_batch().astype(np.uint32, copy=False),
            main_move_action=payload.main_move_action,
            main_pass_action=payload.main_pass_action,
            legal_mask=payload.legal_mask,
            legal_ids=payload.legal_ids,
            legal_offsets=payload.legal_offsets,
            legal_action_meta=payload.legal_action_meta,
        )
        self._last_to_play_seat = step_batch.to_play_seat.copy()
        self._last_done = done.copy()
        self._step_count += 1
        self._latest_batch = step_batch
        return step_batch

    def _coerce_engine_status_codes(self, codes: np.ndarray | None) -> np.ndarray:
        if codes is None:
            arr = self._require_latest_batch().engine_status.astype(np.uint8, copy=False)
        else:
            arr = np.asarray(codes, dtype=np.uint8)
        arr = np.ravel(arr)
        if arr.shape[0] != self._num_envs:
            raise WeissSimError(
                f"codes length must equal num_envs ({self._num_envs}), got {arr.shape[0]}"
            )
        return np.ascontiguousarray(arr, dtype=np.uint8)

    @staticmethod
    def _copy_optional_array(value: np.ndarray | None) -> np.ndarray | None:
        if value is None:
            return None
        return value.copy()

    def _snapshot_step_batch(self, step: StepBatch) -> StepBatch:
        return StepBatch(
            obs=step.obs.copy(),
            to_play_seat=step.to_play_seat.copy(),
            starting_seat=step.starting_seat.copy(),
            episode_seed=step.episode_seed.copy(),
            episode_index=step.episode_index.copy(),
            env_index=step.env_index.copy(),
            episode_key=step.episode_key.copy(),
            decision_id=step.decision_id.copy(),
            engine_status=step.engine_status.copy(),
            spec_hash=step.spec_hash.copy(),
            reward=step.reward.copy(),
            terminated=step.terminated.copy(),
            truncated=step.truncated.copy(),
            terminal_during_internal_opponent=step.terminal_during_internal_opponent.copy(),
            decision_count=step.decision_count.copy(),
            tick_count=step.tick_count.copy(),
            no_progress_count=step.no_progress_count.copy(),
            main_move_action=step.main_move_action.copy(),
            main_pass_action=step.main_pass_action.copy(),
            legal_mask=self._copy_optional_array(step.legal_mask),
            legal_ids=self._copy_optional_array(step.legal_ids),
            legal_offsets=self._copy_optional_array(step.legal_offsets),
            legal_action_meta=self._copy_optional_array(step.legal_action_meta),
        )

    def _copy_common_out_fields_from_nomask(
        self, src: BatchOutMinimal | BatchOutMinimalNoMask
    ) -> None:
        np.copyto(self._out.rewards, src.rewards)
        np.copyto(self._out.terminated, src.terminated)
        np.copyto(self._out.truncated, src.truncated)
        np.copyto(self._out.actor, src.actor)
        np.copyto(self._out.decision_kind, src.decision_kind)
        np.copyto(self._out.decision_id, src.decision_id)
        np.copyto(self._out.engine_status, src.engine_status)
        np.copyto(self._out.spec_hash, src.spec_hash)
        np.copyto(self._out.main_move_action, src.main_move_action)
        np.copyto(self._out.main_pass_action, src.main_pass_action)

    def _copy_i16_out_from_minimal(self, src: BatchOutMinimal) -> None:
        np.clip(src.obs, _I16_MIN, _I16_MAX, out=self._out.obs)
        np.copyto(self._out.masks, src.masks)
        self._copy_common_out_fields_from_nomask(src)

    def _copy_i16_legal_ids_out_from_nomask(self, src: BatchOutMinimalNoMask) -> None:
        np.clip(src.obs, _I16_MIN, _I16_MAX, out=self._out.obs)
        self._copy_common_out_fields_from_nomask(src)
        self.pool.legal_action_ids_into(self._out.legal_ids, self._out.legal_offsets)
        legal_action_meta_into = getattr(self.pool, "legal_action_meta_into", None)
        if callable(legal_action_meta_into) and hasattr(self._out, "legal_action_meta"):
            legal_action_meta_into(self._out.legal_action_meta)

    def _auto_reset_method_for_suffix(self) -> str:
        suffix = self._reset_suffix()
        if suffix in {"", "_i16"}:
            return "auto_reset_on_error_codes_into"
        if suffix in {"_nomask", "_i16_legal_ids"}:
            return "auto_reset_on_error_codes_into_nomask"
        raise WeissSimError(f"unexpected reset method name: {self._reset_method}")

    def _call_auto_reset_on_error_codes(self, codes: np.ndarray) -> int:
        method_name = self._auto_reset_method_for_suffix()
        suffix = self._reset_suffix()
        if suffix == "":
            return int(getattr(self.pool, method_name)(codes, self._out))
        if suffix == "_nomask":
            return int(getattr(self.pool, method_name)(codes, self._out))
        if suffix == "_i16":
            temp_out = BatchOutMinimal(self._num_envs)
            reset_count = int(getattr(self.pool, method_name)(codes, temp_out))
            if reset_count:
                self._copy_i16_out_from_minimal(temp_out)
            return reset_count
        if suffix == "_i16_legal_ids":
            temp_out = BatchOutMinimalNoMask(self._num_envs)
            reset_count = int(getattr(self.pool, method_name)(codes, temp_out))
            if reset_count:
                self._copy_i16_legal_ids_out_from_nomask(temp_out)
            return reset_count
        raise WeissSimError(f"unexpected reset method name: {self._reset_method}")

    def current_to_play_seat(self) -> np.ndarray:
        """Return the last observed `to_play_seat` vector."""
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

    def reset(self, *, seed: int | None = None, indices: object | None = None) -> ResetBatch:
        """Reset all envs or a subset of envs.

        Parameters
        ----------
        seed:
            Optional per-reset seed. When provided, episode seeds are derived
            deterministically for each reset index.
        indices:
            `None` (reset all), an index array/list, or a boolean mask with length
            `num_envs`.
        """
        self._require_open()

        if indices is None:
            if seed is None:
                self._call_reset()
            else:
                all_indices = np.arange(self._num_envs, dtype=np.int64)
                episode_seeds = _episode_seeds_for_indices(seed, all_indices)
                if not self._call_reset_indices_with_episode_seeds(all_indices, episode_seeds):
                    warnings.warn(
                        "seed parameter ignored: pool does not support episode seeds",
                        UserWarning,
                        stacklevel=2,
                    )
                    self._call_reset()
            return self._finalize_reset(reset_indices=None)

        reset_indices = self._coerce_indices(indices, name="indices")
        if seed is None:
            self._call_reset_indices(reset_indices)
        else:
            episode_seeds = _episode_seeds_for_indices(seed, reset_indices)
            if not self._call_reset_indices_with_episode_seeds(reset_indices, episode_seeds):
                warnings.warn(
                    "seed parameter ignored: pool does not support episode seeds",
                    UserWarning,
                    stacklevel=2,
                )
                self._call_reset_indices(reset_indices)
        return self._finalize_reset(reset_indices=reset_indices)

    def reset_done(self, done_mask: object) -> ResetBatch:
        """Reset envs where `done_mask` is true."""
        self._require_open()
        mask = self._coerce_done_mask(done_mask, name="done_mask")
        self._call_reset_done(mask)
        reset_indices = np.flatnonzero(mask).astype(np.int64, copy=False)
        return self._finalize_reset(reset_indices=reset_indices)

    def reset_indices(self, indices: object) -> ResetBatch:
        """Reset a specific list of env indices."""
        self._require_open()
        reset_indices = self._coerce_indices(indices, name="indices")
        self._call_reset_indices(reset_indices)
        return self._finalize_reset(reset_indices=reset_indices)

    def step(self, actions: object) -> StepBatch:
        """Advance each env by applying an action for the current seat."""
        self._require_open()
        arr = self._coerce_actions(actions, name="actions")
        return self._step_with_actions(arr)

    def _step_with_actions(self, actions: np.ndarray) -> StepBatch:
        to_play_before = self._last_to_play_seat.copy()
        self._call_step(actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before)

    def _auto_reset_done_if_needed(
        self, done_mask: np.ndarray, *, enabled: bool
    ) -> ResetBatch | None:
        if not enabled or not np.any(done_mask):
            return None
        return self.reset_done(done_mask)

    def _engine_error_codes_for_auto_reset(
        self, step: StepBatch, *, clear_done_mask: np.ndarray | None
    ) -> np.ndarray | None:
        codes = np.asarray(step.engine_status, dtype=np.uint8).copy()
        if clear_done_mask is not None:
            codes[clear_done_mask] = np.uint8(0)
        if not np.any(codes != 0):
            return None
        return codes

    def _auto_reset_engine_errors_if_needed(
        self, codes: np.ndarray | None, *, enabled: bool
    ) -> ResetBatch | None:
        if not enabled or codes is None:
            return None
        _, reset_batch = self.auto_reset_on_engine_errors(codes)
        return reset_batch

    def _apply_auto_resets(
        self,
        step: StepBatch,
        *,
        reset_done: bool,
        reset_engine_errors: bool,
    ) -> ResetBatch | None:
        done_mask = np.asarray(step.done, dtype=np.bool_)
        reset_batch = self._auto_reset_done_if_needed(done_mask, enabled=reset_done)
        codes: np.ndarray | None = None
        if reset_engine_errors:
            clear_done = done_mask if reset_done else None
            codes = self._engine_error_codes_for_auto_reset(step, clear_done_mask=clear_done)
        error_reset = self._auto_reset_engine_errors_if_needed(codes, enabled=reset_engine_errors)
        if error_reset is not None:
            return error_reset
        return reset_batch

    def step_first_legal(self) -> tuple[StepBatch, np.ndarray]:
        """Step by selecting the first legal action for each env."""
        self._require_open()
        actions = self._require_latest_batch().legal.first_legal()
        return self._step_with_actions(actions), actions

    def step_uniform_legal(
        self, seed: int | np.ndarray | None = None
    ) -> tuple[StepBatch, np.ndarray]:
        """Step by sampling a uniform-random legal action for each env."""
        self._require_open()
        self._require_latest_batch()
        seeds = self._coerce_sample_seeds(seed)
        actions = np.empty(self._num_envs, dtype=np.uint32)
        to_play_before = self._last_to_play_seat.copy()
        self._call_step_uniform_legal(seeds, actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before), actions

    def step_auto(
        self,
        actions: object | None = None,
        *,
        policy: Literal["first", "uniform", "random"] = "first",
        seed: int | np.ndarray | None = None,
        reset_done: bool = True,
        reset_engine_errors: bool = True,
    ) -> tuple[StepBatch, np.ndarray, ResetBatch | None]:
        """Step once, with optional action selection and automatic reset handling."""
        self._require_open()
        batch = self._require_latest_batch()
        if actions is None:
            chosen = batch.legal.choose(policy, seed=seed)
        else:
            chosen = self._coerce_actions(actions, name="actions")
        step = self._step_with_actions(chosen)
        step_snapshot = self._snapshot_step_batch(step)
        reset_batch = self._apply_auto_resets(
            step,
            reset_done=bool(reset_done),
            reset_engine_errors=bool(reset_engine_errors),
        )
        return step_snapshot, chosen, reset_batch

    def rollout(
        self,
        steps: int,
        *,
        policy: Literal["first", "uniform", "random"]
        | Callable[[ResetBatch | StepBatch], object] = "uniform",
        seed: int | np.ndarray | None = None,
        auto_reset: bool = False,
        reset_done: bool = True,
        reset_engine_errors: bool = True,
    ) -> list[StepBatch]:
        """Run `steps` decisions with a policy string or callback action function."""
        self._require_open()
        steps_int = int(steps)
        if steps_int <= 0:
            raise WeissSimError("steps must be > 0")

        if self._latest_batch is None:
            self.reset()

        rollout_rng: np.random.Generator | None = None
        policy_token: str | None = None
        if callable(policy):
            policy_fn = policy
        else:
            policy_fn = None
            policy_token = str(policy).strip().lower()
            if policy_token not in {"first", "uniform", "random"}:
                raise WeissSimError("policy must be one of: first, uniform, random, or callable")
            if policy_token in {"uniform", "random"} and np.isscalar(seed):
                rollout_rng = np.random.default_rng(int(seed))

        trajectory: list[StepBatch] = []
        for _ in range(steps_int):
            current = self._require_latest_batch()
            if policy_fn is not None:
                actions = self._coerce_actions(policy_fn(current), name="policy actions")
            elif policy_token == "first":
                actions = current.legal.first_legal()
            else:
                step_seed: int | np.ndarray | None = seed
                if rollout_rng is not None:
                    step_seed = rollout_rng.integers(
                        0,
                        np.iinfo(np.uint64).max,
                        size=self._num_envs,
                        dtype=np.uint64,
                    )
                actions = current.legal.sample_uniform(seed=step_seed)

            step = self._step_with_actions(actions)
            trajectory.append(self._snapshot_step_batch(step))
            if auto_reset:
                self._apply_auto_resets(
                    step,
                    reset_done=bool(reset_done),
                    reset_engine_errors=bool(reset_engine_errors),
                )

        return trajectory

    def _prepare_logits_for_step(
        self,
        logits: object,
        *,
        illegal_value: float,
        temperature: float = 1.0,
    ) -> np.ndarray:
        logits_arr = self._coerce_logits(logits)
        logits_arr = self._apply_illegal_value_compatibility_mask(
            logits_arr, illegal_value=illegal_value
        )
        if temperature != 1.0:
            logits_arr = np.ascontiguousarray(
                logits_arr / np.float32(temperature), dtype=np.float32
            )
        return logits_arr

    def _execute_step_argmax_logits(self, logits: np.ndarray) -> tuple[StepBatch, np.ndarray]:
        actions = np.empty(self._num_envs, dtype=np.uint32)
        to_play_before = self._last_to_play_seat.copy()
        self._call_step_argmax_logits(logits, actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before), actions

    def _execute_step_sample_logits(
        self, logits: np.ndarray, seeds: np.ndarray
    ) -> tuple[StepBatch, np.ndarray]:
        actions = np.empty(self._num_envs, dtype=np.uint32)
        to_play_before = self._last_to_play_seat.copy()
        self._call_step_sample_logits(logits, seeds, actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before), actions

    def step_argmax_logits(
        self, logits: np.ndarray, illegal_value: float = -1e9
    ) -> tuple[StepBatch, np.ndarray]:
        self._require_open()
        self._require_latest_batch()
        logits_arr = self._prepare_logits_for_step(logits, illegal_value=illegal_value)
        return self._execute_step_argmax_logits(logits_arr)

    def step_sample_logits(
        self,
        logits: np.ndarray,
        seed: int | np.ndarray | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
    ) -> tuple[StepBatch, np.ndarray]:
        self._require_open()
        self._require_latest_batch()
        temp = float(temperature)
        if temp < 0.0:
            raise WeissSimError("temperature must be >= 0")
        if temp == 0.0:
            return self.step_argmax_logits(logits, illegal_value=illegal_value)

        logits_arr = self._prepare_logits_for_step(
            logits, illegal_value=illegal_value, temperature=temp
        )
        seeds = self._coerce_sample_seeds(seed)
        return self._execute_step_sample_logits(logits_arr, seeds)

    def auto_reset_on_engine_errors(
        self, codes: np.ndarray | None = None
    ) -> tuple[int, ResetBatch | None]:
        """Auto-reset envs with non-zero engine-status codes via Rust auto-reset APIs."""
        self._require_open()
        codes_arr = self._coerce_engine_status_codes(codes)
        reset_count = self._call_auto_reset_on_error_codes(codes_arr)
        if reset_count == 0:
            return 0, None
        reset_indices = np.flatnonzero(codes_arr != 0).astype(np.int64, copy=False)
        reset_batch = self._finalize_reset(reset_indices=reset_indices)
        return reset_count, reset_batch

    @staticmethod
    def _render_perspective(batch: ResetBatch | StepBatch, env_i: int) -> int:
        perspective = int(batch.to_play_seat[env_i])
        if perspective in (0, 1):
            return perspective
        starting_seat = int(batch.starting_seat[env_i])
        if starting_seat in (0, 1):
            return starting_seat
        return 0

    def decode_action(self, action_id: int) -> dict[str, object] | None:
        """Decode a numeric action id into a structured Python dict, or `None` if unknown."""
        return decode_action_id(int(action_id))

    def render(self, env_i: int = 0, mode: Literal["ansi"] = "ansi") -> str:
        """Render a human-readable ANSI board view for a single env."""
        self._require_open()
        if mode != "ansi":
            raise WeissSimError(f"unsupported render mode {mode!r}; only 'ansi' is supported")
        batch = self._require_latest_batch()
        idx = int(env_i)
        if idx < 0 or idx >= self._num_envs:
            raise WeissSimError(f"env_i must be in [0, {self._num_envs - 1}], got {idx}")
        render_ansi = getattr(self.pool, "render_ansi", None)
        if callable(render_ansi):
            perspective = self._render_perspective(batch, idx)
            return str(render_ansi(idx, perspective))
        legal_ids = batch.legal.ids(idx)
        obs_row = np.asarray(batch.obs[idx]).ravel()
        preview_len = min(24, obs_row.shape[0])
        obs_preview = np.array2string(obs_row[:preview_len], precision=3, separator=", ")
        if isinstance(batch, StepBatch):
            done_value = bool(batch.done[idx])
            reward_value = float(batch.reward[idx])
        else:
            done_value = False
            reward_value = 0.0
        lines = [
            f"WeissEnv[{idx}]",
            f"to_play_seat={int(batch.to_play_seat[idx])} starting_seat={int(batch.starting_seat[idx])}",
            f"done={done_value} reward={reward_value:.6f} decision_id={int(batch.decision_id[idx])}",
            f"episode_seed={int(batch.episode_seed[idx])} episode_index={int(batch.episode_index[idx])}",
            f"legal_ids={legal_ids.tolist()}",
            f"obs[:{preview_len}]={obs_preview}",
        ]
        return "\n".join(lines)

    def as_single_env(self) -> SingleEnvAdapter:
        """Wrap this batched env as a single-environment adapter."""
        from .adapters import SingleEnvAdapter

        return SingleEnvAdapter(self)

    def as_gym(self) -> GymVectorEnvAdapter:
        """Wrap this env as a Gymnasium-style vector environment adapter."""
        from .adapters import GymVectorEnvAdapter

        return GymVectorEnvAdapter(self)
