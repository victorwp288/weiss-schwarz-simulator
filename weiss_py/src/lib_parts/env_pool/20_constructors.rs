    /// Create a pool tuned for RL training defaults.
    ///
    /// Public visibility keeps hidden information masked by default. `deck_lists` is required and
    /// must contain exactly two decks.
    #[classmethod]
    #[pyo3(signature = (
        num_envs,
        db_path=None,
        deck_lists=None,
        deck_ids=None,
        max_decisions=2000,
        max_ticks=100_000,
        seed=0,
        curriculum_json=None,
        reward_json=None,
        end_condition_policy_json=None,
        error_policy=None,
        observation_visibility=None,
        num_threads=None,
        output_masks=true,
        debug_fingerprint_every_n=0,
        debug_event_ring_capacity=0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new_rl_train(
        _cls: &Bound<'_, PyType>,
        num_envs: usize,
        db_path: Option<String>,
        deck_lists: Option<Vec<Vec<u32>>>,
        deck_ids: Option<Vec<u32>>,
        max_decisions: u32,
        max_ticks: u32,
        seed: u64,
        curriculum_json: Option<String>,
        reward_json: Option<String>,
        end_condition_policy_json: Option<String>,
        error_policy: Option<String>,
        observation_visibility: Option<String>,
        num_threads: Option<usize>,
        output_masks: bool,
        debug_fingerprint_every_n: u32,
        debug_event_ring_capacity: usize,
    ) -> PyResult<Self> {
        let init = parse_pool_init(
            num_envs,
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            curriculum_json,
            reward_json,
            end_condition_policy_json,
            error_policy,
            observation_visibility,
            num_threads,
            debug_fingerprint_every_n,
            debug_event_ring_capacity,
        )?;
        build_rl_pool(RlPoolCtor::Train, num_envs, seed, init, output_masks)
    }

    /// Create a pool tuned for RL evaluation defaults.
    ///
    /// Mirrors training constructor semantics but uses evaluation stepping behavior in Rust.
    #[classmethod]
    #[pyo3(signature = (
        num_envs,
        db_path=None,
        deck_lists=None,
        deck_ids=None,
        max_decisions=2000,
        max_ticks=100_000,
        seed=0,
        curriculum_json=None,
        reward_json=None,
        end_condition_policy_json=None,
        error_policy=None,
        observation_visibility=None,
        num_threads=None,
        output_masks=true,
        debug_fingerprint_every_n=0,
        debug_event_ring_capacity=0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new_rl_eval(
        _cls: &Bound<'_, PyType>,
        num_envs: usize,
        db_path: Option<String>,
        deck_lists: Option<Vec<Vec<u32>>>,
        deck_ids: Option<Vec<u32>>,
        max_decisions: u32,
        max_ticks: u32,
        seed: u64,
        curriculum_json: Option<String>,
        reward_json: Option<String>,
        end_condition_policy_json: Option<String>,
        error_policy: Option<String>,
        observation_visibility: Option<String>,
        num_threads: Option<usize>,
        output_masks: bool,
        debug_fingerprint_every_n: u32,
        debug_event_ring_capacity: usize,
    ) -> PyResult<Self> {
        let init = parse_pool_init(
            num_envs,
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            curriculum_json,
            reward_json,
            end_condition_policy_json,
            error_policy,
            observation_visibility,
            num_threads,
            debug_fingerprint_every_n,
            debug_event_ring_capacity,
        )?;
        build_rl_pool(RlPoolCtor::Eval, num_envs, seed, init, output_masks)
    }

    /// Create a debug pool with explicit configuration knobs.
    ///
    /// Use this when you need direct control over visibility, error policy, and end-condition
    /// behavior without train/eval policy overrides.
    #[classmethod]
    #[pyo3(signature = (
        num_envs,
        db_path=None,
        deck_lists=None,
        deck_ids=None,
        max_decisions=2000,
        max_ticks=100_000,
        seed=0,
        curriculum_json=None,
        reward_json=None,
        end_condition_policy_json=None,
        error_policy=None,
        observation_visibility=None,
        num_threads=None,
        debug_fingerprint_every_n=0,
        debug_event_ring_capacity=0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new_debug(
        _cls: &Bound<'_, PyType>,
        num_envs: usize,
        db_path: Option<String>,
        deck_lists: Option<Vec<Vec<u32>>>,
        deck_ids: Option<Vec<u32>>,
        max_decisions: u32,
        max_ticks: u32,
        seed: u64,
        curriculum_json: Option<String>,
        reward_json: Option<String>,
        end_condition_policy_json: Option<String>,
        error_policy: Option<String>,
        observation_visibility: Option<String>,
        num_threads: Option<usize>,
        debug_fingerprint_every_n: u32,
        debug_event_ring_capacity: usize,
    ) -> PyResult<Self> {
        let init = parse_pool_init(
            num_envs,
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            curriculum_json,
            reward_json,
            end_condition_policy_json,
            error_policy,
            observation_visibility,
            num_threads,
            debug_fingerprint_every_n,
            debug_event_ring_capacity,
        )?;
        let pool = EnvPool::new_debug(
            num_envs,
            init.db,
            init.config,
            init.curriculum,
            seed,
            init.num_threads,
            init.debug,
        )
        .map_err(map_pool_init_error)?;
        Ok(Self { pool })
    }

