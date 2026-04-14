from __future__ import annotations

import builtins
from types import SimpleNamespace

import numpy as np
import pytest
import weiss_sim
import weiss_sim.adapters as adapters
from weiss_sim.errors import WeissSimError
from weiss_sim.types import LegalActions, ResetBatch, StepBatch


def _make_base_arrays(num_envs: int, *, obs_dtype=np.int32) -> dict[str, np.ndarray]:
    return {
        "obs": np.zeros((num_envs, weiss_sim.OBS_LEN), dtype=obs_dtype),
        "to_play_seat": np.zeros((num_envs,), dtype=np.int8),
        "starting_seat": np.zeros((num_envs,), dtype=np.uint8),
        "episode_seed": np.arange(1, num_envs + 1, dtype=np.uint64),
        "episode_index": np.zeros((num_envs,), dtype=np.uint32),
        "env_index": np.arange(num_envs, dtype=np.uint32),
        "episode_key": np.arange(100, 100 + num_envs, dtype=np.uint64),
        "decision_id": np.full((num_envs,), 7, dtype=np.uint32),
        "engine_status": np.zeros((num_envs,), dtype=np.uint8),
        "spec_hash": np.full((num_envs,), np.uint64(123), dtype=np.uint64),
        "main_move_action": np.zeros((num_envs,), dtype=np.bool_),
        "main_pass_action": np.zeros((num_envs,), dtype=np.bool_),
    }


def _make_legal_mask(num_envs: int, action_space: int) -> np.ndarray:
    mask = np.zeros((num_envs, action_space), dtype=np.uint8)
    mask[:, 1] = 1
    mask[:, 3] = 1
    return mask


def _make_reset_batch(num_envs: int, action_space: int, *, obs_dtype=np.int32) -> ResetBatch:
    base = _make_base_arrays(num_envs, obs_dtype=obs_dtype)
    return ResetBatch(
        **base,
        legal_mask=_make_legal_mask(num_envs, action_space),
        legal_ids=None,
        legal_offsets=None,
    )


def _make_step_batch(num_envs: int, action_space: int, *, obs_dtype=np.int32) -> StepBatch:
    base = _make_base_arrays(num_envs, obs_dtype=obs_dtype)
    return StepBatch(
        **base,
        reward=np.full((num_envs,), 0.5, dtype=np.float32),
        terminated=np.zeros((num_envs,), dtype=np.bool_),
        truncated=np.ones((num_envs,), dtype=np.bool_),
        terminal_during_internal_opponent=np.zeros((num_envs,), dtype=np.bool_),
        decision_count=np.full((num_envs,), 1, dtype=np.uint32),
        tick_count=np.full((num_envs,), 2, dtype=np.uint32),
        legal_mask=_make_legal_mask(num_envs, action_space),
        legal_ids=None,
        legal_offsets=None,
    )


def test_import_gym_module_raises_clear_error_when_missing(monkeypatch: pytest.MonkeyPatch) -> None:
    real_import = builtins.__import__

    def _missing_import(name, *args, **kwargs):
        if name in {"gymnasium", "gym"}:
            raise ImportError(f"missing {name}")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", _missing_import)
    with pytest.raises(WeissSimError, match="requires gymnasium or gym"):
        adapters._import_gym_module()


def test_single_env_legal_actions_maps_logits_shape_errors() -> None:
    legal = LegalActions(
        legal_ids=np.array([1, 3], dtype=np.uint16),
        legal_offsets=np.array([0, 2], dtype=np.uint32),
        legal_mask_raw=None,
    )
    single = adapters.SingleEnvLegalActions(legal)

    bad_logits = np.zeros((2, 8), dtype=np.float32)
    with pytest.raises(WeissSimError, match="single-env logits must have shape"):
        single.select_from_logits(bad_logits)
    with pytest.raises(WeissSimError, match="single-env logits must have shape"):
        single.sample_from_logits(bad_logits, seed=9)


class _FakeSingleEnv:
    def __init__(self) -> None:
        self.num_envs = 1
        self.action_space_n = 8
        self._latest_batch = _make_reset_batch(1, self.action_space_n)
        self._step_batch = _make_step_batch(1, self.action_space_n)
        self.closed = False
        self.last_actions: np.ndarray | None = None

    @property
    def legal(self) -> LegalActions:
        return self._latest_batch.legal

    def reset(self, *, seed: int | None = None):
        _ = seed
        self._latest_batch = _make_reset_batch(1, self.action_space_n)
        return self._latest_batch

    def step(self, actions):
        self.last_actions = np.asarray(actions).copy()
        self._latest_batch = self._step_batch
        return self._step_batch

    def render(self, *, env_i: int = 0, mode: str = "ansi"):
        return f"fake-render env={env_i} mode={mode}"

    def close(self) -> None:
        self.closed = True

    def decode_action(self, action_id: int):
        return {"family": "fake", "params": [int(action_id)]}


