from __future__ import annotations

import argparse
import copy
import json
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
class ImpalaConfig:
    num_envs: int
    seed: int
    unroll_len: int
    updates: int
    gamma: float
    lr: float
    ent_coef: float
    vf_coef: float
    max_grad_norm: float
    hidden_dim: int
    rho_bar: float
    c_bar: float
    sync_every: int
    enable_shaping: bool
    damage_reward: float


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
        raise RuntimeError(
            "legal mask is unavailable; construct the env with legal_repr='mask_u8'"
        )
    arr = np.asarray(mask)
    if arr.shape != (num_envs, action_space):
        raise RuntimeError(f"legal mask shape mismatch: got {arr.shape}, expected {(num_envs, action_space)}")
    return arr.astype(np.uint8, copy=False)


def _masked_dist(
    logits: torch.Tensor,
    legal_mask_u8: np.ndarray,
    *,
    pass_action_id: int,
    illegal_value: float = -1e9,
) -> tuple[torch.distributions.Categorical, torch.Tensor]:
    mask = torch.from_numpy(legal_mask_u8 != 0)
    has_legal = mask.any(dim=1)
    if not bool(has_legal.all()):
        mask = mask.clone()
        mask[~has_legal, int(pass_action_id)] = True
    masked_logits = logits.masked_fill(~mask, float(illegal_value))
    return torch.distributions.Categorical(logits=masked_logits), has_legal


@torch.no_grad()
def _vtrace_targets(
    rewards: torch.Tensor,  # (T, N)
    dones: torch.Tensor,  # (T, N) bool
    behavior_logp: torch.Tensor,  # (T, N)
    target_logp: torch.Tensor,  # (T, N)
    values: torch.Tensor,  # (T+1, N)
    *,
    gamma: float,
    rho_bar: float,
    c_bar: float,
) -> tuple[torch.Tensor, torch.Tensor]:
    t_steps, _ = rewards.shape
    discounts = float(gamma) * (~dones).float()

    log_rhos = target_logp - behavior_logp
    rhos = torch.exp(log_rhos)
    clipped_rhos = torch.clamp(rhos, max=float(rho_bar))
    cs = torch.clamp(rhos, max=float(c_bar))

    deltas = clipped_rhos * (rewards + discounts * values[1:] - values[:-1])

    vs = torch.empty_like(values)
    vs[-1] = values[-1]
    for t in reversed(range(t_steps)):
        vs[t] = values[t] + deltas[t] + discounts[t] * cs[t] * (vs[t + 1] - values[t + 1])

    pg_adv = clipped_rhos * (rewards + discounts * vs[1:] - values[:-1])
    return vs[:-1], pg_adv


def parse_args() -> ImpalaConfig:
    parser = argparse.ArgumentParser(description="Minimal IMPALA/V-trace example for weiss_sim.")
    parser.add_argument("--num-envs", type=int, default=32)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--unroll-len", type=int, default=32)
    parser.add_argument("--updates", type=int, default=200)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--lr", type=float, default=5e-4)
    parser.add_argument("--ent-coef", type=float, default=0.01)
    parser.add_argument("--vf-coef", type=float, default=0.5)
    parser.add_argument("--max-grad-norm", type=float, default=0.5)
    parser.add_argument("--hidden-dim", type=int, default=256)
    parser.add_argument("--rho-bar", type=float, default=1.0)
    parser.add_argument("--c-bar", type=float, default=1.0)
    parser.add_argument("--sync-every", type=int, default=1, help="Learner->actor sync interval (updates).")
    parser.add_argument("--enable-shaping", action="store_true", help="Enable damage shaping rewards.")
    parser.add_argument("--damage-reward", type=float, default=0.1)
    args = parser.parse_args()

    if args.num_envs <= 0:
        raise SystemExit("--num-envs must be > 0")
    if args.unroll_len <= 0:
        raise SystemExit("--unroll-len must be > 0")
    if args.updates <= 0:
        raise SystemExit("--updates must be > 0")
    if args.rho_bar <= 0 or args.c_bar <= 0:
        raise SystemExit("--rho-bar and --c-bar must be > 0")
    if args.sync_every <= 0:
        raise SystemExit("--sync-every must be > 0")

    return ImpalaConfig(
        num_envs=int(args.num_envs),
        seed=int(args.seed),
        unroll_len=int(args.unroll_len),
        updates=int(args.updates),
        gamma=float(args.gamma),
        lr=float(args.lr),
        ent_coef=float(args.ent_coef),
        vf_coef=float(args.vf_coef),
        max_grad_norm=float(args.max_grad_norm),
        hidden_dim=int(args.hidden_dim),
        rho_bar=float(args.rho_bar),
        c_bar=float(args.c_bar),
        sync_every=int(args.sync_every),
        enable_shaping=bool(args.enable_shaping),
        damage_reward=float(args.damage_reward),
    )


