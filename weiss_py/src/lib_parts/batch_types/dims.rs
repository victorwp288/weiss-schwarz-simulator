fn ensure_batch_out_minimal_dims(
    py: Python<'_>,
    out: &PyBatchOutMinimal,
    num_envs: usize,
) -> PyResult<()> {
    ensure_first_dim(py, "obs", &out.obs, num_envs, Some(OBS_LEN))?;
    ensure_first_dim(py, "masks", &out.masks, num_envs, Some(ACTION_SPACE_SIZE))?;
    ensure_first_dim(py, "rewards", &out.rewards, num_envs, None)?;
    ensure_first_dim(py, "terminated", &out.terminated, num_envs, None)?;
    ensure_first_dim(py, "truncated", &out.truncated, num_envs, None)?;
    ensure_first_dim(py, "actor", &out.actor, num_envs, None)?;
    ensure_first_dim(py, "decision_kind", &out.decision_kind, num_envs, None)?;
    ensure_first_dim(py, "decision_id", &out.decision_id, num_envs, None)?;
    ensure_first_dim(py, "engine_status", &out.engine_status, num_envs, None)?;
    ensure_first_dim(py, "spec_hash", &out.spec_hash, num_envs, None)?;
    ensure_first_dim(py, "main_move_action", &out.main_move_action, num_envs, None)?;
    ensure_first_dim(py, "main_pass_action", &out.main_pass_action, num_envs, None)?;
    Ok(())
}

fn ensure_batch_out_minimal_i16_dims(
    py: Python<'_>,
    out: &PyBatchOutMinimalI16,
    num_envs: usize,
) -> PyResult<()> {
    ensure_first_dim(py, "obs", &out.obs, num_envs, Some(OBS_LEN))?;
    ensure_first_dim(py, "masks", &out.masks, num_envs, Some(ACTION_SPACE_SIZE))?;
    ensure_first_dim(py, "rewards", &out.rewards, num_envs, None)?;
    ensure_first_dim(py, "terminated", &out.terminated, num_envs, None)?;
    ensure_first_dim(py, "truncated", &out.truncated, num_envs, None)?;
    ensure_first_dim(py, "actor", &out.actor, num_envs, None)?;
    ensure_first_dim(py, "decision_kind", &out.decision_kind, num_envs, None)?;
    ensure_first_dim(py, "decision_id", &out.decision_id, num_envs, None)?;
    ensure_first_dim(py, "engine_status", &out.engine_status, num_envs, None)?;
    ensure_first_dim(py, "spec_hash", &out.spec_hash, num_envs, None)?;
    ensure_first_dim(py, "main_move_action", &out.main_move_action, num_envs, None)?;
    ensure_first_dim(py, "main_pass_action", &out.main_pass_action, num_envs, None)?;
    Ok(())
}

fn ensure_batch_out_minimal_i16_legal_ids_dims(
    py: Python<'_>,
    out: &PyBatchOutMinimalI16LegalIds,
    num_envs: usize,
) -> PyResult<()> {
    ensure_action_space_u16()?;
    ensure_first_dim(py, "obs", &out.obs, num_envs, Some(OBS_LEN))?;
    let expected_legal_ids = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "legal_ids size overflow (num_envs * action_space)",
        )
    })?;
    ensure_first_dim(py, "legal_ids", &out.legal_ids, expected_legal_ids, None)?;
    ensure_first_dim(
        py,
        "legal_action_meta",
        &out.legal_action_meta,
        expected_legal_ids,
        Some(ACTION_META_WIDTH),
    )?;
    let expected_offsets = num_envs.checked_add(1).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "legal_offsets size overflow (num_envs + 1)",
        )
    })?;
    ensure_first_dim(
        py,
        "legal_offsets",
        &out.legal_offsets,
        expected_offsets,
        None,
    )?;
    ensure_first_dim(py, "rewards", &out.rewards, num_envs, None)?;
    ensure_first_dim(py, "terminated", &out.terminated, num_envs, None)?;
    ensure_first_dim(py, "truncated", &out.truncated, num_envs, None)?;
    ensure_first_dim(py, "actor", &out.actor, num_envs, None)?;
    ensure_first_dim(py, "decision_kind", &out.decision_kind, num_envs, None)?;
    ensure_first_dim(py, "decision_id", &out.decision_id, num_envs, None)?;
    ensure_first_dim(py, "engine_status", &out.engine_status, num_envs, None)?;
    ensure_first_dim(py, "spec_hash", &out.spec_hash, num_envs, None)?;
    ensure_first_dim(py, "main_move_action", &out.main_move_action, num_envs, None)?;
    ensure_first_dim(py, "main_pass_action", &out.main_pass_action, num_envs, None)?;
    Ok(())
}

