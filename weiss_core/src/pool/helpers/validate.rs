use anyhow::Result;

use crate::encode::{ACTION_META_WIDTH, ACTION_SPACE_SIZE, OBS_LEN};

use super::super::core::EnvPool;
use super::super::outputs::{
    BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds, BatchOutMinimalNoMask,
};

impl EnvPool {
    pub(super) fn validate_obs_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("obs buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_mask_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("mask buffer size mismatch");
        }
        Ok(())
    }

    fn validate_optional_mask_len(&self, actual: usize, expected: usize) -> Result<()> {
        if self.output_mask_enabled {
            Self::validate_mask_len(actual, expected)?;
        } else if actual != 0 {
            anyhow::bail!("mask buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_legal_ids_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("legal ids buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_legal_offsets_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("legal offsets buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_legal_action_meta_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("legal action meta buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_action_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("action buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_scalar_lens<const N: usize>(
        expected: usize,
        scalar_lens: [usize; N],
    ) -> Result<()> {
        if scalar_lens.into_iter().any(|len| len != expected) {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    pub(super) fn validate_minimal_out(&self, out: &BatchOutMinimal<'_>) -> Result<()> {
        let num_envs = self.envs.len();
        Self::validate_obs_len(out.obs.len(), num_envs * OBS_LEN)?;
        Self::validate_mask_len(out.masks.len(), num_envs * ACTION_SPACE_SIZE)?;
        Self::validate_scalar_lens(
            num_envs,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
                out.main_move_action.len(),
                out.main_pass_action.len(),
            ],
        )?;
        Ok(())
    }

    pub(super) fn validate_minimal_out_i16(&self, out: &BatchOutMinimalI16<'_>) -> Result<()> {
        let num_envs = self.envs.len();
        Self::validate_obs_len(out.obs.len(), num_envs * OBS_LEN)?;
        self.validate_optional_mask_len(out.masks.len(), num_envs * ACTION_SPACE_SIZE)?;
        Self::validate_scalar_lens(
            num_envs,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
                out.main_move_action.len(),
                out.main_pass_action.len(),
            ],
        )?;
        Ok(())
    }

    pub(super) fn validate_minimal_out_i16_legal_ids(
        &self,
        out: &BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        Self::validate_obs_len(out.obs.len(), num_envs * OBS_LEN)?;
        Self::validate_legal_ids_len(out.legal_ids.len(), num_envs * ACTION_SPACE_SIZE)?;
        Self::validate_legal_action_meta_len(
            out.legal_action_meta.len(),
            num_envs * ACTION_SPACE_SIZE * ACTION_META_WIDTH,
        )?;
        Self::validate_legal_offsets_len(out.legal_offsets.len(), num_envs + 1)?;
        Self::validate_scalar_lens(
            num_envs,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
                out.main_move_action.len(),
                out.main_pass_action.len(),
            ],
        )?;
        Ok(())
    }

    pub(super) fn validate_minimal_out_nomask(
        &self,
        out: &BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        Self::validate_obs_len(out.obs.len(), num_envs * OBS_LEN)?;
        Self::validate_scalar_lens(
            num_envs,
            [
                out.rewards.len(),
                out.terminated.len(),
                out.truncated.len(),
                out.actor.len(),
                out.decision_kind.len(),
                out.decision_id.len(),
                out.engine_status.len(),
                out.spec_hash.len(),
                out.main_move_action.len(),
                out.main_pass_action.len(),
            ],
        )?;
        Ok(())
    }
}
