use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use anyhow::Result;
use rayon::prelude::*;

use super::core::EnvPool;
use super::outputs::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalI16LegalIdsNoMeta, BatchOutMinimalNoMask,
};
use crate::config::{CurriculumConfig, EnvConfig, ErrorPolicy};
use crate::db::CardDb;

use crate::encode::OBS_LEN;
use crate::env::{
    DebugConfig, EngineErrorCode, EnvInfo, FaultSource, GameEnv, RewardBreakdown, StepOutcome,
};
use crate::replay::{ReplayConfig, ReplayWriter};

mod rollout;

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
        reward_breakdown: RewardBreakdown::terminal(reward),
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

#[derive(Clone)]
pub(in crate::pool) struct StepBatchContext {
    template_db: Arc<CardDb>,
    template_config: EnvConfig,
    template_curriculum: CurriculumConfig,
    template_replay_config: ReplayConfig,
    template_replay_writer: Option<ReplayWriter>,
    debug_config: DebugConfig,
    output_mask_enabled: bool,
    output_mask_bits_enabled: bool,
    error_policy: ErrorPolicy,
    pool_seed: u64,
}

impl EnvPool {
    const STEP_PARALLEL_MIN_ENVS: usize = 256;

    #[inline]
    pub(in crate::pool) fn step_batch_context(&self) -> StepBatchContext {
        StepBatchContext {
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

    pub(in crate::pool) fn run_step_outcome_with_context(
        context: &StepBatchContext,
        idx: usize,
        env: &mut GameEnv,
        action_id: u32,
        encode_observations: bool,
    ) -> StepOutcome {
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
                return env.build_outcome_maybe_encode_obs(0.0, false, encode_observations);
            }
            if env.decision.is_none() {
                env.advance_until_decision();
                env.update_action_cache();
                env.clear_status_flags();
                return env.build_outcome_maybe_encode_obs(0.0, false, encode_observations);
            }
            let step_result = if encode_observations {
                env.apply_action_id_no_copy(action_id as usize)
            } else {
                env.apply_action_id_without_obs_encode(action_id as usize)
            };
            match step_result {
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
                        context.template_db.clone(),
                        context.template_config.clone(),
                        context.template_curriculum.clone(),
                        context.pool_seed ^ (idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
                        context.template_replay_config.clone(),
                        context.template_replay_writer.clone(),
                        idx as u32,
                    );
                    if let Ok(mut fresh) = rebuilt {
                        fresh.set_debug_config(context.debug_config);
                        fresh.set_output_mask_enabled(context.output_mask_enabled);
                        fresh.set_output_mask_bits_enabled(context.output_mask_bits_enabled);
                        fresh.config.error_policy = context.error_policy;
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
                                .map(|_| context.template_config.reward.terminal_loss)
                                .unwrap_or(context.template_config.reward.terminal_draw),
                            EngineErrorCode::Panic,
                        )
                    }
                }));
                match recover {
                    Ok(outcome) => outcome,
                    Err(_) => {
                        let fallback_reward = meta_actor
                            .map(|_| context.template_config.reward.terminal_loss)
                            .unwrap_or(context.template_config.reward.terminal_draw);
                        let mut rebuilt = false;
                        let mut double_panic_occurred = false;
                        match catch_unwind(AssertUnwindSafe(|| {
                            let rebuilt_env = GameEnv::new(
                                context.template_db.clone(),
                                context.template_config.clone(),
                                context.template_curriculum.clone(),
                                context.pool_seed ^ (idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
                                context.template_replay_config.clone(),
                                context.template_replay_writer.clone(),
                                idx as u32,
                            );
                            if let Ok(mut fresh) = rebuilt_env {
                                fresh.set_debug_config(context.debug_config);
                                fresh.set_output_mask_enabled(context.output_mask_enabled);
                                fresh
                                    .set_output_mask_bits_enabled(context.output_mask_bits_enabled);
                                fresh.config.error_policy = context.error_policy;
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
                                fresh.state.terminal = Some(crate::state::TerminalResult::Timeout);
                                fresh.clear_decision();
                                fresh.update_action_cache();
                                *env = fresh;
                                rebuilt = true;
                            }
                        })) {
                            Ok(()) => {}
                            Err(_) => {
                                double_panic_occurred = true;
                            }
                        }
                        if rebuilt {
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
                        fallback_panic_outcome(meta_actor, fallback_reward, EngineErrorCode::Panic)
                    }
                }
            }
        }
    }

    #[inline]
    fn step_batch_outcomes(&mut self, action_ids: &[u32]) -> Result<()> {
        self.step_batch_outcomes_with_obs_mode(action_ids, true)
    }

    #[inline]
    fn step_batch_transition_outcomes_without_obs_encode(
        &mut self,
        action_ids: &[u32],
    ) -> Result<()> {
        self.step_batch_outcomes_with_obs_mode(action_ids, false)
    }

    #[inline]
    fn step_batch_outcomes_with_obs_mode(
        &mut self,
        action_ids: &[u32],
        encode_observations: bool,
    ) -> Result<()> {
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
        let step_context = self.step_batch_context();
        let run_step = |idx: usize, env: &mut GameEnv, action_id: u32| -> StepOutcome {
            Self::run_step_outcome_with_context(
                &step_context,
                idx,
                env,
                action_id,
                encode_observations,
            )
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
    #[inline]
    pub fn step_into(&mut self, action_ids: &[u32], out: &mut BatchOutMinimal<'_>) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    /// Step all envs with action ids and fill i16 outputs.
    #[inline]
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
    #[inline]
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
        self.fill_minimal_out_i16_legal_ids(outcomes, out)
    }

    /// Step all envs and fill i16 outputs plus legal-id lists, without legal metadata.
    ///
    /// Requires output masks to be disabled.
    #[inline]
    pub fn step_into_i16_legal_ids_nometa(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutMinimalI16LegalIdsNoMeta<'_>,
    ) -> Result<()> {
        if self.output_mask_enabled {
            anyhow::bail!("legal ids output requires output masks disabled");
        }
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16_legal_ids_nometa(outcomes, out)
    }

    /// Step all envs and fill outputs without masks.
    #[inline]
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

    /// Step using the first legal action per env (i16 + legal ids, no metadata).
    pub fn step_first_legal_into_i16_legal_ids_nometa(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIdsNoMeta<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_i16_legal_ids_nometa(actions, out)
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

    /// Step using uniformly sampled legal actions (i16 + legal ids, no metadata).
    pub fn step_sample_legal_action_ids_uniform_into_i16_legal_ids_nometa(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIdsNoMeta<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_i16_legal_ids_nometa(actions, out)
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
}
