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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
        };
        py.allow_threads(|| self.pool.reset_into_i16_legal_ids(&mut out_min))
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
        };
        py.allow_threads(|| self.pool.reset_into_nomask(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
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
        };
        py.allow_threads(|| self.pool.reset_indices_into(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
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
        };
        py.allow_threads(|| self.pool.reset_indices_into_i16(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
        };
        py.allow_threads(|| {
            self.pool
                .reset_indices_into_i16_legal_ids(&indices, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
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
        };
        py.allow_threads(|| self.pool.reset_indices_into_nomask(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_done_into(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_done_into_i16(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
        };
        py.allow_threads(|| self.pool.reset_done_into_i16_legal_ids(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_done_into_nomask(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_into(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_into_i16(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
        };
        py.allow_threads(|| self.pool.step_into_i16_legal_ids(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_into_nomask(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_first_legal_into(actions_slice, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_first_legal_into_i16(actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
        };
        py.allow_threads(|| {
            self.pool
                .step_first_legal_into_i16_legal_ids(actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_first_legal_into_nomask(actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_sample_legal_action_ids_uniform_into(seeds, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool.step_sample_legal_action_ids_uniform_into_i16(
                seeds,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        let mut out_min = BatchOutMinimalI16LegalIds {
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
        };
        py.allow_threads(|| {
            self.pool
                .step_sample_legal_action_ids_uniform_into_i16_legal_ids(
                    seeds,
                    actions_slice,
                    &mut out_min,
                )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool.step_sample_legal_action_ids_uniform_into_nomask(
                seeds,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectory>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectory.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
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
            actions: actions_slice,
        };
        py.allow_threads(|| self.pool.rollout_first_legal_into(steps, &mut out_traj))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectoryI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
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
            actions: actions_slice,
        };
        py.allow_threads(|| self.pool.rollout_first_legal_into_i16(steps, &mut out_traj))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectoryI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_legal_ids_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16LegalIds.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16LegalIds {
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_first_legal_into_i16_legal_ids(steps, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectoryNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_nomask_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryNoMask.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_first_legal_into_nomask(steps, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

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
        let mut actions = array_mut(py, &out.actions);
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
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
        let mut actions = array_mut(py, &out.actions);
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16LegalIds {
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
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
        let mut actions = array_mut(py, &out.actions);
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_nomask(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_debug_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutDebug>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        let event_capacity = self.pool.debug_event_ring_capacity();
        ensure_batch_out_debug_dims(py, &out, num_envs, event_capacity)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
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
        let mut state_fingerprint = array_mut(py, &out.state_fingerprint);
        let state_fingerprint_slice = state_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("state_fingerprint not contiguous")
        })?;
        let mut events_fingerprint = array_mut(py, &out.events_fingerprint);
        let events_fingerprint_slice = events_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("events_fingerprint not contiguous")
        })?;
        let mut mask_fingerprint = array_mut(py, &out.mask_fingerprint);
        let mask_fingerprint_slice = mask_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("mask_fingerprint not contiguous")
        })?;
        let mut event_counts = array_mut(py, &out.event_counts);
        let event_counts_slice = event_counts.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("event_counts not contiguous")
        })?;
        let mut event_codes = array_mut(py, &out.event_codes);
        let event_codes_slice = event_codes.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("event_codes not contiguous")
        })?;
        let mut out_debug = BatchOutDebug {
            minimal: BatchOutMinimal {
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
            },
            state_fingerprint: state_fingerprint_slice,
            events_fingerprint: events_fingerprint_slice,
            mask_fingerprint: mask_fingerprint_slice,
            event_counts: event_counts_slice,
            event_codes: event_codes_slice,
        };
        py.allow_threads(|| self.pool.step_debug_into(actions, &mut out_debug))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_debug_into<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutDebug>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        let event_capacity = self.pool.debug_event_ring_capacity();
        ensure_batch_out_debug_dims(py, &out, num_envs, event_capacity)?;
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
        let mut state_fingerprint = array_mut(py, &out.state_fingerprint);
        let state_fingerprint_slice = state_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("state_fingerprint not contiguous")
        })?;
        let mut events_fingerprint = array_mut(py, &out.events_fingerprint);
        let events_fingerprint_slice = events_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("events_fingerprint not contiguous")
        })?;
        let mut mask_fingerprint = array_mut(py, &out.mask_fingerprint);
        let mask_fingerprint_slice = mask_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("mask_fingerprint not contiguous")
        })?;
        let mut event_counts = array_mut(py, &out.event_counts);
        let event_counts_slice = event_counts.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("event_counts not contiguous")
        })?;
        let mut event_codes = array_mut(py, &out.event_codes);
        let event_codes_slice = event_codes.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("event_codes not contiguous")
        })?;
        let mut out_debug = BatchOutDebug {
            minimal: BatchOutMinimal {
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
            },
            state_fingerprint: state_fingerprint_slice,
            events_fingerprint: events_fingerprint_slice,
            mask_fingerprint: mask_fingerprint_slice,
            event_counts: event_counts_slice,
            event_codes: event_codes_slice,
        };
        py.allow_threads(|| self.pool.reset_debug_into(&mut out_debug))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn auto_reset_on_error_codes_into<'py>(
        &mut self,
        py: Python<'py>,
        codes: PyReadonlyArray1<u8>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<usize> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let codes = codes
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("codes not contiguous"))?;
        ensure_len("codes", codes.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .auto_reset_on_error_codes_into(codes, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn auto_reset_on_error_codes_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        codes: PyReadonlyArray1<u8>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<usize> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let codes = codes
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("codes not contiguous"))?;
        ensure_len("codes", codes.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .auto_reset_on_error_codes_into_nomask(codes, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

