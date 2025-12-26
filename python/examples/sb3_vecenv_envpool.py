from pathlib import Path

import numpy as np

try:
    from stable_baselines3.common.vec_env import VecEnv
except ImportError as exc:
    raise SystemExit("stable-baselines3 is required for this example") from exc

try:
    from sb3_contrib.common.maskable.wrappers import MaskableVecEnvWrapper
    from sb3_contrib.ppo_mask import MaskablePPO
except ImportError as exc:
    raise SystemExit("sb3-contrib is required for this example") from exc

import weiss_sim


class EnvPoolVecEnv(VecEnv):
    def __init__(self, db_path: str, deck_lists, deck_ids=None, seed: int = 0, num_envs: int = 8):
        self.pool = weiss_sim.EnvPool.new_rl_train(
            num_envs,
            db_path,
            deck_lists=deck_lists,
            deck_ids=deck_ids,
            max_decisions=10_000,
            max_ticks=100_000,
            seed=seed,
        )
        self.buffers = weiss_sim.EnvPoolBuffers(self.pool)
        self.num_envs = num_envs
        self.actions = None
        obs_shape = (self.pool.obs_len,)
        action_space = self.pool.action_space
        self._obs_space = (
            -1 * np.ones(obs_shape, dtype=np.int32),
            np.iinfo(np.int32).max * np.ones(obs_shape, dtype=np.int32),
        )
        self._action_space = action_space
        self.reset_infos = [{} for _ in range(num_envs)]

        from gymnasium import spaces

        obs_space = spaces.Box(
            low=self._obs_space[0],
            high=self._obs_space[1],
            dtype=np.int32,
        )
        act_space = spaces.Discrete(self._action_space)
        super().__init__(num_envs, obs_space, act_space)

    @property
    def observation_space(self):
        low, high = self._obs_space
        from gymnasium import spaces

        return spaces.Box(low=low, high=high, dtype=np.int32)

    @property
    def action_space(self):
        from gymnasium import spaces

        return spaces.Discrete(self._action_space)

    def reset(self):
        out = self.buffers.reset()
        self.reset_infos = [
            {"actor": int(a), "engine_error_code": int(c)}
            for a, c in zip(out.actor, out.engine_status)
        ]
        return out.obs

    def step_async(self, actions):
        self.actions = np.asarray(actions, dtype=np.int64)

    def step_wait(self):
        out = self.buffers.step(self.actions)
        done = np.logical_or(out.terminated, out.truncated)
        infos = [
            {"actor": int(a), "engine_error_code": int(c)}
            for a, c in zip(out.actor, out.engine_status)
        ]
        if np.any(done):
            done_indices = np.flatnonzero(done).tolist()
            if done_indices:
                reset_out = self.buffers.reset_indices(done_indices)
                for idx in done_indices:
                    infos[idx]["terminal_observation"] = out.obs[idx].copy()
                    out.obs[idx] = reset_out.obs[idx]
                self.buffers.masks[:] = reset_out.masks
        return out.obs, out.rewards, done, infos

    def close(self):
        return None

    def seed(self, seed=None):
        return None

    def get_attr(self, attr_name, indices=None):
        raise AttributeError(attr_name)

    def set_attr(self, attr_name, value, indices=None):
        raise AttributeError(attr_name)

    def env_method(self, method_name, *method_args, indices=None, **method_kwargs):
        if method_name == "action_masks":
            return [mask.copy() for mask in self.action_masks()]
        raise AttributeError(method_name)

    def env_is_wrapped(self, wrapper_class, indices=None):
        return [False] * self.num_envs

    def action_masks(self):
        return self.buffers.masks.astype(bool, copy=False)


def main() -> None:
    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    vec_env = EnvPoolVecEnv(
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        num_envs=8,
        seed=0,
    )
    vec_env = MaskableVecEnvWrapper(vec_env)

    model = MaskablePPO("MlpPolicy", vec_env, verbose=1)
    model.learn(total_timesteps=50_000)


if __name__ == "__main__":
    main()
