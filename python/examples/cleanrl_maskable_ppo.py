import argparse
from pathlib import Path

import numpy as np

try:
    import torch
    import torch.nn as nn
    import torch.optim as optim
except ImportError as exc:
    raise SystemExit("torch is required for this example") from exc

import weiss_sim


class Policy(nn.Module):
    def __init__(self, obs_dim: int, act_dim: int):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(obs_dim, 256),
            nn.ReLU(),
            nn.Linear(256, 256),
            nn.ReLU(),
        )
        self.policy_head = nn.Linear(256, act_dim)
        self.value_head = nn.Linear(256, 1)

    def forward(self, x):
        h = self.net(x)
        return self.policy_head(h), self.value_head(h)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-envs", type=int, default=8)
    parser.add_argument("--steps", type=int, default=256)
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
    db_path = fixture_dir / "cards.wsdb"
    legal_deck = (list(range(1, 14)) * 4)[:50]

    pool = weiss_sim.EnvPool.new_rl_train(
        args.num_envs,
        str(db_path),
        deck_lists=[legal_deck, legal_deck],
        deck_ids=[1, 2],
        seed=args.seed,
    )
    buffers = weiss_sim.EnvPoolBuffers(pool)
    out = buffers.reset()

    device = torch.device("cpu")
    policy = Policy(pool.obs_len, pool.action_space).to(device)
    optimizer = optim.Adam(policy.parameters(), lr=2.5e-4)

    for _ in range(args.steps):
        obs_t = torch.from_numpy(out.obs.astype(np.float32, copy=False)).to(device)
        logits, values = policy(obs_t)
        mask_t = torch.from_numpy(out.masks.astype(bool, copy=False)).to(device)
        logits = logits.masked_fill(~mask_t, -1e9)
        dist = torch.distributions.Categorical(logits=logits)
        actions = dist.sample().cpu().numpy()

        out = buffers.step(actions)

        # Keep the example executable without bundling a full PPO objective.
        rewards_t = torch.from_numpy(out.rewards.astype(np.float32, copy=False)).to(device)
        loss = -(rewards_t.mean()) + 0.01 * (values.mean() ** 2)
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        if bool(np.any(out.terminated)) or bool(np.any(out.truncated)):
            out = buffers.reset()

    print("done", "last_loss", float(loss.item()))


if __name__ == "__main__":
    main()
