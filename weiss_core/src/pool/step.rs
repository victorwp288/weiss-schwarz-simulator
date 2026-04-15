use std::panic::{catch_unwind, AssertUnwindSafe};

use anyhow::Result;
use rayon::prelude::*;

use super::core::EnvPool;
use super::outputs::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask, BatchOutTrajectory, BatchOutTrajectoryI16,
    BatchOutTrajectoryI16LegalIds, BatchOutTrajectoryNoMask,
};

use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN};
use crate::env::{EngineErrorCode, EnvInfo, FaultSource, GameEnv, StepOutcome};

#[cold]
#[inline(never)]
fn fallback_panic_outcome(
    actor: Option<u8>,
    reward: f32,
    engine_code: EngineErrorCode,
) -> StepOutcome {
    StepOutcome {
        obs: vec![0; OBS_LEN],
        reward,
        terminated: false,
        truncated: true,
        info: EnvInfo {
            obs_version: crate::encode::OBS_ENCODING_VERSION,
            action_version: crate::encode::ACTION_ENCODING_VERSION,
            decision_kind: crate::encode::DECISION_KIND_NONE,
            current_player: -1,
            actor: actor
                .and_then(|a| i8::try_from(a).ok())
                .unwrap_or(crate::encode::ACTOR_NONE),
            decision_count: 0,
            tick_count: 0,
            terminal: Some(crate::state::TerminalResult::Timeout),
            illegal_action: false,
            engine_error: true,
            engine_error_code: engine_code as u8,
            main_move_action: false,
            main_pass_action: false,
        },
    }
}

#[cold]
#[inline(never)]
fn latch_fallback_step_fault(
    env: &mut GameEnv,
    env_id: u32,
    episode_index: u32,
    episode_seed: u64,
    decision_id: u32,
    actor: Option<u8>,
) {
    let fingerprint = EnvPool::panic_fingerprint_from_meta(
        env_id,
        episode_index,
        episode_seed,
        decision_id,
        EngineErrorCode::Panic,
    );
    env.last_engine_error = true;
    env.last_engine_error_code = EngineErrorCode::Panic;
    if let Some(a) = actor {
        env.last_perspective = a;
    }
    env.fault_latched = Some(crate::env::FaultRecord {
        code: EngineErrorCode::Panic,
        actor,
        fingerprint,
        source: FaultSource::Step,
        reward_emitted: true,
    });
    env.state.terminal = Some(crate::state::TerminalResult::Timeout);
    env.decision = None;
    env.action_cache.clear();
}

impl EnvPool {
    const STEP_PARALLEL_MIN_ENVS: usize = 256;

