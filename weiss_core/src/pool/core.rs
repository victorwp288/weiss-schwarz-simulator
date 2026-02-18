use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use rayon::ThreadPool;

use crate::config::{CurriculumConfig, EnvConfig, ErrorPolicy};
use crate::db::CardDb;
use crate::encode::ACTION_SPACE_SIZE;
use crate::env::{DebugConfig, GameEnv, StepOutcome};
use crate::replay::{ReplayConfig, ReplayWriter};

use super::threading;

/// Pool of independent environments stepped in parallel.
///
/// # Examples
/// ```no_run
/// use std::sync::Arc;
/// use weiss_core::{
///     BatchOutMinimalBuffers, CardDb, CurriculumConfig, DebugConfig, EnvConfig, EnvPool,
/// };
///
/// # let db: CardDb = todo!("load card db");
/// # let config: EnvConfig = todo!("build env config");
/// let mut pool = EnvPool::new_rl_train(
///     8,
///     Arc::new(db),
///     config,
///     CurriculumConfig::default(),
///     0,
///     None,
///     DebugConfig::default(),
/// )?;
/// let mut buffers = BatchOutMinimalBuffers::new(pool.envs.len());
/// let mut out = buffers.view_mut();
/// pool.reset_into(&mut out)?;
///
/// let actions = vec![weiss_core::encode::PASS_ACTION_ID as u32; pool.envs.len()];
/// pool.step_into(&actions, &mut out)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct EnvPool {
    /// Backing environments (one per slot).
    pub envs: Vec<GameEnv>,
    /// Fixed action space size used by all envs.
    pub action_space: usize,
    /// Error policy applied during stepping.
    pub error_policy: ErrorPolicy,
    pub(super) output_mask_enabled: bool,
    pub(super) output_mask_bits_enabled: bool,
    pub(super) i16_clamp_enabled: bool,
    pub(super) i16_overflow_counter_enabled: AtomicBool,
    pub(super) i16_overflow_count: AtomicU64,
    pub(super) thread_pool: Option<ThreadPool>,
    pub(super) thread_pool_size: Option<usize>,
    pub(super) engine_error_reset_count: u64,
    pub(super) outcomes_scratch: Vec<StepOutcome>,
    pub(super) reset_flags: Vec<bool>,
    pub(super) reset_seed_scratch: Vec<Option<u64>>,
    pub(super) legal_counts_scratch: Vec<usize>,
    pub(super) debug_config: DebugConfig,
    pub(super) debug_step_counter: u64,
    pub(super) template_db: Arc<CardDb>,
    pub(super) template_config: EnvConfig,
    pub(super) template_curriculum: CurriculumConfig,
    pub(super) template_replay_config: ReplayConfig,
    pub(super) template_replay_writer: Option<ReplayWriter>,
    pub(super) pool_seed: u64,
}

impl EnvPool {
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
        config.validate_with_db(&db).map_err(anyhow::Error::from)?;
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
            )
            .map_err(anyhow::Error::from)?;
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
            template_db: db,
            template_config: config,
            template_curriculum: curriculum,
            template_replay_config: replay_config,
            template_replay_writer: None,
            pool_seed: seed,
        };
        let (thread_pool, thread_pool_size) = threading::build_thread_pool(num_threads, num_envs)?;
        pool.thread_pool = thread_pool;
        pool.thread_pool_size = thread_pool_size;
        Ok(pool)
    }

    /// Create a pool configured for RL training (public visibility + lenient errors).
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

    /// Create a pool configured for RL evaluation (public visibility).
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

    /// Create a pool with explicit config and curriculum.
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

    /// Update debug settings for all envs in the pool.
    pub fn set_debug_config(&mut self, debug: DebugConfig) {
        self.debug_config = debug;
        for env in &mut self.envs {
            env.set_debug_config(debug);
        }
    }

    /// Count of auto-resets triggered by engine errors.
    pub fn engine_error_reset_count(&self) -> u64 {
        self.engine_error_reset_count
    }

    /// Effective thread count used by this pool (1 when running serially).
    pub fn effective_num_threads(&self) -> usize {
        self.thread_pool_size.unwrap_or(1)
    }

    /// Replace curriculum settings for all envs in the pool.
    pub fn set_curriculum(&mut self, curriculum: CurriculumConfig) {
        let mut curriculum = curriculum;
        curriculum.rebuild_cache();
        self.template_curriculum = curriculum.clone();
        for env in &mut self.envs {
            env.curriculum = curriculum.clone();
        }
    }

    /// Update error policy for all envs in the pool.
    pub fn set_error_policy(&mut self, error_policy: ErrorPolicy) {
        self.error_policy = error_policy;
        self.template_config.error_policy = error_policy;
        for env in &mut self.envs {
            env.config.error_policy = error_policy;
        }
    }

    /// Enable or disable output action masks for all envs.
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

    /// Enable or disable output action mask bits for all envs.
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

    /// Enable or disable i16 clamping for i16 output buffers.
    pub fn set_i16_clamp_enabled(&mut self, enabled: bool) {
        self.i16_clamp_enabled = enabled;
    }

    /// Enable or disable counting of i16 overflows.
    pub fn set_i16_overflow_counter_enabled(&self, enabled: bool) {
        self.i16_overflow_counter_enabled
            .store(enabled, Ordering::Relaxed);
    }

    /// Total count of i16 clamp overflows since last reset.
    pub fn i16_overflow_count(&self) -> u64 {
        self.i16_overflow_count.load(Ordering::Relaxed)
    }

    /// Stable hash of the pool's config and curriculum.
    pub fn config_hash(&self) -> u64 {
        self.envs
            .first()
            .map(|env| env.config.config_hash(&env.curriculum))
            .unwrap_or(0)
    }

    /// Maximum card id available in the underlying database.
    pub fn max_card_id(&self) -> u32 {
        self.envs
            .first()
            .map(|env| env.db.max_card_id())
            .unwrap_or(0)
    }

    /// Episode seeds for each env in the pool.
    pub fn episode_seed_batch(&self) -> Vec<u64> {
        self.envs.iter().map(|env| env.episode_seed).collect()
    }

    /// Episode indices for each env in the pool.
    pub fn episode_index_batch(&self) -> Vec<u32> {
        self.envs.iter().map(|env| env.episode_index).collect()
    }

    /// Environment indices for each env in the pool.
    pub fn env_index_batch(&self) -> Vec<u32> {
        self.envs.iter().map(|env| env.env_id).collect()
    }

    /// Starting player for each env in the pool.
    pub fn starting_player_batch(&self) -> Vec<u8> {
        self.envs
            .iter()
            .map(|env| env.state.turn.starting_player)
            .collect()
    }

    /// Decision counts for each env in the pool.
    pub fn decision_count_batch(&self) -> Vec<u32> {
        self.envs
            .iter()
            .map(|env| env.state.turn.decision_count)
            .collect()
    }

    /// Tick counts for each env in the pool.
    pub fn tick_count_batch(&self) -> Vec<u32> {
        self.envs
            .iter()
            .map(|env| env.state.turn.tick_count)
            .collect()
    }

    /// Enable replay sampling for all envs in the pool.
    pub fn enable_replay_sampling(&mut self, config: ReplayConfig) -> Result<()> {
        let mut config = config;
        config.rebuild_cache();
        let writer = if config.enabled {
            Some(ReplayWriter::new(&config)?)
        } else {
            None
        };
        self.template_replay_config = config.clone();
        self.template_replay_writer = writer.clone();
        for env in &mut self.envs {
            env.replay_config = config.clone();
            env.replay_writer = writer.clone();
        }
        Ok(())
    }
}
