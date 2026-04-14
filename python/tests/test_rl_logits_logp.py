import numpy as np

import weiss_sim


def _manual_sampled_logp(
    logits: np.ndarray,
    legal_ids: np.ndarray,
    legal_offsets: np.ndarray,
    actions: np.ndarray,
) -> np.ndarray:
    out = np.zeros((actions.shape[0],), dtype=np.float32)
    for env_index in range(actions.shape[0]):
        start = int(legal_offsets[env_index])
        end = int(legal_offsets[env_index + 1])
        ids = np.asarray(legal_ids[start:end], dtype=np.int64)
        if ids.size == 0:
            out[env_index] = 0.0
            continue
        row = np.asarray(logits[env_index, ids], dtype=np.float64)
        max_logit = float(np.max(row))
        probs = np.exp(row - max_logit)
        total = float(np.sum(probs))
        chosen = int(actions[env_index])
        chosen_index = int(np.flatnonzero(ids == chosen)[0])
        out[env_index] = float((row[chosen_index] - max_logit) - np.log(total))
    return out


def test_step_rl_sample_from_logits_with_logp_matches_manual_softmax() -> None:
    legal_deck = (list(range(1, 14)) * 4)[:50]
    pool, _ = weiss_sim.make_pool(
        mode="train",
        num_envs=4,
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[101, 102],
        max_decisions=200,
        max_ticks=10_000,
        seed=909,
        layout="i16_legal_ids",
    )
    reset = weiss_sim.reset_rl(pool, layout="i16_legal_ids")
    assert reset.legal_ids is not None
    assert reset.legal_offsets is not None
    legal_ids_before = np.asarray(reset.legal_ids, dtype=np.uint32).copy()
    legal_offsets_before = np.asarray(reset.legal_offsets, dtype=np.uint32).copy()
    logits = np.random.default_rng(77).standard_normal(
        (pool.envs_len, pool.action_space), dtype=np.float32
    )
    step, actions, action_logp = weiss_sim.step_rl_sample_from_logits_with_logp(
        pool,
        logits,
        seeds=np.array([11, 12, 13, 14], dtype=np.uint64),
        layout="i16_legal_ids",
    )
    assert step.obs.shape == (pool.envs_len, pool.obs_len)
    expected = _manual_sampled_logp(logits, legal_ids_before, legal_offsets_before, actions)
    np.testing.assert_allclose(action_logp, expected, rtol=1e-6, atol=1e-6)