    fn step_batch_outcomes(&mut self, action_ids: &[u32]) -> Result<()> {
        if action_ids.len() != self.envs.len() {
            anyhow::bail!("Action batch size mismatch");
        }
        #[cfg(feature = "tracing")]
        let _span = tracing::trace_span!(
            "pool.step_batch_outcomes",
            num_envs = self.envs.len(),
            action_batch = action_ids.len(),
            effective_threads = self.thread_pool_size.unwrap_or(1),
        )
        .entered();
        self.ensure_outcomes_scratch();
        if self.envs.is_empty() {
            return Ok(());
        }
        let template_db = self.template_db.clone();
        let template_config = self.template_config.clone();
        let template_curriculum = self.template_curriculum.clone();
        let template_replay_config = self.template_replay_config.clone();
        let template_replay_writer = self.template_replay_writer.clone();
        let debug_config = self.debug_config;
        let output_mask_enabled = self.output_mask_enabled;
        let output_mask_bits_enabled = self.output_mask_bits_enabled;
        let error_policy = self.error_policy;
        let pool_seed = self.pool_seed;

        let run_step = |idx: usize, env: &mut GameEnv, action_id: u32| -> StepOutcome {
            let mut meta_actor: Option<u8> = None;
            let meta_episode_index = env.episode_index;
            let meta_episode_seed = env.episode_seed;
            let mut meta_decision_id = env.decision_id();

            let result = catch_unwind(AssertUnwindSafe(|| -> StepOutcome {
                meta_actor = env
                    .decision
                    .as_ref()
                    .map(|d| d.player)
                    .or_else(|| env.fault_actor());
                meta_decision_id = env.decision_id();
                if env.is_fault_latched() {
                    return env.build_fault_step_outcome_no_copy();
                }
                if env.state.terminal.is_some() {
                    env.clear_status_flags();
                    return env.build_outcome_no_copy(0.0);
                }
                if env.decision.is_none() {
                    env.advance_until_decision();
                    env.update_action_cache();
                    env.clear_status_flags();
                    return env.build_outcome_no_copy(0.0);
                }
                match env.apply_action_id_no_copy(action_id as usize) {
                    Ok(outcome) => outcome,
                    Err(_) => env.latch_fault(
                        EngineErrorCode::ActionError,
                        meta_actor,
                        FaultSource::Step,
                        false,
                    ),
                }
            }));

            match result {
                Ok(outcome) => outcome,
                Err(_) => {
                    let recover = catch_unwind(AssertUnwindSafe(|| {
                        let rebuilt = GameEnv::new(
                            template_db.clone(),
                            template_config.clone(),
                            template_curriculum.clone(),
                            pool_seed ^ (idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
                            template_replay_config.clone(),
                            template_replay_writer.clone(),
                            idx as u32,
                        );
                        if let Ok(mut fresh) = rebuilt {
                            fresh.set_debug_config(debug_config);
                            fresh.set_output_mask_enabled(output_mask_enabled);
                            fresh.set_output_mask_bits_enabled(output_mask_bits_enabled);
                            fresh.config.error_policy = error_policy;
                            *env = fresh;
                            let mut out = env.latch_fault(
                                EngineErrorCode::Panic,
                                meta_actor,
                                FaultSource::Step,
                                false,
                            );
                            let fingerprint = Self::panic_fingerprint_from_meta(
                                idx as u32,
                                meta_episode_index,
                                meta_episode_seed,
                                meta_decision_id,
                                EngineErrorCode::Panic,
                            );
                            if let Some(mut record) = env.fault_record() {
                                record.fingerprint = fingerprint;
                                env.fault_latched = Some(record);
                            }
                            out.info.engine_error = true;
                            out.info.engine_error_code = EngineErrorCode::Panic as u8;
                            out
                        } else {
                            latch_fallback_step_fault(
                                env,
                                idx as u32,
                                meta_episode_index,
                                meta_episode_seed,
                                meta_decision_id,
                                meta_actor,
                            );
                            fallback_panic_outcome(
                                meta_actor,
                                meta_actor
                                    .map(|_| template_config.reward.terminal_loss)
                                    .unwrap_or(template_config.reward.terminal_draw),
                                EngineErrorCode::Panic,
                            )
                        }
                    }));
                    match recover {
                        Ok(outcome) => outcome,
                        Err(_) => {
                            let fallback_reward = meta_actor
                                .map(|_| template_config.reward.terminal_loss)
                                .unwrap_or(template_config.reward.terminal_draw);
                            let mut rebuilt = false;
                            let mut double_panic_occurred = false;
                            match catch_unwind(AssertUnwindSafe(|| {
                                let rebuilt_env = GameEnv::new(
                                    template_db.clone(),
                                    template_config.clone(),
                                    template_curriculum.clone(),
                                    pool_seed ^ (idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
                                    template_replay_config.clone(),
                                    template_replay_writer.clone(),
                                    idx as u32,
                                );
                                if let Ok(mut fresh) = rebuilt_env {
                                    fresh.set_debug_config(debug_config);
                                    fresh.set_output_mask_enabled(output_mask_enabled);
                                    fresh.set_output_mask_bits_enabled(output_mask_bits_enabled);
                                    fresh.config.error_policy = error_policy;
                                    let fingerprint = Self::panic_fingerprint_from_meta(
                                        idx as u32,
                                        meta_episode_index,
                                        meta_episode_seed,
                                        meta_decision_id,
                                        EngineErrorCode::Panic,
                                    );
                                    fresh.fault_latched = Some(crate::env::FaultRecord {
                                        code: EngineErrorCode::Panic,
                                        actor: meta_actor,
                                        fingerprint,
                                        source: FaultSource::Step,
                                        reward_emitted: true,
                                    });
                                    fresh.last_engine_error = true;
                                    fresh.last_engine_error_code = EngineErrorCode::Panic;
                                    if let Some(actor) = meta_actor {
                                        fresh.last_perspective = actor;
                                    }
                                    fresh.state.terminal =
                                        Some(crate::state::TerminalResult::Timeout);
                                    fresh.clear_decision();
                                    fresh.update_action_cache();
                                    *env = fresh;
                                    rebuilt = true;
                                }
                            })) {
                                Ok(()) => {}
                                Err(_) => {
                                    double_panic_occurred = true;
                                    // Double-panic containment failed; do not touch the
                                    // potentially corrupted env and rely on fallback outcome.
                                }
                            }
                            if rebuilt {
                                // Rebuilt env already carries latched panic metadata.
                            } else if !double_panic_occurred {
                                latch_fallback_step_fault(
                                    env,
                                    idx as u32,
                                    meta_episode_index,
                                    meta_episode_seed,
                                    meta_decision_id,
                                    meta_actor,
                                );
                            }
                            fallback_panic_outcome(
                                meta_actor,
                                fallback_reward,
                                EngineErrorCode::Panic,
                            )
                        }
                    }
                }
            }
        };

        if let Some(pool) = self.thread_pool.as_ref().filter(|_| {
            self.thread_pool_size.is_some() && self.envs.len() >= Self::STEP_PARALLEL_MIN_ENVS
        }) {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .zip(action_ids.par_iter())
                    .enumerate()
                    .for_each(|(idx, ((slot, env), &action_id))| {
                        *slot = run_step(idx, env, action_id);
                    });
            });
        } else {
            for (idx, ((slot, env), &action_id)) in self
                .outcomes_scratch
                .iter_mut()
                .zip(self.envs.iter_mut())
                .zip(action_ids.iter())
                .enumerate()
            {
                *slot = run_step(idx, env, action_id);
            }
        }

        for env in &mut self.envs {
            if env.state.terminal.is_some() {
                env.finish_episode_replay();
            }
        }

        Ok(())
    }