def test_single_env_adapter_step_info_and_context_close() -> None:
    fake_env = _FakeSingleEnv()
    with adapters.SingleEnvAdapter(fake_env) as env:
        obs = env.reset(seed=5)
        assert obs.shape == (weiss_sim.OBS_LEN,)

        obs2, reward, terminated, truncated, info = env.step(1)
        assert obs2.shape == (weiss_sim.OBS_LEN,)
        assert isinstance(reward, float)
        assert isinstance(terminated, bool)
        assert isinstance(truncated, bool)
        assert info["done"] == (terminated or truncated)
        assert "legal_ids" in info
        assert info["legal_ids"].ndim == 1
        assert "legal_mask" in info
        assert info["legal_mask"].shape == (fake_env.action_space_n,)
        assert fake_env.last_actions is not None
        assert fake_env.last_actions.dtype == np.uint32
        assert int(fake_env.last_actions[0]) == 1

    assert fake_env.closed


class _FakeLegal:
    def __init__(self, mask: np.ndarray):
        self._mask = mask
        self.last_action_space: int | None = None

    def mask_for_action_space(self, action_space: int) -> np.ndarray:
        self.last_action_space = int(action_space)
        return self._mask


class _FakeVectorEnv:
    def __init__(self) -> None:
        self.num_envs = 2
        self.action_space_n = 8
        self.obs_shape = (weiss_sim.OBS_LEN,)
        self._out = SimpleNamespace(
            obs=np.zeros((self.num_envs, weiss_sim.OBS_LEN), dtype=np.int32)
        )
        self._legal_mask = _make_legal_mask(self.num_envs, self.action_space_n)
        self.legal = _FakeLegal(self._legal_mask)
        self._reset_batch = _make_reset_batch(self.num_envs, self.action_space_n)
        self._step_batch = _make_step_batch(self.num_envs, self.action_space_n)
        self.closed = False

    def reset(self, *, seed: int | None = None):
        _ = seed
        return self._reset_batch

    def step(self, actions):
        _ = actions
        return self._step_batch

    def render(self, *, env_i: int = 0, mode: str = "ansi"):
        return f"vec-render env={env_i} mode={mode}"

    def close(self) -> None:
        self.closed = True


class _FakeBox:
    def __init__(self, *, low, high, shape, dtype):
        self.low = low
        self.high = high
        self.shape = tuple(shape)
        self.dtype = np.dtype(dtype)


class _FakeDiscrete:
    def __init__(self, n: int):
        self.n = int(n)


def test_gym_vector_adapter_info_and_mask_plumbing(monkeypatch: pytest.MonkeyPatch) -> None:
    fake_gym = SimpleNamespace(spaces=SimpleNamespace(Box=_FakeBox, Discrete=_FakeDiscrete))
    monkeypatch.setattr(adapters, "_import_gym_module", lambda: fake_gym)

    env = _FakeVectorEnv()
    gym_env = adapters.GymVectorEnvAdapter(env)

    obs, info = gym_env.reset(seed=9, options={"ignored": True})
    assert obs.shape == (env.num_envs, weiss_sim.OBS_LEN)
    assert "action_mask" in info
    assert info["action_mask"].shape == (env.num_envs, env.action_space_n)
    assert gym_env.single_observation_space.shape == env.obs_shape
    assert gym_env.single_action_space.n == env.action_space_n

    obs2, reward, terminated, truncated, info2 = gym_env.step(np.array([1, 3], dtype=np.uint32))
    assert obs2.shape == (env.num_envs, weiss_sim.OBS_LEN)
    assert reward.shape == (env.num_envs,)
    assert terminated.shape == (env.num_envs,)
    assert truncated.shape == (env.num_envs,)
    assert "action_mask" in info2
    assert info2["action_mask"].shape == (env.num_envs, env.action_space_n)

    action_masks = gym_env.action_masks()
    assert action_masks is not None
    assert np.array_equal(action_masks, env._legal_mask)
    assert env.legal.last_action_space == env.action_space_n

    gym_env.close()
    assert env.closed