fn ensure_batch_out_minimal_i16_legal_ids_nometa_dims(
    py: Python<'_>,
    out: &PyBatchOutMinimalI16LegalIdsNoMeta,
    num_envs: usize,
) -> PyResult<()> {
    ensure_action_space_u16()?;
    ensure_first_dim(py, "obs", &out.obs, num_envs, Some(OBS_LEN))?;
    let expected_legal_ids = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "legal_ids size overflow (num_envs * action_space)",
        )
    })?;
    ensure_first_dim(py, "legal_ids", &out.legal_ids, expected_legal_ids, None)?;
    let expected_offsets = num_envs.checked_add(1).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "legal_offsets size overflow (num_envs + 1)",
        )
    })?;
    ensure_first_dim(
        py,
        "legal_offsets",
        &out.legal_offsets,
        expected_offsets,
        None,
    )?;
    ensure_first_dim(py, "rewards", &out.rewards, num_envs, None)?;
    ensure_first_dim(py, "terminated", &out.terminated, num_envs, None)?;
    ensure_first_dim(py, "truncated", &out.truncated, num_envs, None)?;
    ensure_first_dim(py, "actor", &out.actor, num_envs, None)?;
    ensure_first_dim(py, "decision_kind", &out.decision_kind, num_envs, None)?;
    ensure_first_dim(py, "decision_id", &out.decision_id, num_envs, None)?;
    ensure_first_dim(py, "engine_status", &out.engine_status, num_envs, None)?;
    ensure_first_dim(py, "spec_hash", &out.spec_hash, num_envs, None)?;
    ensure_first_dim(py, "main_move_action", &out.main_move_action, num_envs, None)?;
    ensure_first_dim(py, "main_pass_action", &out.main_pass_action, num_envs, None)?;
    Ok(())
}

fn with_batch_out_minimal_i16_legal_ids_nometa<T>(
    py: Python<'_>,
    out: &PyBatchOutMinimalI16LegalIdsNoMeta,
    num_envs: usize,
    f: impl FnOnce(BatchOutMinimalI16LegalIdsNoMeta<'_>) -> PyResult<T>,
) -> PyResult<T> {
    ensure_batch_out_minimal_i16_legal_ids_nometa_dims(py, out, num_envs)?;
    let mut obs = array_mut(py, &out.obs);
    let obs_slice = obs
        .as_slice_mut()
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
    let mut legal_ids = array_mut(py, &out.legal_ids);
    let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
    })?;
    let mut legal_offsets = array_mut(py, &out.legal_offsets);
    let legal_offsets_slice = legal_offsets.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_offsets not contiguous")
    })?;
    let mut rewards = array_mut(py, &out.rewards);
    let rewards_slice = rewards.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("rewards not contiguous")
    })?;
    let mut terminated = array_mut(py, &out.terminated);
    let terminated_slice = terminated.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("terminated not contiguous")
    })?;
    let mut truncated = array_mut(py, &out.truncated);
    let truncated_slice = truncated.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("truncated not contiguous")
    })?;
    let mut actor = array_mut(py, &out.actor);
    let actor_slice = actor.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("actor not contiguous")
    })?;
    let mut decision_kind = array_mut(py, &out.decision_kind);
    let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
    })?;
    let mut decision_id = array_mut(py, &out.decision_id);
    let decision_id_slice = decision_id.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_id not contiguous")
    })?;
    let mut engine_status = array_mut(py, &out.engine_status);
    let engine_status_slice = engine_status.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("engine_status not contiguous")
    })?;
    let mut spec_hash = array_mut(py, &out.spec_hash);
    let spec_hash_slice = spec_hash.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("spec_hash not contiguous")
    })?;
    let mut main_move_action = array_mut(py, &out.main_move_action);
    let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
    })?;
    let mut main_pass_action = array_mut(py, &out.main_pass_action);
    let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
    })?;
    f(BatchOutMinimalI16LegalIdsNoMeta {
        obs: obs_slice,
        legal_ids: legal_ids_slice,
        legal_offsets: legal_offsets_slice,
        rewards: rewards_slice,
        terminated: terminated_slice,
        truncated: truncated_slice,
        actor: actor_slice,
        decision_kind: decision_kind_slice,
        decision_id: decision_id_slice,
        engine_status: engine_status_slice,
        spec_hash: spec_hash_slice,
        main_move_action: main_move_action_slice,
        main_pass_action: main_pass_action_slice,
    })
}

