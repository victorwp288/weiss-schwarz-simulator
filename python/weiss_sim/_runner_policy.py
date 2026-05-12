from __future__ import annotations

from typing import TYPE_CHECKING, Callable, Literal

import numpy as np

from .errors import WeissSimError
from .types import ResetBatch, StepBatch
from .weiss_sim import decode_action_id

if TYPE_CHECKING:
    from .adapters import GymVectorEnvAdapter, SingleEnvAdapter


class _PolicyRenderMixin:
    """Convenience policy, rollout, rendering, and adapter methods for WeissEnv."""

    def step_first_legal(self) -> tuple[StepBatch, np.ndarray]:
        """Step by selecting the first legal action for each env."""
        self._require_open()
        actions = self._require_latest_batch().legal.first_legal()
        return self._step_with_actions(actions), actions

    def step_uniform_legal(
        self, seed: int | np.ndarray | None = None
    ) -> tuple[StepBatch, np.ndarray]:
        """Step by sampling a uniform-random legal action for each env."""
        self._require_open()
        self._require_latest_batch()
        seeds = self._coerce_sample_seeds(seed)
        actions = np.empty(self._num_envs, dtype=np.uint32)
        to_play_before = self._last_to_play_seat.copy()
        self._call_step_uniform_legal(seeds, actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before), actions

    def step_auto(
        self,
        actions: object | None = None,
        *,
        policy: Literal["first", "uniform", "random"] = "first",
        seed: int | np.ndarray | None = None,
        reset_done: bool = True,
        reset_engine_errors: bool = True,
    ) -> tuple[StepBatch, np.ndarray, ResetBatch | None]:
        """Step once, with optional action selection and automatic reset handling."""
        self._require_open()
        batch = self._require_latest_batch()
        if actions is None:
            chosen = batch.legal.choose(policy, seed=seed)
        else:
            chosen = self._coerce_actions(actions, name="actions")
        step = self._step_with_actions(chosen)
        step_snapshot = self._snapshot_step_batch(step)
        reset_batch = self._apply_auto_resets(
            step,
            reset_done=bool(reset_done),
            reset_engine_errors=bool(reset_engine_errors),
        )
        return step_snapshot, chosen, reset_batch

    def rollout(
        self,
        steps: int,
        *,
        policy: Literal["first", "uniform", "random"]
        | Callable[[ResetBatch | StepBatch], object] = "uniform",
        seed: int | np.ndarray | None = None,
        auto_reset: bool = False,
        reset_done: bool = True,
        reset_engine_errors: bool = True,
    ) -> list[StepBatch]:
        """Run `steps` decisions with a policy string or callback action function."""
        self._require_open()
        steps_int = int(steps)
        if steps_int <= 0:
            raise WeissSimError("steps must be > 0")

        if self._latest_batch is None:
            self.reset()

        rollout_rng: np.random.Generator | None = None
        policy_token: str | None = None
        if callable(policy):
            policy_fn = policy
        else:
            policy_fn = None
            policy_token = str(policy).strip().lower()
            if policy_token not in {"first", "uniform", "random"}:
                raise WeissSimError("policy must be one of: first, uniform, random, or callable")
            if policy_token in {"uniform", "random"} and np.isscalar(seed):
                rollout_rng = np.random.default_rng(int(seed))

        trajectory: list[StepBatch] = []
        for _ in range(steps_int):
            current = self._require_latest_batch()
            if policy_fn is not None:
                actions = self._coerce_actions(policy_fn(current), name="policy actions")
            elif policy_token == "first":
                actions = current.legal.first_legal()
            else:
                step_seed: int | np.ndarray | None = seed
                if rollout_rng is not None:
                    step_seed = rollout_rng.integers(
                        0,
                        np.iinfo(np.uint64).max,
                        size=self._num_envs,
                        dtype=np.uint64,
                    )
                actions = current.legal.sample_uniform(seed=step_seed)

            step = self._step_with_actions(actions)
            trajectory.append(self._snapshot_step_batch(step))
            if auto_reset:
                self._apply_auto_resets(
                    step,
                    reset_done=bool(reset_done),
                    reset_engine_errors=bool(reset_engine_errors),
                )

        return trajectory

    def _prepare_logits_for_step(
        self,
        logits: object,
        *,
        illegal_value: float,
        temperature: float = 1.0,
    ) -> np.ndarray:
        logits_arr = self._coerce_logits(logits)
        logits_arr = self._apply_illegal_value_compatibility_mask(
            logits_arr, illegal_value=illegal_value
        )
        if temperature != 1.0:
            logits_arr = np.ascontiguousarray(
                logits_arr / np.float32(temperature), dtype=np.float32
            )
        return logits_arr

    def _execute_step_argmax_logits(self, logits: np.ndarray) -> tuple[StepBatch, np.ndarray]:
        actions = np.empty(self._num_envs, dtype=np.uint32)
        to_play_before = self._last_to_play_seat.copy()
        self._call_step_argmax_logits(logits, actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before), actions

    def _execute_step_sample_logits(
        self, logits: np.ndarray, seeds: np.ndarray
    ) -> tuple[StepBatch, np.ndarray]:
        actions = np.empty(self._num_envs, dtype=np.uint32)
        to_play_before = self._last_to_play_seat.copy()
        self._call_step_sample_logits(logits, seeds, actions)
        return self._finalize_step_from_current_out(to_play_before=to_play_before), actions

    def step_argmax_logits(
        self, logits: np.ndarray, illegal_value: float = -1e9
    ) -> tuple[StepBatch, np.ndarray]:
        self._require_open()
        self._require_latest_batch()
        logits_arr = self._prepare_logits_for_step(logits, illegal_value=illegal_value)
        return self._execute_step_argmax_logits(logits_arr)

    def step_sample_logits(
        self,
        logits: np.ndarray,
        seed: int | np.ndarray | None = None,
        temperature: float = 1.0,
        illegal_value: float = -1e9,
    ) -> tuple[StepBatch, np.ndarray]:
        self._require_open()
        self._require_latest_batch()
        temp = float(temperature)
        if temp < 0.0:
            raise WeissSimError("temperature must be >= 0")
        if temp == 0.0:
            return self.step_argmax_logits(logits, illegal_value=illegal_value)

        logits_arr = self._prepare_logits_for_step(
            logits, illegal_value=illegal_value, temperature=temp
        )
        seeds = self._coerce_sample_seeds(seed)
        return self._execute_step_sample_logits(logits_arr, seeds)

    def auto_reset_on_engine_errors(
        self, codes: np.ndarray | None = None
    ) -> tuple[int, ResetBatch | None]:
        """Auto-reset envs with non-zero engine-status codes via Rust auto-reset APIs."""
        self._require_open()
        codes_arr = self._coerce_engine_status_codes(codes)
        reset_count = self._call_auto_reset_on_error_codes(codes_arr)
        if reset_count == 0:
            return 0, None
        reset_indices = np.flatnonzero(codes_arr != 0).astype(np.int64, copy=False)
        reset_batch = self._finalize_reset(reset_indices=reset_indices)
        return reset_count, reset_batch

    @staticmethod
    def _render_perspective(batch: ResetBatch | StepBatch, env_i: int) -> int:
        perspective = int(batch.to_play_seat[env_i])
        if perspective in (0, 1):
            return perspective
        starting_seat = int(batch.starting_seat[env_i])
        if starting_seat in (0, 1):
            return starting_seat
        return 0

    def decode_action(self, action_id: int) -> dict[str, object] | None:
        """Decode a numeric action id into a structured Python dict, or `None` if unknown."""
        return decode_action_id(int(action_id))

    def render(self, env_i: int = 0, mode: Literal["ansi"] = "ansi") -> str:
        """Render a human-readable ANSI board view for a single env."""
        self._require_open()
        if mode != "ansi":
            raise WeissSimError(f"unsupported render mode {mode!r}; only 'ansi' is supported")
        batch = self._require_latest_batch()
        idx = int(env_i)
        if idx < 0 or idx >= self._num_envs:
            raise WeissSimError(f"env_i must be in [0, {self._num_envs - 1}], got {idx}")
        render_ansi = getattr(self.pool, "render_ansi", None)
        if callable(render_ansi):
            perspective = self._render_perspective(batch, idx)
            return str(render_ansi(idx, perspective))
        legal_ids = batch.legal.ids(idx)
        obs_row = np.asarray(batch.obs[idx]).ravel()
        preview_len = min(24, obs_row.shape[0])
        obs_preview = np.array2string(obs_row[:preview_len], precision=3, separator=", ")
        if isinstance(batch, StepBatch):
            done_value = bool(batch.done[idx])
            reward_value = float(batch.reward[idx])
        else:
            done_value = False
            reward_value = 0.0
        lines = [
            f"WeissEnv[{idx}]",
            f"to_play_seat={int(batch.to_play_seat[idx])} starting_seat={int(batch.starting_seat[idx])}",
            f"done={done_value} reward={reward_value:.6f} decision_id={int(batch.decision_id[idx])}",
            f"episode_seed={int(batch.episode_seed[idx])} episode_index={int(batch.episode_index[idx])}",
            f"legal_ids={legal_ids.tolist()}",
            f"obs[:{preview_len}]={obs_preview}",
        ]
        return "\n".join(lines)

    def as_single_env(self) -> SingleEnvAdapter:
        """Wrap this batched env as a single-environment adapter."""
        from .adapters import SingleEnvAdapter

        return SingleEnvAdapter(self)

    def as_gym(self) -> GymVectorEnvAdapter:
        """Wrap this env as a Gymnasium-style vector environment adapter."""
        from .adapters import GymVectorEnvAdapter

        return GymVectorEnvAdapter(self)
