from pathlib import Path

import numpy as np

try:
    import gymnasium as gym
    from gymnasium import spaces
except ImportError as exc:
    raise SystemExit("gymnasium is required for this example") from exc

try:
    from sb3_contrib.common.maskable.utils import ActionMasker
    from sb3_contrib.ppo_mask import MaskablePPO
except ImportError as exc:
    raise SystemExit("sb3-contrib is required for this example") from exc

import weiss_sim


class WeissSB3Env(gym.Env):
    metadata = {"render_modes": []}

    def __init__(self, db_path: str, deck_lists, deck_ids=None, seed: int = 0):
        super().__init__()
        self.pool = weiss_sim.EnvPool.new_rl_train(
            1,
            db_path,
            deck_lists=deck_lists,
            deck_ids=deck_ids,
            max_decisions=10_000,
            max_ticks=100_000,
            seed=seed,
        )
        self._out = weiss_sim.BatchOutMinimal(1)
        self.action_space = spaces.Discrete(self.pool.action_space)
        self.observation_space = spaces.Box(
            low=-1,
            high=np.iinfo(np.int32).max,
            shape=(self.pool.obs_len,),
            dtype=np.int32,
        )
        self._last_mask = None

    def reset(self, seed=None, options=None):
        self.pool.reset_into(self._out)
        self._last_mask = self._out.masks[0].astype(bool, copy=False)
        info = {
            "actor": int(self._out.actor[0]),
            "engine_error_code": int(self._out.engine_status[0]),
        }
        return self._out.obs[0], info

    def step(self, action):
        self.pool.step_into(np.array([int(action)], dtype=np.uint32), self._out)
        self._last_mask = self._out.masks[0].astype(bool, copy=False)
        info = {
            "actor": int(self._out.actor[0]),
            "engine_error_code": int(self._out.engine_status[0]),
        }
        return (
            self._out.obs[0],
            float(self._out.rewards[0]),
            bool(self._out.terminated[0]),
            bool(self._out.truncated[0]),
            info,
        )

    def action_masks(self):
        if self._last_mask is None:
            raise RuntimeError("action_masks called before reset")
        return self._last_mask


def main() -> None:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    def mask_fn(env: WeissSB3Env):
        return env.action_masks()

    env = WeissSB3Env(str(db_path), deck_lists=[legal_deck, legal_deck], deck_ids=[1, 2])
    env = ActionMasker(env, mask_fn)

    model = MaskablePPO("MlpPolicy", env, verbose=1)
    model.learn(total_timesteps=10_000)


if __name__ == "__main__":
    main()
