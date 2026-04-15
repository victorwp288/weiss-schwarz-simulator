import numpy as np
import weiss_sim

from tests.support import (
    first_legal_actions as _first_legal_actions,
    make_rl_train_pool,
)


def _make_pool(seed=2024):
    return make_rl_train_pool(
        seed=seed,
        num_envs=1,
        deck_ids=(9, 10),
    )


def test_step_into_buffers_match_between_pools():
    pool_a = _make_pool(seed=2024)
    pool_b = _make_pool(seed=2024)
    out_a = weiss_sim.BatchOutMinimal(1)
    out_b = weiss_sim.BatchOutMinimal(1)
    pool_a.reset_into(out_a)
    pool_b.reset_into(out_b)
    for _ in range(10):
        actions = _first_legal_actions(out_a.masks)
        pool_a.step_into(np.array(actions, dtype=np.uint32), out_a)
        pool_b.step_into(np.array(actions, dtype=np.uint32), out_b)
        assert np.array_equal(out_a.obs, out_b.obs)
        assert np.array_equal(out_a.masks, out_b.masks)
        if bool(out_a.terminated[0]) or bool(out_a.truncated[0]):
            break


def test_env_pool_timing_counters_cover_packed_paths() -> None:
    pool, buffers = make_rl_train_pool(
        seed=2025,
        num_envs=2,
        deck_ids=(9, 10),
        layout="i16_legal_ids",
        use_make_pool=True,
    )
    buffers.set_timing_enabled(True)
    buffers.reset_timing_counters()
    buffers.reset()

    logits = np.random.default_rng(7).standard_normal(
        (pool.envs_len, pool.action_space), dtype=np.float32
    )
    seeds = np.array([11, 12], dtype=np.uint64)

    legal_ids, offsets = buffers.legal_action_ids()
    ids, meta, offsets2 = buffers.legal_action_data()
    assert np.array_equal(legal_ids, ids)
    assert np.array_equal(offsets, offsets2)
    assert meta is not None

    buffers.select_actions_from_logits(logits)
    buffers.sample_actions_from_logits(logits, seeds)
    _, _, action_logp = weiss_sim.step_rl_sample_from_logits_with_logp(
        pool,
        logits,
        seeds,
        layout="i16_legal_ids",
    )
    assert action_logp.shape == (pool.envs_len,)

    timing = buffers.timing_counters()
    assert timing["timing_enabled"] is True
    assert timing["legal_ids_materialize_count"] == 2
    assert timing["legal_action_meta_materialize_count"] == 1
    assert timing["select_actions_from_logits_count"] == 1
    assert timing["sample_actions_from_logits_count"] == 1
    assert timing["step_sample_from_logits_with_logp_into_i16_legal_ids_count"] == 1
    assert timing["legal_ids_materialize_ns"] >= 0
    assert timing["legal_action_meta_materialize_ns"] >= 0
    assert timing["select_actions_from_logits_ns"] >= 0
    assert timing["sample_actions_from_logits_ns"] >= 0
    assert timing["step_sample_from_logits_with_logp_into_i16_legal_ids_ns"] >= 0

    buffers.reset_timing_counters()
    reset_timing = buffers.timing_counters()
    assert reset_timing["legal_ids_materialize_count"] == 0
    assert reset_timing["legal_action_meta_materialize_count"] == 0
    assert reset_timing["select_actions_from_logits_count"] == 0
    assert reset_timing["sample_actions_from_logits_count"] == 0
    assert reset_timing["step_sample_from_logits_with_logp_into_i16_legal_ids_count"] == 0
