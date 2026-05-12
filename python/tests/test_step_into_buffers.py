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
    context, offsets3 = buffers.legal_action_context_v1()
    assert np.array_equal(legal_ids, ids)
    assert np.array_equal(offsets, offsets2)
    assert np.array_equal(offsets, offsets3)
    assert meta is not None
    assert context.shape[1] == weiss_sim.LEGAL_ACTION_CONTEXT_V1_WIDTH
    assert context.dtype == np.int32

    buffers.select_actions_from_logits(logits)
    buffers.sample_actions_from_logits(logits, seeds)
    action_logp_out = np.empty(pool.envs_len, dtype=np.float32)
    _, _, action_logp = buffers.step_sample_from_logits_with_logp(
        logits,
        seeds,
        action_logp_out,
    )
    assert action_logp is action_logp_out
    assert action_logp.shape == (pool.envs_len,)

    timing = buffers.timing_counters()
    assert timing["timing_enabled"] is True
    assert timing["legal_ids_materialize_count"] == 3
    assert timing["legal_action_meta_materialize_count"] == 1
    assert timing["legal_action_context_v1_materialize_count"] == 1
    assert timing["select_actions_from_logits_count"] == 1
    assert timing["sample_actions_from_logits_count"] == 1
    assert timing["step_sample_from_logits_with_logp_into_i16_legal_ids_count"] == 1
    assert timing["legal_ids_materialize_ns"] >= 0
    assert timing["legal_action_meta_materialize_ns"] >= 0
    assert timing["legal_action_context_v1_materialize_ns"] >= 0
    assert timing["select_actions_from_logits_ns"] >= 0
    assert timing["sample_actions_from_logits_ns"] >= 0
    assert timing["step_sample_from_logits_with_logp_into_i16_legal_ids_ns"] >= 0

    buffers.reset_timing_counters()
    reset_timing = buffers.timing_counters()
    assert reset_timing["legal_ids_materialize_count"] == 0
    assert reset_timing["legal_action_meta_materialize_count"] == 0
    assert reset_timing["legal_action_context_v1_materialize_count"] == 0
    assert reset_timing["select_actions_from_logits_count"] == 0
    assert reset_timing["sample_actions_from_logits_count"] == 0
    assert reset_timing["step_sample_from_logits_with_logp_into_i16_legal_ids_count"] == 0


def test_i16_legal_ids_nometa_layout_matches_packed_layout() -> None:
    pool_meta, buffers_meta = make_rl_train_pool(
        seed=3030,
        num_envs=2,
        deck_ids=(9, 10),
        layout="i16_legal_ids",
        use_make_pool=True,
    )
    pool_nometa, buffers_nometa = make_rl_train_pool(
        seed=3030,
        num_envs=2,
        deck_ids=(9, 10),
        layout="i16_legal_ids_nometa",
        use_make_pool=True,
    )

    out_meta = buffers_meta.reset()
    out_nometa = buffers_nometa.reset()
    assert not hasattr(out_nometa, "legal_action_meta")
    for name in (
        "obs",
        "legal_ids",
        "legal_offsets",
        "rewards",
        "terminated",
        "truncated",
        "actor",
        "decision_kind",
        "decision_id",
        "engine_status",
        "spec_hash",
        "main_move_action",
        "main_pass_action",
    ):
        assert np.array_equal(getattr(out_meta, name), getattr(out_nometa, name))

    ids, meta, offsets = buffers_nometa.legal_action_data()
    assert meta is None
    assert np.array_equal(ids, out_nometa.legal_ids[: int(out_nometa.legal_offsets[-1])])
    assert np.array_equal(offsets, out_nometa.legal_offsets)

    logits = np.random.default_rng(17).standard_normal(
        (pool_meta.envs_len, pool_meta.action_space), dtype=np.float32
    )
    seeds = np.array([21, 22], dtype=np.uint64)
    logp_meta = np.empty(pool_meta.envs_len, dtype=np.float32)
    logp_nometa = np.empty(pool_nometa.envs_len, dtype=np.float32)
    out_meta, actions_meta, logp_meta_ret = buffers_meta.step_sample_from_logits_with_logp(
        logits, seeds, logp_meta
    )
    out_nometa, actions_nometa, logp_nometa_ret = buffers_nometa.step_sample_from_logits_with_logp(
        logits, seeds, logp_nometa
    )
    assert logp_meta_ret is logp_meta
    assert logp_nometa_ret is logp_nometa
    assert np.array_equal(actions_meta, actions_nometa)
    assert np.allclose(logp_meta, logp_nometa)
    for name in (
        "obs",
        "legal_ids",
        "legal_offsets",
        "rewards",
        "terminated",
        "truncated",
        "actor",
        "decision_kind",
        "decision_id",
        "engine_status",
        "spec_hash",
        "main_move_action",
        "main_pass_action",
    ):
        assert np.array_equal(getattr(out_meta, name), getattr(out_nometa, name))


def test_legal_action_context_v1_rows_align_with_legal_ids() -> None:
    pool, buffers = make_rl_train_pool(
        seed=5050,
        num_envs=2,
        deck_ids=(9, 10),
        layout="i16_legal_ids_nometa",
        use_make_pool=True,
    )
    buffers.reset()
    ids, offsets = buffers.legal_action_ids()
    context, context_offsets = buffers.legal_action_context_v1()
    assert np.array_equal(offsets, context_offsets)
    assert context.shape == (int(offsets[-1]), weiss_sim.LEGAL_ACTION_CONTEXT_V1_WIDTH)
    assert context.dtype == np.int32
    assert context.shape[0] == ids.shape[0]
    assert np.all(context[:, 0] >= 0)
    assert np.all((context[:, 4] >= 0) | (context[:, 4] == weiss_sim.LEGAL_ACTION_CONTEXT_UNUSED))
    assert np.all(np.isin(context[:, 5], [0, 1, weiss_sim.LEGAL_ACTION_CONTEXT_UNUSED]))
