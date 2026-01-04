use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::config::{CurriculumConfig, EnvConfig, ErrorPolicy};
use crate::db::CardDb;
use crate::encode::{ACTION_SPACE_SIZE, OBS_LEN, SPEC_HASH};
use crate::env::{DebugConfig, EngineErrorCode, EnvInfo, GameEnv, StepOutcome};
use crate::legal::ActionDesc;
use crate::replay::{ReplayConfig, ReplayWriter};

/// Minimal RL batch output, filled in-place.
pub struct BatchOutMinimal<'a> {
    pub obs: &'a mut [i32],
    pub masks: &'a mut [u8],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
}

/// Debug batch output, filled in-place.
pub struct BatchOutDebug<'a> {
    pub minimal: BatchOutMinimal<'a>,
    pub decision_kind: &'a mut [i8],
    pub state_fingerprint: &'a mut [u64],
    pub events_fingerprint: &'a mut [u64],
    pub event_counts: &'a mut [u16],
    pub event_codes: &'a mut [u32],
}

/// Owned buffers for minimal output (Rust-side convenience).
#[derive(Clone, Debug)]
pub struct BatchOutMinimalBuffers {
    pub obs: Vec<i32>,
    pub masks: Vec<u8>,
    pub rewards: Vec<f32>,
    pub terminated: Vec<bool>,
    pub truncated: Vec<bool>,
    pub actor: Vec<i8>,
    pub decision_id: Vec<u32>,
    pub engine_status: Vec<u8>,
    pub spec_hash: Vec<u64>,
}

impl BatchOutMinimalBuffers {
    pub fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0; num_envs * OBS_LEN],
            masks: vec![0u8; num_envs * ACTION_SPACE_SIZE],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
        }
    }

    pub fn view_mut(&mut self) -> BatchOutMinimal<'_> {
        BatchOutMinimal {
            obs: &mut self.obs,
            masks: &mut self.masks,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
        }
    }
}

/// Owned buffers for debug output (Rust-side convenience).
#[derive(Clone, Debug)]
pub struct BatchOutDebugBuffers {
    pub minimal: BatchOutMinimalBuffers,
    pub decision_kind: Vec<i8>,
    pub state_fingerprint: Vec<u64>,
    pub events_fingerprint: Vec<u64>,
    pub event_counts: Vec<u16>,
    pub event_codes: Vec<u32>,
}

impl BatchOutDebugBuffers {
    pub fn new(num_envs: usize, event_capacity: usize) -> Self {
        Self {
            minimal: BatchOutMinimalBuffers::new(num_envs),
            decision_kind: vec![0; num_envs],
            state_fingerprint: vec![0; num_envs],
            events_fingerprint: vec![0; num_envs],
            event_counts: vec![0; num_envs],
            event_codes: vec![0; num_envs * event_capacity],
        }
    }

    pub fn view_mut(&mut self) -> BatchOutDebug<'_> {
        BatchOutDebug {
            minimal: self.minimal.view_mut(),
            decision_kind: &mut self.decision_kind,
            state_fingerprint: &mut self.state_fingerprint,
            events_fingerprint: &mut self.events_fingerprint,
            event_counts: &mut self.event_counts,
            event_codes: &mut self.event_codes,
        }
    }
}

/// Pool of independent environments stepped in parallel.
pub struct EnvPool {
    pub envs: Vec<GameEnv>,
    pub action_space: usize,
    pub error_policy: ErrorPolicy,
    thread_pool: Option<ThreadPool>,
    engine_error_reset_count: u64,
    outcomes_scratch: Vec<StepOutcome>,
    debug_config: DebugConfig,
    debug_step_counter: u64,
}

