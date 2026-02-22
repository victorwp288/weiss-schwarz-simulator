#[allow(clippy::too_many_arguments)]
fn parse_pool_init(
    num_envs: usize,
    db_path: Option<String>,
    deck_lists: Vec<Vec<u32>>,
    deck_ids: Option<Vec<u32>>,
    max_decisions: u32,
    max_ticks: u32,
    curriculum_json: Option<String>,
    reward_json: Option<String>,
    end_condition_policy_json: Option<String>,
    error_policy: Option<String>,
    observation_visibility: Option<String>,
    num_threads: Option<usize>,
    debug_fingerprint_every_n: u32,
    debug_event_ring_capacity: usize,
) -> PyResult<ParsedPoolInit> {
    if num_envs == 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "num_envs must be > 0",
        ));
    }
    let reward = parse_reward_config(reward_json)?;
    let end_condition_policy = parse_end_condition_policy(end_condition_policy_json)?;
    let curriculum = parse_curriculum_config(curriculum_json)?;
    let error_policy = parse_error_policy(error_policy)?;
    let visibility = parse_observation_visibility(observation_visibility)?;
    let (db, config) = build_env_config(
        db_path,
        deck_lists,
        deck_ids,
        max_decisions,
        max_ticks,
        reward,
        error_policy,
        visibility,
        end_condition_policy,
    )?;
    let debug = build_debug_config(
        Some(debug_fingerprint_every_n),
        Some(debug_event_ring_capacity),
    );
    Ok(ParsedPoolInit {
        db,
        config,
        curriculum,
        error_policy,
        visibility,
        num_threads: resolve_num_threads(num_envs, num_threads),
        debug,
    })
}

fn build_rl_pool(
    ctor: RlPoolCtor,
    num_envs: usize,
    seed: u64,
    init: ParsedPoolInit,
    output_masks: bool,
) -> PyResult<PyEnvPool> {
    let ParsedPoolInit {
        db,
        config,
        mut curriculum,
        error_policy,
        visibility,
        num_threads,
        debug,
    } = init;
    let mut pool = if visibility == ObservationVisibility::Public {
        match ctor {
            RlPoolCtor::Train => {
                EnvPool::new_rl_train(num_envs, db, config, curriculum, seed, num_threads, debug)
            }
            RlPoolCtor::Eval => {
                EnvPool::new_rl_eval(num_envs, db, config, curriculum, seed, num_threads, debug)
            }
        }
    } else {
        curriculum.enable_visibility_policies = true;
        curriculum.allow_concede = false;
        EnvPool::new_debug(num_envs, db, config, curriculum, seed, num_threads, debug)
    }
    .map_err(map_pool_init_error)?;
    // Align behavior across train/eval/debug paths: caller-provided policy always wins.
    pool.set_error_policy(error_policy);
    pool.set_output_mask_enabled(output_masks);
    Ok(PyEnvPool { pool })
}
