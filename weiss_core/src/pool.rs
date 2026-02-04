use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
}

/// Minimal RL batch output with i16 observations, filled in-place.
pub struct BatchOutMinimalI16<'a> {
    pub obs: &'a mut [i16],
    pub masks: &'a mut [u8],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
}

/// Minimal RL batch output with i16 observations and legal id lists, filled in-place.
pub struct BatchOutMinimalI16LegalIds<'a> {
    pub obs: &'a mut [i16],
    pub legal_ids: &'a mut [u16],
    pub legal_offsets: &'a mut [u32],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
}

/// Minimal RL batch output without masks, filled in-place.
pub struct BatchOutMinimalNoMask<'a> {
    pub obs: &'a mut [i32],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
}

/// Trajectory output with masks, filled in-place.
pub struct BatchOutTrajectory<'a> {
    pub obs: &'a mut [i32],
    pub masks: &'a mut [u8],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
    pub actions: &'a mut [u32],
}

/// Trajectory output with masks and i16 observations, filled in-place.
pub struct BatchOutTrajectoryI16<'a> {
    pub obs: &'a mut [i16],
    pub masks: &'a mut [u8],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
    pub actions: &'a mut [u32],
}

/// Trajectory output with i16 observations and legal id lists, filled in-place.
pub struct BatchOutTrajectoryI16LegalIds<'a> {
    pub obs: &'a mut [i16],
    pub legal_ids: &'a mut [u16],
    pub legal_offsets: &'a mut [u32],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
    pub actions: &'a mut [u32],
}

/// Trajectory output without masks, filled in-place.
pub struct BatchOutTrajectoryNoMask<'a> {
    pub obs: &'a mut [i32],
    pub rewards: &'a mut [f32],
    pub terminated: &'a mut [bool],
    pub truncated: &'a mut [bool],
    pub actor: &'a mut [i8],
    pub decision_kind: &'a mut [i8],
    pub decision_id: &'a mut [u32],
    pub engine_status: &'a mut [u8],
    pub spec_hash: &'a mut [u64],
    pub actions: &'a mut [u32],
}

/// Debug batch output, filled in-place.
pub struct BatchOutDebug<'a> {
    pub minimal: BatchOutMinimal<'a>,
    pub state_fingerprint: &'a mut [u64],
    pub events_fingerprint: &'a mut [u64],
    pub mask_fingerprint: &'a mut [u64],
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
    pub decision_kind: Vec<i8>,
    pub decision_id: Vec<u32>,
    pub engine_status: Vec<u8>,
    pub spec_hash: Vec<u64>,
}

/// Owned buffers for minimal output with i16 observations.
#[derive(Clone, Debug)]
pub struct BatchOutMinimalI16Buffers {
    pub obs: Vec<i16>,
    pub masks: Vec<u8>,
    pub rewards: Vec<f32>,
    pub terminated: Vec<bool>,
    pub truncated: Vec<bool>,
    pub actor: Vec<i8>,
    pub decision_kind: Vec<i8>,
    pub decision_id: Vec<u32>,
    pub engine_status: Vec<u8>,
    pub spec_hash: Vec<u64>,
}

impl BatchOutMinimalI16Buffers {
    pub fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0; num_envs * OBS_LEN],
            masks: vec![0u8; num_envs * ACTION_SPACE_SIZE],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![crate::encode::DECISION_KIND_NONE; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
        }
    }

    pub fn view_mut(&mut self) -> BatchOutMinimalI16<'_> {
        BatchOutMinimalI16 {
            obs: &mut self.obs,
            masks: &mut self.masks,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
        }
    }
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
            decision_kind: vec![crate::encode::DECISION_KIND_NONE; num_envs],
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
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
        }
    }
}

/// Owned buffers for minimal output without masks (Rust-side convenience).
#[derive(Clone, Debug)]
pub struct BatchOutMinimalNoMaskBuffers {
    pub obs: Vec<i32>,
    pub rewards: Vec<f32>,
    pub terminated: Vec<bool>,
    pub truncated: Vec<bool>,
    pub actor: Vec<i8>,
    pub decision_kind: Vec<i8>,
    pub decision_id: Vec<u32>,
    pub engine_status: Vec<u8>,
    pub spec_hash: Vec<u64>,
}

