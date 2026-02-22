"""Lightweight RL helpers for the Weiss simulator."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal

import numpy as np

from ._coerce import coerce_actions_u32, coerce_logits, coerce_seeds
from ._layout_specs import (
    LAYOUT_SPECS,
    LayoutName,
    LayoutSpec,
    normalize_layout,
    pool_method_name,
)
from .weiss_sim import (
    ACTOR_NONE,
    PASS_ACTION_ID,
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


_RL_LAYOUTS = frozenset({"mask", "nomask", "i16_legal_ids"})


def _resolve_layout(layout: str) -> tuple[LayoutName, LayoutSpec]:
    layout_name = normalize_layout(layout)
    if layout_name not in _RL_LAYOUTS:
        allowed = ", ".join(sorted(_RL_LAYOUTS))
        raise ValueError(f"unknown layout {layout!r}; expected one of: {allowed}")
    return layout_name, LAYOUT_SPECS[layout_name]


def _prepare_out(pool: EnvPool, out, spec):
    if out is None:
        return spec.out_cls(pool.envs_len)
    if not isinstance(out, spec.out_cls):
        raise TypeError(f"out must be {spec.out_cls.__name__} for layout, got {type(out).__name__}")
    return out


def _prepare_actions(actions, num_envs: int) -> np.ndarray:
    return coerce_actions_u32(actions, num_envs=num_envs, allow_none=True)


def _pack_step(out, spec: LayoutSpec) -> RlStep:
    return RlStep(
        obs=out.obs,
        masks=out.masks if spec.has_masks else None,
        legal_ids=out.legal_ids if spec.has_legal_ids else None,
        legal_offsets=out.legal_offsets if spec.has_legal_ids else None,
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
    _, spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    getattr(pool, pool_method_name("reset_into", spec))(out_buf)
    return _pack_step(out_buf, spec)


def step_rl(
    pool: EnvPool,
    actions: Sequence[int] | np.ndarray,
    *,
    layout: Layout = "mask",
    out: object | None = None,
) -> RlStep:
    """Step the pool once and return an `RlStep` view over the output buffers."""
    _, spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    getattr(pool, pool_method_name("step_into", spec))(actions, out_buf)
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
    _, spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    logits_buf = coerce_logits(logits, num_envs=pool.envs_len, action_space=pool.action_space)
    actions_buf = _prepare_actions(actions, pool.envs_len)
    getattr(pool, pool_method_name("step_select_from_logits_into", spec))(
        logits_buf,
        actions_buf,
        out_buf,
    )
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
    _, spec = _resolve_layout(layout)
    out_buf = _prepare_out(pool, out, spec)
    logits_buf = coerce_logits(logits, num_envs=pool.envs_len, action_space=pool.action_space)
    seeds_buf = coerce_seeds(seeds, num_envs=pool.envs_len)
    actions_buf = _prepare_actions(actions, pool.envs_len)
    getattr(pool, pool_method_name("step_sample_from_logits_into", spec))(
        logits_buf,
        seeds_buf,
        actions_buf,
        out_buf,
    )
    return _pack_step(out_buf, spec), actions_buf
