use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rayon::prelude::*;

use crate::config::{CurriculumConfig, EnvConfig, ErrorPolicy};
use crate::db::CardDb;
use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN};
use crate::env::{EnvInfo, GameEnv, StepOutcome};
use crate::legal::ActionDesc;
use crate::replay::{ReplayConfig, ReplayWriter};

/// Batched results from stepping multiple environments.
#[derive(Clone, Debug)]
pub struct StepBatchResult {
    pub obs: Vec<i32>,
    pub rewards: Vec<f32>,
    pub terminated: Vec<bool>,
    pub truncated: Vec<bool>,
    pub infos: Vec<EnvInfo>,
}

/// Pool of independent environments stepped in parallel.
pub struct EnvPool {
    pub envs: Vec<GameEnv>,
    pub action_space: usize,
    pub error_policy: ErrorPolicy,
}

impl EnvPool {
    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        if let Some(msg) = panic.downcast_ref::<&str>() {
            (*msg).to_string()
        } else if let Some(msg) = panic.downcast_ref::<String>() {
            msg.clone()
        } else {
            "unknown panic".to_string()
        }
    }

    pub fn new(num_envs: usize, db: Arc<CardDb>, config: EnvConfig, curriculum: CurriculumConfig, seed: u64) -> Self {
        let replay_config = ReplayConfig::default();
        let mut envs = Vec::with_capacity(num_envs);
        for i in 0..num_envs {
            let env_seed = seed ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
            envs.push(GameEnv::new(db.clone(), config.clone(), curriculum.clone(), env_seed, replay_config.clone(), None));
        }
        debug_assert!(envs.iter().all(|e| e.config.error_policy == config.error_policy));
        Self { envs, action_space: ACTION_SPACE_SIZE, error_policy: config.error_policy }
    }

    pub fn reset_all(&mut self) -> StepBatchResult {
        let outcomes: Vec<StepOutcome> = self.envs.par_iter_mut().map(|env| env.reset_no_copy()).collect();
        self.pack_outcomes(outcomes)
    }

    pub fn reset_indices(&mut self, indices: &[usize]) -> StepBatchResult {
        let mut outcomes = Vec::with_capacity(self.envs.len());
        let mut reset_set = vec![false; self.envs.len()];
        for &idx in indices {
            if idx < reset_set.len() {
                reset_set[idx] = true;
            }
        }
        for (i, env) in self.envs.iter_mut().enumerate() {
            let outcome = if reset_set[i] {
                env.reset_no_copy()
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
            outcomes.push(outcome);
        }
        self.pack_outcomes(outcomes)
    }

    pub fn step_batch(&mut self, action_ids: &[u32]) -> Result<StepBatchResult> {
        if action_ids.len() != self.envs.len() {
            anyhow::bail!("Action batch size mismatch");
        }
        if self.envs.is_empty() {
            return Ok(self.pack_outcomes(Vec::new()));
        }
        let strict = self.error_policy == ErrorPolicy::Strict;
        let outcomes: Vec<StepOutcome> = if strict {
            let mut out = Vec::with_capacity(self.envs.len());
            for (env, &action_id) in self.envs.iter_mut().zip(action_ids.iter()) {
                let result = catch_unwind(AssertUnwindSafe(|| {
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
                }))
                .map_err(|panic| anyhow!("panic in env step: {}", Self::panic_message(panic)))?;
                let outcome = result?;
                out.push(outcome);
            }
            out
        } else {
            self.envs.par_iter_mut().zip(action_ids.par_iter()).map(|(env, &action_id)| {
                let result = catch_unwind(AssertUnwindSafe(|| {
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
                }));
                match result {
                    Ok(Ok(outcome)) => outcome,
                    Ok(Err(_)) | Err(_) => {
                        let acting_player = env.decision.as_ref().map(|d| d.player).unwrap_or(env.last_perspective);
                        env.last_engine_error = true;
                        env.last_perspective = acting_player;
                        env.state.terminal = Some(crate::state::TerminalResult::Win { winner: 1 - acting_player });
                        env.decision = None;
                        env.update_action_cache();
                        env.build_outcome_no_copy(env.terminal_reward_for(acting_player))
                    }
                }
            }).collect()
        };

        for env in &mut self.envs {
            if env.state.terminal.is_some() {
                env.finish_episode_replay();
            }
        }

        Ok(self.pack_outcomes(outcomes))
    }

    pub fn action_masks_batch(&self) -> Vec<u8> {
        let mut masks = vec![0u8; self.envs.len() * ACTION_SPACE_SIZE];
        for (i, env) in self.envs.iter().enumerate() {
            let offset = i * ACTION_SPACE_SIZE;
            masks[offset..offset + ACTION_SPACE_SIZE].copy_from_slice(&env.last_action_mask);
        }
        masks
    }

    pub fn legal_actions_batch(&self) -> Vec<Vec<ActionDesc>> {
        self.envs.iter().map(|env| {
            env.last_legal_actions.clone()
        }).collect()
    }

    pub fn get_current_player_batch(&self) -> Vec<i8> {
        self.envs.iter().map(|env| env.decision.as_ref().map(|d| d.player as i8).unwrap_or(-1)).collect()
    }

    pub fn render_ansi(&self, env_index: usize, perspective: u8) -> String {
        if env_index >= self.envs.len() {
            return "Invalid env index".to_string();
        }
        let env = &self.envs[env_index];
        let p0 = perspective as usize;
        let p1 = 1 - p0;
        let state = &env.state;
        let mut out = String::new();
        out.push_str(&format!("Phase: {:?}\n", state.turn.phase));
        out.push_str(&format!("Active: {}\n", state.turn.active_player));
        out.push_str(&format!("P{} Level: {} Clock: {} Hand: {} Deck: {}\n", p0, state.players[p0].level.len(), state.players[p0].clock.len(), state.players[p0].hand.len(), state.players[p0].deck.len()));
        out.push_str(&format!("P{} Level: {} Clock: {} Hand: {} Deck: {}\n", p1, state.players[p1].level.len(), state.players[p1].clock.len(), state.players[p1].hand.len(), state.players[p1].deck.len()));
        out.push_str("Stage:\n");
        out.push_str(&format!(" P{}: {:?}\n", p0, state.players[p0].stage));
        out.push_str(&format!(" P{}: {:?}\n", p1, state.players[p1].stage));
        if let Some(action) = &env.last_action_desc {
            out.push_str(&format!("Last action: {:?}\n", action));
        }
        out
    }

    pub fn set_curriculum(&mut self, curriculum: CurriculumConfig) {
        let mut curriculum = curriculum;
        curriculum.rebuild_cache();
        for env in &mut self.envs {
            env.curriculum = curriculum.clone();
        }
    }

    pub fn enable_replay_sampling(&mut self, config: ReplayConfig) -> Result<()> {
        let writer = if config.enabled { Some(ReplayWriter::new(&config)?) } else { None };
        for env in &mut self.envs {
            env.replay_config = config.clone();
            env.replay_writer = writer.clone();
        }
        Ok(())
    }

    fn pack_outcomes(&self, outcomes: Vec<StepOutcome>) -> StepBatchResult {
        let mut obs = vec![0i32; self.envs.len() * OBS_LEN];
        let mut rewards = vec![0.0f32; self.envs.len()];
        let mut terminated = vec![false; self.envs.len()];
        let mut truncated = vec![false; self.envs.len()];
        let mut infos = Vec::with_capacity(self.envs.len());

        for (i, outcome) in outcomes.into_iter().enumerate() {
            let offset = i * OBS_LEN;
            if outcome.obs.is_empty() {
                obs[offset..offset + OBS_LEN].copy_from_slice(&self.envs[i].obs_buf);
            } else {
                obs[offset..offset + OBS_LEN].copy_from_slice(&outcome.obs);
            }
            rewards[i] = outcome.reward;
            terminated[i] = outcome.terminated;
            truncated[i] = outcome.truncated;
            infos.push(outcome.info);
        }

        StepBatchResult { obs, rewards, terminated, truncated, infos }
    }
}