fn ensure_batch_out_minimal_nomask_dims(
    py: Python<'_>,
    out: &PyBatchOutMinimalNoMask,
    num_envs: usize,
) -> PyResult<()> {
    ensure_first_dim(py, "obs", &out.obs, num_envs, Some(OBS_LEN))?;
    ensure_first_dim(py, "rewards", &out.rewards, num_envs, None)?;
    ensure_first_dim(py, "terminated", &out.terminated, num_envs, None)?;
    ensure_first_dim(py, "truncated", &out.truncated, num_envs, None)?;
    ensure_first_dim(py, "actor", &out.actor, num_envs, None)?;
    ensure_first_dim(py, "decision_kind", &out.decision_kind, num_envs, None)?;
    ensure_first_dim(py, "decision_id", &out.decision_id, num_envs, None)?;
    ensure_first_dim(py, "engine_status", &out.engine_status, num_envs, None)?;
    ensure_first_dim(py, "spec_hash", &out.spec_hash, num_envs, None)?;
    ensure_first_dim(py, "main_move_action", &out.main_move_action, num_envs, None)?;
    ensure_first_dim(py, "main_pass_action", &out.main_pass_action, num_envs, None)?;
    Ok(())
}

fn ensure_batch_out_trajectory_dims(
    py: Python<'_>,
    out: &PyBatchOutTrajectory,
    steps: usize,
    num_envs: usize,
) -> PyResult<()> {
    ensure_first_two_dims(py, "obs", &out.obs, steps, num_envs)?;
    ensure_first_two_dims(py, "masks", &out.masks, steps, num_envs)?;
    ensure_third_dim(py, "obs", &out.obs, OBS_LEN)?;
    ensure_third_dim(py, "masks", &out.masks, ACTION_SPACE_SIZE)?;
    ensure_first_two_dims(py, "rewards", &out.rewards, steps, num_envs)?;
    ensure_first_two_dims(py, "terminated", &out.terminated, steps, num_envs)?;
    ensure_first_two_dims(py, "truncated", &out.truncated, steps, num_envs)?;
    ensure_first_two_dims(py, "actor", &out.actor, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_kind", &out.decision_kind, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_id", &out.decision_id, steps, num_envs)?;
    ensure_first_two_dims(py, "engine_status", &out.engine_status, steps, num_envs)?;
    ensure_first_two_dims(py, "spec_hash", &out.spec_hash, steps, num_envs)?;
    ensure_first_two_dims(py, "main_move_action", &out.main_move_action, steps, num_envs)?;
    ensure_first_two_dims(py, "main_pass_action", &out.main_pass_action, steps, num_envs)?;
    ensure_first_two_dims(py, "actions", &out.actions, steps, num_envs)?;
    Ok(())
}

fn ensure_batch_out_trajectory_i16_dims(
    py: Python<'_>,
    out: &PyBatchOutTrajectoryI16,
    steps: usize,
    num_envs: usize,
) -> PyResult<()> {
    ensure_first_two_dims(py, "obs", &out.obs, steps, num_envs)?;
    ensure_first_two_dims(py, "masks", &out.masks, steps, num_envs)?;
    ensure_third_dim(py, "obs", &out.obs, OBS_LEN)?;
    ensure_third_dim(py, "masks", &out.masks, ACTION_SPACE_SIZE)?;
    ensure_first_two_dims(py, "rewards", &out.rewards, steps, num_envs)?;
    ensure_first_two_dims(py, "terminated", &out.terminated, steps, num_envs)?;
    ensure_first_two_dims(py, "truncated", &out.truncated, steps, num_envs)?;
    ensure_first_two_dims(py, "actor", &out.actor, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_kind", &out.decision_kind, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_id", &out.decision_id, steps, num_envs)?;
    ensure_first_two_dims(py, "engine_status", &out.engine_status, steps, num_envs)?;
    ensure_first_two_dims(py, "spec_hash", &out.spec_hash, steps, num_envs)?;
    ensure_first_two_dims(py, "main_move_action", &out.main_move_action, steps, num_envs)?;
    ensure_first_two_dims(py, "main_pass_action", &out.main_pass_action, steps, num_envs)?;
    ensure_first_two_dims(py, "actions", &out.actions, steps, num_envs)?;
    Ok(())
}

