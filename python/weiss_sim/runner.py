from __future__ import annotations

import copy
from types import TracebackType
from typing import TYPE_CHECKING, Callable, Literal

import numpy as np

from .config_types import IdsSafety, LegalRepr, RuntimeMode
from .errors import ConfigConflictError, WeissSimError
from ._legal_payloads import cast_legal_ids, cast_legal_offsets, materialize_legal_ids_u16
from .types import LegalActions, ResetBatch, StepBatch
from .weiss_sim import EnvPool, PASS_ACTION_ID, decode_action_id

_U64_MASK = np.uint64(0xFFFFFFFFFFFFFFFF)
_LEGAL_SPOTCHECK_INTERVAL = 4096

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

    def _coerce_actions(self, actions, *, name: str) -> np.ndarray:
        arr = np.asarray(actions, dtype=np.uint32).ravel()
        if arr.shape[0] != self._num_envs:
            raise WeissSimError(
                f"{name} length must equal num_envs ({self._num_envs}), got {arr.shape[0]}"
            )
        return arr

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

    def _legal_ids_payload(self) -> tuple[np.ndarray | None, np.ndarray | None]:
        if self._legal_repr not in {"ids_u16", "ids_u32", "both"}:
            return None, None
        ids_u16, offsets_u32 = materialize_legal_ids_u16(
            embedded_legal_ids=self._embedded_legal_ids,
            out=self._out,
            legal_ids_buffer=self._legal_ids_buf,
            legal_offsets_buffer=self._legal_offsets_buf,
            legal_action_ids_into=self.pool.legal_action_ids_into,
        )
        ids_payload = cast_legal_ids(ids_u16, as_uint32=self._legal_repr in {"ids_u32", "both"})
        offsets_payload = cast_legal_offsets(offsets_u32)
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

    def _build_reset_batch(self) -> ResetBatch:
        payload, ids, offsets, mask, episode_key = self._collect_batch_materialized()
        return ResetBatch(
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

    def _finalize_reset(self, *, reset_indices: np.ndarray | None) -> ResetBatch:
        batch = self._build_reset_batch()
        self._last_to_play_seat = batch.to_play_seat.copy()
        if reset_indices is None:
            self._last_done.fill(False)
        elif reset_indices.size:
            self._last_done[reset_indices] = False
        self._latest_batch = batch
        return batch

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
                    self._call_reset()
            return self._finalize_reset(reset_indices=None)

        reset_indices = self._coerce_indices(indices, name="indices")
        if seed is None:
            self._call_reset_indices(reset_indices)
        else:
            episode_seeds = _episode_seeds_for_indices(seed, reset_indices)
            if not self._call_reset_indices_with_episode_seeds(reset_indices, episode_seeds):
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
        self._latest_batch = step_batch
        return step_batch

    def step_select_from_logits(
        self, logits: np.ndarray, illegal_value: float = -1e9
    ) -> tuple[StepBatch, np.ndarray]:
        actions = self.legal.select_from_logits(logits, illegal_value=illegal_value)
        return self.step(actions), actions

    def step_sample_from_logits(
        self,
        logits: np.ndarray,
        seed: int | np.ndarray | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
    ) -> tuple[StepBatch, np.ndarray]:
        actions = self.legal.sample_from_logits(
            logits,
            seed=seed,
            temperature=temperature,
            illegal_value=illegal_value,
        )
        return self.step(actions), actions

    def decode_action(self, action_id: int) -> dict[str, object]:
        """Decode a numeric action id into a structured Python dict."""
        return decode_action_id(int(action_id))

    def render(self, env_i: int = 0, mode: Literal["ansi"] = "ansi") -> str:
        """Render a compact debugging view for a single env."""
        self._require_open()
        if mode != "ansi":
            raise WeissSimError(f"unsupported render mode {mode!r}; only 'ansi' is supported")
        batch = self._require_latest_batch()
        idx = int(env_i)
        if idx < 0 or idx >= self._num_envs:
            raise WeissSimError(f"env_i must be in [0, {self._num_envs - 1}], got {idx}")
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