impl BatchOutMinimalNoMaskBuffers {
    pub fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0; num_envs * OBS_LEN],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![crate::encode::DECISION_KIND_NONE; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
        }
    }

    pub fn view_mut(&mut self) -> BatchOutMinimalNoMask<'_> {
        BatchOutMinimalNoMask {
            obs: &mut self.obs,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
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
    pub state_fingerprint: Vec<u64>,
    pub events_fingerprint: Vec<u64>,
    pub mask_fingerprint: Vec<u64>,
    pub event_counts: Vec<u16>,
    pub event_codes: Vec<u32>,
}

impl BatchOutDebugBuffers {
    pub fn new(num_envs: usize, event_capacity: usize) -> Self {
        Self {
            minimal: BatchOutMinimalBuffers::new(num_envs),
            state_fingerprint: vec![0; num_envs],
            events_fingerprint: vec![0; num_envs],
            mask_fingerprint: vec![0; num_envs],
            event_counts: vec![0; num_envs],
            event_codes: vec![0; num_envs * event_capacity],
        }
    }

    pub fn view_mut(&mut self) -> BatchOutDebug<'_> {
        BatchOutDebug {
            minimal: self.minimal.view_mut(),
            state_fingerprint: &mut self.state_fingerprint,
            events_fingerprint: &mut self.events_fingerprint,
            mask_fingerprint: &mut self.mask_fingerprint,
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
    output_mask_enabled: bool,
    output_mask_bits_enabled: bool,
    i16_clamp_enabled: bool,
    i16_overflow_counter_enabled: AtomicBool,
    i16_overflow_count: AtomicU64,
    thread_pool: Option<ThreadPool>,
    thread_pool_size: Option<usize>,
    engine_error_reset_count: u64,
    outcomes_scratch: Vec<StepOutcome>,
    reset_flags: Vec<bool>,
    reset_seed_scratch: Vec<Option<u64>>,
    legal_counts_scratch: Vec<usize>,
    debug_config: DebugConfig,
    debug_step_counter: u64,
}

fn empty_info() -> EnvInfo {
    EnvInfo {
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
    const PAR_CHUNK_SIZE: usize = 64;
    fn par_chunk_size(&self) -> usize {
        let Some(threads) = self.thread_pool_size else {
            return Self::PAR_CHUNK_SIZE;
        };
        let num_envs = self.envs.len().max(1);
        let target_chunks = threads.saturating_mul(4).max(1);
        let chunk = num_envs.div_ceil(target_chunks);
        chunk.clamp(8, 256)
    }
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

    fn ensure_legal_counts_scratch(&mut self) {
        let len = self.envs.len();
        if self.legal_counts_scratch.len() != len {
            self.legal_counts_scratch = vec![0usize; len];
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
        if let Err(err) = config.reward.validate_zero_sum() {
            anyhow::bail!("Invalid RewardConfig: {err}");
        }
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
            output_mask_enabled: true,
            output_mask_bits_enabled: true,
            i16_clamp_enabled: true,
            i16_overflow_counter_enabled: AtomicBool::new(false),
            i16_overflow_count: AtomicU64::new(0),
            thread_pool: None,
            thread_pool_size: None,
            engine_error_reset_count: 0,
            outcomes_scratch: Vec::new(),
            reset_flags: Vec::new(),
            reset_seed_scratch: Vec::new(),
            legal_counts_scratch: Vec::new(),
            debug_config: debug,
            debug_step_counter: 0,
        };
        if let Some(threads) = num_threads {
            if threads == 0 {
                anyhow::bail!("num_threads must be > 0");
            }
            let capped = threads.min(num_envs.max(1));
            if capped > 1 {
                pool.thread_pool = Some(ThreadPoolBuilder::new().num_threads(capped).build()?);
                pool.thread_pool_size = Some(capped);
            }
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

    fn fill_outcomes_for_flags(
        envs: &mut [GameEnv],
        outcomes: &mut [StepOutcome],
        flags: &[bool],
    ) -> Result<()> {
        if flags.len() != envs.len() || outcomes.len() != envs.len() {
            anyhow::bail!("reset flags size mismatch");
        }
        for ((slot, env), reset) in outcomes.iter_mut().zip(envs.iter_mut()).zip(flags.iter()) {
            *slot = if *reset {
                env.reset_no_copy()
            } else {
                env.clear_status_flags();
                env.build_outcome_no_copy(0.0)
            };
        }
        Ok(())
    }

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
            if idx < num_envs {
                self.reset_flags[idx] = true;
            }
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
            if idx < num_envs {
                self.reset_flags[idx] = true;
            }
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
            if idx < num_envs {
                self.reset_flags[idx] = true;
            }
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
            if idx < num_envs {
                self.reset_flags[idx] = true;
            }
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

    pub fn reset_done_into(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    pub fn reset_done_into_i16(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16(outcomes, out)
    }

    pub fn reset_done_into_i16_legal_ids(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
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

    pub fn reset_done_into_nomask(
        &mut self,
        done_mask: &[bool],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.ensure_outcomes_scratch();
        Self::fill_outcomes_for_flags(&mut self.envs, &mut self.outcomes_scratch, done_mask)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
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

    pub fn step_into(&mut self, action_ids: &[u32], out: &mut BatchOutMinimal<'_>) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out(outcomes, out)
    }

    pub fn step_into_i16(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_i16(outcomes, out)
    }

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

    pub fn step_into_nomask(
        &mut self,
        action_ids: &[u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.step_batch_outcomes(action_ids)?;
        let outcomes = &self.outcomes_scratch;
        self.fill_minimal_out_nomask(outcomes, out)
    }

    pub fn step_first_legal_into_i16_legal_ids(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }

    pub fn step_sample_legal_action_ids_uniform_into_i16_legal_ids(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_i16_legal_ids(actions, out)
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

    pub fn debug_event_ring_capacity(&self) -> usize {
        self.debug_config.event_ring_capacity
    }

    pub fn action_mask_bits_batch(&self) -> Vec<u64> {
        let words_per_env = crate::encode::ACTION_SPACE_WORDS;
        let mut bits = vec![0u64; self.envs.len() * words_per_env];
        self.action_mask_bits_batch_into(&mut bits)
            .expect("mask bits buffer size mismatch");
        bits
    }

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

    pub fn sample_legal_action_ids_uniform(&self, seeds: &[u64]) -> Result<Vec<u32>> {
        let mut out = vec![0u32; self.envs.len()];
        self.sample_legal_action_ids_uniform_into(seeds, &mut out)?;
        Ok(out)
    }

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
                            let mut guard = error_store.lock().expect("error store poisoned");
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
                let err = error_store.lock().expect("error store poisoned").take();
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
                            let mut guard = error_store.lock().expect("error store poisoned");
                            if guard.is_none() {
                                *guard = Some(anyhow!("no legal actions for env {idx}"));
                            }
                            return;
                        }
                        *slot = legal[0] as u32;
                    });
            });
            if error_flag.load(Ordering::Relaxed) {
                let err = error_store.lock().expect("error store poisoned").take();
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

    pub fn select_actions_from_logits_into(&self, logits: &[f32], out: &mut [u32]) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
        }
        if logits.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("logits buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let legal = env.action_ids_cache();
            if legal.is_empty() {
                anyhow::bail!("no legal actions for env {i}");
            }
            let base = i * ACTION_SPACE_SIZE;
            let mut best_id = legal[0] as u32;
            let mut best_logit = logits[base + best_id as usize];
            for &id_u16 in legal.iter().skip(1) {
                let id = id_u16 as usize;
                let logit = logits[base + id];
                if logit > best_logit {
                    best_logit = logit;
                    best_id = id_u16 as u32;
                }
            }
            out[i] = best_id;
        }
        Ok(())
    }

    pub fn sample_actions_from_logits_into(
        &self,
        logits: &[f32],
        seeds: &[u64],
        out: &mut [u32],
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if out.len() != num_envs {
            anyhow::bail!("output size mismatch");
        }
        if logits.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("logits buffer size mismatch");
        }
        if seeds.len() != num_envs {
            anyhow::bail!("seed buffer size mismatch");
        }
        for (i, env) in self.envs.iter().enumerate() {
            let legal = env.action_ids_cache();
            if legal.is_empty() {
                anyhow::bail!("no legal actions for env {i}");
            }
            let base = i * ACTION_SPACE_SIZE;
            let mut max_logit = f64::NEG_INFINITY;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                if logit > max_logit {
                    max_logit = logit;
                }
            }
            let mut total = 0.0f64;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                total += (logit - max_logit).exp();
            }
            if total <= 0.0 || !total.is_finite() {
                out[i] = legal[0] as u32;
                continue;
            }
            let u = (seeds[i] as f64) / (u64::MAX as f64);
            let mut threshold = u * total;
            for &id_u16 in legal.iter() {
                let logit = logits[base + id_u16 as usize] as f64;
                threshold -= (logit - max_logit).exp();
                if threshold <= 0.0 {
                    out[i] = id_u16 as u32;
                    break;
                }
            }
            if threshold > 0.0 {
                out[i] = *legal.last().unwrap() as u32;
            }
        }
        Ok(())
    }

