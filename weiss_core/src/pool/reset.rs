use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;

use anyhow::Result;
use rayon::prelude::*;

use super::core::EnvPool;
use super::outputs::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask,
};
use crate::env::{EngineErrorCode, EnvInfo, FaultSource, GameEnv, StepOutcome};

#[derive(Clone)]
struct ResetSlotTemplate {
    template_db: std::sync::Arc<crate::db::CardDb>,
    template_config: crate::config::EnvConfig,
    template_curriculum: crate::config::CurriculumConfig,
    template_replay_config: crate::replay::ReplayConfig,
    template_replay_writer: Option<crate::replay::ReplayWriter>,
    debug_config: crate::env::DebugConfig,
    output_mask_enabled: bool,
    output_mask_bits_enabled: bool,
    error_policy: crate::config::ErrorPolicy,
    pool_seed: u64,
}

#[cold]
#[inline(never)]
fn fallback_reset_panic_outcome(reward: f32) -> StepOutcome {
    StepOutcome {
        obs: vec![0; crate::encode::OBS_LEN],
        reward,
        terminated: false,
        truncated: true,
        info: EnvInfo {
            obs_version: crate::encode::OBS_ENCODING_VERSION,
            action_version: crate::encode::ACTION_ENCODING_VERSION,
            decision_kind: crate::encode::DECISION_KIND_NONE,
            current_player: -1,
            actor: crate::encode::ACTOR_NONE,
            decision_count: 0,
            tick_count: 0,
            terminal: Some(crate::state::TerminalResult::Timeout),
            illegal_action: false,
            engine_error: true,
            engine_error_code: EngineErrorCode::ResetPanic as u8,
        },
    }
}

#[cold]
#[inline(never)]
fn latch_fallback_reset_fault(
    env: &mut GameEnv,
    env_id: u32,
    episode_index: u32,
    episode_seed: u64,
    decision_id: u32,
) {
    let fingerprint = EnvPool::panic_fingerprint_from_meta(
        env_id,
        episode_index,
        episode_seed,
        decision_id,
        EngineErrorCode::ResetPanic,
    );
    env.last_engine_error = true;
    env.last_engine_error_code = EngineErrorCode::ResetPanic;
    env.fault_latched = Some(crate::env::FaultRecord {
        code: EngineErrorCode::ResetPanic,
        actor: None,
        fingerprint,
        source: FaultSource::Reset,
        reward_emitted: true,
    });
    env.state.terminal = Some(crate::state::TerminalResult::Timeout);
    env.decision = None;
    env.action_cache.clear();
}

