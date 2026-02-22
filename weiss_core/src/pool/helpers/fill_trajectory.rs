use anyhow::Result;

use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN};

use super::super::core::EnvPool;
use super::super::outputs::{
    BatchOutTrajectory, BatchOutTrajectoryI16, BatchOutTrajectoryI16LegalIds,
    BatchOutTrajectoryNoMask,
};

impl EnvPool {
    pub(in crate::pool) fn validate_trajectory(
        &self,
        out: &BatchOutTrajectory<'_>,
        steps: usize,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let Some(total) = steps.checked_mul(num_envs) else {
            anyhow::bail!("trajectory size overflow (steps * num_envs)");
        };
        let Some(obs_total) = total.checked_mul(OBS_LEN) else {
            anyhow::bail!("trajectory size overflow (total * OBS_LEN)");
        };
        let Some(mask_total) = total.checked_mul(ACTION_SPACE_SIZE) else {
            anyhow::bail!("trajectory size overflow (total * ACTION_SPACE_SIZE)");
        };
        Self::validate_obs_len(out.obs.len(), obs_total)?;
        Self::validate_mask_len(out.masks.len(), mask_total)?;
        Self::validate_action_len(out.actions.len(), total)?;
        Self::validate_scalar_lens(
            total,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
            ],
        )?;
        Ok(())
    }

    pub(in crate::pool) fn validate_trajectory_i16(
        &self,
        out: &BatchOutTrajectoryI16<'_>,
        steps: usize,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let Some(total) = steps.checked_mul(num_envs) else {
            anyhow::bail!("trajectory size overflow (steps * num_envs)");
        };
        let Some(obs_total) = total.checked_mul(OBS_LEN) else {
            anyhow::bail!("trajectory size overflow (total * OBS_LEN)");
        };
        let Some(mask_total) = total.checked_mul(ACTION_SPACE_SIZE) else {
            anyhow::bail!("trajectory size overflow (total * ACTION_SPACE_SIZE)");
        };
        Self::validate_obs_len(out.obs.len(), obs_total)?;
        Self::validate_mask_len(out.masks.len(), mask_total)?;
        Self::validate_action_len(out.actions.len(), total)?;
        Self::validate_scalar_lens(
            total,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
            ],
        )?;
        Ok(())
    }

    pub(in crate::pool) fn validate_trajectory_i16_legal_ids(
        &self,
        out: &BatchOutTrajectoryI16LegalIds<'_>,
        steps: usize,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let Some(total) = steps.checked_mul(num_envs) else {
            anyhow::bail!("trajectory size overflow (steps * num_envs)");
        };
        let Some(obs_total) = total.checked_mul(OBS_LEN) else {
            anyhow::bail!("trajectory size overflow (total * OBS_LEN)");
        };
        let Some(ids_total) = total.checked_mul(ACTION_SPACE_SIZE) else {
            anyhow::bail!("trajectory size overflow (total * ACTION_SPACE_SIZE)");
        };
        let Some(offsets_per_step) = num_envs.checked_add(1) else {
            anyhow::bail!("trajectory size overflow (num_envs + 1)");
        };
        let Some(offsets_total) = steps.checked_mul(offsets_per_step) else {
            anyhow::bail!("trajectory size overflow (steps * (num_envs + 1))");
        };
        Self::validate_obs_len(out.obs.len(), obs_total)?;
        Self::validate_legal_ids_len(out.legal_ids.len(), ids_total)?;
        Self::validate_legal_offsets_len(out.legal_offsets.len(), offsets_total)?;
        Self::validate_action_len(out.actions.len(), total)?;
        Self::validate_scalar_lens(
            total,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
            ],
        )?;
        Ok(())
    }

    pub(in crate::pool) fn validate_trajectory_nomask(
        &self,
        out: &BatchOutTrajectoryNoMask<'_>,
        steps: usize,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let Some(total) = steps.checked_mul(num_envs) else {
            anyhow::bail!("trajectory size overflow (steps * num_envs)");
        };
        let Some(obs_total) = total.checked_mul(OBS_LEN) else {
            anyhow::bail!("trajectory size overflow (total * OBS_LEN)");
        };
        Self::validate_obs_len(out.obs.len(), obs_total)?;
        Self::validate_action_len(out.actions.len(), total)?;
        Self::validate_scalar_lens(
            total,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
            ],
        )?;
        Ok(())
    }
}