fn ensure_batch_out_trajectory_i16_legal_ids_dims(
    py: Python<'_>,
    out: &PyBatchOutTrajectoryI16LegalIds,
    steps: usize,
    num_envs: usize,
) -> PyResult<()> {
    ensure_action_space_u16()?;
    ensure_first_two_dims(py, "obs", &out.obs, steps, num_envs)?;
    ensure_third_dim(py, "obs", &out.obs, OBS_LEN)?;
    let expected_legal_ids = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "legal_ids size overflow (num_envs * action_space)",
        )
    })?;
    ensure_first_two_dims(py, "legal_ids", &out.legal_ids, steps, expected_legal_ids)?;
    ensure_first_two_dims(
        py,
        "legal_action_meta",
        &out.legal_action_meta,
        steps,
        expected_legal_ids,
    )?;
    ensure_third_dim(py, "legal_action_meta", &out.legal_action_meta, ACTION_META_WIDTH)?;
    let expected_offsets = num_envs.checked_add(1).ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "legal_offsets size overflow (num_envs + 1)",
        )
    })?;
    ensure_first_two_dims(
        py,
        "legal_offsets",
        &out.legal_offsets,
        steps,
        expected_offsets,
    )?;
    ensure_first_two_dims(py, "rewards", &out.rewards, steps, num_envs)?;
    ensure_first_two_dims(py, "terminated", &out.terminated, steps, num_envs)?;
    ensure_first_two_dims(py, "truncated", &out.truncated, steps, num_envs)?;
    ensure_first_two_dims(py, "actor", &out.actor, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_kind", &out.decision_kind, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_id", &out.decision_id, steps, num_envs)?;
    ensure_first_two_dims(py, "engine_status", &out.engine_status, steps, num_envs)?;
    ensure_first_two_dims(py, "episode_seed", &out.episode_seed, steps, num_envs)?;
    ensure_first_two_dims(py, "spec_hash", &out.spec_hash, steps, num_envs)?;
    ensure_first_two_dims(py, "main_move_action", &out.main_move_action, steps, num_envs)?;
    ensure_first_two_dims(py, "main_pass_action", &out.main_pass_action, steps, num_envs)?;
    ensure_first_two_dims(py, "actions", &out.actions, steps, num_envs)?;
    Ok(())
}

fn ensure_batch_out_trajectory_nomask_dims(
    py: Python<'_>,
    out: &PyBatchOutTrajectoryNoMask,
    steps: usize,
    num_envs: usize,
) -> PyResult<()> {
    ensure_first_two_dims(py, "obs", &out.obs, steps, num_envs)?;
    ensure_third_dim(py, "obs", &out.obs, OBS_LEN)?;
    ensure_first_two_dims(py, "rewards", &out.rewards, steps, num_envs)?;
    ensure_first_two_dims(py, "terminated", &out.terminated, steps, num_envs)?;
    ensure_first_two_dims(py, "truncated", &out.truncated, steps, num_envs)?;
    ensure_first_two_dims(py, "actor", &out.actor, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_kind", &out.decision_kind, steps, num_envs)?;
    ensure_first_two_dims(py, "decision_id", &out.decision_id, steps, num_envs)?;
    ensure_first_two_dims(py, "engine_status", &out.engine_status, steps, num_envs)?;
    ensure_first_two_dims(py, "spec_hash", &out.spec_hash, steps, num_envs)?;
    ensure_first_two_dims(py, "main_move_action", &out.main_move_action, steps, num_envs)?;
    ensure_first_two_dims(py, "main_pass_action", &out.main_pass_action, steps, num_envs)?;
    ensure_first_two_dims(py, "actions", &out.actions, steps, num_envs)?;
    Ok(())
}

fn ensure_batch_out_debug_dims(
    py: Python<'_>,
    out: &PyBatchOutDebug,
    num_envs: usize,
    event_capacity: usize,
) -> PyResult<()> {
    ensure_first_dim(py, "obs", &out.obs, num_envs, Some(OBS_LEN))?;
    ensure_first_dim(py, "masks", &out.masks, num_envs, Some(ACTION_SPACE_SIZE))?;
    ensure_first_dim(py, "rewards", &out.rewards, num_envs, None)?;
    ensure_first_dim(py, "terminated", &out.terminated, num_envs, None)?;
    ensure_first_dim(py, "truncated", &out.truncated, num_envs, None)?;
    ensure_first_dim(py, "actor", &out.actor, num_envs, None)?;
    ensure_first_dim(py, "decision_id", &out.decision_id, num_envs, None)?;
    ensure_first_dim(py, "engine_status", &out.engine_status, num_envs, None)?;
    ensure_first_dim(py, "spec_hash", &out.spec_hash, num_envs, None)?;
    ensure_first_dim(py, "decision_kind", &out.decision_kind, num_envs, None)?;
    ensure_first_dim(py, "main_move_action", &out.main_move_action, num_envs, None)?;
    ensure_first_dim(py, "main_pass_action", &out.main_pass_action, num_envs, None)?;
    ensure_first_dim(
        py,
        "reward_components",
        &out.reward_components,
        num_envs,
        Some(REWARD_COMPONENT_WIDTH),
    )?;
    ensure_first_dim(
        py,
        "state_fingerprint",
        &out.state_fingerprint,
        num_envs,
        None,
    )?;
    ensure_first_dim(
        py,
        "events_fingerprint",
        &out.events_fingerprint,
        num_envs,
        None,
    )?;
    ensure_first_dim(
        py,
        "mask_fingerprint",
        &out.mask_fingerprint,
        num_envs,
        None,
    )?;
    ensure_first_dim(py, "event_counts", &out.event_counts, num_envs, None)?;
    ensure_first_dim(
        py,
        "event_codes",
        &out.event_codes,
        num_envs,
        Some(event_capacity),
    )?;
    Ok(())
}