    pub fn step_select_from_logits_into(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into(actions, out)
    }

    pub fn step_select_from_logits_into_i16(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into_i16(actions, out)
    }

    pub fn step_select_from_logits_into_nomask(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into_nomask(actions, out)
    }

    pub fn step_select_from_logits_into_i16_legal_ids(
        &mut self,
        logits: &[f32],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.select_actions_from_logits_into(logits, actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }

    pub fn step_sample_from_logits_into(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into(actions, out)
    }

    pub fn step_sample_from_logits_into_i16(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into_i16(actions, out)
    }

    pub fn step_sample_from_logits_into_nomask(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into_nomask(actions, out)
    }

    pub fn step_sample_from_logits_into_i16_legal_ids(
        &mut self,
        logits: &[f32],
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.sample_actions_from_logits_into(logits, seeds, actions)?;
        self.step_into_i16_legal_ids(actions, out)
    }

    pub fn step_first_legal_into(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into(actions, out)
    }

    pub fn step_first_legal_into_i16(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_i16(actions, out)
    }

    pub fn step_first_legal_into_nomask(
        &mut self,
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.first_legal_action_ids_into(actions)?;
        self.step_into_nomask(actions, out)
    }

    pub fn step_sample_legal_action_ids_uniform_into(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimal<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into(actions, out)
    }

    pub fn step_sample_legal_action_ids_uniform_into_i16(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalI16<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_i16(actions, out)
    }

    pub fn step_sample_legal_action_ids_uniform_into_nomask(
        &mut self,
        seeds: &[u64],
        actions: &mut [u32],
        out: &mut BatchOutMinimalNoMask<'_>,
    ) -> Result<()> {
        self.sample_legal_action_ids_uniform_into(seeds, actions)?;
        self.step_into_nomask(actions, out)
    }

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
                            let mut guard = error_store.lock().expect("error store poisoned");
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
                let err = error_store.lock().expect("error store poisoned").take();
                if let Some(err) = err {
                    return Err(err);
                }
            }
        }
        Ok(total)
    }

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
        if self.thread_pool.is_none() {
            offsets[0] = 0;
            let mut cursor = 0usize;
            for (i, env) in self.envs.iter().enumerate() {
                let legal = env.action_ids_cache();
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
        self.ensure_legal_counts_scratch();
        let counts = &mut self.legal_counts_scratch;
        if let Some(pool) = self.thread_pool.as_ref() {
            let envs = &self.envs;
            pool.install(|| {
                counts
                    .par_iter_mut()
                    .zip(envs.par_iter())
                    .for_each(|(slot, env)| {
                        *slot = env.action_ids_cache().len();
                    });
            });
        } else {
            for (slot, env) in counts.iter_mut().zip(self.envs.iter()) {
                *slot = env.action_ids_cache().len();
            }
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

    pub fn legal_actions_batch(&self) -> Vec<Vec<ActionDesc>> {
        self.envs.iter().map(|env| env.legal_actions()).collect()
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

    pub fn set_error_policy(&mut self, error_policy: ErrorPolicy) {
        self.error_policy = error_policy;
        for env in &mut self.envs {
            env.config.error_policy = error_policy;
        }
    }

    pub fn set_output_mask_enabled(&mut self, enabled: bool) {
        if self.output_mask_enabled == enabled {
            return;
        }
        self.output_mask_enabled = enabled;
        for env in &mut self.envs {
            env.set_output_mask_enabled(enabled);
            if enabled {
                env.update_action_cache();
            }
        }
    }

    pub fn set_output_mask_bits_enabled(&mut self, enabled: bool) {
        if self.output_mask_bits_enabled == enabled {
            return;
        }
        self.output_mask_bits_enabled = enabled;
        for env in &mut self.envs {
            env.set_output_mask_bits_enabled(enabled);
            if enabled {
                env.update_action_cache();
            }
        }
    }

    pub fn set_i16_clamp_enabled(&mut self, enabled: bool) {
        self.i16_clamp_enabled = enabled;
    }

    pub fn set_i16_overflow_counter_enabled(&self, enabled: bool) {
        self.i16_overflow_counter_enabled
            .store(enabled, Ordering::Relaxed);
    }

    pub fn i16_overflow_count(&self) -> u64 {
        self.i16_overflow_count.load(Ordering::Relaxed)
    }

    pub fn reset_i16_overflow_count(&self) {
        self.i16_overflow_count.store(0, Ordering::Relaxed);
    }

    pub fn config_hash(&self) -> u64 {
        self.envs
            .first()
            .map(|env| env.config.config_hash(&env.curriculum))
            .unwrap_or(0)
    }

    pub fn max_card_id(&self) -> u32 {
        self.envs
            .first()
            .map(|env| env.db.max_card_id())
            .unwrap_or(0)
    }

    pub fn episode_seed_batch(&self) -> Vec<u64> {
        self.envs.iter().map(|env| env.episode_seed).collect()
    }

    pub fn episode_index_batch(&self) -> Vec<u32> {
        self.envs.iter().map(|env| env.episode_index).collect()
    }

    pub fn env_index_batch(&self) -> Vec<u32> {
        self.envs.iter().map(|env| env.env_id).collect()
    }

    pub fn starting_player_batch(&self) -> Vec<u8> {
        self.envs
            .iter()
            .map(|env| env.state.turn.starting_player)
            .collect()
    }

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
            if idx < num_envs {
                self.reset_seed_scratch[idx] = Some(seed);
            }
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
            if idx < num_envs {
                self.reset_seed_scratch[idx] = Some(seed);
            }
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
            if idx < num_envs {
                self.reset_seed_scratch[idx] = Some(seed);
            }
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
            if idx < num_envs {
                self.reset_seed_scratch[idx] = Some(seed);
            }
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
            || out.decision_kind.len() != num_envs
            || out.decision_id.len() != num_envs
            || out.engine_status.len() != num_envs
            || out.spec_hash.len() != num_envs
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_minimal_out_i16(&self, out: &BatchOutMinimalI16<'_>) -> Result<()> {
        let num_envs = self.envs.len();
        if out.obs.len() != num_envs * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if self.output_mask_enabled {
            if out.masks.len() != num_envs * ACTION_SPACE_SIZE {
                anyhow::bail!("mask buffer size mismatch");
            }
        } else if !out.masks.is_empty() && out.masks.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("mask buffer size mismatch");
        }
        if out.rewards.len() != num_envs
            || out.terminated.len() != num_envs
            || out.truncated.len() != num_envs
            || out.actor.len() != num_envs
            || out.decision_kind.len() != num_envs
            || out.decision_id.len() != num_envs
            || out.engine_status.len() != num_envs
            || out.spec_hash.len() != num_envs
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_minimal_out_i16_legal_ids(
        &self,
        out: &BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        if out.obs.len() != num_envs * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.legal_ids.len() != num_envs * ACTION_SPACE_SIZE {
            anyhow::bail!("legal ids buffer size mismatch");
        }
        if out.legal_offsets.len() != num_envs + 1 {
            anyhow::bail!("legal offsets buffer size mismatch");
        }
        if out.rewards.len() != num_envs
            || out.terminated.len() != num_envs
            || out.truncated.len() != num_envs
            || out.actor.len() != num_envs
            || out.decision_kind.len() != num_envs
            || out.decision_id.len() != num_envs
            || out.engine_status.len() != num_envs
            || out.spec_hash.len() != num_envs
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_minimal_out_nomask(&self, out: &BatchOutMinimalNoMask<'_>) -> Result<()> {
        let num_envs = self.envs.len();
        if out.obs.len() != num_envs * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.rewards.len() != num_envs
            || out.terminated.len() != num_envs
            || out.truncated.len() != num_envs
            || out.actor.len() != num_envs
            || out.decision_kind.len() != num_envs
            || out.decision_id.len() != num_envs
            || out.engine_status.len() != num_envs
            || out.spec_hash.len() != num_envs
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_trajectory(&self, out: &BatchOutTrajectory<'_>, steps: usize) -> Result<()> {
        let num_envs = self.envs.len();
        let total = steps * num_envs;
        if out.obs.len() != total * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.masks.len() != total * ACTION_SPACE_SIZE {
            anyhow::bail!("mask buffer size mismatch");
        }
        if out.actions.len() != total {
            anyhow::bail!("action buffer size mismatch");
        }
        if out.rewards.len() != total
            || out.terminated.len() != total
            || out.truncated.len() != total
            || out.actor.len() != total
            || out.decision_kind.len() != total
            || out.decision_id.len() != total
            || out.engine_status.len() != total
            || out.spec_hash.len() != total
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_trajectory_i16(&self, out: &BatchOutTrajectoryI16<'_>, steps: usize) -> Result<()> {
        let num_envs = self.envs.len();
        let total = steps * num_envs;
        if out.obs.len() != total * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.masks.len() != total * ACTION_SPACE_SIZE {
            anyhow::bail!("mask buffer size mismatch");
        }
        if out.actions.len() != total {
            anyhow::bail!("action buffer size mismatch");
        }
        if out.rewards.len() != total
            || out.terminated.len() != total
            || out.truncated.len() != total
            || out.actor.len() != total
            || out.decision_kind.len() != total
            || out.decision_id.len() != total
            || out.engine_status.len() != total
            || out.spec_hash.len() != total
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_trajectory_i16_legal_ids(
        &self,
        out: &BatchOutTrajectoryI16LegalIds<'_>,
        steps: usize,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let total = steps * num_envs;
        if out.obs.len() != total * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.legal_ids.len() != total * ACTION_SPACE_SIZE {
            anyhow::bail!("legal ids buffer size mismatch");
        }
        if out.legal_offsets.len() != steps * (num_envs + 1) {
            anyhow::bail!("legal offsets buffer size mismatch");
        }
        if out.actions.len() != total {
            anyhow::bail!("action buffer size mismatch");
        }
        if out.rewards.len() != total
            || out.terminated.len() != total
            || out.truncated.len() != total
            || out.actor.len() != total
            || out.decision_kind.len() != total
            || out.decision_id.len() != total
            || out.engine_status.len() != total
            || out.spec_hash.len() != total
        {
            anyhow::bail!("scalar buffer size mismatch");
        }
        Ok(())
    }

    fn validate_trajectory_nomask(
        &self,
        out: &BatchOutTrajectoryNoMask<'_>,
        steps: usize,
    ) -> Result<()> {
        let num_envs = self.envs.len();
        let total = steps * num_envs;
        if out.obs.len() != total * OBS_LEN {
            anyhow::bail!("obs buffer size mismatch");
        }
        if out.actions.len() != total {
            anyhow::bail!("action buffer size mismatch");
        }
        if out.rewards.len() != total
            || out.terminated.len() != total
            || out.truncated.len() != total
            || out.actor.len() != total
            || out.decision_kind.len() != total
            || out.decision_id.len() != total
            || out.engine_status.len() != total
            || out.spec_hash.len() != total
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
            if self.output_mask_enabled {
                out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE]
                    .copy_from_slice(env.action_mask());
            }
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            out.actor[i] = if outcome.terminated || outcome.truncated {
                crate::encode::ACTOR_NONE
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.engine_status[i] = env.last_engine_error_code as u8;
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

    fn fill_minimal_out_i16(
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
            out.actor[i] = if outcome.terminated || outcome.truncated {
                crate::encode::ACTOR_NONE
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.engine_status[i] = env.last_engine_error_code as u8;
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

    fn fill_minimal_out_i16_legal_ids(
        &self,
        outcomes: &[StepOutcome],
        out: &mut BatchOutMinimalI16LegalIds<'_>,
    ) -> Result<()> {
        self.validate_minimal_out_i16_legal_ids(out)?;
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
            out.rewards[i] = outcome.reward;
            out.terminated[i] = outcome.terminated;
            out.truncated[i] = outcome.truncated;
            out.actor[i] = if outcome.terminated || outcome.truncated {
                crate::encode::ACTOR_NONE
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.engine_status[i] = env.last_engine_error_code as u8;
            out.spec_hash[i] = SPEC_HASH;
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

    fn fill_minimal_out_nomask(
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
            out.actor[i] = if outcome.terminated || outcome.truncated {
                crate::encode::ACTOR_NONE
            } else {
                outcome.info.actor
            };
            out.decision_kind[i] = outcome.info.decision_kind;
            out.decision_id[i] = env.decision_id();
            out.engine_status[i] = env.last_engine_error_code as u8;
            out.spec_hash[i] = SPEC_HASH;
            debug_assert!(
                out.terminated[i] || out.truncated[i] || (out.actor[i] == 0 || out.actor[i] == 1)
            );
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
