import numpy as np

from tests.support import (
    first_legal_actions as _first_legal_actions,
    make_rl_train_pool,
)


def _make_pool(seed=888, num_envs=2, *, layout="mask"):
    return make_rl_train_pool(
        seed=seed,
        num_envs=num_envs,
        layout=layout,
        deck_ids=(1, 2),
        use_make_pool=True,
    )


def test_envpool_buffers_step_reuses_buffers():
    pool, buffers = _make_pool(layout="mask")
    out_reset = buffers.reset()
    assert out_reset is buffers.out

    actions = _first_legal_actions(buffers.masks)
    out_step = buffers.step(np.array(actions, dtype=np.uint32))
    assert out_step is buffers.out
    assert buffers.obs is buffers.out.obs
    assert buffers.masks is buffers.out.masks
    assert buffers.rewards is buffers.out.rewards
    assert buffers.terminated is buffers.out.terminated
    assert buffers.truncated is buffers.out.truncated
    assert buffers.actor is buffers.out.actor
    assert buffers.decision_id is buffers.out.decision_id
    assert buffers.engine_status is buffers.out.engine_status
    assert buffers.layout == "mask"
    assert pool.envs_len == 2
