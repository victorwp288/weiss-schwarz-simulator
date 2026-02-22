from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np
import weiss_sim

_FIXTURE_DB_PATH = Path(__file__).parent / "fixtures" / "cards.wsdb"
_DEFAULT_LEGAL_DECK = (list(range(1, 14)) * 4)[:50]


def first_legal_actions(batch_or_masks: Any, num_envs: int | None = None) -> np.ndarray:
    if isinstance(batch_or_masks, np.ndarray):
        return first_legal_actions_from_mask(batch_or_masks)

    legal = getattr(batch_or_masks, "legal", None)
    if legal is not None:
        return np.asarray(legal.first_legal(), dtype=np.uint32)

    legal_ids = getattr(batch_or_masks, "legal_ids", None)
    legal_offsets = getattr(batch_or_masks, "legal_offsets", None)
    if legal_ids is not None and legal_offsets is not None:
        if num_envs is None:
            num_envs = int(legal_offsets.shape[0]) - 1
        return first_legal_actions_from_ids(legal_ids, legal_offsets, num_envs)

    legal_mask = getattr(batch_or_masks, "legal_mask", None)
    if legal_mask is not None:
        return first_legal_actions_from_mask(legal_mask)

    masks = getattr(batch_or_masks, "masks", None)
    if masks is not None:
        return first_legal_actions_from_mask(masks)

    raise AssertionError("batch does not expose legal ids/mask and is not a mask array")


def first_legal_actions_from_ids(
    legal_ids: np.ndarray, legal_offsets: np.ndarray, num_envs: int
) -> np.ndarray:
    actions = np.empty((num_envs,), dtype=np.uint32)
    for env_i in range(num_envs):
        start = int(legal_offsets[env_i])
        end = int(legal_offsets[env_i + 1])
        actions[env_i] = (
            np.uint32(weiss_sim.PASS_ACTION_ID)
            if end <= start
            else np.uint32(int(legal_ids[start]))
        )
    return actions


def first_legal_actions_from_mask(mask: np.ndarray) -> np.ndarray:
    actions = np.empty((int(mask.shape[0]),), dtype=np.uint32)
    for env_i in range(mask.shape[0]):
        ids = np.flatnonzero(mask[env_i])
        actions[env_i] = (
            np.uint32(weiss_sim.PASS_ACTION_ID)
            if ids.size == 0
            else np.uint32(int(ids[0]))
        )
    return actions


def assert_same_optional_array(lhs: np.ndarray | None, rhs: np.ndarray | None) -> None:
    if lhs is None or rhs is None:
        assert lhs is None and rhs is None
        return
    assert np.array_equal(lhs, rhs)


def assert_reset_batches_equal(lhs: Any, rhs: Any) -> None:
    assert np.array_equal(lhs.obs, rhs.obs)
    assert np.array_equal(lhs.to_play_seat, rhs.to_play_seat)
    assert np.array_equal(lhs.starting_seat, rhs.starting_seat)
    assert np.array_equal(lhs.episode_seed, rhs.episode_seed)
    assert np.array_equal(lhs.episode_index, rhs.episode_index)
    assert np.array_equal(lhs.env_index, rhs.env_index)
    assert np.array_equal(lhs.episode_key, rhs.episode_key)
    assert np.array_equal(lhs.decision_id, rhs.decision_id)
    assert np.array_equal(lhs.engine_status, rhs.engine_status)
    assert np.array_equal(lhs.spec_hash, rhs.spec_hash)
    assert_same_optional_array(lhs.legal_mask, rhs.legal_mask)
    assert_same_optional_array(lhs.legal_ids, rhs.legal_ids)
    assert_same_optional_array(lhs.legal_offsets, rhs.legal_offsets)


def assert_step_batches_equal(lhs: Any, rhs: Any) -> None:
    assert_reset_batches_equal(lhs, rhs)
    assert np.array_equal(lhs.reward, rhs.reward)
    assert np.array_equal(lhs.terminated, rhs.terminated)
    assert np.array_equal(lhs.truncated, rhs.truncated)
    assert np.array_equal(
        lhs.terminal_during_internal_opponent, rhs.terminal_during_internal_opponent
    )
    assert np.array_equal(lhs.decision_count, rhs.decision_count)
    assert np.array_equal(lhs.tick_count, rhs.tick_count)


def make_rl_train_pool(
    *,
    seed: int,
    num_envs: int,
    layout: str = "mask",
    deck_ids: tuple[int, int],
    max_decisions: int = 200,
    max_ticks: int = 10_000,
    use_make_pool: bool = False,
):
    if use_make_pool:
        return weiss_sim.make_pool(
            mode="train",
            num_envs=num_envs,
            db_path=str(_FIXTURE_DB_PATH),
            deck_lists=[_DEFAULT_LEGAL_DECK, _DEFAULT_LEGAL_DECK],
            deck_ids=list(deck_ids),
            max_decisions=max_decisions,
            max_ticks=max_ticks,
            seed=seed,
            layout=layout,
        )

    return weiss_sim.EnvPool.new_rl_train(
        num_envs,
        str(_FIXTURE_DB_PATH),
        deck_lists=[_DEFAULT_LEGAL_DECK, _DEFAULT_LEGAL_DECK],
        deck_ids=list(deck_ids),
        max_decisions=max_decisions,
        max_ticks=max_ticks,
        seed=seed,
    )
