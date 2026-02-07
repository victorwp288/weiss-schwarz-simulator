use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rayon::prelude::*;

use super::core::EnvPool;
use super::outputs::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask, BatchOutTrajectory, BatchOutTrajectoryI16,
    BatchOutTrajectoryI16LegalIds, BatchOutTrajectoryNoMask,
};

use crate::config::ErrorPolicy;
use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN};
use crate::env::{EngineErrorCode, GameEnv, StepOutcome};

impl EnvPool {
    fn step_batch_outcomes(&mut self, action_ids: &[u32]) -> Result<()> {
        if action_ids.len() != self.envs.len() {
            anyhow::bail!("Action batch size mismatch");
        }
        self.ensure_outcomes_scratch();
        if self.envs.is_empty() {
            return Ok(());
        }
        let strict = self.error_policy == ErrorPolicy::Strict;
        let step_inner = |env: &mut GameEnv, action_id: u32| -> Result<StepOutcome> {
            if env.state.terminal.is_some() {
                env.clear_status_flags();
                return Ok(env.build_outcome_no_copy(0.0));
            }
            if env.decision.is_none() {
                env.advance_until_decision();
                env.update_action_cache();
                env.clear_status_flags();
                return Ok(env.build_outcome_no_copy(0.0));
            }
            env.apply_action_id_no_copy(action_id as usize)
        };
        let step_lenient = |env: &mut GameEnv, action_id: u32| -> StepOutcome {
            let result = catch_unwind(AssertUnwindSafe(|| step_inner(env, action_id)));
            match result {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(_)) | Err(_) => {
                    let acting_player = env
                        .decision
                        .as_ref()
                        .map(|d| d.player)
                        .unwrap_or(env.last_perspective);
                    env.last_engine_error = true;
                    env.last_engine_error_code = EngineErrorCode::Panic;
                    env.last_perspective = acting_player;
                    env.state.terminal = Some(crate::state::TerminalResult::Win {
                        winner: 1 - acting_player,
                    });
                    env.clear_decision();
                    env.update_action_cache();
                    env.build_outcome_no_copy(env.terminal_reward_for(acting_player))
                }
            }
        };

        if strict {
            if let Some(pool) = self.thread_pool.as_ref() {
                let envs = &mut self.envs;
                let outcomes = &mut self.outcomes_scratch;
                let error_flag = Arc::new(AtomicBool::new(false));
                let error_store: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
                pool.install(|| {
                    outcomes
                        .par_iter_mut()
                        .zip(envs.par_iter_mut())
                        .zip(action_ids.par_iter())
                        .for_each(|((slot, env), &action_id)| {
                            if error_flag.load(Ordering::Acquire) {
                                return;
                            }
                            let result =
                                catch_unwind(AssertUnwindSafe(|| step_inner(env, action_id)))
                                    .map_err(|panic| {
                                        anyhow!("panic in env step: {}", Self::panic_message(panic))
                                    })
                                    .and_then(|res| res);
                            match result {
                                Ok(outcome) => {
                                    if error_flag.load(Ordering::Acquire) {
                                        return;
                                    }
                                    *slot = outcome;
                                }
                                Err(err) => {
                                    if !error_flag.swap(true, Ordering::AcqRel) {
                                        let mut guard =
                                            error_store.lock().expect("error store poisoned");
                                        if guard.is_none() {
                                            *guard = Some(err);
                                        }
                                    }
                                }
                            }
                        });
                });
                let err = error_store.lock().expect("error store poisoned").take();
                if let Some(err) = err {
                    return Err(err);
                }
            } else {
                for ((slot, env), &action_id) in self
                    .outcomes_scratch
                    .iter_mut()
                    .zip(self.envs.iter_mut())
                    .zip(action_ids.iter())
                {
                    let result = catch_unwind(AssertUnwindSafe(|| step_inner(env, action_id)))
                        .map_err(|panic| {
                            anyhow!("panic in env step: {}", Self::panic_message(panic))
                        })?;
                    *slot = result?;
                }
            }
        } else if let Some(pool) = self.thread_pool.as_ref() {
            let chunk = self.par_chunk_size();
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_chunks_mut(chunk)
                    .zip(envs.par_chunks_mut(chunk))
                    .zip(action_ids.par_chunks(chunk))
                    .for_each(|((out_chunk, env_chunk), action_chunk)| {
                        for ((slot, env), &action_id) in out_chunk
                            .iter_mut()
                            .zip(env_chunk.iter_mut())
                            .zip(action_chunk.iter())
                        {
                            *slot = step_lenient(env, action_id);
                        }
                    });
            });
        } else {
            for ((slot, env), &action_id) in self
                .outcomes_scratch
                .iter_mut()
                .zip(self.envs.iter_mut())
                .zip(action_ids.iter())
            {
                *slot = step_lenient(env, action_id);
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
            };
            self.step_into_i16(action_slice, &mut out_min)?;
            let ids_offset = t * num_envs * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let ids_slice =
                &mut out.legal_ids[ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE];
            let offsets_slice =
                &mut out.legal_offsets[offsets_offset..offsets_offset + num_envs + 1];
            self.legal_action_ids_batch_into(ids_slice, offsets_slice)?;
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
            };
            self.step_into_i16(action_slice, &mut out_min)?;
            let ids_offset = t * num_envs * ACTION_SPACE_SIZE;
            let offsets_offset = t * (num_envs + 1);
            let ids_slice =
                &mut out.legal_ids[ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE];
            let offsets_slice =
                &mut out.legal_offsets[offsets_offset..offsets_offset + num_envs + 1];
            self.legal_action_ids_batch_into(ids_slice, offsets_slice)?;
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
            };
            self.step_into_nomask(action_slice, &mut out_min)?;
        }
        Ok(())
    }
}
