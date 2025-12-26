from pathlib import Path

import numpy as np
import weiss_sim


def main() -> None:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    pool = weiss_sim.EnvPool.new_rl_train(
        2,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        max_decisions=200,
        max_ticks=10_000,
        seed=42,
        num_threads=None,
    )
    buffers = weiss_sim.EnvPoolBuffers(pool)
    out = buffers.reset()
    actions = np.empty(out.masks.shape[0], dtype=np.uint32)
    for _ in range(50):
        ids, offsets = buffers.legal_action_ids()
        done = np.logical_or(out.terminated, out.truncated)
        for i in range(out.masks.shape[0]):
            start = int(offsets[i])
            end = int(offsets[i + 1])
            if start == end:
                if not bool(done[i]):
                    raise RuntimeError(
                        f"no legal actions for live env {i}: "
                        f"decision_id={int(out.decision_id[i])} "
                        f"engine_status={int(out.engine_status[i])} "
                        f"actor={int(out.actor[i])}"
                    )
                actions[i] = weiss_sim.PASS_ACTION_ID
            else:
                actions[i] = int(ids[start])
        out = buffers.step(actions)
        if bool(out.terminated.any()) or bool(out.truncated.any()):
            break

    print("obs:", out.obs.shape, "masks:", out.masks.shape, "rewards:", out.rewards.shape)
    print("actor:", out.actor.tolist(), "engine_status:", out.engine_status.tolist())


if __name__ == "__main__":
    main()
