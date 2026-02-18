from __future__ import annotations

from dataclasses import dataclass

import numpy as np


@dataclass(slots=True, frozen=True)
class CardRef:
    id: int
    card_no: str
    name: str
    card_type: str
    card_set: str | None
    strict_ok: bool
    approx_ok: bool


@dataclass(slots=True)
class ResetBatch:
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


@dataclass(slots=True)
class StepBatch:
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
