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

