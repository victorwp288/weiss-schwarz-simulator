use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rayon::prelude::*;

use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN, SPEC_HASH};
use crate::env::{EngineErrorCode, StepOutcome};
use crate::legal::ActionDesc;

use super::core::EnvPool;
use super::outputs::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask, BatchOutTrajectory, BatchOutTrajectoryI16,
    BatchOutTrajectoryI16LegalIds, BatchOutTrajectoryNoMask,
};

fn empty_info() -> crate::env::EnvInfo {
    crate::env::EnvInfo {
        obs_version: 0,
        action_version: 0,
        decision_kind: crate::encode::DECISION_KIND_NONE,
        current_player: -1,
        actor: -1,
        decision_count: 0,
        tick_count: 0,
        terminal: None,
        illegal_action: false,
        engine_error: false,
        engine_error_code: 0,
    }
}

fn empty_outcome() -> StepOutcome {
    StepOutcome {
        obs: Vec::new(),
        reward: 0.0,
        terminated: false,
        truncated: false,
        info: empty_info(),
    }
}

impl EnvPool {
    pub(super) fn panic_fingerprint_from_meta(
        env_id: u32,
        episode_index: u32,
        episode_seed: u64,
        decision_id: u32,
        code: EngineErrorCode,
    ) -> u64 {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&env_id.to_le_bytes());
        bytes.extend_from_slice(&episode_index.to_le_bytes());
        bytes.extend_from_slice(&episode_seed.to_le_bytes());
        bytes.extend_from_slice(&decision_id.to_le_bytes());
        bytes.push(code as u8);
        crate::fingerprint::hash_bytes(&bytes)
    }

    pub(super) fn ensure_outcomes_scratch(&mut self) {
        let len = self.envs.len();
        if self.outcomes_scratch.len() != len {
            self.outcomes_scratch = (0..len).map(|_| empty_outcome()).collect();
        }
    }

    pub(super) fn ensure_legal_counts_scratch(&mut self) {
        let len = self.envs.len();
        if self.legal_counts_scratch.len() != len {
            self.legal_counts_scratch = vec![0usize; len];
        }
    }

    pub(super) fn debug_compute_fingerprints(&mut self) -> bool {
        if self.debug_config.fingerprint_every_n == 0 {
            return false;
        }
        self.debug_step_counter = self.debug_step_counter.wrapping_add(1);
        self.debug_step_counter
            .is_multiple_of(self.debug_config.fingerprint_every_n as u64)
    }

    /// Compute state fingerprints for each env.
    pub fn state_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| crate::fingerprint::state_fingerprint(&env.state))
            .collect()
    }

    /// Compute event-stream fingerprints for each env.
    pub fn events_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| crate::fingerprint::events_fingerprint(env.canonical_events()))
            .collect()
    }

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

    /// Debug event ring capacity configured for the pool.
    pub fn debug_event_ring_capacity(&self) -> usize {
        self.debug_config.event_ring_capacity
    }

    /// Fetch packed action mask bits for all envs.
    pub fn action_mask_bits_batch(&self) -> Vec<u64> {
        if !self.output_mask_bits_enabled {
            return Vec::new();
        }
        let words_per_env = crate::encode::ACTION_SPACE_WORDS;
        let mut bits = vec![0u64; self.envs.len() * words_per_env];
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
        let words_per_env = crate::encode::ACTION_SPACE_WORDS;
        let expected = self.envs.len() * words_per_env;
        if bits.len() != expected {
            anyhow::bail!("mask bits buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let base = i * words_per_env;
            let slice = &mut bits[base..base + words_per_env];
            slice.copy_from_slice(env.action_mask_bits());
        }
        Ok(())
    }

    /// Sample a legal action id uniformly per env.
    pub fn sample_legal_action_ids_uniform(&self, seeds: &[u64]) -> Result<Vec<u32>> {
        let mut out = vec![0u32; self.envs.len()];
        self.sample_legal_action_ids_uniform_into(seeds, &mut out)?;
        Ok(out)
    }

    /// Sample a legal action id uniformly per env into a buffer.
    pub fn sample_legal_action_ids_uniform_into(
        &self,
        seeds: &[u64],
        out: &mut [u32],
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if seeds.len() != num_envs || out.len() != num_envs {
            anyhow::bail!("seed/output size mismatch");
        }
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
            pool.install(|| {
                out.par_iter_mut()
                    .zip(envs.par_iter())
                    .zip(seeds.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), &seed))| {
                        let legal = env.action_ids_cache();
                        if legal.is_empty() {
                            error_flag.store(true, Ordering::Relaxed);
                            let mut guard = error_store
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner());
                            if guard.is_none() {
                                *guard = Some(anyhow!("no legal actions for env {idx}"));
                            }
                            return;
                        }
                        let pick = (seed % legal.len() as u64) as usize;
                        *slot = legal[pick] as u32;
                    });
            });
            if error_flag.load(Ordering::Relaxed) {
                let err = error_store
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take();
                if let Some(err) = err {
                    return Err(err);
                }
            }
        } else {
            for (i, ((slot, env), &seed)) in out
                .iter_mut()
                .zip(self.envs.iter())
                .zip(seeds.iter())
                .enumerate()
            {
                let legal = env.action_ids_cache();
                if legal.is_empty() {
                    anyhow::bail!("no legal actions for env {i}");
                }
                let pick = (seed % legal.len() as u64) as usize;
                *slot = legal[pick] as u32;
            }
        }
        Ok(())
    }

    /// Write the first legal action id per env into a buffer.
    pub fn first_legal_action_ids_into(&self, out: &mut [u32]) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
        }
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
            pool.install(|| {
                out.par_iter_mut()
                    .zip(envs.par_iter())
                    .enumerate()
                    .for_each(|(idx, (slot, env))| {
                        let legal = env.action_ids_cache();
                        if legal.is_empty() {
                            error_flag.store(true, Ordering::Relaxed);
                            let mut guard = error_store
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner());
                            if guard.is_none() {
                                *guard = Some(anyhow!("no legal actions for env {idx}"));
                            }
                            return;
                        }
                        *slot = legal[0] as u32;
                    });
            });
            if error_flag.load(Ordering::Relaxed) {
                let err = error_store
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take();
                if let Some(err) = err {
                    return Err(err);
                }
            }
        } else {
            for (i, (slot, env)) in out.iter_mut().zip(self.envs.iter()).enumerate() {
                let legal = env.action_ids_cache();
                if legal.is_empty() {
                    anyhow::bail!("no legal actions for env {i}");
                }
                *slot = legal[0] as u32;
            }
        }
        Ok(())
    }

    /// Fill legal-id buffers and sample one action per env.
    pub fn legal_action_ids_and_sample_uniform_into(
        &mut self,
        ids: &mut [u16],
        offsets: &mut [u32],
        seeds: &[u64],
        sampled: &mut [u32],
    ) -> Result<usize> {
        let num_envs = self.envs.len();
        if seeds.len() != num_envs || sampled.len() != num_envs {
            anyhow::bail!("seed/output size mismatch");
        }
        if offsets.len() != num_envs + 1 {
            anyhow::bail!("offset buffer size mismatch");
        }
        if ACTION_SPACE_SIZE > u16::MAX as usize {
            anyhow::bail!("action space too large for u16 ids");
        }
        if self.thread_pool.is_none() {
            offsets[0] = 0;
            let mut cursor = 0usize;
            for (i, ((env, &seed), slot)) in self
                .envs
                .iter()
                .zip(seeds.iter())
                .zip(sampled.iter_mut())
                .enumerate()
            {
                let legal = env.action_ids_cache();
                if legal.is_empty() {
                    anyhow::bail!("no legal actions for env {i}");
                }
                let pick = (seed % legal.len() as u64) as usize;
                *slot = legal[pick] as u32;
                let next = cursor.saturating_add(legal.len());
                if next > ids.len() {
                    anyhow::bail!("ids buffer size mismatch");
                }
                ids[cursor..next].copy_from_slice(legal);
                offsets[i + 1] = next as u32;
                cursor = next;
            }
            return Ok(cursor);
        }
        let total = self.legal_action_ids_batch_into(ids, offsets)?;
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            let error_flag = Arc::new(AtomicBool::new(false));
            let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
            pool.install(|| {
                sampled
                    .par_iter_mut()
                    .zip(envs.par_iter())
                    .zip(seeds.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), &seed))| {
                        let legal = env.action_ids_cache();
                        if legal.is_empty() {
                            error_flag.store(true, Ordering::Relaxed);
                            let mut guard = error_store
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner());
                            if guard.is_none() {
                                *guard = Some(anyhow!("no legal actions for env {idx}"));
                            }
                            return;
                        }
                        let pick = (seed % legal.len() as u64) as usize;
                        *slot = legal[pick] as u32;
                    });
            });
            if error_flag.load(Ordering::Relaxed) {
                let err = error_store
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take();
                if let Some(err) = err {
                    return Err(err);
                }
            }
        }
        Ok(total)
    }

    /// Fill legal-id buffers for all envs.
    pub fn legal_action_ids_batch_into(
        &mut self,
        ids: &mut [u16],
        offsets: &mut [u32],
    ) -> Result<usize> {
        let num_envs = self.envs.len();
        if offsets.len() != num_envs + 1 {
            anyhow::bail!("offset buffer size mismatch");
        }
        if ACTION_SPACE_SIZE > u16::MAX as usize {
            anyhow::bail!("action space too large for u16 ids");
        }
        self.ensure_legal_counts_scratch();
        let counts = &mut self.legal_counts_scratch;
        // This path is called every policy step in legal-id workflows.
        // Per-env work here is tiny (cache length read), and rayon setup/coordination
        // dominates at typical batch sizes, so keep this pass serial.
        for (slot, env) in counts.iter_mut().zip(self.envs.iter()) {
            *slot = env.action_ids_cache().len();
        }
        offsets[0] = 0;
        let mut total = 0usize;
        for (i, &count) in counts.iter().enumerate() {
            total = total.saturating_add(count);
            if total > ids.len() {
                anyhow::bail!("ids buffer size mismatch");
            }
            offsets[i + 1] = total as u32;
        }
        let mut cursor = 0usize;
        for (i, env) in self.envs.iter().enumerate() {
            for &action_id in env.action_ids_cache() {
                ids[cursor] = action_id;
                cursor += 1;
            }
            debug_assert_eq!(cursor, offsets[i + 1] as usize);
        }
        Ok(total)
    }

    /// Compute legal action descriptors for all envs.
    pub fn legal_actions_batch(&self) -> Vec<Vec<ActionDesc>> {
        self.envs.iter().map(|env| env.legal_actions()).collect()
    }

    /// Current decision player per env (-1 if none).
    pub fn get_current_player_batch(&self) -> Vec<i8> {
        self.envs
            .iter()
            .map(|env| env.decision.as_ref().map(|d| d.player as i8).unwrap_or(-1))
            .collect()
    }

    /// Render a single env to an ANSI string for debugging.
    pub fn render_ansi(&self, env_index: usize, perspective: u8) -> String {
        if env_index >= self.envs.len() {
            return "Invalid env index".to_string();
        }
        if perspective > 1 {
            return "Invalid perspective".to_string();
        }
        let env = &self.envs[env_index];
        let p0 = perspective as usize;
        let p1 = 1 - p0;
        let state = &env.state;
        let mut out = String::new();
        out.push_str(&format!("Phase: {:?}\n", state.turn.phase));
        out.push_str(&format!("Active: {}\n", state.turn.active_player));
        out.push_str(&format!(
            "P{} Level: {} Clock: {} Hand: {} Deck: {}\n",
            p0,
            state.players[p0].level.len(),
            state.players[p0].clock.len(),
            state.players[p0].hand.len(),
            state.players[p0].deck.len()
        ));
        out.push_str(&format!(
            "P{} Level: {} Clock: {} Hand: {} Deck: {}\n",
            p1,
            state.players[p1].level.len(),
            state.players[p1].clock.len(),
            state.players[p1].hand.len(),
            state.players[p1].deck.len()
        ));
        out.push_str("Stage:\n");
        out.push_str(&format!(
            " P{}: {}\n",
            p0,
            Self::format_stage(&state.players[p0].stage)
        ));
        out.push_str(&format!(
            " P{}: {}\n",
            p1,
            Self::format_stage(&state.players[p1].stage)
        ));
        if let Some(action) = &env.last_action_desc {
            let hide_action = env.curriculum.enable_visibility_policies
                && env.config.observation_visibility
                    == crate::config::ObservationVisibility::Public
                && env
                    .last_action_player
                    .map(|p| p != perspective)
                    .unwrap_or(false);
            if !hide_action {
                out.push_str(&format!("Last action: {:?}\n", action));
            }
        }
        out
    }

    fn format_stage(stage: &[crate::state::StageSlot; 5]) -> String {
        let mut parts = Vec::with_capacity(stage.len());
        for slot in stage {
            if let Some(card) = slot.card {
                parts.push(format!("{}:{:?}", card.id, slot.status));
            } else {
                parts.push("Empty".to_string());
            }
        }
        format!("[{}]", parts.join(", "))
    }

    /// Compute observation fingerprints for each env.
    pub fn obs_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        env.obs_buf.as_ptr() as *const u8,
                        env.obs_buf.len() * std::mem::size_of::<i32>(),
                    )
                };
                crate::fingerprint::hash_bytes(bytes)
            })
            .collect()
    }

    fn validate_obs_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("obs buffer size mismatch");
        }
        Ok(())
    }

    fn validate_mask_len(actual: usize, expected: usize) -> Result<()> {
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

    fn validate_legal_ids_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("legal ids buffer size mismatch");
        }
        Ok(())
    }

    fn validate_legal_offsets_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("legal offsets buffer size mismatch");
        }
        Ok(())
    }

    fn validate_action_len(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            anyhow::bail!("action buffer size mismatch");
        }
        Ok(())
    }

    fn validate_scalar_lens(expected: usize, scalar_lens: [usize; 8]) -> Result<()> {
        if scalar_lens.into_iter().any(|len| len != expected) {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_minimal_out(&self, out: &BatchOutMinimal<'_>) -> Result<()> {
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
            ],
        )?;
        Ok(())
    }

    fn validate_minimal_out_i16(&self, out: &BatchOutMinimalI16<'_>) -> Result<()> {
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
            ],
        )?;
        Ok(())
    }

    fn validate_minimal_out_i16_legal_ids(
        &self,
        out: &BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        Self::validate_obs_len(out.obs.len(), num_envs * OBS_LEN)?;
        Self::validate_legal_ids_len(out.legal_ids.len(), num_envs * ACTION_SPACE_SIZE)?;
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
            ],
        )?;
        Ok(())
    }

    fn validate_minimal_out_nomask(&self, out: &BatchOutMinimalNoMask<'_>) -> Result<()> {
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
            ],
        )?;
        Ok(())
    }

    pub(super) fn validate_trajectory(
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

    pub(super) fn validate_trajectory_i16(
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

    pub(super) fn validate_trajectory_i16_legal_ids(
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

    pub(super) fn validate_trajectory_nomask(
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

    pub(super) fn fill_minimal_out(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.validate_minimal_out(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            if outcome.obs.is_empty() {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&env.obs_buf);
            } else {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&outcome.obs);
            }
            let mask_offset = i * ACTION_SPACE_SIZE;
            if self.output_mask_enabled {
                out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE]
                    .copy_from_slice(env.action_mask());
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
            if self.output_mask_enabled {
                debug_assert!(
                    out.terminated[i]
                        || out.truncated[i]
                        || out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE].contains(&1)
                );
            }
        }
        Ok(())
    }

    pub(super) fn fill_minimal_out_i16(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_i16(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        let count_overflow = self.i16_overflow_counter_enabled.load(Ordering::Relaxed);
        let mut overflow_count = 0u64;
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            let src = if outcome.obs.is_empty() {
                &env.obs_buf
            } else {
                &outcome.obs
            };
            for (dst, &val) in out.obs[obs_offset..obs_offset + OBS_LEN]
                .iter_mut()
                .zip(src.iter())
            {
                if count_overflow && (val < i16::MIN as i32 || val > i16::MAX as i32) {
                    overflow_count = overflow_count.saturating_add(1);
                }
                if self.i16_clamp_enabled {
                    let clamped = val.clamp(i16::MIN as i32, i16::MAX as i32);
                    *dst = clamped as i16;
                } else {
                    *dst = val as i16;
                }
            }
            let mask_offset = i * ACTION_SPACE_SIZE;
            if self.output_mask_enabled {
                out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE]
                    .copy_from_slice(env.action_mask());
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
            if self.output_mask_enabled {
                debug_assert!(
                    out.terminated[i]
                        || out.truncated[i]
                        || out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE].contains(&1)
                );
            }
        }
        if count_overflow && overflow_count > 0 {
            self.i16_overflow_count
                .fetch_add(overflow_count, Ordering::Relaxed);
        }
        Ok(())
    }

    pub(super) fn fill_minimal_out_i16_legal_ids(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_i16_legal_ids(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        let count_overflow = self.i16_overflow_counter_enabled.load(Ordering::Relaxed);
        let mut overflow_count = 0u64;
        out.legal_offsets[0] = 0;
        let mut legal_cursor = 0usize;
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            let src = if outcome.obs.is_empty() {
                &env.obs_buf
            } else {
                &outcome.obs
            };
            for (dst, &val) in out.obs[obs_offset..obs_offset + OBS_LEN]
                .iter_mut()
                .zip(src.iter())
            {
                if count_overflow && (val < i16::MIN as i32 || val > i16::MAX as i32) {
                    overflow_count = overflow_count.saturating_add(1);
                }
                if self.i16_clamp_enabled {
                    let clamped = val.clamp(i16::MIN as i32, i16::MAX as i32);
                    *dst = clamped as i16;
                } else {
                    *dst = val as i16;
                }
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            let legal_ids = env.action_ids_cache();
            let next = legal_cursor.saturating_add(legal_ids.len());
            if next > out.legal_ids.len() {
                anyhow::bail!("legal ids buffer size mismatch");
            }
            out.legal_ids[legal_cursor..next].copy_from_slice(legal_ids);
            out.legal_offsets[i + 1] = next as u32;
            legal_cursor = next;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
        }
        if count_overflow && overflow_count > 0 {
            self.i16_overflow_count
                .fetch_add(overflow_count, Ordering::Relaxed);
        }
        Ok(())
    }

    pub(super) fn fill_minimal_out_nomask(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_nomask(out)?;
        let num_envs = self.envs.len();
        debug_assert_eq!(outcomes.len(), num_envs);
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            let obs_offset = i * OBS_LEN;
            if outcome.obs.is_empty() {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&env.obs_buf);
            } else {
                out.obs[obs_offset..obs_offset + OBS_LEN].copy_from_slice(&outcome.obs);
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            let engine_status = if outcome.info.engine_error {
                outcome.info.engine_error_code
            } else {
                env.last_engine_error_code as u8
            };
            out.engine_status[i] = engine_status;
            let keep_fault_actor = engine_status != EngineErrorCode::None as u8
                && (env.fault_actor().is_some() || outcome.info.actor != crate::encode::ACTOR_NONE);
            out.actor[i] = if outcome.terminated || outcome.truncated {
                if keep_fault_actor {
                    env.fault_actor()
                        .or_else(|| {
                            (outcome.info.actor != crate::encode::ACTOR_NONE)
                                .then_some(outcome.info.actor as u8)
                        })
                        .map(|a| a as i8)
                        .unwrap_or(crate::encode::ACTOR_NONE)
                } else {
                    crate::encode::ACTOR_NONE
                }
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.spec_hash[i] = SPEC_HASH;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
        }
        Ok(())
    }

    pub(super) fn fill_debug_out(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutDebug<'_>,
        compute_fingerprints: bool,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if out.state_fingerprint.len() != num_envs
            || out.events_fingerprint.len() != num_envs
            || out.mask_fingerprint.len() != num_envs
            || out.event_counts.len() != num_envs
        {
            anyhow::bail!("debug buffer size mismatch");
        }
        let event_capacity = if num_envs == 0 {
            0
        } else if !out.event_codes.len().is_multiple_of(num_envs) {
            anyhow::bail!("event code buffer size mismatch");
        } else {
            out.event_codes.len() / num_envs
        };
        for (i, (env, outcome)) in self.envs.iter().zip(outcomes.iter()).enumerate() {
            if compute_fingerprints {
                out.state_fingerprint[i] = crate::fingerprint::state_fingerprint(&env.state);
                out.events_fingerprint[i] =
                    crate::fingerprint::events_fingerprint(env.canonical_events());
                if self.output_mask_enabled {
                    let mask_offset = i * ACTION_SPACE_SIZE;
                    let mask = &out.minimal.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE];
                    out.mask_fingerprint[i] = crate::fingerprint::hash_bytes(mask);
                } else if self.output_mask_bits_enabled {
                    let bits = env.action_mask_bits();
                    let byte_len = std::mem::size_of_val(bits);
                    let bytes =
                        unsafe { std::slice::from_raw_parts(bits.as_ptr() as *const u8, byte_len) };
                    out.mask_fingerprint[i] = crate::fingerprint::hash_bytes(bytes);
                } else {
                    let ids = env.action_ids_cache();
                    let byte_len = std::mem::size_of_val(ids);
                    let bytes =
                        unsafe { std::slice::from_raw_parts(ids.as_ptr() as *const u8, byte_len) };
                    out.mask_fingerprint[i] = crate::fingerprint::hash_bytes(bytes);
                }
            } else {
                out.state_fingerprint[i] = 0;
                out.events_fingerprint[i] = 0;
                out.mask_fingerprint[i] = 0;
            }
            if event_capacity == 0 {
                out.event_counts[i] = 0;
            } else {
                let actor = outcome.info.actor;
                let viewer = if actor < 0 { 0 } else { actor as u8 };
                let offset = i * event_capacity;
                let count = env.debug_event_ring_codes(
                    viewer,
                    &mut out.event_codes[offset..offset + event_capacity],
                );
                out.event_counts[i] = count;
            }
        }
        Ok(())
    }
}
