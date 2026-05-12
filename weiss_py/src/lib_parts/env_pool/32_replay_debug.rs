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

