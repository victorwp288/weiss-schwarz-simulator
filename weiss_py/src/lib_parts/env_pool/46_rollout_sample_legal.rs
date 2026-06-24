    fn rollout_sample_legal_action_ids_uniform_into<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectory>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectory.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs)?;
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut masks = array_mut(py, &out.masks)?;
        let mask_slice = masks.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("masks not contiguous")
        })?;
        let mut rewards = array_mut(py, &out.rewards)?;
        let rewards_slice = rewards.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("rewards not contiguous")
        })?;
        let mut terminated = array_mut(py, &out.terminated)?;
        let terminated_slice = terminated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("terminated not contiguous")
        })?;
        let mut truncated = array_mut(py, &out.truncated)?;
        let truncated_slice = truncated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("truncated not contiguous")
        })?;
        let mut actor = array_mut(py, &out.actor)?;
        let actor_slice = actor.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actor not contiguous")
        })?;
        let mut decision_kind = array_mut(py, &out.decision_kind)?;
        let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
        })?;
        let mut decision_id = array_mut(py, &out.decision_id)?;
        let decision_id_slice = decision_id.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_id not contiguous")
        })?;
        let mut engine_status = array_mut(py, &out.engine_status)?;
        let engine_status_slice = engine_status.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("engine_status not contiguous")
        })?;
        let mut spec_hash = array_mut(py, &out.spec_hash)?;
        let spec_hash_slice = spec_hash.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("spec_hash not contiguous")
        })?;
        let mut main_move_action = array_mut(py, &out.main_move_action)?;
        let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
        })?;
        let mut main_pass_action = array_mut(py, &out.main_pass_action)?;
        let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
        })?;
        let mut actions = array_mut(py, &out.actions)?;
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectory {
            obs: obs_slice,
            masks: mask_slice,
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
            actions: actions_slice,
        };
        py.detach(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectoryI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs)?;
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut masks = array_mut(py, &out.masks)?;
        let mask_slice = masks.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("masks not contiguous")
        })?;
        let mut rewards = array_mut(py, &out.rewards)?;
        let rewards_slice = rewards.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("rewards not contiguous")
        })?;
        let mut terminated = array_mut(py, &out.terminated)?;
        let terminated_slice = terminated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("terminated not contiguous")
        })?;
        let mut truncated = array_mut(py, &out.truncated)?;
        let truncated_slice = truncated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("truncated not contiguous")
        })?;
        let mut actor = array_mut(py, &out.actor)?;
        let actor_slice = actor.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actor not contiguous")
        })?;
        let mut decision_kind = array_mut(py, &out.decision_kind)?;
        let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
        })?;
        let mut decision_id = array_mut(py, &out.decision_id)?;
        let decision_id_slice = decision_id.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_id not contiguous")
        })?;
        let mut engine_status = array_mut(py, &out.engine_status)?;
        let engine_status_slice = engine_status.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("engine_status not contiguous")
        })?;
        let mut spec_hash = array_mut(py, &out.spec_hash)?;
        let spec_hash_slice = spec_hash.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("spec_hash not contiguous")
        })?;
        let mut main_move_action = array_mut(py, &out.main_move_action)?;
        let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
        })?;
        let mut main_pass_action = array_mut(py, &out.main_pass_action)?;
        let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
        })?;
        let mut actions = array_mut(py, &out.actions)?;
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16 {
            obs: obs_slice,
            masks: mask_slice,
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
            actions: actions_slice,
        };
        py.detach(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_i16(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectoryI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_legal_ids_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16LegalIds.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs)?;
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids)?;
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
        })?;
        let mut legal_action_meta = array_mut(py, &out.legal_action_meta)?;
        let legal_action_meta_slice = legal_action_meta.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_action_meta not contiguous")
        })?;
        let mut legal_offsets = array_mut(py, &out.legal_offsets)?;
        let legal_offsets_slice = legal_offsets.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_offsets not contiguous")
        })?;
        let mut rewards = array_mut(py, &out.rewards)?;
        let rewards_slice = rewards.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("rewards not contiguous")
        })?;
        let mut terminated = array_mut(py, &out.terminated)?;
        let terminated_slice = terminated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("terminated not contiguous")
        })?;
        let mut truncated = array_mut(py, &out.truncated)?;
        let truncated_slice = truncated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("truncated not contiguous")
        })?;
        let mut actor = array_mut(py, &out.actor)?;
        let actor_slice = actor.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actor not contiguous")
        })?;
        let mut decision_kind = array_mut(py, &out.decision_kind)?;
        let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
        })?;
        let mut decision_id = array_mut(py, &out.decision_id)?;
        let decision_id_slice = decision_id.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_id not contiguous")
        })?;
        let mut engine_status = array_mut(py, &out.engine_status)?;
        let engine_status_slice = engine_status.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("engine_status not contiguous")
        })?;
        let mut episode_seed = array_mut(py, &out.episode_seed)?;
        let episode_seed_slice = episode_seed.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("episode_seed not contiguous")
        })?;
        let mut spec_hash = array_mut(py, &out.spec_hash)?;
        let spec_hash_slice = spec_hash.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("spec_hash not contiguous")
        })?;
        let mut main_move_action = array_mut(py, &out.main_move_action)?;
        let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
        })?;
        let mut main_pass_action = array_mut(py, &out.main_pass_action)?;
        let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
        })?;
        let mut actions = array_mut(py, &out.actions)?;
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_action_meta: legal_action_meta_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            episode_seed: episode_seed_slice,
            spec_hash: spec_hash_slice,
            main_move_action: main_move_action_slice,
            main_pass_action: main_pass_action_slice,
            actions: actions_slice,
        };
        py.detach(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_i16_legal_ids(
                    steps,
                    seeds,
                    &mut out_traj,
                )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectoryNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_nomask_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryNoMask.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs)?;
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut rewards = array_mut(py, &out.rewards)?;
        let rewards_slice = rewards.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("rewards not contiguous")
        })?;
        let mut terminated = array_mut(py, &out.terminated)?;
        let terminated_slice = terminated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("terminated not contiguous")
        })?;
        let mut truncated = array_mut(py, &out.truncated)?;
        let truncated_slice = truncated.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("truncated not contiguous")
        })?;
        let mut actor = array_mut(py, &out.actor)?;
        let actor_slice = actor.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actor not contiguous")
        })?;
        let mut decision_kind = array_mut(py, &out.decision_kind)?;
        let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
        })?;
        let mut decision_id = array_mut(py, &out.decision_id)?;
        let decision_id_slice = decision_id.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_id not contiguous")
        })?;
        let mut engine_status = array_mut(py, &out.engine_status)?;
        let engine_status_slice = engine_status.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("engine_status not contiguous")
        })?;
        let mut spec_hash = array_mut(py, &out.spec_hash)?;
        let spec_hash_slice = spec_hash.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("spec_hash not contiguous")
        })?;
        let mut main_move_action = array_mut(py, &out.main_move_action)?;
        let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
        })?;
        let mut main_pass_action = array_mut(py, &out.main_pass_action)?;
        let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
        })?;
        let mut actions = array_mut(py, &out.actions)?;
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryNoMask {
            obs: obs_slice,
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
            actions: actions_slice,
        };
        py.detach(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_nomask(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

