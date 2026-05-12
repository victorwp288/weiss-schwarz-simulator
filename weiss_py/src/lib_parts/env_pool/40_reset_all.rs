    fn reset_into<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut masks = array_mut(py, &out.masks);
        let mask_slice = masks.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("masks not contiguous")
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
        let mut out_min = BatchOutMinimal {
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
        };
        py.allow_threads(|| self.pool.reset_into(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut masks = array_mut(py, &out.masks);
        let mask_slice = masks.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("masks not contiguous")
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
        let mut out_min = BatchOutMinimalI16 {
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
        };
        py.allow_threads(|| self.pool.reset_into_i16(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
        })?;
        let mut legal_action_meta = array_mut(py, &out.legal_action_meta);
        let legal_action_meta_slice = legal_action_meta.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_action_meta not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
            spec_hash: spec_hash_slice,
            main_move_action: main_move_action_slice,
            main_pass_action: main_pass_action_slice,
        };
        py.allow_threads(|| self.pool.reset_into_i16_legal_ids(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_i16_legal_ids_nometa<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIdsNoMeta>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_nometa_dims(py, &out, num_envs)?;
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
        let mut out_min = BatchOutMinimalI16LegalIdsNoMeta {
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
        };
        py.allow_threads(|| self.pool.reset_into_i16_legal_ids_nometa(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
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
        let mut out_min = BatchOutMinimalNoMask {
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
        };
        py.allow_threads(|| self.pool.reset_into_nomask(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