    /// Step all envs with action ids and fill minimal outputs.
    pub fn step_into(&mut self, action_ids: &[u32], out: &mut BatchOutMinimal<'_>) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    /// Step all envs with action ids and fill i16 outputs.
    pub fn step_into_i16(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16(outcomes, out)
    }

    /// Step all envs and fill i16 outputs plus legal-id lists.
    ///
    /// Requires output masks to be disabled.
    pub fn step_into_i16_legal_ids(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids output requires output masks disabled");
        }
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16_legal_ids(outcomes, out)?;
        self.legal_action_ids_batch_into(out.legal_ids, out.legal_offsets)?;
        Ok(())
    }

    /// Step all envs and fill outputs without masks.
    pub fn step_into_nomask(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
    }

    /// Step using the first legal action per env (i16 + legal ids).
    pub fn step_first_legal_into_i16_legal_ids(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }

    /// Step using uniformly sampled legal actions (i16 + legal ids).
    pub fn step_sample_legal_action_ids_uniform_into_i16_legal_ids(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }

    /// Step all envs and fill debug outputs.
    pub fn step_debug_into(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutDebug<'_>,
    ) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let compute_fingerprints = self.debug_compute_fingerprints();
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, &mut out.minimal)?;
        self.fill_debug_out(outcomes, out, compute_fingerprints)
    }

    /// Step using the first legal action per env.
    pub fn step_first_legal_into(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into(actions, out)
    }

    /// Step using the first legal action per env (i16 outputs).
    pub fn step_first_legal_into_i16(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_i16(actions, out)
    }

    /// Step using the first legal action per env (no masks).
    pub fn step_first_legal_into_nomask(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_nomask(actions, out)
    }

    /// Step using uniformly sampled legal actions.
    pub fn step_sample_legal_action_ids_uniform_into(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into(actions, out)
    }

    /// Step using uniformly sampled legal actions (i16 outputs).
    pub fn step_sample_legal_action_ids_uniform_into_i16(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_i16(actions, out)
    }

    /// Step using uniformly sampled legal actions (no masks).
    pub fn step_sample_legal_action_ids_uniform_into_nomask(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_nomask(actions, out)
    }

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
            let ids_offset = t * num_envs * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let ids_slice =
                &mut out.legal_ids[ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE];
            let meta_slice = &mut out.legal_action_meta
                [ids_offset * crate::encode::ACTION_META_WIDTH
                    ..(ids_offset + num_envs * ACTION_SPACE_SIZE)
                        * crate::encode::ACTION_META_WIDTH];
            let offsets_slice =
                &mut out.legal_offsets[offsets_offset..offsets_offset + num_envs + 1];
            self.legal_action_ids_batch_into(ids_slice, offsets_slice)?;
            self.legal_action_meta_batch_into(meta_slice)?;
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
            let ids_offset = t * num_envs * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let ids_slice =
                &mut out.legal_ids[ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE];
            let meta_slice = &mut out.legal_action_meta
                [ids_offset * crate::encode::ACTION_META_WIDTH
                    ..(ids_offset + num_envs * ACTION_SPACE_SIZE)
                        * crate::encode::ACTION_META_WIDTH];
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
