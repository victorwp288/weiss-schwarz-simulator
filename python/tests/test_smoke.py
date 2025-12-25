from pathlib import Path

import numpy as np
import weiss_sim


def _first_legal_actions(masks):
    actions = []
    for i in range(masks.shape[0]):
        row = masks[i]
        idxs = np.flatnonzero(row)
        assert idxs.size > 0
        actions.append(int(idxs[0]))
    return actions


def test_envpool_smoke(tmp_path):
    assert hasattr(weiss_sim, "EnvPool")
    assert isinstance(weiss_sim.__version__, str)


def test_envpool_step_shapes_and_turn_cycle():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=123,
        observation_visibility="public",
    )
    obs = pool.reset_all()
    assert obs.shape == (1, pool.obs_len)

    starting_player = int(pool.get_current_player_batch()[0])
    seen_other_turn = False
    for _ in range(50):
        masks = pool.action_masks_batch()
        actions = _first_legal_actions(masks)
        result = pool.step_batch_fast(actions)
        assert len(result) == 9
        next_obs, rewards, terminated, truncated, current_player, decision_kind, actor, illegal_action, engine_error = result
        assert next_obs.shape == (1, pool.obs_len)
        assert rewards.shape == (1,)
        assert terminated.shape == (1,)
        assert truncated.shape == (1,)
        assert current_player.shape == (1,)
        assert decision_kind.shape == (1,)
        assert actor.shape == (1,)
        assert illegal_action.shape == (1,)
        assert engine_error.shape == (1,)
        if int(current_player[0]) != starting_player and int(decision_kind[0]) == 1:
            seen_other_turn = True
            break
    assert seen_other_turn


def test_action_mask_legality_alignment():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=456,
        observation_visibility="public",
    )
    pool.reset_all()
    for _ in range(30):
        masks = pool.action_masks_batch()
        actions = _first_legal_actions(masks)
        assert masks[0, actions[0]] == 1
        pool.step_batch_fast(actions)


def test_envpool_determinism_across_pools():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool_a = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=999,
        observation_visibility="public",
    )
    pool_b = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=999,
        observation_visibility="public",
    )
    obs_a = pool_a.reset_all()
    obs_b = pool_b.reset_all()
    assert np.array_equal(obs_a, obs_b)
    for _ in range(30):
        masks_a = pool_a.action_masks_batch()
        masks_b = pool_b.action_masks_batch()
        assert np.array_equal(masks_a, masks_b)
        actions = _first_legal_actions(masks_a)
        out_a = pool_a.step_batch_fast(actions)
        out_b = pool_b.step_batch_fast(actions)
        for left, right in zip(out_a, out_b):
            assert np.array_equal(left, right)
        terminated = bool(out_a[2][0])
        truncated = bool(out_a[3][0])
        if terminated or truncated:
            break


def test_concede_action_mask_bit():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool = weiss_sim.EnvPool(
        1,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=321,
        observation_visibility="public",
    )
    pool.reset_all()
    masks = pool.action_masks_batch()
    legal = pool.legal_actions_batch()
    concede_id = pool.action_space - 1
    assert masks.shape[1] == pool.action_space
    assert masks[0, concede_id] == 1
    assert any(action.get("kind") == "concede" for action in legal[0])
