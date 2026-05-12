from __future__ import annotations

import argparse
from dataclasses import dataclass

import numpy as np

try:
    import torch
    import torch.nn as nn
    import torch.optim as optim
except ImportError as exc:
    raise SystemExit("torch is required for this example (pip install torch)") from exc

import weiss_sim


@dataclass(frozen=True)
class PpoConfig:
    num_envs: int
    seed: int
    rollout_len: int
    updates: int
    gamma: float
    gae_lambda: float
    lr: float
    clip_coef: float
    ent_coef: float
    vf_coef: float
    epochs: int
    minibatches: int
    max_grad_norm: float
    hidden_dim: int
    enable_shaping: bool
    damage_reward: float
    level_reward: float
    board_reward: float
    no_progress_penalty: float


class Net(nn.Module):
    def __init__(self, obs_dim: int, act_dim: int, hidden_dim: int) -> None:
        super().__init__()
        self.trunk = nn.Sequential(
            nn.Linear(obs_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )
        self.policy = nn.Linear(hidden_dim, act_dim)
        self.value = nn.Linear(hidden_dim, 1)

    def forward(self, obs: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        h = self.trunk(obs)
        return self.policy(h), self.value(h).squeeze(-1)


def _coerce_legal_mask(mask: np.ndarray | None, *, num_envs: int, action_space: int) -> np.ndarray:
    if mask is None:
        raise RuntimeError("legal mask is unavailable; construct the env with legal_repr='mask_u8'")
    arr = np.asarray(mask)
    if arr.shape != (num_envs, action_space):
        raise RuntimeError(
            f"legal mask shape mismatch: got {arr.shape}, expected {(num_envs, action_space)}"
        )
    return arr.astype(np.uint8, copy=False)


def _masked_categorical(
    logits: torch.Tensor,
    legal_mask_u8: np.ndarray,
    *,
    pass_action_id: int,
    illegal_value: float = -1e9,
) -> tuple[torch.distributions.Categorical, torch.Tensor]:
    mask = torch.from_numpy(legal_mask_u8 != 0)
    has_legal = mask.any(dim=1)
    if not bool(has_legal.all()):
        # Ensure distribution validity for rows with no legal actions (should be rare).
        mask = mask.clone()
        mask[~has_legal, int(pass_action_id)] = True
    masked_logits = logits.masked_fill(~mask, float(illegal_value))
    return torch.distributions.Categorical(logits=masked_logits), has_legal


def _compute_gae(
    rewards: np.ndarray,
    dones: np.ndarray,
    values: np.ndarray,
    last_value: np.ndarray,
    *,
    gamma: float,
    gae_lambda: float,
) -> tuple[np.ndarray, np.ndarray]:
    t_steps, n_envs = rewards.shape
    advantages = np.zeros((t_steps, n_envs), dtype=np.float32)
    last_gae = np.zeros((n_envs,), dtype=np.float32)
    for t in reversed(range(t_steps)):
        next_nonterminal = 1.0 - dones[t].astype(np.float32)
        next_value = last_value if t == t_steps - 1 else values[t + 1]
        delta = rewards[t] + gamma * next_value * next_nonterminal - values[t]
        last_gae = delta + gamma * gae_lambda * next_nonterminal * last_gae
        advantages[t] = last_gae
    returns = advantages + values
    return advantages, returns


def parse_args() -> PpoConfig:
    parser = argparse.ArgumentParser(
        description="Minimal PPO example for weiss_sim (masked discrete)."
    )
    parser.add_argument("--num-envs", type=int, default=32)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--rollout-len", type=int, default=128)
    parser.add_argument("--updates", type=int, default=50)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--gae-lambda", type=float, default=0.95)
    parser.add_argument("--lr", type=float, default=2.5e-4)
    parser.add_argument("--clip-coef", type=float, default=0.2)
    parser.add_argument("--ent-coef", type=float, default=0.01)
    parser.add_argument("--vf-coef", type=float, default=0.5)
    parser.add_argument("--epochs", type=int, default=4)
    parser.add_argument("--minibatches", type=int, default=4)
    parser.add_argument("--max-grad-norm", type=float, default=0.5)
    parser.add_argument("--hidden-dim", type=int, default=256)
    parser.add_argument(
        "--enable-shaping", action="store_true", help="Enable simulator reward shaping."
    )
    parser.add_argument("--damage-reward", type=float, default=0.1)
    parser.add_argument("--level-reward", type=float, default=0.0)
    parser.add_argument("--board-reward", type=float, default=0.0)
    parser.add_argument("--no-progress-penalty", type=float, default=0.0)
    args = parser.parse_args()

    if args.num_envs <= 0:
        raise SystemExit("--num-envs must be > 0")
    if args.rollout_len <= 0:
        raise SystemExit("--rollout-len must be > 0")
    if args.updates <= 0:
        raise SystemExit("--updates must be > 0")
    if args.epochs <= 0:
        raise SystemExit("--epochs must be > 0")
    if args.minibatches <= 0:
        raise SystemExit("--minibatches must be > 0")

    return PpoConfig(
        num_envs=int(args.num_envs),
        seed=int(args.seed),
        rollout_len=int(args.rollout_len),
        updates=int(args.updates),
        gamma=float(args.gamma),
        gae_lambda=float(args.gae_lambda),
        lr=float(args.lr),
        clip_coef=float(args.clip_coef),
        ent_coef=float(args.ent_coef),
        vf_coef=float(args.vf_coef),
        epochs=int(args.epochs),
        minibatches=int(args.minibatches),
        max_grad_norm=float(args.max_grad_norm),
        hidden_dim=int(args.hidden_dim),
        enable_shaping=bool(args.enable_shaping),
        damage_reward=float(args.damage_reward),
        level_reward=float(args.level_reward),
        board_reward=float(args.board_reward),
        no_progress_penalty=float(args.no_progress_penalty),
    )


def main() -> None:
    cfg = parse_args()
    torch.manual_seed(cfg.seed)
    np.random.seed(cfg.seed)

    reward = None
    if cfg.enable_shaping:
        reward = weiss_sim.RewardOverrides(
            enable_shaping=True,
            damage_reward=cfg.damage_reward,
            level_reward=cfg.level_reward,
            board_reward=cfg.board_reward,
            no_progress_penalty=cfg.no_progress_penalty,
        )

    with weiss_sim.fast(
        num_envs=cfg.num_envs,
        seed=cfg.seed,
        legal_repr="mask_u8",
        obs_dtype="i16",
        reward=reward,
    ) as sim:
        batch = sim.reset()
        obs_dim = int(batch.obs.shape[1])
        action_space = int(sim.action_space)

        model = Net(obs_dim, action_space, cfg.hidden_dim)
        optimizer = optim.Adam(model.parameters(), lr=cfg.lr)

        for update in range(1, cfg.updates + 1):
            obs_buf = np.zeros((cfg.rollout_len, cfg.num_envs, obs_dim), dtype=np.float32)
            mask_buf = np.zeros((cfg.rollout_len, cfg.num_envs, action_space), dtype=np.uint8)
            act_buf = np.zeros((cfg.rollout_len, cfg.num_envs), dtype=np.int64)
            logp_buf = np.zeros((cfg.rollout_len, cfg.num_envs), dtype=np.float32)
            val_buf = np.zeros((cfg.rollout_len, cfg.num_envs), dtype=np.float32)
            rew_buf = np.zeros((cfg.rollout_len, cfg.num_envs), dtype=np.float32)
            done_buf = np.zeros((cfg.rollout_len, cfg.num_envs), dtype=np.bool_)

            for t in range(cfg.rollout_len):
                obs_buf[t] = batch.obs.astype(np.float32, copy=False)
                legal_mask = _coerce_legal_mask(
                    batch.legal.mask, num_envs=cfg.num_envs, action_space=action_space
                )
                mask_buf[t] = legal_mask

                obs_t = torch.from_numpy(obs_buf[t])
                logits_t, value_t = model(obs_t)
                dist, has_legal = _masked_categorical(
                    logits_t, legal_mask, pass_action_id=int(weiss_sim.PASS_ACTION_ID)
                )
                actions_t = dist.sample()
                actions_t = torch.where(
                    has_legal,
                    actions_t,
                    torch.full_like(actions_t, int(weiss_sim.PASS_ACTION_ID)),
                )
                logp_t = dist.log_prob(actions_t)
                logp_t = torch.where(has_legal, logp_t, torch.zeros_like(logp_t))

                actions_np = actions_t.numpy().astype(np.uint32, copy=False)
                step, _, reset_batch = sim.step_auto(
                    actions_np, reset_done=True, reset_engine_errors=True
                )

                act_buf[t] = actions_t.numpy().astype(np.int64, copy=False)
                logp_buf[t] = logp_t.detach().numpy().astype(np.float32, copy=False)
                val_buf[t] = value_t.detach().numpy().astype(np.float32, copy=False)
                rew_buf[t] = step.reward.astype(np.float32, copy=False)
                done_buf[t] = step.done.astype(np.bool_, copy=False)

                batch = reset_batch if reset_batch is not None else step

            with torch.no_grad():
                last_obs = torch.from_numpy(batch.obs.astype(np.float32, copy=False))
                _, last_value_t = model(last_obs)
                last_value = last_value_t.numpy().astype(np.float32, copy=False)

            advantages, returns = _compute_gae(
                rew_buf,
                done_buf,
                val_buf,
                last_value,
                gamma=cfg.gamma,
                gae_lambda=cfg.gae_lambda,
            )
            advantages = (advantages - advantages.mean()) / (advantages.std() + 1e-8)

            b_obs = torch.from_numpy(obs_buf.reshape((-1, obs_dim)))
            b_mask = torch.from_numpy(mask_buf.reshape((-1, action_space)) != 0)
            b_actions = torch.from_numpy(act_buf.reshape((-1,))).long()
            b_logp = torch.from_numpy(logp_buf.reshape((-1,)))
            b_adv = torch.from_numpy(advantages.reshape((-1,)))
            b_ret = torch.from_numpy(returns.reshape((-1,)))

            batch_size = b_actions.shape[0]
            minibatch_size = batch_size // cfg.minibatches
            if minibatch_size <= 0:
                raise RuntimeError(
                    "minibatch_size is 0; decrease --minibatches or increase rollout size"
                )

            approx_kl = 0.0
            clip_frac = 0.0
            for _ in range(cfg.epochs):
                perm = torch.randperm(batch_size)
                for start in range(0, batch_size, minibatch_size):
                    idx = perm[start : start + minibatch_size]
                    logits, values = model(b_obs[idx])
                    mask = b_mask[idx]
                    has_legal = mask.any(dim=1)
                    if not bool(has_legal.all()):
                        mask = mask.clone()
                        mask[~has_legal, int(weiss_sim.PASS_ACTION_ID)] = True
                    dist = torch.distributions.Categorical(logits=logits.masked_fill(~mask, -1e9))
                    new_logp = dist.log_prob(b_actions[idx])
                    new_logp = torch.where(has_legal, new_logp, torch.zeros_like(new_logp))
                    entropy = dist.entropy().mean()

                    ratio = torch.exp(new_logp - b_logp[idx])
                    unclipped = ratio * b_adv[idx]
                    clipped = (
                        torch.clamp(ratio, 1.0 - cfg.clip_coef, 1.0 + cfg.clip_coef) * b_adv[idx]
                    )
                    pg_loss = -torch.min(unclipped, clipped).mean()
                    v_loss = 0.5 * ((values - b_ret[idx]) ** 2).mean()
                    loss = pg_loss + cfg.vf_coef * v_loss - cfg.ent_coef * entropy

                    optimizer.zero_grad()
                    loss.backward()
                    nn.utils.clip_grad_norm_(model.parameters(), cfg.max_grad_norm)
                    optimizer.step()

                    with torch.no_grad():
                        approx_kl += float((b_logp[idx] - new_logp).mean().item())
                        clip_frac += float(
                            (torch.abs(ratio - 1.0) > cfg.clip_coef).float().mean().item()
                        )

            approx_kl /= float(cfg.epochs * cfg.minibatches)
            clip_frac /= float(cfg.epochs * cfg.minibatches)

            explained_var = float("nan")
            with torch.no_grad():
                y = returns.reshape(-1)
                y_pred = val_buf.reshape(-1)
                var_y = float(np.var(y))
                if var_y > 1e-12:
                    explained_var = 1.0 - float(np.var(y - y_pred) / var_y)

            print(
                f"update={update:04d} "
                f"mean_reward={float(rew_buf.mean()):+.4f} "
                f"done_rate={float(done_buf.mean()):.3f} "
                f"approx_kl={approx_kl:.4f} "
                f"clip_frac={clip_frac:.3f} "
                f"explained_var={explained_var:.3f}"
            )


if __name__ == "__main__":
    main()