def main() -> None:
    cfg = parse_args()
    torch.manual_seed(cfg.seed)
    np.random.seed(cfg.seed)

    reward_json = None
    if cfg.enable_shaping:
        reward_json = json.dumps(
            {
                "enable_shaping": True,
                "damage_reward": float(cfg.damage_reward),
            }
        )

    with weiss_sim.fast(
        num_envs=cfg.num_envs,
        seed=cfg.seed,
        legal_repr="mask_u8",
        obs_dtype="i16",
        reward_json=reward_json,
    ) as sim:
        batch = sim.reset()
        obs_dim = int(batch.obs.shape[1])
        action_space = int(sim.action_space)

        learner = Net(obs_dim, action_space, cfg.hidden_dim)
        actor = copy.deepcopy(learner)
        optimizer = optim.Adam(learner.parameters(), lr=cfg.lr)

        for update in range(1, cfg.updates + 1):
            if update == 1 or (update - 1) % cfg.sync_every == 0:
                actor.load_state_dict(learner.state_dict())

            obs_buf = np.zeros((cfg.unroll_len + 1, cfg.num_envs, obs_dim), dtype=np.float32)
            mask_buf = np.zeros((cfg.unroll_len + 1, cfg.num_envs, action_space), dtype=np.uint8)
            act_buf = np.zeros((cfg.unroll_len, cfg.num_envs), dtype=np.int64)
            beh_logp_buf = np.zeros((cfg.unroll_len, cfg.num_envs), dtype=np.float32)
            rew_buf = np.zeros((cfg.unroll_len, cfg.num_envs), dtype=np.float32)
            done_buf = np.zeros((cfg.unroll_len, cfg.num_envs), dtype=np.bool_)

            for t in range(cfg.unroll_len):
                obs_buf[t] = batch.obs.astype(np.float32, copy=False)
                legal_mask = _coerce_legal_mask(
                    batch.legal.mask, num_envs=cfg.num_envs, action_space=action_space
                )
                mask_buf[t] = legal_mask

                obs_t = torch.from_numpy(obs_buf[t])
                logits_t, _ = actor(obs_t)
                dist, has_legal = _masked_dist(
                    logits_t, legal_mask, pass_action_id=int(weiss_sim.PASS_ACTION_ID)
                )
                actions_t = dist.sample()
                actions_t = torch.where(
                    has_legal,
                    actions_t,
                    torch.full_like(actions_t, int(weiss_sim.PASS_ACTION_ID)),
                )
                beh_logp_t = dist.log_prob(actions_t)
                beh_logp_t = torch.where(has_legal, beh_logp_t, torch.zeros_like(beh_logp_t))

                actions_np = actions_t.numpy().astype(np.uint32, copy=False)
                step, _, reset_batch = sim.step_auto(
                    actions_np, reset_done=True, reset_engine_errors=True
                )

                act_buf[t] = actions_t.numpy().astype(np.int64, copy=False)
                beh_logp_buf[t] = beh_logp_t.numpy().astype(np.float32, copy=False)
                rew_buf[t] = step.reward.astype(np.float32, copy=False)
                done_buf[t] = step.done.astype(np.bool_, copy=False)

                batch = reset_batch if reset_batch is not None else step

            obs_buf[cfg.unroll_len] = batch.obs.astype(np.float32, copy=False)
            mask_buf[cfg.unroll_len] = _coerce_legal_mask(
                batch.legal.mask, num_envs=cfg.num_envs, action_space=action_space
            )

            # Learner update.
            obs_t = torch.from_numpy(obs_buf.reshape((-1, obs_dim)))
            logits_all, values_all = learner(obs_t)
            logits_all = logits_all.reshape((cfg.unroll_len + 1, cfg.num_envs, action_space))
            values_all = values_all.reshape((cfg.unroll_len + 1, cfg.num_envs))

            actions = torch.from_numpy(act_buf).long()
            behavior_logp = torch.from_numpy(beh_logp_buf)
            rewards = torch.from_numpy(rew_buf)
            dones = torch.from_numpy(done_buf)

            target_logp = torch.empty_like(behavior_logp)
            entropy = torch.zeros((), dtype=torch.float32)

            for t in range(cfg.unroll_len):
                dist, has_legal = _masked_dist(
                    logits_all[t],
                    mask_buf[t],
                    pass_action_id=int(weiss_sim.PASS_ACTION_ID),
                )
                logp = dist.log_prob(actions[t])
                logp = torch.where(has_legal, logp, torch.zeros_like(logp))
                target_logp[t] = logp
                entropy = entropy + dist.entropy().mean()

            entropy = entropy / float(cfg.unroll_len)

            vs_targets, pg_adv = _vtrace_targets(
                rewards,
                dones,
                behavior_logp,
                target_logp.detach(),  # treat target logp as fixed for v-trace computation
                values_all,
                gamma=cfg.gamma,
                rho_bar=cfg.rho_bar,
                c_bar=cfg.c_bar,
            )

            policy_loss = -(pg_adv.detach() * target_logp).mean()
            value_loss = 0.5 * ((values_all[:-1] - vs_targets) ** 2).mean()
            loss = policy_loss + cfg.vf_coef * value_loss - cfg.ent_coef * entropy

            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(learner.parameters(), cfg.max_grad_norm)
            optimizer.step()

            with torch.no_grad():
                mean_reward = float(rewards.mean().item())
                done_rate = float(dones.float().mean().item())
                approx_kl = float((behavior_logp - target_logp.detach()).mean().item())
            print(
                f"update={update:04d} "
                f"mean_reward={mean_reward:+.4f} "
                f"done_rate={done_rate:.3f} "
                f"approx_kl={approx_kl:.4f} "
                f"policy_loss={float(policy_loss.item()):.4f} "
                f"value_loss={float(value_loss.item()):.4f}"
            )


if __name__ == "__main__":
    main()

