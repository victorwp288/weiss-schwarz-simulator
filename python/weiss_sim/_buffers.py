"""Canonical low-level buffer helpers for high-throughput stepping.

This module is the primary integration surface for users who want direct access
to packed numpy buffers (for example, to avoid per-step allocations).
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from time import perf_counter_ns
from typing import Literal, overload

import numpy as np

from ._coerce import coerce_logits, coerce_seeds
from .config_types import ErrorPolicy
from ._legal_payloads import materialize_legal_action_meta_u16, materialize_legal_ids_u16
from ._layout_specs import (
    FLAGS_TO_LAYOUT as _FLAGS_TO_LAYOUT,
    LAYOUT_FLAGS as _LAYOUT_FLAGS,
    LAYOUT_SPECS as _LAYOUT_SPECS,
    LayoutName,
    LayoutSpec as _LayoutSpec,
    normalize_layout as _normalize_layout,
    pool_method_name,
)
from .weiss_sim import (
    ACTOR_NONE,
    ACTION_META_UNUSED,
    ACTION_META_WIDTH,
    BatchOutDebug,
    EnvPool,
)

ModeName = Literal["train", "eval"]
ProfileName = Literal["fast", "balanced", "eval", "debug"]

DeckLists = Sequence[Sequence[int]]
DeckIds = Sequence[int]

MinimalOut = object
TrajectoryOut = object

_PROFILE_FAST = "fast"
_PROFILE_BALANCED = "balanced"
_PROFILE_EVAL = "eval"
_PROFILE_DEBUG = "debug"


@dataclass(frozen=True)
class _ProfileSpec:
    layout: LayoutName
    unsafe_i16: bool


_PROFILE_SPECS: dict[str, _ProfileSpec] = {
    _PROFILE_FAST: _ProfileSpec(layout="i16_legal_ids", unsafe_i16=True),
    _PROFILE_BALANCED: _ProfileSpec(layout="mask", unsafe_i16=False),
    _PROFILE_EVAL: _ProfileSpec(layout="mask", unsafe_i16=False),
    _PROFILE_DEBUG: _ProfileSpec(layout="mask", unsafe_i16=False),
}

_DEFAULT_PROFILE_BY_MODE: dict[ModeName, str] = {
    "train": _PROFILE_FAST,
    "eval": _PROFILE_BALANCED,
}


class _EngineStatusMixin:
    @property
    def engine_error(self) -> np.ndarray:
        """Boolean vector indicating an engine error for each env."""
        return self.engine_status != 0

    @property
    def reset_recommended(self) -> np.ndarray:
        """Alias for `engine_error` (resetting is usually appropriate after errors)."""
        return self.engine_error

    @property
    def actor_known(self) -> np.ndarray:
        """Boolean vector indicating whether the acting seat is known."""
        return self.actor != ACTOR_NONE


def _prepare_pool_for_legal_ids(pool: EnvPool) -> None:
    """Legal-id outputs require dense output masks to be disabled."""
    pool.set_output_mask_enabled(False)
    pool.set_output_mask_bits_enabled(False)


def _normalize_mode(mode: str) -> ModeName:
    mode_norm = mode.lower().strip()
    if mode_norm not in _DEFAULT_PROFILE_BY_MODE:
        raise ValueError(f"unknown mode '{mode}' (expected train or eval)")
    return mode_norm  # type: ignore[return-value]


def _resolve_profile(profile: str | None, mode: ModeName) -> _ProfileSpec:
    if profile is None:
        profile = _DEFAULT_PROFILE_BY_MODE[mode]
    profile_norm = profile.lower().strip()
    spec = _PROFILE_SPECS.get(profile_norm)
    if spec is None:
        allowed = ", ".join(_PROFILE_SPECS)
        raise ValueError(f"unknown profile '{profile}' (expected {allowed})")
    return spec


def _resolve_layout(
    *,
    profile_layout: LayoutName,
    output_masks: bool | None,
    use_i16: bool | None,
    legal_ids: bool | None,
    layout: str | None,
) -> tuple[LayoutName, bool, bool, bool]:
    if layout is not None:
        layout_name = _normalize_layout(layout)
        layout_flags = _LAYOUT_FLAGS[layout_name]
        if output_masks is not None and bool(output_masks) != layout_flags[0]:
            raise ValueError("layout conflicts with output_masks")
        if use_i16 is not None and bool(use_i16) != layout_flags[1]:
            raise ValueError("layout conflicts with use_i16")
        if legal_ids is not None and bool(legal_ids) != layout_flags[2]:
            raise ValueError("layout conflicts with legal_ids")
        return layout_name, layout_flags[0], layout_flags[1], layout_flags[2]

    defaults = _LAYOUT_FLAGS[profile_layout]
    resolved_output_masks = defaults[0] if output_masks is None else bool(output_masks)
    resolved_use_i16 = defaults[1] if use_i16 is None else bool(use_i16)
    resolved_legal_ids = defaults[2] if legal_ids is None else bool(legal_ids)

    if resolved_legal_ids:
        if resolved_output_masks:
            raise ValueError("legal_ids requires output_masks=False")
        if resolved_use_i16 is False:
            raise ValueError("legal_ids currently requires use_i16=True")
        resolved_output_masks = False
        resolved_use_i16 = True

    resolved_layout = _FLAGS_TO_LAYOUT.get(
        (resolved_output_masks, resolved_use_i16, resolved_legal_ids)
    )
    if resolved_layout is None:
        raise ValueError(
            "unsupported combination of output_masks/use_i16/legal_ids "
            f"= ({resolved_output_masks}, {resolved_use_i16}, {resolved_legal_ids})"
        )
    return resolved_layout, resolved_output_masks, resolved_use_i16, resolved_legal_ids


def _resolve_unsafe_i16(
    *,
    profile_default: bool,
    unsafe_i16: bool | None,
    use_i16: bool,
) -> bool:
    if unsafe_i16 is None:
        return bool(profile_default and use_i16)
    resolved = bool(unsafe_i16)
    if resolved and not use_i16:
        raise ValueError("unsafe_i16 requires use_i16=True")
    return resolved


def _bind_pool_method(pool: EnvPool, base_name: str, spec: _LayoutSpec):
    method_name = pool_method_name(base_name, spec)
    return getattr(pool, method_name)


def make_batch_out_debug(pool: EnvPool, *, event_capacity: int | None = None) -> BatchOutDebug:
    """Allocate a `BatchOutDebug` buffer with safe defaults for an existing pool."""
    if event_capacity is None:
        event_capacity = int(pool.debug_event_ring_capacity())
    capacity = int(event_capacity)
    if capacity < 0:
        raise ValueError(f"event_capacity must be >= 0 (got {event_capacity!r})")
    return BatchOutDebug(int(pool.num_envs), capacity)


@overload
def make_pool(
    mode: ModeName | str,
    num_envs: int,
    db_path: str | None = None,
    deck_lists: DeckLists | None = None,
    deck_ids: DeckIds | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    end_condition_policy_json: str | None = None,
    error_policy: str | None = None,
    observation_visibility: str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: ProfileName | str | None = None,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: None = None,
    layout: LayoutName | str | None = None,
) -> tuple[EnvPool, EnvPoolBuffers]: ...


@overload
def make_pool(
    mode: ModeName | str,
    num_envs: int,
    db_path: str | None = None,
    deck_lists: DeckLists | None = None,
    deck_ids: DeckIds | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    end_condition_policy_json: str | None = None,
    error_policy: str | None = None,
    observation_visibility: str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: ProfileName | str | None = None,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: int,
    layout: LayoutName | str | None = None,
) -> tuple[EnvPool, EnvPoolTrajectoryBuffers]: ...


def make_pool(
    mode: ModeName | str,
    num_envs: int,
    db_path: str | None = None,
    deck_lists: DeckLists | None = None,
    deck_ids: DeckIds | None = None,
    max_decisions: int = 2000,
    max_ticks: int = 100_000,
    seed: int = 0,
    curriculum_json: str | None = None,
    reward_json: str | None = None,
    end_condition_policy_json: str | None = None,
    error_policy: ErrorPolicy | str | None = None,
    observation_visibility: Literal["public", "full"] | str | None = None,
    num_threads: int | None = None,
    debug_fingerprint_every_n: int = 0,
    debug_event_ring_capacity: int = 0,
    *,
    profile: ProfileName | str | None = None,
    output_masks: bool | None = None,
    use_i16: bool | None = None,
    legal_ids: bool | None = None,
    unsafe_i16: bool | None = None,
    rollout_steps: int | None = None,
    layout: LayoutName | str | None = None,
):
    """Create an `EnvPool` plus canonical preallocated numpy buffers.

    This is the recommended low-level API for high-throughput stepping. It returns
    `(pool, buffers)` where `buffers` is one of:

    - `EnvPoolBuffers` if `rollout_steps=None` (single-step reset/step).
    - `EnvPoolTrajectoryBuffers` if `rollout_steps=<int>` (multi-step rollouts).

    Parameters
    ----------
    mode:
        `"train"` or `"eval"`. Controls which Rust stepping defaults are used.
    deck_lists:
        Required. A list of per-player deck id lists (each deck must contain 50 ids).
    profile:
        Optional preset that selects a reasonable default layout and safety knobs.
        Supported values: `"fast"`, `"balanced"`, `"eval"`, `"debug"`.
    layout:
        Optional explicit layout override. Supported values: `"mask"`, `"nomask"`,
        `"i16"`, `"i16_legal_ids"`.
    """
    mode_name = _normalize_mode(mode)
    profile_spec = _resolve_profile(profile, mode_name)

    layout_name, output_masks_v, use_i16_v, legal_ids_v = _resolve_layout(
        profile_layout=profile_spec.layout,
        output_masks=output_masks,
        use_i16=use_i16,
        legal_ids=legal_ids,
        layout=layout,
    )
    unsafe_i16_v = _resolve_unsafe_i16(
        profile_default=profile_spec.unsafe_i16,
        unsafe_i16=unsafe_i16,
        use_i16=use_i16_v,
    )

    if deck_lists is None:
        raise ValueError("deck_lists is required")
    if rollout_steps is not None and rollout_steps <= 0:
        raise ValueError("rollout_steps must be > 0")

    constructor = EnvPool.new_rl_train if mode_name == "train" else EnvPool.new_rl_eval
    pool = constructor(
        num_envs,
        db_path,
        deck_lists=deck_lists,
        deck_ids=deck_ids,
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        seed=seed,
        curriculum_json=curriculum_json,
        reward_json=reward_json,
        end_condition_policy_json=end_condition_policy_json,
        error_policy=error_policy,
        observation_visibility=observation_visibility,
        num_threads=num_threads,
        output_masks=output_masks_v,
        debug_fingerprint_every_n=debug_fingerprint_every_n,
        debug_event_ring_capacity=debug_event_ring_capacity,
    )

    if unsafe_i16_v:
        pool.set_i16_clamp_enabled(False)

    if rollout_steps is not None:
        return pool, EnvPoolTrajectoryBuffers(pool, rollout_steps, layout=layout_name)
    return pool, EnvPoolBuffers(pool, layout=layout_name)


class EnvPoolBuffers(_EngineStatusMixin):
    """Preallocated numpy buffers for high-throughput stepping."""

    def __init__(self, pool: EnvPool, *, layout: LayoutName | str = "mask") -> None:
        self.pool = pool
        self.layout = _normalize_layout(layout)
        spec = _LAYOUT_SPECS[self.layout]
        self._embedded_legal_ids = spec.has_legal_ids

        if spec.has_legal_ids:
            _prepare_pool_for_legal_ids(self.pool)

        num_envs = pool.envs_len
        self._num_envs = int(num_envs)
        self._action_space = int(pool.action_space)
        self.out: MinimalOut = spec.out_cls(num_envs)
        self.obs: np.ndarray = self.out.obs
        self.masks: np.ndarray | None = self.out.masks if spec.has_masks else None
        self.rewards: np.ndarray = self.out.rewards
        self.terminated: np.ndarray = self.out.terminated
        self.truncated: np.ndarray = self.out.truncated
        self.actor: np.ndarray = self.out.actor
        self.decision_kind: np.ndarray = self.out.decision_kind
        self.decision_id: np.ndarray = self.out.decision_id
        self.engine_status: np.ndarray = self.out.engine_status
        self.spec_hash: np.ndarray = self.out.spec_hash
        self.main_move_action: np.ndarray = self.out.main_move_action
        self.main_pass_action: np.ndarray = self.out.main_pass_action
        if spec.has_legal_ids:
            self.legal_ids: np.ndarray = self.out.legal_ids
            self.legal_action_meta: np.ndarray = getattr(
                self.out,
                "legal_action_meta",
                np.full(
                    (num_envs * pool.action_space, ACTION_META_WIDTH),
                    ACTION_META_UNUSED,
                    dtype=np.uint16,
                ),
            )
            self.legal_offsets: np.ndarray = self.out.legal_offsets
        else:
            self.legal_ids = np.empty(num_envs * pool.action_space, dtype=np.uint16)
            self.legal_action_meta = np.full(
                (num_envs * pool.action_space, ACTION_META_WIDTH),
                ACTION_META_UNUSED,
                dtype=np.uint16,
            )
            self.legal_offsets = np.zeros(num_envs + 1, dtype=np.uint32)
        self.actions: np.ndarray = np.empty(num_envs, dtype=np.uint32)

        self._reset_into = _bind_pool_method(pool, "reset_into", spec)
        self._reset_indices_into = _bind_pool_method(pool, "reset_indices_into", spec)
        self._reset_done_into = _bind_pool_method(pool, "reset_done_into", spec)
        self._reset_indices_with_episode_seeds_into = _bind_pool_method(
            pool, "reset_indices_with_episode_seeds_into", spec
        )
        self._step_into = _bind_pool_method(pool, "step_into", spec)
        self._step_first_legal_into = _bind_pool_method(pool, "step_first_legal_into", spec)
        self._step_sample_legal_action_ids_uniform_into = _bind_pool_method(
            pool, "step_sample_legal_action_ids_uniform_into", spec
        )
        self._step_select_from_logits_into = _bind_pool_method(
            pool, "step_select_from_logits_into", spec
        )
        self._step_sample_from_logits_into = _bind_pool_method(
            pool, "step_sample_from_logits_into", spec
        )
        self._select_actions_from_logits_into = pool.select_actions_from_logits_into
        self._sample_actions_from_logits_into = pool.sample_actions_from_logits_into
        self._legal_action_ids_into = pool.legal_action_ids_into
        self._legal_action_meta_into = getattr(pool, "legal_action_meta_into", None)
        self._legal_action_ids_and_sample_uniform_into = (
            pool.legal_action_ids_and_sample_uniform_into
        )
        self._timing_enabled = False
        self._timing_counters = {
            "legal_ids_materialize_count": 0,
            "legal_ids_materialize_ns": 0,
            "legal_action_meta_materialize_count": 0,
            "legal_action_meta_materialize_ns": 0,
        }

    def _coerce_logits(self, logits: object) -> np.ndarray:
        return coerce_logits(logits, num_envs=self._num_envs, action_space=self._action_space)

    def _coerce_seeds(self, seeds: object) -> np.ndarray:
        return coerce_seeds(seeds, num_envs=self._num_envs)

    def reset(self) -> MinimalOut:
        """Reset all envs into the preallocated buffers."""
        self._reset_into(self.out)
        return self.out

    def reset_indices(self, indices: Sequence[int] | np.ndarray) -> MinimalOut:
        """Reset a subset of env indices into the preallocated buffers."""
        self._reset_indices_into(list(indices), self.out)
        return self.out

    def reset_done(self, done_mask: Sequence[bool] | np.ndarray) -> MinimalOut:
        """Reset envs where `done_mask` is true."""
        self._reset_done_into(done_mask, self.out)
        return self.out

    def reset_indices_with_episode_seeds(
        self,
        indices: Sequence[int] | np.ndarray,
        episode_seeds: Sequence[int] | np.ndarray,
    ) -> MinimalOut:
        """Reset a subset of envs with explicit per-env episode seeds."""
        self._reset_indices_with_episode_seeds_into(list(indices), list(episode_seeds), self.out)
        return self.out

    def step(self, actions: Sequence[int] | np.ndarray) -> MinimalOut:
        """Step all envs using the provided per-env `actions` vector."""
        self._step_into(actions, self.out)
        return self.out

    def step_first_legal(self) -> tuple[MinimalOut, np.ndarray]:
        """Step by selecting the first legal action for each env."""
        self._step_first_legal_into(self.actions, self.out)
        return self.out, self.actions

    def step_random_legal(
        self, seeds: int | Sequence[int] | np.ndarray
    ) -> tuple[MinimalOut, np.ndarray]:
        """Step by sampling a uniform-random legal action for each env."""
        self._step_sample_legal_action_ids_uniform_into(
            self._coerce_seeds(seeds),
            self.actions,
            self.out,
        )
        return self.out, self.actions

    def step_select_from_logits(self, logits: object) -> tuple[MinimalOut, np.ndarray]:
        """Step by selecting argmax actions from `logits`, respecting legality."""
        logits = self._coerce_logits(logits)
        self._step_select_from_logits_into(logits, self.actions, self.out)
        return self.out, self.actions

    def step_sample_from_logits(
        self,
        logits: object,
        seeds: int | Sequence[int] | np.ndarray,
    ) -> tuple[MinimalOut, np.ndarray]:
        """Step by sampling actions from `logits`, respecting legality."""
        logits = self._coerce_logits(logits)
        seeds = self._coerce_seeds(seeds)
        self._step_sample_from_logits_into(logits, seeds, self.actions, self.out)
        return self.out, self.actions

    def select_actions_from_logits(self, logits: object) -> np.ndarray:
        """Write selected actions from `logits` into `self.actions` and return it."""
        logits = self._coerce_logits(logits)
        self._select_actions_from_logits_into(logits, self.actions)
        return self.actions

    def sample_actions_from_logits(
        self,
        logits: object,
        seeds: int | Sequence[int] | np.ndarray,
    ) -> np.ndarray:
        """Write sampled actions from `logits` into `self.actions` and return it."""
        logits = self._coerce_logits(logits)
        seeds = self._coerce_seeds(seeds)
        self._sample_actions_from_logits_into(logits, seeds, self.actions)
        return self.actions

    def set_output_mask_enabled(self, enabled: bool) -> None:
        self.pool.set_output_mask_enabled(enabled)
        if not enabled and hasattr(self.out, "masks"):
            self.out.masks.fill(0)

    def set_output_mask_bits_enabled(self, enabled: bool) -> None:
        self.pool.set_output_mask_bits_enabled(enabled)

    def set_i16_clamp_enabled(self, enabled: bool) -> None:
        self.pool.set_i16_clamp_enabled(enabled)

    def set_i16_overflow_counter_enabled(self, enabled: bool) -> None:
        self.pool.set_i16_overflow_counter_enabled(enabled)

    def i16_overflow_count(self) -> int:
        return int(self.pool.i16_overflow_count())

    def reset_i16_overflow_count(self) -> None:
        self.pool.reset_i16_overflow_count()

    def set_timing_enabled(self, enabled: bool) -> None:
        enabled = bool(enabled)
        self._timing_enabled = enabled
        setter = getattr(self.pool, "set_timing_enabled", None)
        if setter is not None:
            setter(enabled)

    def reset_timing_counters(self) -> None:
        self._timing_counters = {
            "legal_ids_materialize_count": 0,
            "legal_ids_materialize_ns": 0,
            "legal_action_meta_materialize_count": 0,
            "legal_action_meta_materialize_ns": 0,
        }
        resetter = getattr(self.pool, "reset_timing_counters", None)
        if resetter is not None:
            resetter()

    def timing_counters(self) -> dict[str, int]:
        snapshot = dict(self._timing_counters)
        getter = getattr(self.pool, "timing_counters", None)
        if getter is not None:
            snapshot.update(getter())
        return snapshot

    def _timed_legal_ids(self) -> tuple[np.ndarray, np.ndarray]:
        start_ns = perf_counter_ns() if self._timing_enabled else None
        ids = materialize_legal_ids_u16(
            embedded_legal_ids=self._embedded_legal_ids,
            out=self.out,
            legal_ids_buffer=self.legal_ids,
            legal_offsets_buffer=self.legal_offsets,
            legal_action_ids_into=self._legal_action_ids_into,
        )
        if start_ns is not None:
            elapsed = perf_counter_ns() - start_ns
            self._timing_counters["legal_ids_materialize_count"] += 1
            self._timing_counters["legal_ids_materialize_ns"] += elapsed
        return ids

    def legal_action_ids(self) -> tuple[np.ndarray, np.ndarray]:
        """Materialize packed legal ids/offsets into `self.legal_ids` buffers."""
        return self._timed_legal_ids()

    def legal_action_data(self) -> tuple[np.ndarray, np.ndarray | None, np.ndarray]:
        """Materialize packed legal ids plus aligned action metadata."""
        ids, offsets = self._timed_legal_ids()
        used_rows = int(offsets[-1]) if offsets.size else 0
        start_ns = perf_counter_ns() if self._timing_enabled else None
        meta = materialize_legal_action_meta_u16(
            embedded_legal_ids=self._embedded_legal_ids,
            out=self.out,
            legal_action_meta_buffer=self.legal_action_meta,
            used_rows=used_rows,
            legal_action_meta_into=self._legal_action_meta_into,
        )
        if start_ns is not None:
            elapsed = perf_counter_ns() - start_ns
            self._timing_counters["legal_action_meta_materialize_count"] += 1
            self._timing_counters["legal_action_meta_materialize_ns"] += elapsed
        return ids, meta, offsets

    def legal_action_ids_and_sample_uniform(
        self, seeds: int | Sequence[int] | np.ndarray
    ) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        """Materialize legal ids/offsets and sample uniform legal actions."""
        count = self._legal_action_ids_and_sample_uniform_into(
            self.legal_ids,
            self.legal_offsets,
            self._coerce_seeds(seeds),
            self.actions,
        )
        return self.legal_ids[:count], self.legal_offsets, self.actions


class EnvPoolTrajectoryBuffers(_EngineStatusMixin):
    """Preallocated numpy buffers for multi-step rollouts."""

    def __init__(self, pool: EnvPool, steps: int, *, layout: LayoutName | str = "mask") -> None:
        if steps <= 0:
            raise ValueError("steps must be > 0")

        self.pool = pool
        self.layout = _normalize_layout(layout)
        self.steps = steps
        self._num_envs = int(pool.envs_len)
        spec = _LAYOUT_SPECS[self.layout]

        if spec.has_legal_ids:
            _prepare_pool_for_legal_ids(self.pool)

        self.out: TrajectoryOut = spec.trajectory_out_cls(steps, pool.envs_len)
        self.obs: np.ndarray = self.out.obs
        self.masks: np.ndarray | None = self.out.masks if spec.has_masks else None
        self.legal_ids: np.ndarray | None = self.out.legal_ids if spec.has_legal_ids else None
        self.legal_action_meta: np.ndarray | None = (
            getattr(self.out, "legal_action_meta", None) if spec.has_legal_ids else None
        )
        self.legal_offsets: np.ndarray | None = (
            self.out.legal_offsets if spec.has_legal_ids else None
        )
        self.rewards: np.ndarray = self.out.rewards
        self.terminated: np.ndarray = self.out.terminated
        self.truncated: np.ndarray = self.out.truncated
        self.actor: np.ndarray = self.out.actor
        self.decision_kind: np.ndarray = self.out.decision_kind
        self.decision_id: np.ndarray = self.out.decision_id
        self.engine_status: np.ndarray = self.out.engine_status
        self.spec_hash: np.ndarray = self.out.spec_hash
        self.main_move_action: np.ndarray = self.out.main_move_action
        self.main_pass_action: np.ndarray = self.out.main_pass_action
        self.actions: np.ndarray = self.out.actions

        self._rollout_first_legal_into = _bind_pool_method(pool, "rollout_first_legal_into", spec)
        self._rollout_sample_legal_action_ids_uniform_into = _bind_pool_method(
            pool,
            "rollout_sample_legal_action_ids_uniform_into",
            spec,
        )

    def rollout_first_legal(self) -> TrajectoryOut:
        """Rollout for `self.steps` using the first legal action each step."""
        self._rollout_first_legal_into(self.steps, self.out)
        return self.out

    def rollout_random_legal(self, seeds: int | Sequence[int] | np.ndarray) -> TrajectoryOut:
        """Rollout for `self.steps` using uniform-random legal actions."""
        seeds = coerce_seeds(seeds, num_envs=self._num_envs)
        self._rollout_sample_legal_action_ids_uniform_into(self.steps, seeds, self.out)
        return self.out


__all__ = ["EnvPoolBuffers", "EnvPoolTrajectoryBuffers", "make_pool"]
