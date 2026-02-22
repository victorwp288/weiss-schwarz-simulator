use anyhow::Result;

use crate::encode::{ACTION_SPACE_SIZE, ACTION_SPACE_WORDS};

use super::super::core::EnvPool;

impl EnvPool {
    /// Fetch dense action masks for all envs.
    pub fn action_masks_batch(&self) -> Result<Vec<u8>> {
        let mut masks = vec![0u8; self.envs.len() * ACTION_SPACE_SIZE];
        self.action_masks_batch_into(&mut masks)?;
        Ok(masks)
    }

    /// Fill a provided buffer with dense action masks.
    pub fn action_masks_batch_into(&self, masks: &mut [u8]) -> Result<()> {
        if !self.output_mask_enabled {
            anyhow::bail!("action masks disabled (enable with set_output_mask_enabled)");
        }
        let num_envs = self.envs.len();
        if masks.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("mask buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let offset = i * ACTION_SPACE_SIZE;
            masks[offset..offset + ACTION_SPACE_SIZE].copy_from_slice(env.action_mask());
        }
        Ok(())
    }

    /// Fetch packed action mask bits for all envs.
    pub fn action_mask_bits_batch(&self) -> Vec<u64> {
        if !self.output_mask_bits_enabled {
            return Vec::new();
        }
        let mut bits = vec![0u64; self.envs.len() * ACTION_SPACE_WORDS];
        if let Err(err) = self.action_mask_bits_batch_into(&mut bits) {
            eprintln!("action_mask_bits_batch_into failed: {err}");
            return Vec::new();
        }
        bits
    }

    /// Fill a provided buffer with packed action mask bits.
    pub fn action_mask_bits_batch_into(&self, bits: &mut [u64]) -> Result<()> {
        if !self.output_mask_bits_enabled {
            anyhow::bail!("action mask bits disabled (enable with set_output_mask_bits_enabled)");
        }
        let expected = self.envs.len() * ACTION_SPACE_WORDS;
        if bits.len() != expected {
            anyhow::bail!("mask bits buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let base = i * ACTION_SPACE_WORDS;
            let slice = &mut bits[base..base + ACTION_SPACE_WORDS];
            slice.copy_from_slice(env.action_mask_bits());
        }
        Ok(())
    }
}
