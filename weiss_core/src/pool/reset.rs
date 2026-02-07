use std::sync::atomic::Ordering;

use anyhow::Result;
use rayon::prelude::*;

use super::core::EnvPool;
use super::outputs::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask,
};

impl EnvPool {
    /// Reset all envs and fill a minimal output batch (i32 obs + masks).
    pub fn reset_into(&mut self, out: &mut BatchOutMinimal<'_>) -> Result<()> {
        self.ensure_outcomes_scratch();
        let outcomes = if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .for_each(|(slot, env)| {
                        *slot = env.reset_no_copy();
                    });
            });
            &self.outcomes_scratch
        } else {
            for (slot, env) in self.outcomes_scratch.iter_mut().zip(self.envs.iter_mut()) {
                *slot = env.reset_no_copy();
            }
            &self.outcomes_scratch
        };
        self.fill_minimal_out(outcomes, out)
    }

    /// Reset all envs and fill a minimal output batch (i16 obs + masks).
    pub fn reset_into_i16(&mut self, out: &mut BatchOutMinimalI16<'_>) -> Result<()> {
        self.ensure_outcomes_scratch();
        let outcomes = if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .for_each(|(slot, env)| {
                        *slot = env.reset_no_copy();
                    });
            });
            &self.outcomes_scratch
        } else {
            for (slot, env) in self.outcomes_scratch.iter_mut().zip(self.envs.iter_mut()) {
                *slot = env.reset_no_copy();
            }
            &self.outcomes_scratch
        };
        self.fill_minimal_out_i16(outcomes, out)
    }

    /// Reset all envs and fill i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn reset_into_i16_legal_ids(
        &mut self,
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids output requires output masks disabled");
        }
        self.ensure_outcomes_scratch();
        let outcomes = if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .for_each(|(slot, env)| {
                        *slot = env.reset_no_copy();
                    });
            });
            &self.outcomes_scratch
        } else {
            for (slot, env) in self.outcomes_scratch.iter_mut().zip(self.envs.iter_mut()) {
                *slot = env.reset_no_copy();
            }
            &self.outcomes_scratch
        };
        self.fill_minimal_out_i16_legal_ids(outcomes, out)?;
        self.legal_action_ids_batch_into(out.legal_ids, out.legal_offsets)?;
        Ok(())
    }

    /// Reset all envs and fill a minimal output batch without masks.
    pub fn reset_into_nomask(&mut self, out: &mut BatchOutMinimalNoMask<'_>) -> Result<()> {
        self.ensure_outcomes_scratch();
        let outcomes = if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .for_each(|(slot, env)| {
                        *slot = env.reset_no_copy();
                    });
            });
            &self.outcomes_scratch
        } else {
            for (slot, env) in self.outcomes_scratch.iter_mut().zip(self.envs.iter_mut()) {
                *slot = env.reset_no_copy();
            }
            &self.outcomes_scratch
        };
        self.fill_minimal_out_nomask(outcomes, out)
    }

    /// Reset a subset of envs by index and fill minimal outputs.
    ///
    /// Returns Err if any index is out of bounds (>= num_envs).
    pub fn reset_indices_into(
        &mut self,
        indices: &[usize],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if self.reset_flags.len() != num_envs {
            self.reset_flags.resize(num_envs, false);
        }
        self.reset_flags.fill(false);
        for &idx in indices {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_flags[idx] = true;
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(
            &mut self.envs,
            &mut self.outcomes_scratch,
            &self.reset_flags,
        )?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs by index and fill i16 outputs.
    pub fn reset_indices_into_i16(
        &mut self,
        indices: &[usize],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if self.reset_flags.len() != num_envs {
            self.reset_flags.resize(num_envs, false);
        }
        self.reset_flags.fill(false);
        for &idx in indices {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_flags[idx] = true;
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(
            &mut self.envs,
            &mut self.outcomes_scratch,
            &self.reset_flags,
        )?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16(outcomes, out)
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs by index and fill i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn reset_indices_into_i16_legal_ids(
        &mut self,
        indices: &[usize],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids output requires output masks disabled");
        }
        let num_envs = self.envs.len();
        if self.reset_flags.len() != num_envs {
            self.reset_flags.resize(num_envs, false);
        }
        self.reset_flags.fill(false);
        for &idx in indices {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_flags[idx] = true;
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(
            &mut self.envs,
            &mut self.outcomes_scratch,
            &self.reset_flags,
        )?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16_legal_ids(outcomes, out)?;
        self.legal_action_ids_batch_into(out.legal_ids, out.legal_offsets)?;
        Ok(())
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs by index and fill outputs without masks.
    pub fn reset_indices_into_nomask(
        &mut self,
        indices: &[usize],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if self.reset_flags.len() != num_envs {
            self.reset_flags.resize(num_envs, false);
        }
        self.reset_flags.fill(false);
        for &idx in indices {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_flags[idx] = true;
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(
            &mut self.envs,
            &mut self.outcomes_scratch,
            &self.reset_flags,
        )?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
    }

    /// Reset envs where `done_mask` is true and fill minimal outputs.
    pub fn reset_done_into(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let len = done_mask.len();
        if len != num_envs {
            anyhow::bail!("done_mask length mismatch: {len} != num_envs={num_envs}");
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    /// Reset envs where `done_mask` is true and fill i16 outputs.
    pub fn reset_done_into_i16(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let len = done_mask.len();
        if len != num_envs {
            anyhow::bail!("done_mask length mismatch: {len} != num_envs={num_envs}");
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16(outcomes, out)
    }

    /// Reset envs where `done_mask` is true and fill i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn reset_done_into_i16_legal_ids(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let len = done_mask.len();
        if len != num_envs {
            anyhow::bail!("done_mask length mismatch: {len} != num_envs={num_envs}");
        }
        if self.output_mask_enabled {
            anyhow::bail!("legal ids output requires output masks disabled");
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16_legal_ids(outcomes, out)?;
        self.legal_action_ids_batch_into(out.legal_ids, out.legal_offsets)?;
        Ok(())
    }

    /// Reset envs where `done_mask` is true and fill outputs without masks.
    pub fn reset_done_into_nomask(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let len = done_mask.len();
        if len != num_envs {
            anyhow::bail!("done_mask length mismatch: {len} != num_envs={num_envs}");
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
    }

    /// Reset all envs and fill debug outputs.
    pub fn reset_debug_into(&mut self, out: &mut BatchOutDebug<'_>) -> Result<()> {
        self.reset_into(&mut out.minimal)?;
        let compute_fingerprints = self.debug_compute_fingerprints();
        let outcomes = &self.outcomes_scratch;
        self.fill_debug_out(outcomes, out, compute_fingerprints)
    }

    /// Reset a subset of envs by index and fill debug outputs.
    pub fn reset_indices_debug_into(
        &mut self,
        indices: &[usize],
        out: &mut BatchOutDebug<'_>,
    ) -> Result<()> {
        self.reset_indices_into(indices, &mut out.minimal)?;
        let compute_fingerprints = self.debug_compute_fingerprints();
        let outcomes = &self.outcomes_scratch;
        self.fill_debug_out(outcomes, out, compute_fingerprints)
    }

    /// Reset envs where `done_mask` is true and fill debug outputs.
    pub fn reset_done_debug_into(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutDebug<'_>,
    ) -> Result<()> {
        if done_mask.len() != self.envs.len() {
            anyhow::bail!("done mask batch size mismatch");
        }
        self.reset_done_into(done_mask, &mut out.minimal)?;
        let compute_fingerprints = self.debug_compute_fingerprints();
        let outcomes = &self.outcomes_scratch;
        self.fill_debug_out(outcomes, out, compute_fingerprints)
    }

    /// Clear the engine error reset counter.
    pub fn reset_engine_error_reset_count(&mut self) {
        self.engine_error_reset_count = 0;
    }

    /// Auto-reset envs with non-zero error codes and fill minimal outputs.
    pub fn auto_reset_on_error_codes_into(
        &mut self,
        codes: &[u8],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<usize> {
        if codes.len() != self.envs.len() {
            anyhow::bail!("Error code batch size mismatch");
        }
        let num_envs = self.envs.len();
        if self.reset_flags.len() != num_envs {
            self.reset_flags.resize(num_envs, false);
        }
        let mut reset_count = 0usize;
        for (flag, &code) in self.reset_flags.iter_mut().zip(codes.iter()) {
            *flag = code != 0;
            if *flag {
                reset_count += 1;
            }
        }
        if reset_count == 0 {
            return Ok(0);
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(
            &mut self.envs,
            &mut self.outcomes_scratch,
            &self.reset_flags,
        )?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)?;
        self.engine_error_reset_count = self
            .engine_error_reset_count
            .saturating_add(reset_count as u64);
        Ok(reset_count)
    }

    /// Auto-reset envs with non-zero error codes and fill outputs without masks.
    pub fn auto_reset_on_error_codes_into_nomask(
        &mut self,
        codes: &[u8],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<usize> {
        if codes.len() != self.envs.len() {
            anyhow::bail!("Error code batch size mismatch");
        }
        let num_envs = self.envs.len();
        if self.reset_flags.len() != num_envs {
            self.reset_flags.resize(num_envs, false);
        }
        let mut reset_count = 0usize;
        for (flag, &code) in self.reset_flags.iter_mut().zip(codes.iter()) {
            *flag = code != 0;
            if *flag {
                reset_count += 1;
            }
        }
        if reset_count == 0 {
            return Ok(0);
        }
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(
            &mut self.envs,
            &mut self.outcomes_scratch,
            &self.reset_flags,
        )?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)?;
        self.engine_error_reset_count = self
            .engine_error_reset_count
            .saturating_add(reset_count as u64);
        Ok(reset_count)
    }

    /// Clear the i16 overflow counter.
    pub fn reset_i16_overflow_count(&self) {
        self.i16_overflow_count.store(0, Ordering::Relaxed);
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs with explicit episode seeds and fill minimal outputs.
    pub fn reset_indices_with_episode_seeds_into(
        &mut self,
        indices: &[usize],
        episode_seeds: &[u64],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        if indices.len() != episode_seeds.len() {
            anyhow::bail!("indices and episode_seeds length mismatch");
        }
        self.ensure_outcomes_scratch();
        let num_envs = self.envs.len();
        if self.reset_seed_scratch.len() != num_envs {
            self.reset_seed_scratch.resize(num_envs, None);
        }
        self.reset_seed_scratch.fill(None);
        for (&idx, &seed) in indices.iter().zip(episode_seeds.iter()) {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_seed_scratch[idx] = Some(seed);
        }
        for ((slot, env), seed_opt) in self
            .outcomes_scratch
            .iter_mut()
            .zip(self.envs.iter_mut())
            .zip(self.reset_seed_scratch.iter().copied())
        {
            *slot = if let Some(seed) = seed_opt {
                env.reset_with_episode_seed_no_copy(seed)
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
        }
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs with explicit episode seeds and fill i16 outputs.
    pub fn reset_indices_with_episode_seeds_into_i16(
        &mut self,
        indices: &[usize],
        episode_seeds: &[u64],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        if indices.len() != episode_seeds.len() {
            anyhow::bail!("indices and episode_seeds length mismatch");
        }
        self.ensure_outcomes_scratch();
        let num_envs = self.envs.len();
        if self.reset_seed_scratch.len() != num_envs {
            self.reset_seed_scratch.resize(num_envs, None);
        }
        self.reset_seed_scratch.fill(None);
        for (&idx, &seed) in indices.iter().zip(episode_seeds.iter()) {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_seed_scratch[idx] = Some(seed);
        }
        for ((slot, env), seed_opt) in self
            .outcomes_scratch
            .iter_mut()
            .zip(self.envs.iter_mut())
            .zip(self.reset_seed_scratch.iter().copied())
        {
            *slot = if let Some(seed) = seed_opt {
                env.reset_with_episode_seed_no_copy(seed)
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
        }
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16(outcomes, out)
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs with explicit episode seeds and fill i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn reset_indices_with_episode_seeds_into_i16_legal_ids(
        &mut self,
        indices: &[usize],
        episode_seeds: &[u64],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids output requires output masks disabled");
        }
        if indices.len() != episode_seeds.len() {
            anyhow::bail!("indices and episode_seeds length mismatch");
        }
        self.ensure_outcomes_scratch();
        let num_envs = self.envs.len();
        if self.reset_seed_scratch.len() != num_envs {
            self.reset_seed_scratch.resize(num_envs, None);
        }
        self.reset_seed_scratch.fill(None);
        for (&idx, &seed) in indices.iter().zip(episode_seeds.iter()) {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_seed_scratch[idx] = Some(seed);
        }
        for ((slot, env), seed_opt) in self
            .outcomes_scratch
            .iter_mut()
            .zip(self.envs.iter_mut())
            .zip(self.reset_seed_scratch.iter().copied())
        {
            *slot = if let Some(seed) = seed_opt {
                env.reset_with_episode_seed_no_copy(seed)
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
        }
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16_legal_ids(outcomes, out)?;
        self.legal_action_ids_batch_into(out.legal_ids, out.legal_offsets)?;
        Ok(())
    }

    /// Returns Err if any index is out of bounds (>= num_envs).
    /// Reset a subset of envs with explicit episode seeds and fill outputs without masks.
    pub fn reset_indices_with_episode_seeds_into_nomask(
        &mut self,
        indices: &[usize],
        episode_seeds: &[u64],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        if indices.len() != episode_seeds.len() {
            anyhow::bail!("indices and episode_seeds length mismatch");
        }
        self.ensure_outcomes_scratch();
        let num_envs = self.envs.len();
        if self.reset_seed_scratch.len() != num_envs {
            self.reset_seed_scratch.resize(num_envs, None);
        }
        self.reset_seed_scratch.fill(None);
        for (&idx, &seed) in indices.iter().zip(episode_seeds.iter()) {
            if idx >= num_envs {
                anyhow::bail!("reset index out of bounds: {idx} (num_envs={num_envs})");
            }
            self.reset_seed_scratch[idx] = Some(seed);
        }
        for ((slot, env), seed_opt) in self
            .outcomes_scratch
            .iter_mut()
            .zip(self.envs.iter_mut())
            .zip(self.reset_seed_scratch.iter().copied())
        {
            *slot = if let Some(seed) = seed_opt {
                env.reset_with_episode_seed_no_copy(seed)
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
        }
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
    }
}
