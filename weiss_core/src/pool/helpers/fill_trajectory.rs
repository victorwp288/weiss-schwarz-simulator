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
        let total = steps * num_envs;
        Self::validate_obs_len(out.obs.len(), total * OBS_LEN)?;
        Self::validate_mask_len(out.masks.len(), total * ACTION_SPACE_SIZE)?;
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
        let total = steps * num_envs;
        Self::validate_obs_len(out.obs.len(), total * OBS_LEN)?;
        Self::validate_mask_len(out.masks.len(), total * ACTION_SPACE_SIZE)?;
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
        let total = steps * num_envs;
        Self::validate_obs_len(out.obs.len(), total * OBS_LEN)?;
        Self::validate_legal_ids_len(out.legal_ids.len(), total * ACTION_SPACE_SIZE)?;
        Self::validate_legal_offsets_len(out.legal_offsets.len(), steps * (num_envs + 1))?;
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
        let total = steps * num_envs;
        Self::validate_obs_len(out.obs.len(), total * OBS_LEN)?;
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
