use anyhow::Result;

use super::EnvPool;
use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN};
use crate::pool::outputs::{
    BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds, BatchOutMinimalNoMask,
    BatchOutTrajectory, BatchOutTrajectoryI16, BatchOutTrajectoryI16LegalIds,
    BatchOutTrajectoryNoMask,
};

impl EnvPool {
    /// Roll out a trajectory using first legal actions.
    pub fn rollout_first_legal_into(
        &mut self,
        steps: usize,
        out: &mut BatchOutTrajectory<'_>,
    ) -> Result<()> {
        self.validate_trajectory(out, steps)?;
        let num_envs = self.envs.len();
        for t in 0..steps {
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.first_legal_action_ids_into(action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mask_offset = t * num_envs * ACTION_SPACE_SIZE;
            let mut out_min = BatchOutMinimal {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                masks: &mut out.masks[mask_offset..mask_offset + num_envs * ACTION_SPACE_SIZE],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into(action_slice, &mut out_min)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using first legal actions (i16 outputs).
    pub fn rollout_first_legal_into_i16(
        &mut self,
        steps: usize,
        out: &mut BatchOutTrajectoryI16<'_>,
    ) -> Result<()> {
        self.validate_trajectory_i16(out, steps)?;
        let num_envs = self.envs.len();
        for t in 0..steps {
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.first_legal_action_ids_into(action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mask_offset = t * num_envs * ACTION_SPACE_SIZE;
            let mut out_min = BatchOutMinimalI16 {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                masks: &mut out.masks[mask_offset..mask_offset + num_envs * ACTION_SPACE_SIZE],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into_i16(action_slice, &mut out_min)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using first legal actions (i16 + legal ids).
    ///
    /// Requires output masks to be disabled.
    pub fn rollout_first_legal_into_i16_legal_ids(
        &mut self,
        steps: usize,
        out: &mut BatchOutTrajectoryI16LegalIds<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids trajectory requires output masks disabled");
        }
        self.validate_trajectory_i16_legal_ids(out, steps)?;
        let num_envs = self.envs.len();
        for t in 0..steps {
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.first_legal_action_ids_into(action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mut out_min = BatchOutMinimalI16 {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                masks: &mut [],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into_i16(action_slice, &mut out_min)?;
            for (dst, env) in out.episode_seed[t * num_envs..(t + 1) * num_envs]
                .iter_mut()
                .zip(self.envs.iter())
            {
                *dst = env.episode_seed;
            }
            let ids_offset = t * num_envs * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let ids_slice =
                &mut out.legal_ids[ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE];
            let meta_slice = &mut out.legal_action_meta[ids_offset
                * crate::encode::ACTION_META_WIDTH
                ..(ids_offset + num_envs * ACTION_SPACE_SIZE) * crate::encode::ACTION_META_WIDTH];
            let offsets_slice =
                &mut out.legal_offsets[offsets_offset..offsets_offset + num_envs + 1];
            self.legal_action_ids_batch_into(ids_slice, offsets_slice)?;
            self.legal_action_meta_batch_into(meta_slice)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using heuristic-public actions with internal auto-reset.
    ///
    /// This transition-oriented helper is specialized for RL collection: `obs`,
    /// `legal_ids`, `legal_action_meta`, `legal_offsets`, `actor`,
    /// `decision_kind`, and `decision_id` describe the pre-action state at each
    /// step, while rewards/terminal flags/engine status/main-action flags come
    /// from the post-action transition. `episode_seed` carries the per-step
    /// episode seed for this specialized transport, while `spec_hash` remains
    /// the simulator compatibility hash.
    ///
    /// Requires output masks to be disabled.
    pub fn rollout_heuristic_public_into_i16_legal_ids(
        &mut self,
        steps: usize,
        out: &mut BatchOutTrajectoryI16LegalIds<'_>,
    ) -> Result<()> {
        self.rollout_heuristic_public_profile_into_i16_legal_ids(steps, out, "base")
    }

    /// Roll out a trajectory using a named heuristic-public profile with internal auto-reset.
    ///
    /// Profile names match the Python heuristic surface: `base`, `aggressive`, or `control`.
    /// Requires output masks to be disabled.
    pub fn rollout_heuristic_public_profile_into_i16_legal_ids(
        &mut self,
        steps: usize,
        out: &mut BatchOutTrajectoryI16LegalIds<'_>,
        profile_name: &str,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids trajectory requires output masks disabled");
        }
        self.validate_trajectory_i16_legal_ids(out, steps)?;
        let num_envs = self.envs.len();
        if num_envs == 0 {
            return Ok(());
        }

        let keep_flags = vec![false; num_envs];
        let env_indices: Vec<usize> = (0..num_envs).collect();
        let mut chosen_actions = vec![0u16; num_envs];
        let mut done_flags = vec![false; num_envs];

        for t in 0..steps {
            self.fill_outcomes_for_flags(&keep_flags)?;

            let step_offset = t * num_envs;
            let obs_offset = step_offset * OBS_LEN;
            let ids_offset = step_offset * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let meta_offset = ids_offset * crate::encode::ACTION_META_WIDTH;

            let mut pre_step = BatchOutMinimalI16LegalIds {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                legal_ids: &mut out.legal_ids
                    [ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE],
                legal_action_meta: &mut out.legal_action_meta[meta_offset
                    ..meta_offset
                        + num_envs * ACTION_SPACE_SIZE * crate::encode::ACTION_META_WIDTH],
                legal_offsets: &mut out.legal_offsets
                    [offsets_offset..offsets_offset + num_envs + 1],
                rewards: &mut out.rewards[step_offset..step_offset + num_envs],
                terminated: &mut out.terminated[step_offset..step_offset + num_envs],
                truncated: &mut out.truncated[step_offset..step_offset + num_envs],
                actor: &mut out.actor[step_offset..step_offset + num_envs],
                decision_kind: &mut out.decision_kind[step_offset..step_offset + num_envs],
                decision_id: &mut out.decision_id[step_offset..step_offset + num_envs],
                engine_status: &mut out.engine_status[step_offset..step_offset + num_envs],
                spec_hash: &mut out.spec_hash[step_offset..step_offset + num_envs],
                main_move_action: &mut out.main_move_action[step_offset..step_offset + num_envs],
                main_pass_action: &mut out.main_pass_action[step_offset..step_offset + num_envs],
            };
            let outcomes = &self.outcomes_scratch;
            self.fill_minimal_out_i16_legal_ids(outcomes, &mut pre_step)?;
            for (dst, env) in out.episode_seed[step_offset..step_offset + num_envs]
                .iter_mut()
                .zip(self.envs.iter())
            {
                *dst = env.episode_seed;
            }

            self.choose_heuristic_public_profile_actions_into(
                &env_indices,
                &mut chosen_actions,
                profile_name,
            )?;
            let action_slice = &mut out.actions[step_offset..step_offset + num_envs];
            for (dst, &action_id) in action_slice.iter_mut().zip(chosen_actions.iter()) {
                *dst = u32::from(action_id);
            }

            self.step_batch_transition_outcomes_without_obs_encode(action_slice)?;
            let outcomes = &self.outcomes_scratch;
            let reward_slice = &mut out.rewards[step_offset..step_offset + num_envs];
            let terminated_slice = &mut out.terminated[step_offset..step_offset + num_envs];
            let truncated_slice = &mut out.truncated[step_offset..step_offset + num_envs];
            let engine_status_slice = &mut out.engine_status[step_offset..step_offset + num_envs];
            let main_move_slice = &mut out.main_move_action[step_offset..step_offset + num_envs];
            let main_pass_slice = &mut out.main_pass_action[step_offset..step_offset + num_envs];
            for (env_index, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
                reward_slice[env_index] = outcome.reward;
                terminated_slice[env_index] = outcome.terminated;
                truncated_slice[env_index] = outcome.truncated;
                engine_status_slice[env_index] = if outcome.info.engine_error {
                    outcome.info.engine_error_code
                } else {
                    env.last_engine_error_code as u8
                };
                let (main_move_action, main_pass_action) = env.last_action_main_flags();
                main_move_slice[env_index] = main_move_action;
                main_pass_slice[env_index] = main_pass_action;
                done_flags[env_index] = outcome.terminated || outcome.truncated;
            }

            if done_flags.iter().any(|&done| done) {
                self.fill_outcomes_for_flags(&done_flags)?;
            }
        }
        Ok(())
    }

    /// Roll out a trajectory using first legal actions (no masks).
    pub fn rollout_first_legal_into_nomask(
        &mut self,
        steps: usize,
        out: &mut BatchOutTrajectoryNoMask<'_>,
    ) -> Result<()> {
        self.validate_trajectory_nomask(out, steps)?;
        let num_envs = self.envs.len();
        for t in 0..steps {
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.first_legal_action_ids_into(action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mut out_min = BatchOutMinimalNoMask {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into_nomask(action_slice, &mut out_min)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using uniformly sampled legal actions.
    pub fn rollout_sample_legal_action_ids_uniform_into(
        &mut self,
        steps: usize,
        seeds: &[u64],
        out: &mut BatchOutTrajectory<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if seeds.len() != steps * num_envs {
            anyhow::bail!("seed buffer size mismatch");
        }
        self.validate_trajectory(out, steps)?;
        for t in 0..steps {
            let seed_slice = &seeds[t * num_envs..(t + 1) * num_envs];
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.sample_legal_action_ids_uniform_into(seed_slice, action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mask_offset = t * num_envs * ACTION_SPACE_SIZE;
            let mut out_min = BatchOutMinimal {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                masks: &mut out.masks[mask_offset..mask_offset + num_envs * ACTION_SPACE_SIZE],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into(action_slice, &mut out_min)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using uniformly sampled legal actions (i16 outputs).
    pub fn rollout_sample_legal_action_ids_uniform_into_i16(
        &mut self,
        steps: usize,
        seeds: &[u64],
        out: &mut BatchOutTrajectoryI16<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if seeds.len() != steps * num_envs {
            anyhow::bail!("seed buffer size mismatch");
        }
        self.validate_trajectory_i16(out, steps)?;
        for t in 0..steps {
            let seed_slice = &seeds[t * num_envs..(t + 1) * num_envs];
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.sample_legal_action_ids_uniform_into(seed_slice, action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mask_offset = t * num_envs * ACTION_SPACE_SIZE;
            let mut out_min = BatchOutMinimalI16 {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                masks: &mut out.masks[mask_offset..mask_offset + num_envs * ACTION_SPACE_SIZE],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into_i16(action_slice, &mut out_min)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using uniformly sampled legal actions (i16 + legal ids).
    ///
    /// Requires output masks to be disabled.
    pub fn rollout_sample_legal_action_ids_uniform_into_i16_legal_ids(
        &mut self,
        steps: usize,
        seeds: &[u64],
        out: &mut BatchOutTrajectoryI16LegalIds<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids trajectory requires output masks disabled");
        }
        let num_envs = self.envs.len();
        if seeds.len() != steps * num_envs {
            anyhow::bail!("seed buffer size mismatch");
        }
        self.validate_trajectory_i16_legal_ids(out, steps)?;
        for t in 0..steps {
            let seed_slice = &seeds[t * num_envs..(t + 1) * num_envs];
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.sample_legal_action_ids_uniform_into(seed_slice, action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mut out_min = BatchOutMinimalI16 {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                masks: &mut [],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into_i16(action_slice, &mut out_min)?;
            for (dst, env) in out.episode_seed[t * num_envs..(t + 1) * num_envs]
                .iter_mut()
                .zip(self.envs.iter())
            {
                *dst = env.episode_seed;
            }
            let ids_offset = t * num_envs * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let ids_slice =
                &mut out.legal_ids[ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE];
            let meta_slice = &mut out.legal_action_meta[ids_offset
                * crate::encode::ACTION_META_WIDTH
                ..(ids_offset + num_envs * ACTION_SPACE_SIZE) * crate::encode::ACTION_META_WIDTH];
            let offsets_slice =
                &mut out.legal_offsets[offsets_offset..offsets_offset + num_envs + 1];
            self.legal_action_ids_batch_into(ids_slice, offsets_slice)?;
            self.legal_action_meta_batch_into(meta_slice)?;
        }
        Ok(())
    }

    /// Roll out a trajectory using uniformly sampled legal actions (no masks).
    pub fn rollout_sample_legal_action_ids_uniform_into_nomask(
        &mut self,
        steps: usize,
        seeds: &[u64],
        out: &mut BatchOutTrajectoryNoMask<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if seeds.len() != steps * num_envs {
            anyhow::bail!("seed buffer size mismatch");
        }
        self.validate_trajectory_nomask(out, steps)?;
        for t in 0..steps {
            let seed_slice = &seeds[t * num_envs..(t + 1) * num_envs];
            let action_slice = &mut out.actions[t * num_envs..(t + 1) * num_envs];
            self.sample_legal_action_ids_uniform_into(seed_slice, action_slice)?;
            let obs_offset = t * num_envs * OBS_LEN;
            let mut out_min = BatchOutMinimalNoMask {
                obs: &mut out.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                rewards: &mut out.rewards[t * num_envs..(t + 1) * num_envs],
                terminated: &mut out.terminated[t * num_envs..(t + 1) * num_envs],
                truncated: &mut out.truncated[t * num_envs..(t + 1) * num_envs],
                actor: &mut out.actor[t * num_envs..(t + 1) * num_envs],
                decision_kind: &mut out.decision_kind[t * num_envs..(t + 1) * num_envs],
                decision_id: &mut out.decision_id[t * num_envs..(t + 1) * num_envs],
                engine_status: &mut out.engine_status[t * num_envs..(t + 1) * num_envs],
                spec_hash: &mut out.spec_hash[t * num_envs..(t + 1) * num_envs],
                main_move_action: &mut out.main_move_action[t * num_envs..(t + 1) * num_envs],
                main_pass_action: &mut out.main_pass_action[t * num_envs..(t + 1) * num_envs],
            };
            self.step_into_nomask(action_slice, &mut out_min)?;
        }
        Ok(())
    }
}
