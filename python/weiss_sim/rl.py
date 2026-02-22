"""Lightweight RL helpers for the Weiss simulator."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal

import numpy as np

from ._logits_utils import prepare_logits, prepare_seeds
from .weiss_sim import (
    ACTOR_NONE,
    PASS_ACTION_ID,
    BatchOutMinimal,
    BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask,
    EnvPool,
)

Layout = Literal["mask", "nomask", "i16_legal_ids"]


@dataclass(frozen=True)
class RlStep:
    obs: np.ndarray
    rewards: np.ndarray
    terminated: np.ndarray
    truncated: np.ndarray
    actor: np.ndarray
    decision_kind: np.ndarray
    decision_id: np.ndarray
    engine_status: np.ndarray
    spec_hash: np.ndarray
    masks: np.ndarray | None = None
    legal_ids: np.ndarray | None = None
    legal_offsets: np.ndarray | None = None

    @property
    def engine_error(self) -> np.ndarray:
        return self.engine_status != 0

    @property
    def reset_recommended(self) -> np.ndarray:
        return self.engine_status != 0

    @property
    def actor_known(self) -> np.ndarray:
        return self.actor != ACTOR_NONE


@dataclass(frozen=True)
class _LayoutSpec:
    out_cls: type
    reset_method: str
    step_method: str
    step_select_logits_method: str
    step_sample_logits_method: str
    include_masks: bool
    include_legal_ids: bool


_LAYOUT_DISPATCH: dict[str, _LayoutSpec] = {
    "mask": _LayoutSpec(
        out_cls=BatchOutMinimal,
        reset_method="reset_into",
        step_method="step_into",
        step_select_logits_method="step_select_from_logits_into",
        step_sample_logits_method="step_sample_from_logits_into",
        include_masks=True,
        include_legal_ids=False,
    ),
    "nomask": _LayoutSpec(
        out_cls=BatchOutMinimalNoMask,
        reset_method="reset_into_nomask",
        step_method="step_into_nomask",
        step_select_logits_method="step_select_from_logits_into_nomask",
        step_sample_logits_method="step_sample_from_logits_into_nomask",
        include_masks=False,
        include_legal_ids=False,
    ),
    "i16_legal_ids": _LayoutSpec(
        out_cls=BatchOutMinimalI16LegalIds,
        reset_method="reset_into_i16_legal_ids",
        step_method="step_into_i16_legal_ids",
        step_select_logits_method="step_select_from_logits_into_i16_legal_ids",
        step_sample_logits_method="step_sample_from_logits_into_i16_legal_ids",
        include_masks=False,
        include_legal_ids=True,
    ),
}


def _resolve_layout(layout: str) -> _LayoutSpec:
    try:
        return _LAYOUT_DISPATCH[layout]
    except KeyError as exc:
        allowed = ", ".join(sorted(_LAYOUT_DISPATCH))
        raise ValueError(f"unknown layout {layout!r}; expected one of: {allowed}") from exc


def _prepare_out(pool: EnvPool, out, spec: _LayoutSpec):
    if out is None:
        return spec.out_cls(pool.envs_len)
    if not isinstance(out, spec.out_cls):
        raise TypeError(f"out must be {spec.out_cls.__name__} for layout, got {type(out).__name__}")
    return out


def _prepare_actions(actions, num_envs: int) -> np.ndarray:
    if actions is None:
        return np.empty(num_envs, dtype=np.uint32)
    arr = np.asarray(actions, dtype=np.uint32)
    arr = np.ravel(arr)
    if arr.shape[0] != int(num_envs):
        raise ValueError(f"actions length must equal num_envs ({num_envs}), got {arr.shape[0]}")
    return np.ascontiguousarray(arr, dtype=np.uint32)


def _pack_step(out, spec: _LayoutSpec) -> RlStep:
    return RlStep(
        obs=out.obs,
        masks=out.masks if spec.include_masks else None,
        legal_ids=out.legal_ids if spec.include_legal_ids else None,
        legal_offsets=out.legal_offsets if spec.include_legal_ids else None,
        rewards=out.rewards,
        terminated=out.terminated,
        truncated=out.truncated,
        actor=out.actor,
        decision_kind=out.decision_kind,
        decision_id=out.decision_id,
        engine_status=out.engine_status,
        spec_hash=out.spec_hash,
    )


def pass_action_id_for_decision_kind(decision_kind: object) -> int:
    """Return the action id corresponding to "pass" for a decision kind.

    This is currently a thin wrapper around the global `PASS_ACTION_ID`.
    """
    return PASS_ACTION_ID


def reset_rl(pool: EnvPool, *, layout: Layout = "mask", out: object | None = None) -> RlStep:
    """Reset the pool and return an `RlStep` view over the output buffers."""
    spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    getattr(pool, spec.reset_method)(out_buf)
    return _pack_step(out_buf, spec)


def step_rl(
    pool: EnvPool,
    actions: Sequence[int] | np.ndarray,
    *,
    layout: Layout = "mask",
    out: object | None = None,
) -> RlStep:
    """Step the pool once and return an `RlStep` view over the output buffers."""
    spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    getattr(pool, spec.step_method)(actions, out_buf)
    return _pack_step(out_buf, spec)


def step_rl_select_from_logits(
    pool: EnvPool,
    logits: object,
    *,
    layout: Layout = "i16_legal_ids",
    actions: Sequence[int] | np.ndarray | None = None,
    out: object | None = None,
):
    """Select argmax actions from `logits` (respecting legality) and step the pool."""
    spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    logits_buf = prepare_logits(logits, pool.envs_len, action_space=pool.action_space)
    actions_buf = _prepare_actions(actions, pool.envs_len)
    getattr(pool, spec.step_select_logits_method)(logits_buf, actions_buf, out_buf)
    return _pack_step(out_buf, spec), actions_buf


def step_rl_sample_from_logits(
    pool: EnvPool,
    logits: object,
    seeds: int | Sequence[int] | np.ndarray,
    *,
    layout: Layout = "i16_legal_ids",
    actions: Sequence[int] | np.ndarray | None = None,
    out: object | None = None,
):
    """Sample actions from `logits` (respecting legality) and step the pool."""
    spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    logits_buf = prepare_logits(logits, pool.envs_len, action_space=pool.action_space)
    seeds_buf = prepare_seeds(seeds, pool.envs_len)
    actions_buf = _prepare_actions(actions, pool.envs_len)
    getattr(pool, spec.step_sample_logits_method)(logits_buf, seeds_buf, actions_buf, out_buf)
    return _pack_step(out_buf, spec), actions_buf
