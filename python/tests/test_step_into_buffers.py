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
