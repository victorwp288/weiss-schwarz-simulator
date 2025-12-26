from pathlib import Path
import json

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


def test_priority_window_includes_pass_option():
    fixture_dir = Path(__file__).parent / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    deck = ([14] * 4) + ([1] * 4) + ([2] * 4) + ([3] * 4) + ([4] * 4) + ([5] * 4) + ([6] * 4) + ([7] * 4) + ([8] * 4) + ([9] * 4) + ([10] * 4) + ([11] * 4) + ([12] * 2)
    curriculum = json.dumps(
        {
            "enable_priority_windows": True,
            "enable_activated_abilities": True,
            "priority_allow_pass": True,
            "strict_priority_mode": False,
        }
    )
    for seed in range(30):
        pool = weiss_sim.EnvPool.new_rl_train(
            1,
            str(db_path),
            deck_lists=[deck, deck],
            deck_ids=[41, 42],
            max_decisions=200,
            max_ticks=10_000,
            seed=seed,
            curriculum_json=curriculum,
        )
        out = weiss_sim.BatchOutMinimal(1)
        pool.reset_into(out)

        played_ability = False
        for _ in range(80):
            info = pool.decision_info_batch()[0]
            if info.get("choice_reason") == "PriorityActionSelect":
                zones = info.get("choice_option_zones")
                assert zones is not None
                assert "PriorityPass" in zones
                return

            legal_ids = np.flatnonzero(out.masks[0]).tolist()
            lookup = pool.action_lookup_batch()[0]

            chosen = None
            decision_kind = info.get("decision_kind")
            if decision_kind == "Main":
                if not played_ability:
                    for action_id in legal_ids:
                        desc = lookup[action_id]
                        if desc and desc.get("kind") == "main_play_character":
                            chosen = action_id
                            played_ability = True
                            break
                if chosen is None:
                    for action_id in legal_ids:
                        desc = lookup[action_id]
                        if desc and desc.get("kind") == "pass":
                            chosen = action_id
                            break
            if chosen is None:
                chosen = legal_ids[0]

            pool.step_into(np.array([chosen], dtype=np.uint32), out)

    raise AssertionError("priority window did not appear within seed/step budget")