impl EnvPool {
    fn reset_slot_no_copy(
        idx: usize,
        env: &mut GameEnv,
        should_reset: bool,
        episode_seed: Option<u64>,
        template: &ResetSlotTemplate,
    ) -> crate::env::StepOutcome {
        let mut meta_episode_index = 0u32;
        let mut meta_episode_seed = 0u64;
        let mut meta_decision_id = 0u32;
        let result = catch_unwind(AssertUnwindSafe(|| {
            meta_episode_index = env.episode_index;
            meta_episode_seed = env.episode_seed;
            meta_decision_id = env.decision_id();
            if should_reset {
                if let Some(seed) = episode_seed {
                    env.reset_with_episode_seed_no_copy(seed)
                } else {
                    env.reset_no_copy()
                }
            } else if env.is_fault_latched() {
                env.build_fault_step_outcome_no_copy()
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            }
        }));
        match result {
            Ok(outcome) => outcome,
            Err(_) => {
                let recover = catch_unwind(AssertUnwindSafe(|| {
                    let rebuilt = GameEnv::new(
                        template.template_db.clone(),
                        template.template_config.clone(),
                        template.template_curriculum.clone(),
                        template.pool_seed ^ (idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
                        template.template_replay_config.clone(),
                        template.template_replay_writer.clone(),
                        idx as u32,
                    );
                    if let Ok(mut fresh) = rebuilt {
                        fresh.set_debug_config(template.debug_config);
                        fresh.set_output_mask_enabled(template.output_mask_enabled);
                        fresh.set_output_mask_bits_enabled(template.output_mask_bits_enabled);
                        fresh.config.error_policy = template.error_policy;
                        *env = fresh;
                        let out = env.latch_fault(
                            EngineErrorCode::ResetPanic,
                            None,
                            FaultSource::Reset,
                            false,
                        );
                        let fingerprint = Self::panic_fingerprint_from_meta(
                            idx as u32,
                            meta_episode_index,
                            meta_episode_seed,
                            meta_decision_id,
                            EngineErrorCode::ResetPanic,
                        );
                        if let Some(mut record) = env.fault_record() {
                            record.fingerprint = fingerprint;
                            env.fault_latched = Some(record);
                        }
                        out
                    } else {
                        latch_fallback_reset_fault(
                            env,
                            idx as u32,
                            meta_episode_index,
                            meta_episode_seed,
                            meta_decision_id,
                        );
                        fallback_reset_panic_outcome(template.template_config.reward.terminal_draw)
                    }
                }));
                match recover {
                    Ok(outcome) => outcome,
                    Err(_) => {
                        latch_fallback_reset_fault(
                            env,
                            idx as u32,
                            meta_episode_index,
                            meta_episode_seed,
                            meta_decision_id,
                        );
                        fallback_reset_panic_outcome(template.template_config.reward.terminal_draw)
                    }
                }
            }
        }
    }

    fn make_reset_template(&self) -> ResetSlotTemplate {
        ResetSlotTemplate {
            template_db: self.template_db.clone(),
            template_config: self.template_config.clone(),
            template_curriculum: self.template_curriculum.clone(),
            template_replay_config: self.template_replay_config.clone(),
            template_replay_writer: self.template_replay_writer.clone(),
            debug_config: self.debug_config,
            output_mask_enabled: self.output_mask_enabled,
            output_mask_bits_enabled: self.output_mask_bits_enabled,
            error_policy: self.error_policy,
            pool_seed: self.pool_seed,
        }
    }

    fn fill_outcomes_for_all_reset(&mut self) {
        self.ensure_outcomes_scratch();
        let template = self.make_reset_template();
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .enumerate()
                    .for_each(|(idx, (slot, env))| {
                        *slot = Self::reset_slot_no_copy(idx, env, true, None, &template);
                    });
            });
        } else {
            for (idx, (slot, env)) in self
                .outcomes_scratch
                .iter_mut()
                .zip(self.envs.iter_mut())
                .enumerate()
            {
                *slot = Self::reset_slot_no_copy(idx, env, true, None, &template);
            }
        }
    }

    fn fill_outcomes_for_flags(&mut self, flags: &[bool]) -> Result<()> {
        if flags.len() != self.envs.len() {
            anyhow::bail!("reset flags size mismatch");
        }
        self.ensure_outcomes_scratch();
        let template = self.make_reset_template();
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .zip(flags.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), &should_reset))| {
                        *slot = Self::reset_slot_no_copy(idx, env, should_reset, None, &template);
                    });
            });
        } else {
            for (idx, ((slot, env), &should_reset)) in self
                .outcomes_scratch
                .iter_mut()
                .zip(self.envs.iter_mut())
                .zip(flags.iter())
                .enumerate()
            {
                *slot = Self::reset_slot_no_copy(idx, env, should_reset, None, &template);
            }
        }
        Ok(())
    }

    fn fill_outcomes_for_seed_options(&mut self, seeds: &[Option<u64>]) -> Result<()> {
        if seeds.len() != self.envs.len() {
            anyhow::bail!("seed options size mismatch");
        }
        self.ensure_outcomes_scratch();
        let template = self.make_reset_template();
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .zip(seeds.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), seed_opt))| {
                        *slot = Self::reset_slot_no_copy(
                            idx,
                            env,
                            seed_opt.is_some(),
                            *seed_opt,
                            &template,
                        );
                    });
            });
        } else {
            for (idx, ((slot, env), seed_opt)) in self
                .outcomes_scratch
                .iter_mut()
                .zip(self.envs.iter_mut())
                .zip(seeds.iter())
                .enumerate()
            {
                *slot =
                    Self::reset_slot_no_copy(idx, env, seed_opt.is_some(), *seed_opt, &template);
            }
        }
        Ok(())
    }

    /// Reset all envs and fill a minimal output batch (i32 obs + masks).
    pub fn reset_into(&mut self, out: &mut BatchOutMinimal<'_>) -> Result<()> {
        self.fill_outcomes_for_all_reset();
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    /// Reset all envs and fill a minimal output batch (i16 obs + masks).
    pub fn reset_into_i16(&mut self, out: &mut BatchOutMinimalI16<'_>) -> Result<()> {
        self.fill_outcomes_for_all_reset();
        let outcomes = &self.outcomes_scratch;
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
        self.fill_outcomes_for_all_reset();
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16_legal_ids(outcomes, out)?;
        self.legal_action_ids_batch_into(out.legal_ids, out.legal_offsets)?;
        Ok(())
    }

    /// Reset all envs and fill a minimal output batch without masks.
    pub fn reset_into_nomask(&mut self, out: &mut BatchOutMinimalNoMask<'_>) -> Result<()> {
        self.fill_outcomes_for_all_reset();
        let outcomes = &self.outcomes_scratch;
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
        let flags = self.reset_flags.clone();
        self.fill_outcomes_for_flags(&flags)?;
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
        let flags = self.reset_flags.clone();
        self.fill_outcomes_for_flags(&flags)?;
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
        let flags = self.reset_flags.clone();
        self.fill_outcomes_for_flags(&flags)?;
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
        let flags = self.reset_flags.clone();
        self.fill_outcomes_for_flags(&flags)?;
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
        self.fill_outcomes_for_flags(done_mask)?;
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
        self.fill_outcomes_for_flags(done_mask)?;
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
        self.fill_outcomes_for_flags(done_mask)?;
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
        self.fill_outcomes_for_flags(done_mask)?;
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
        let flags = self.reset_flags.clone();
        self.fill_outcomes_for_flags(&flags)?;
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
        let flags = self.reset_flags.clone();
        self.fill_outcomes_for_flags(&flags)?;
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
        let seed_opts = self.reset_seed_scratch.clone();
        self.fill_outcomes_for_seed_options(&seed_opts)?;
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
        let seed_opts = self.reset_seed_scratch.clone();
        self.fill_outcomes_for_seed_options(&seed_opts)?;
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
        let seed_opts = self.reset_seed_scratch.clone();
        self.fill_outcomes_for_seed_options(&seed_opts)?;
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
        let seed_opts = self.reset_seed_scratch.clone();
        self.fill_outcomes_for_seed_options(&seed_opts)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
    }
}