fn empty_info() -> EnvInfo {
    EnvInfo {
        obs_version: 0,
        action_version: 0,
        decision_kind: -1,
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
    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        if let Some(msg) = panic.downcast_ref::<&str>() {
            (*msg).to_string()
        } else if let Some(msg) = panic.downcast_ref::<String>() {
            msg.clone()
        } else {
            "unknown panic".to_string()
        }
    }

    fn ensure_outcomes_scratch(&mut self) {
        let len = self.envs.len();
        if self.outcomes_scratch.len() != len {
            self.outcomes_scratch = (0..len).map(|_| empty_outcome()).collect();
        }
    }

    fn new_internal(
        num_envs: usize,
        db: Arc<CardDb>,
        config: EnvConfig,
        curriculum: CurriculumConfig,
        seed: u64,
        num_threads: Option<usize>,
        debug: DebugConfig,
    ) -> Result<Self> {
        let replay_config = ReplayConfig::default();
        let mut envs = Vec::with_capacity(num_envs);
        for i in 0..num_envs {
            let env_seed = seed ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
            let mut env = GameEnv::new(
                db.clone(),
                config.clone(),
                curriculum.clone(),
                env_seed,
                replay_config.clone(),
                None,
                i as u32,
            );
            env.set_debug_config(debug);
            envs.push(env);
        }
        debug_assert!(envs
            .iter()
            .all(|e| e.config.error_policy == config.error_policy));
        let mut pool = Self {
            envs,
            action_space: ACTION_SPACE_SIZE,
            error_policy: config.error_policy,
            thread_pool: None,
            engine_error_reset_count: 0,
            outcomes_scratch: Vec::new(),
            debug_config: debug,
            debug_step_counter: 0,
        };
        if let Some(threads) = num_threads {
            if threads == 0 {
                anyhow::bail!("num_threads must be > 0");
            }
            pool.thread_pool = Some(ThreadPoolBuilder::new().num_threads(threads).build()?);
        }
        Ok(pool)
    }

    pub fn new_rl_train(
        num_envs: usize,
        db: Arc<CardDb>,
        mut config: EnvConfig,
        mut curriculum: CurriculumConfig,
        seed: u64,
        num_threads: Option<usize>,
        debug: DebugConfig,
    ) -> Result<Self> {
        config.observation_visibility = crate::config::ObservationVisibility::Public;
        config.error_policy = ErrorPolicy::LenientTerminate;
        curriculum.enable_visibility_policies = true;
        curriculum.allow_concede = false;
        Self::new_internal(num_envs, db, config, curriculum, seed, num_threads, debug)
    }

    pub fn new_rl_eval(
        num_envs: usize,
        db: Arc<CardDb>,
        mut config: EnvConfig,
        mut curriculum: CurriculumConfig,
        seed: u64,
        num_threads: Option<usize>,
        debug: DebugConfig,
    ) -> Result<Self> {
        config.observation_visibility = crate::config::ObservationVisibility::Public;
        config.error_policy = ErrorPolicy::LenientTerminate;
        curriculum.enable_visibility_policies = true;
        curriculum.allow_concede = false;
        Self::new_internal(num_envs, db, config, curriculum, seed, num_threads, debug)
    }

    pub fn new_debug(
        num_envs: usize,
        db: Arc<CardDb>,
        config: EnvConfig,
        curriculum: CurriculumConfig,
        seed: u64,
        num_threads: Option<usize>,
        debug: DebugConfig,
    ) -> Result<Self> {
        Self::new_internal(num_envs, db, config, curriculum, seed, num_threads, debug)
    }

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

    pub fn reset_indices_into(
        &mut self,
        indices: &[usize],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.ensure_outcomes_scratch();
        let mut reset_set = vec![false; self.envs.len()];
        for &idx in indices {
            if idx < reset_set.len() {
                reset_set[idx] = true;
            }
        }
        for ((slot, env), reset) in self
            .outcomes_scratch
            .iter_mut()
            .zip(self.envs.iter_mut())
            .zip(reset_set.into_iter())
        {
            *slot = if reset {
                env.reset_no_copy()
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
        }
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    pub fn reset_done_into(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        if done_mask.len() != self.envs.len() {
            anyhow::bail!("Done mask size mismatch");
        }
        let indices: Vec<usize> = done_mask
            .iter()
            .enumerate()
            .filter_map(|(i, done)| if *done { Some(i) } else { None })
            .collect();
        if indices.is_empty() {
            return self.reset_indices_into(&[], out);
        }
        self.reset_indices_into(&indices, out)
    }

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
        } else if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &mut self.envs;
            let outcomes = &mut self.outcomes_scratch;
            pool.install(|| {
                outcomes
                    .par_iter_mut()
                    .zip(envs.par_iter_mut())
                    .zip(action_ids.par_iter())
                    .for_each(|((slot, env), &action_id)| {
                        *slot = step_lenient(env, action_id);
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

    pub fn step_into(&mut self, action_ids: &[u32], out: &mut BatchOutMinimal<'_>) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

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

    pub fn reset_debug_into(&mut self, out: &mut BatchOutDebug<'_>) -> Result<()> {
        self.reset_into(&mut out.minimal)?;
        let compute_fingerprints = self.debug_compute_fingerprints();
        let outcomes = &self.outcomes_scratch;
        self.fill_debug_out(outcomes, out, compute_fingerprints)
    }

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

    pub fn reset_done_debug_into(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutDebug<'_>,
    ) -> Result<()> {
        self.reset_done_into(done_mask, &mut out.minimal)?;
        let compute_fingerprints = self.debug_compute_fingerprints();
        let outcomes = &self.outcomes_scratch;
        self.fill_debug_out(outcomes, out, compute_fingerprints)
    }

    fn debug_compute_fingerprints(&mut self) -> bool {
        if self.debug_config.fingerprint_every_n == 0 {
            return false;
        }
        self.debug_step_counter = self.debug_step_counter.wrapping_add(1);
        self.debug_step_counter
            .is_multiple_of(self.debug_config.fingerprint_every_n as u64)
    }

    pub fn set_debug_config(&mut self, debug: DebugConfig) {
        self.debug_config = debug;
        for env in &mut self.envs {
            env.set_debug_config(debug);
        }
    }

    pub fn state_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| crate::fingerprint::state_fingerprint(&env.state))
            .collect()
    }

    pub fn engine_error_reset_count(&self) -> u64 {
        self.engine_error_reset_count
    }

    pub fn reset_engine_error_reset_count(&mut self) {
        self.engine_error_reset_count = 0;
    }

    pub fn auto_reset_on_error_codes_into(
        &mut self,
        codes: &[u8],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<usize> {
        if codes.len() != self.envs.len() {
            anyhow::bail!("Error code batch size mismatch");
        }
        let mut indices = Vec::new();
        for (idx, &code) in codes.iter().enumerate() {
            if code != 0 {
                indices.push(idx);
            }
        }
        if indices.is_empty() {
            return Ok(0);
        }
        let reset_count = indices.len() as u64;
        self.reset_indices_into(&indices, out)?;
        self.engine_error_reset_count = self.engine_error_reset_count.saturating_add(reset_count);
        Ok(indices.len())
    }

    pub fn events_fingerprint_batch(&self) -> Vec<u64> {
        self.envs
            .iter()
            .map(|env| crate::fingerprint::events_fingerprint(env.canonical_events()))
            .collect()
    }

    pub fn action_masks_batch(&self) -> Vec<u8> {
        let mut masks = vec![0u8; self.envs.len() * ACTION_SPACE_SIZE];
        self.action_masks_batch_into(&mut masks)
            .expect("mask buffer size mismatch");
        masks
    }

    pub fn action_masks_batch_into(&self, masks: &mut [u8]) -> Result<()> {
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

    pub fn legal_action_ids_batch_into(
        &self,
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
        offsets[0] = 0;
        let mut total = 0usize;
        for (i, env) in self.envs.iter().enumerate() {
            let mut count = 0usize;
            for &value in env.action_mask().iter() {
                if value != 0 {
                    count += 1;
                }
            }
            total = total.saturating_add(count);
            if total > ids.len() {
                anyhow::bail!("ids buffer size mismatch");
            }
            offsets[i + 1] = total as u32;
        }
        let mut cursor = 0usize;
        for (i, env) in self.envs.iter().enumerate() {
            for (action_id, &value) in env.action_mask().iter().enumerate() {
                if value != 0 {
                    ids[cursor] = action_id as u16;
                    cursor += 1;
                }
            }
            debug_assert_eq!(cursor, offsets[i + 1] as usize);
        }
        Ok(total)
    }

    pub fn legal_actions_batch(&self) -> Vec<Vec<ActionDesc>> {
        self.envs
            .iter()
            .map(|env| env.legal_actions().to_vec())
            .collect()
    }

    pub fn get_current_player_batch(&self) -> Vec<i8> {
        self.envs
            .iter()
            .map(|env| env.decision.as_ref().map(|d| d.player as i8).unwrap_or(-1))
            .collect()
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

        out.push_str("Stage:\n");
        out.push_str(&format!(
            " P{}: {}\n",
            p0,
            format_stage(&state.players[p0].stage)
        ));
        out.push_str(&format!(
            " P{}: {}\n",
            p1,
            format_stage(&state.players[p1].stage)
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

    pub fn set_curriculum(&mut self, curriculum: CurriculumConfig) {
        let mut curriculum = curriculum;
        curriculum.rebuild_cache();
        for env in &mut self.envs {
            env.curriculum = curriculum.clone();
        }
    }

    pub fn enable_replay_sampling(&mut self, config: ReplayConfig) -> Result<()> {
        let mut config = config;
        config.rebuild_cache();
        let writer = if config.enabled {
            Some(ReplayWriter::new(&config)?)
        } else {
            None
        };
        for env in &mut self.envs {
            env.replay_config = config.clone();
            env.replay_writer = writer.clone();
        }
        Ok(())
    }

    fn validate_minimal_out(&self, out: &BatchOutMinimal<'_>) -> Result<()> {
        let num_envs = self.envs.len();
        if out.obs.len() != num_envs * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.masks.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("mask buffer size mismatch");
        }
        if out.rewards.len() != num_envs
            || out.terminated.len() != num_envs
            || out.truncated.len() != num_envs
            || out.actor.len() != num_envs
            || out.decision_id.len() != num_envs
            || out.engine_status.len() != num_envs
            || out.spec_hash.len() != num_envs
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn fill_minimal_out(
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
            out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE]
                .copy_from_slice(env.action_mask());
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            out.actor[i] = outcome.info.actor;
            out.decision_id[i] = env.decision_id();
            out.engine_status[i] = env.last_engine_error_code as u8;
            out.spec_hash[i] = SPEC_HASH;
        }
        Ok(())
    }

    fn fill_debug_out(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutDebug<'_>,
        compute_fingerprints: bool,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if out.decision_kind.len() != num_envs
            || out.state_fingerprint.len() != num_envs
            || out.events_fingerprint.len() != num_envs
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
            out.decision_kind[i] = outcome.info.decision_kind;
            if compute_fingerprints {
                out.state_fingerprint[i] = crate::fingerprint::state_fingerprint(&env.state);
                out.events_fingerprint[i] =
                    crate::fingerprint::events_fingerprint(env.canonical_events());
            } else {
                out.state_fingerprint[i] = 0;
                out.events_fingerprint[i] = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EnvConfig, ObservationVisibility, RewardConfig};
    use crate::db::{CardColor, CardDb, CardStatic, CardType};
    use std::sync::Arc;

    fn make_db() -> Arc<CardDb> {
        let mut cards = Vec::new();
        for id in 1..=13u32 {
            cards.push(CardStatic {
                id,
                card_set: None,
                card_type: CardType::Character,
                color: CardColor::Red,
                level: 0,
                cost: 0,
                power: 500,
                soul: 1,
                triggers: vec![],
                traits: vec![],
                abilities: vec![],
                ability_defs: vec![],
                counter_timing: false,
                raw_text: None,
            });
        }
        Arc::new(CardDb::new(cards).expect("db build"))
    }

    fn make_deck() -> Vec<u32> {
        let mut deck = Vec::new();
        for id in 1..=12u32 {
            deck.extend(std::iter::repeat_n(id, 4));
        }
        deck.extend(std::iter::repeat_n(13u32, 2));
        assert_eq!(deck.len(), 50);
        deck
    }

    fn make_config(deck: Vec<u32>) -> EnvConfig {
        EnvConfig {
            deck_lists: [deck.clone(), deck],
            deck_ids: [1, 2],
            max_decisions: 10,
            max_ticks: 100,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        }
    }

    #[test]
    fn thread_pool_is_per_env_pool() {
        let db = make_db();
        let config = make_config(make_deck());
        let curriculum = CurriculumConfig::default();
        let pool = EnvPool::new_debug(
            2,
            db,
            config,
            curriculum,
            7,
            Some(2),
            DebugConfig::default(),
        )
        .expect("pool");
        assert_eq!(pool.envs.len(), 2);
        assert!(pool.thread_pool.is_some());
        assert_eq!(pool.thread_pool.as_ref().unwrap().current_num_threads(), 2);
    }

    #[test]
    fn reset_indices_with_masks_matches_action_masks() {
        let db = make_db();
        let config = make_config(make_deck());
        let curriculum = CurriculumConfig::default();
        let mut pool =
            EnvPool::new_debug(2, db, config, curriculum, 11, None, DebugConfig::default())
                .expect("pool");
        let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
        let _ = pool.reset_into(&mut out.view_mut());

        let mut reset_out = BatchOutMinimalBuffers::new(pool.envs.len());
        let _ = pool.reset_indices_into(&[0], &mut reset_out.view_mut());
        let masks_snapshot = reset_out.masks.clone();
        let masks = pool.action_masks_batch();
        assert_eq!(
            masks_snapshot.as_slice(),
            masks.as_slice(),
            "mask scratch must match action_masks_batch"
        );
    }

    #[test]
    fn legal_action_ids_match_action_masks() {
        let db = make_db();
        let config = make_config(make_deck());
        let curriculum = CurriculumConfig::default();
        let mut pool =
            EnvPool::new_debug(2, db, config, curriculum, 13, None, DebugConfig::default())
                .expect("pool");
        let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
        let _ = pool.reset_into(&mut out.view_mut());

        let num_envs = pool.envs.len();
        let mut ids = vec![0u16; num_envs * ACTION_SPACE_SIZE];
        let mut offsets = vec![0u32; num_envs + 1];
        let total = pool
            .legal_action_ids_batch_into(&mut ids, &mut offsets)
            .expect("ids");
        assert!(total <= ids.len());

        for env_idx in 0..num_envs {
            let start = offsets[env_idx] as usize;
            let end = offsets[env_idx + 1] as usize;
            let mask_offset = env_idx * ACTION_SPACE_SIZE;
            let mask = &out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE];
            let mut expected = Vec::new();
            for (action_id, &value) in mask.iter().enumerate() {
                if value != 0 {
                    expected.push(action_id as u16);
                }
            }
            assert_eq!(&ids[start..end], expected.as_slice());
        }
    }

    #[test]
    fn engine_error_reset_count_tracks_auto_resets() {
        let db = make_db();
        let config = make_config(make_deck());
        let curriculum = CurriculumConfig::default();
        let mut pool =
            EnvPool::new_debug(2, db, config, curriculum, 9, None, DebugConfig::default())
                .expect("pool");
        let mut out = BatchOutMinimalBuffers::new(pool.envs.len());

        assert_eq!(pool.engine_error_reset_count(), 0);
        let codes = vec![1u8, 0u8];
        let reset = pool
            .auto_reset_on_error_codes_into(&codes, &mut out.view_mut())
            .expect("auto reset");
        assert_eq!(reset, 1);
        assert_eq!(pool.engine_error_reset_count(), 1);

        pool.reset_engine_error_reset_count();
        assert_eq!(pool.engine_error_reset_count(), 0);
    }
}
