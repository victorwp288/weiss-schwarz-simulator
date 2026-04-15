    fn engine_error_reset_count(&self) -> u64 {
        self.pool.engine_error_reset_count()
    }

    fn reset_engine_error_reset_count(&mut self) {
        self.pool.reset_engine_error_reset_count();
    }

    fn set_timing_enabled(&mut self, enabled: bool) {
        self.pool.set_timing_enabled(enabled);
    }

    fn reset_timing_counters(&mut self) {
        self.pool.reset_timing_counters();
    }

    fn timing_counters<'py>(&self, py: Python<'py>) -> PyResult<Py<PyDict>> {
        let counters = self.pool.timing_counters();
        let [
            select_actions_from_logits_count,
            select_actions_from_logits_ns,
            sample_actions_from_logits_count,
            sample_actions_from_logits_ns,
            step_select_from_logits_into_i16_legal_ids_count,
            step_select_from_logits_into_i16_legal_ids_ns,
            step_sample_from_logits_into_i16_legal_ids_count,
            step_sample_from_logits_into_i16_legal_ids_ns,
            step_sample_from_logits_with_logp_into_i16_legal_ids_count,
            step_sample_from_logits_with_logp_into_i16_legal_ids_ns,
        ] = counters;
        let dict = PyDict::new(py);
        dict.set_item("timing_enabled", self.pool.timing_enabled())?;
        dict.set_item(
            "select_actions_from_logits_count",
            select_actions_from_logits_count,
        )?;
        dict.set_item(
            "select_actions_from_logits_ns",
            select_actions_from_logits_ns,
        )?;
        dict.set_item(
            "sample_actions_from_logits_count",
            sample_actions_from_logits_count,
        )?;
        dict.set_item(
            "sample_actions_from_logits_ns",
            sample_actions_from_logits_ns,
        )?;
        dict.set_item(
            "step_select_from_logits_into_i16_legal_ids_count",
            step_select_from_logits_into_i16_legal_ids_count,
        )?;
        dict.set_item(
            "step_select_from_logits_into_i16_legal_ids_ns",
            step_select_from_logits_into_i16_legal_ids_ns,
        )?;
        dict.set_item(
            "step_sample_from_logits_into_i16_legal_ids_count",
            step_sample_from_logits_into_i16_legal_ids_count,
        )?;
        dict.set_item(
            "step_sample_from_logits_into_i16_legal_ids_ns",
            step_sample_from_logits_into_i16_legal_ids_ns,
        )?;
        dict.set_item(
            "step_sample_from_logits_with_logp_into_i16_legal_ids_count",
            step_sample_from_logits_with_logp_into_i16_legal_ids_count,
        )?;
        dict.set_item(
            "step_sample_from_logits_with_logp_into_i16_legal_ids_ns",
            step_sample_from_logits_with_logp_into_i16_legal_ids_ns,
        )?;
        Ok(dict.unbind())
    }

    fn set_error_policy(&mut self, error_policy: String) -> PyResult<()> {
        let policy = parse_error_policy(Some(error_policy))?;
        self.pool.set_error_policy(policy);
        Ok(())
    }

    #[staticmethod]
    #[pyo3(signature = (deck_lists, db_path=None, deck_ids=None))]
    fn validate_deck_issues<'py>(
        py: Python<'py>,
        deck_lists: Vec<Vec<u32>>,
        db_path: Option<String>,
        deck_ids: Option<Vec<u32>>,
    ) -> PyResult<Vec<Py<PyDict>>> {
        if deck_lists.len() != 2 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "deck_lists must have length 2",
            ));
        }
        let deck_ids_vec = deck_ids.unwrap_or_else(|| vec![0, 1]);
        if deck_ids_vec.len() != 2 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "deck_ids must have length 2",
            ));
        }
        let db = load_card_db(db_path)?;
        let config = EnvConfig {
            deck_lists: [deck_lists[0].clone(), deck_lists[1].clone()],
            deck_ids: [deck_ids_vec[0], deck_ids_vec[1]],
            max_decisions: 1,
            max_ticks: 1,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::LenientNoop,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: EndConditionPolicy::default(),
        };
        config
            .validate_with_db_all_issues(&db)
            .into_iter()
            .map(|issue| config_error_to_issue_dict(py, issue))
            .collect()
    }

    fn set_output_mask_enabled(&mut self, enabled: bool) {
        self.pool.set_output_mask_enabled(enabled);
    }

    fn set_output_mask_bits_enabled(&mut self, enabled: bool) {
        self.pool.set_output_mask_bits_enabled(enabled);
    }

    fn set_i16_clamp_enabled(&mut self, enabled: bool) {
        self.pool.set_i16_clamp_enabled(enabled);
    }

    fn set_i16_overflow_counter_enabled(&mut self, enabled: bool) {
        self.pool.set_i16_overflow_counter_enabled(enabled);
    }

    fn i16_overflow_count(&self) -> u64 {
        self.pool.i16_overflow_count()
    }

    fn reset_i16_overflow_count(&mut self) {
        self.pool.reset_i16_overflow_count();
    }

    fn action_mask_bits_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u64>>> {
        let bits = self.pool.action_mask_bits_batch();
        let arr = Array1::<u64>::from(bits);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn sample_legal_actions_uniform<'py>(
        &self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
    ) -> PyResult<Py<PyArray1<u32>>> {
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        let ids = py
            .allow_threads(|| self.pool.sample_legal_action_ids_uniform(seeds))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
        let arr = Array1::<u32>::from(ids);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn config_hash(&self) -> u64 {
        self.pool.config_hash()
    }

    fn debug_event_ring_capacity(&self) -> usize {
        self.pool.debug_event_ring_capacity()
    }

    fn max_card_id(&self) -> u32 {
        self.pool.max_card_id()
    }

    fn episode_seed_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u64>>> {
        let vals = self.pool.episode_seed_batch();
        let arr = Array1::<u64>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn episode_index_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u32>>> {
        let vals = self.pool.episode_index_batch();
        let arr = Array1::<u32>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn env_index_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u32>>> {
        let vals = self.pool.env_index_batch();
        let arr = Array1::<u32>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn starting_player_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u8>>> {
        let vals = self.pool.starting_player_batch();
        let arr = Array1::<u8>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn decision_count_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u32>>> {
        let vals = self.pool.decision_count_batch();
        let arr = Array1::<u32>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn tick_count_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u32>>> {
        let vals = self.pool.tick_count_batch();
        let arr = Array1::<u32>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn no_progress_count_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u32>>> {
        let vals = self.pool.no_progress_count_batch();
        let arr = Array1::<u32>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn obs_fingerprint_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u64>>> {
        let vals = self.pool.obs_fingerprint_batch();
        let arr = Array1::<u64>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn reset_indices_with_episode_seeds_into<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        episode_seeds: Vec<u64>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        validate_indices_and_seeds(&indices, &episode_seeds, num_envs)?;
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
                .reset_indices_with_episode_seeds_into(&indices, &episode_seeds, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_with_episode_seeds_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        episode_seeds: Vec<u64>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        validate_indices_and_seeds(&indices, &episode_seeds, num_envs)?;
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
        py.allow_threads(|| {
            self.pool.reset_indices_with_episode_seeds_into_i16(
                &indices,
                &episode_seeds,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_with_episode_seeds_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        episode_seeds: Vec<u64>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        validate_indices_and_seeds(&indices, &episode_seeds, num_envs)?;
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
        py.allow_threads(|| {
            self.pool
                .reset_indices_with_episode_seeds_into_i16_legal_ids(
                    &indices,
                    &episode_seeds,
                    &mut out_min,
                )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_with_episode_seeds_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        episode_seeds: Vec<u64>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        validate_indices_and_seeds(&indices, &episode_seeds, num_envs)?;
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
            self.pool.reset_indices_with_episode_seeds_into_nomask(
                &indices,
                &episode_seeds,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    #[pyo3(signature = (
        sample_rate,
        out_dir=None,
        compress=false,
        include_trigger_card_id=false,
        visibility_mode=None,
        store_actions=true
    ))]
    fn enable_replay_sampling(
        &mut self,
        sample_rate: f32,
        out_dir: Option<String>,
        compress: bool,
        include_trigger_card_id: bool,
        visibility_mode: Option<String>,
        store_actions: bool,
    ) -> PyResult<()> {
        if !(0.0..=1.0).contains(&sample_rate) {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "sample_rate must be within [0.0, 1.0], got {sample_rate}"
            )));
        }
        let visibility_mode = parse_replay_visibility_mode(visibility_mode)?;
        let out_dir = out_dir.unwrap_or_else(|| "replays".to_string());
        let mut config = ReplayConfig {
            enabled: sample_rate > 0.0,
            sample_rate,
            out_dir: PathBuf::from(out_dir),
            compress,
            include_trigger_card_id,
            visibility_mode,
            store_actions,
            sample_threshold: 0,
        };
        config.rebuild_cache();
        self.pool
            .enable_replay_sampling(config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn action_lookup_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyList>> {
        let outer = PyList::empty(py);
        for env in &self.pool.envs {
            let inner = PyList::empty(py);
            for action_id in 0..weiss_core::encode::ACTION_SPACE_SIZE {
                match env.action_for_id(action_id) {
                    Some(action) => inner.append(action_desc_to_pydict(py, &action)?)?,
                    None => inner.append(py.None())?,
                }
            }
            outer.append(inner)?;
        }
        Ok(outer.unbind())
    }

    fn describe_action_ids<'py>(
        &self,
        py: Python<'py>,
        action_ids: Vec<u32>,
    ) -> PyResult<Py<PyList>> {
        if action_ids.len() != self.pool.envs.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "action_ids length must match env count",
            ));
        }
        let out = PyList::empty(py);
        for (env, action_id) in self.pool.envs.iter().zip(action_ids.iter()) {
            match env.action_for_id(*action_id as usize) {
                Some(desc) => out.append(action_desc_to_pydict(py, &desc)?)?,
                None => out.append(py.None())?,
            }
        }
        Ok(out.unbind())
    }

    fn decision_info_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyList>> {
        let outer = PyList::empty(py);
        for env in &self.pool.envs {
            let dict = PyDict::new(py);
            if let Some(decision) = &env.decision {
                dict.set_item("decision_kind", format!("{:?}", decision.kind))?;
                dict.set_item("current_player", decision.player)?;
                dict.set_item("focus_slot", decision.focus_slot)?;
            } else {
                dict.set_item("decision_kind", py.None())?;
                dict.set_item("current_player", -1)?;
                dict.set_item("focus_slot", py.None())?;
            }
            dict.set_item("decision_id", env.decision_id())?;
            if let Some(choice) = &env.state.turn.choice {
                dict.set_item("choice_reason", format!("{:?}", choice.reason))?;
                let mut zones: std::collections::BTreeSet<String> =
                    std::collections::BTreeSet::new();
                for option in &choice.options {
                    zones.insert(format!("{:?}", option.zone));
                }
                dict.set_item("choice_option_zones", zones.into_iter().collect::<Vec<_>>())?;
            }
            outer.append(dict)?;
        }
        Ok(outer.unbind())
    }

    fn state_fingerprint_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u64>>> {
        let vals = self.pool.state_fingerprint_batch();
        let arr = Array1::<u64>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn events_fingerprint_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<u64>>> {
        let vals = self.pool.events_fingerprint_batch();
        let arr = Array1::<u64>::from(vals);
        Ok(PyArray1::from_owned_array(py, arr).unbind())
    }

    fn legal_action_ids_into<'py>(
        &mut self,
        py: Python<'py>,
        ids: Py<PyArray1<u16>>,
        offsets: Py<PyArray1<u32>>,
    ) -> PyResult<usize> {
        let num_envs = self.pool.envs.len();
        let mut ids_arr = array_mut(py, &ids);
        let ids_slice = ids_arr
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("ids not contiguous"))?;
        let mut offsets_arr = array_mut(py, &offsets);
        let offsets_slice = offsets_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("offsets not contiguous")
        })?;
        let expected_ids = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "ids size overflow (num_envs * action_space)",
            )
        })?;
        if ids_slice.len() < expected_ids {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "ids buffer too small (got {}, need at least {})",
                ids_slice.len(),
                expected_ids
            )));
        }
        let expected_offsets = num_envs.checked_add(1).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("offsets size overflow (num_envs + 1)")
        })?;
        if offsets_slice.len() < expected_offsets {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "offsets buffer too small (got {}, need at least {})",
                offsets_slice.len(),
                expected_offsets
            )));
        }
        py.allow_threads(|| {
            self.pool
                .legal_action_ids_batch_into(ids_slice, offsets_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn legal_action_meta_into<'py>(
        &mut self,
        py: Python<'py>,
        meta: Py<PyArray2<u16>>,
    ) -> PyResult<usize> {
        let expected_rows = self.pool.envs.len().checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_action_meta size overflow (num_envs * action_space)",
            )
        })?;
        ensure_first_two_dims(py, "legal_action_meta", &meta, expected_rows, ACTION_META_WIDTH)?;
        let mut meta_arr = array_mut(py, &meta);
        let meta_slice = meta_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_action_meta not contiguous")
        })?;
        py.allow_threads(|| self.pool.legal_action_meta_batch_into(meta_slice))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn choose_heuristic_public_actions_into<'py>(
        &mut self,
        py: Python<'py>,
        env_indices: PyReadonlyArray1<u32>,
        actions: Py<PyArray1<u16>>,
    ) -> PyResult<()> {
        let indices_u32 = env_indices
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("env_indices not contiguous"))?;
        let num_envs = self.pool.envs.len();
        let mut indices = Vec::with_capacity(indices_u32.len());
        for (position, &env_index) in indices_u32.iter().enumerate() {
            let env_index_usize = env_index as usize;
            if env_index_usize >= num_envs {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "env_indices[{position}] out of bounds (got {env_index_usize}, max {})",
                    num_envs.saturating_sub(1)
                )));
            }
            indices.push(env_index_usize);
        }
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        if actions_slice.len() != indices.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "actions length must match env_indices length (got {}, expected {})",
                actions_slice.len(),
                indices.len()
            )));
        }
        py.allow_threads(|| self.pool.choose_heuristic_public_actions_into(&indices, actions_slice))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn sample_legal_action_ids_uniform_into<'py>(
        &self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        if seeds.len() != num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "seeds length must match num_envs (got {}, expected {})",
                seeds.len(),
                num_envs
            )));
        }
        if actions_slice.len() != num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "actions length must match num_envs (got {}, expected {})",
                actions_slice.len(),
                num_envs
            )));
        }
        py.allow_threads(|| {
            self.pool
                .sample_legal_action_ids_uniform_into(seeds, actions_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn select_actions_from_logits_into<'py>(
        &self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        actions: Py<PyArray1<u32>>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        let shape = logits.shape();
        if shape[0] != self.pool.envs.len() || shape[1] != ACTION_SPACE_SIZE {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "logits shape must be (num_envs, action_space)",
            ));
        }
        let logits = logits.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("logits not contiguous")
        })?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
        let timing_start = self.pool.timing_start();
        py.allow_threads(|| {
            self.pool
                .select_actions_from_logits_into(logits, actions_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
        if let Some(timing_start) = timing_start {
            self.pool
                .record_select_actions_from_logits(timing_start.elapsed());
        }
        Ok(())
    }

    fn sample_actions_from_logits_into<'py>(
        &self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        let shape = logits.shape();
        if shape[0] != self.pool.envs.len() || shape[1] != ACTION_SPACE_SIZE {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "logits shape must be (num_envs, action_space)",
            ));
        }
        let logits = logits.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("logits not contiguous")
        })?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
        let timing_start = self.pool.timing_start();
        py.allow_threads(|| {
            self.pool
                .sample_actions_from_logits_into(logits, seeds, actions_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?;
        if let Some(timing_start) = timing_start {
            self.pool
                .record_sample_actions_from_logits(timing_start.elapsed());
        }
        Ok(())
    }

    fn legal_action_ids_and_sample_uniform_into<'py>(
        &mut self,
        py: Python<'py>,
        ids: Py<PyArray1<u16>>,
        offsets: Py<PyArray1<u32>>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
    ) -> PyResult<usize> {
        let num_envs = self.pool.envs.len();
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        let mut ids_arr = array_mut(py, &ids);
        let ids_slice = ids_arr
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("ids not contiguous"))?;
        let mut offsets_arr = array_mut(py, &offsets);
        let offsets_slice = offsets_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("offsets not contiguous")
        })?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let expected_ids = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "ids size overflow (num_envs * action_space)",
            )
        })?;
        if ids_slice.len() < expected_ids {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "ids buffer too small (got {}, need at least {})",
                ids_slice.len(),
                expected_ids
            )));
        }
        let expected_offsets = num_envs.checked_add(1).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("offsets size overflow (num_envs + 1)")
        })?;
        if offsets_slice.len() < expected_offsets {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "offsets buffer too small (got {}, need at least {})",
                offsets_slice.len(),
                expected_offsets
            )));
        }
        if seeds.len() != num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "seeds length must match num_envs (got {}, expected {})",
                seeds.len(),
                num_envs
            )));
        }
        if actions_slice.len() != num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "actions length must match num_envs (got {}, expected {})",
                actions_slice.len(),
                num_envs
            )));
        }
        py.allow_threads(|| {
            self.pool.legal_action_ids_and_sample_uniform_into(
                ids_slice,
                offsets_slice,
                seeds,
                actions_slice,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn render_ansi(&self, env_index: usize, perspective: u8) -> PyResult<String> {
        let num_envs = self.pool.envs.len();
        if env_index >= num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "env_index {env_index} out of bounds (num_envs = {num_envs})"
            )));
        }
        Ok(self.pool.render_ansi(env_index, perspective))
    }

    #[getter]
    fn envs_len(&self) -> usize {
        self.pool.envs.len()
    }

    #[getter]
    fn num_envs(&self) -> usize {
        self.pool.envs.len()
    }

    #[getter]
    fn obs_len(&self) -> usize {
        OBS_LEN
    }

    #[getter]
    fn action_space(&self) -> usize {
        ACTION_SPACE_SIZE
    }

    #[getter]
    fn num_threads(&self) -> usize {
        self.pool.effective_num_threads()
    }
