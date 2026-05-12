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
        let mut main_move_action = array_mut(py, &out.main_move_action);
        let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
        })?;
        let mut main_pass_action = array_mut(py, &out.main_pass_action);
        let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
        })?;
        let mut reward_components = array_mut(py, &out.reward_components);
        let reward_components_slice = reward_components.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("reward_components not contiguous")
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
        let mut event_codes;
        let event_codes_slice: &mut [u32] = if event_capacity == 0 {
            &mut []
        } else {
            event_codes = array_mut(py, &out.event_codes);
            event_codes.as_slice_mut().ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("event_codes not contiguous")
            })?
        };
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
            main_move_action: main_move_action_slice,
            main_pass_action: main_pass_action_slice,
            },
            reward_components: reward_components_slice,
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
        let mut main_move_action = array_mut(py, &out.main_move_action);
        let main_move_action_slice = main_move_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_move_action not contiguous")
        })?;
        let mut main_pass_action = array_mut(py, &out.main_pass_action);
        let main_pass_action_slice = main_pass_action.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("main_pass_action not contiguous")
        })?;
        let mut reward_components = array_mut(py, &out.reward_components);
        let reward_components_slice = reward_components.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("reward_components not contiguous")
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
        let mut event_codes;
        let event_codes_slice: &mut [u32] = if event_capacity == 0 {
            &mut []
        } else {
            event_codes = array_mut(py, &out.event_codes);
            event_codes.as_slice_mut().ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("event_codes not contiguous")
            })?
        };
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
            main_move_action: main_move_action_slice,
            main_pass_action: main_pass_action_slice,
            },
            reward_components: reward_components_slice,
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
        py.allow_threads(|| {
            self.pool
                .auto_reset_on_error_codes_into_nomask(codes, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

