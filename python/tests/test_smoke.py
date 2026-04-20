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


def _make_pool(seed=123, num_envs=1, num_threads=None, output_masks=True):
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]
    return weiss_sim.EnvPool.new_rl_train(
        num_envs,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=seed,
        num_threads=num_threads,
        output_masks=output_masks,
    )


def test_envpool_smoke():
    assert hasattr(weiss_sim, "EnvPool")
    assert hasattr(weiss_sim, "BatchOutMinimal")
    assert isinstance(weiss_sim.__version__, str)


def test_envpool_step_shapes_and_turn_cycle():
    pool = _make_pool(seed=123, num_envs=1)
    out = weiss_sim.BatchOutMinimal(1)
    pool.reset_into(out)
    assert out.obs.shape == (1, pool.obs_len)
    assert out.masks.shape == (1, pool.action_space)
    assert out.spec_hash.shape == (1,)

    starting_actor = int(out.actor[0])
    seen_other_turn = False
    for _ in range(50):
        actions = _first_legal_actions(out.masks)
        pool.step_into(np.array(actions, dtype=np.uint32), out)
        assert out.obs.shape == (1, pool.obs_len)
        assert out.rewards.shape == (1,)
        assert out.terminated.shape == (1,)
        assert out.truncated.shape == (1,)
        assert out.actor.shape == (1,)
        if int(out.actor[0]) != starting_actor:
            seen_other_turn = True
            break
    assert seen_other_turn


def test_action_mask_legality_alignment():
    pool = _make_pool(seed=456, num_envs=1)
    out = weiss_sim.BatchOutMinimal(1)
    pool.reset_into(out)
    for _ in range(30):
        actions = _first_legal_actions(out.masks)
        assert out.masks[0, actions[0]] == 1
        pool.step_into(np.array(actions, dtype=np.uint32), out)


def test_envpool_determinism_across_pools():
    pool_a = _make_pool(seed=999, num_envs=1)
    pool_b = _make_pool(seed=999, num_envs=1)
    out_a = weiss_sim.BatchOutMinimal(1)
    out_b = weiss_sim.BatchOutMinimal(1)
    pool_a.reset_into(out_a)
    pool_b.reset_into(out_b)
    assert np.array_equal(out_a.obs, out_b.obs)
    assert np.array_equal(out_a.masks, out_b.masks)
    for _ in range(30):
        actions = _first_legal_actions(out_a.masks)
        pool_a.step_into(np.array(actions, dtype=np.uint32), out_a)
        pool_b.step_into(np.array(actions, dtype=np.uint32), out_b)
        assert np.array_equal(out_a.obs, out_b.obs)
        assert np.array_equal(out_a.masks, out_b.masks)
        if bool(out_a.terminated[0]) or bool(out_a.truncated[0]):
            break


def test_envpool_num_threads_optional_and_deterministic():
    pool_default = _make_pool(seed=1234, num_envs=1)
    pool_threaded = _make_pool(seed=1234, num_envs=1, num_threads=2)
    out_a = weiss_sim.BatchOutMinimal(1)
    out_b = weiss_sim.BatchOutMinimal(1)
    pool_default.reset_into(out_a)
    pool_threaded.reset_into(out_b)
    assert np.array_equal(out_a.obs, out_b.obs)
    for _ in range(20):
        actions = _first_legal_actions(out_a.masks)
        pool_default.step_into(np.array(actions, dtype=np.uint32), out_a)
        pool_threaded.step_into(np.array(actions, dtype=np.uint32), out_b)
        assert np.array_equal(out_a.obs, out_b.obs)
        assert np.array_equal(out_a.masks, out_b.masks)
        if bool(out_a.terminated[0]) or bool(out_a.truncated[0]):
            break


def test_envpool_num_threads_default_and_overrides():
    pool_default = _make_pool(seed=2026, num_envs=8)
    assert pool_default.num_threads >= 1
    assert pool_default.num_threads <= 8

    pool_serial = _make_pool(seed=2026, num_envs=8, num_threads=1)
    assert pool_serial.num_threads == 1

    pool_capped = _make_pool(seed=2026, num_envs=3, num_threads=4)
    assert pool_capped.num_threads == 3


def test_heuristic_public_rollout_keeps_spec_hash_and_records_episode_seed():
    pool = _make_pool(seed=31415, num_envs=2, output_masks=False)
    reset = weiss_sim.BatchOutMinimalI16LegalIds(2)
    pool.reset_into_i16_legal_ids(reset)
    initial_episode_seed = np.asarray(pool.episode_seed_batch(), dtype=np.uint64)

    trajectory = weiss_sim.BatchOutTrajectoryI16LegalIds(2, 2)
    pool.rollout_heuristic_public_into_i16_legal_ids(2, trajectory)

    assert trajectory.episode_seed.shape == (2, 2)
    assert trajectory.episode_seed.dtype == np.uint64
    assert np.array_equal(trajectory.episode_seed[0], initial_episode_seed)
    assert trajectory.spec_hash.shape == (2, 2)
    assert trajectory.spec_hash.dtype == np.uint64
    assert np.all(trajectory.spec_hash == np.uint64(weiss_sim.SPEC_HASH))
