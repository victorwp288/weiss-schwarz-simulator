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

    fn legal_action_context_v1_into<'py>(
        &mut self,
        py: Python<'py>,
        context: Py<PyArray2<i32>>,
    ) -> PyResult<usize> {
        let expected_rows = self.pool.envs.len().checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_action_context size overflow (num_envs * action_space)",
            )
        })?;
        ensure_first_two_dims(
            py,
            "legal_action_context",
            &context,
            expected_rows,
            LEGAL_ACTION_CONTEXT_V1_WIDTH,
        )?;
        let mut context_arr = array_mut(py, &context);
        let context_slice = context_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_action_context not contiguous",
            )
        })?;
        py.allow_threads(|| self.pool.legal_action_context_v1_batch_into(context_slice))
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

    fn choose_heuristic_public_profile_actions_into<'py>(
        &mut self,
        py: Python<'py>,
        env_indices: PyReadonlyArray1<u32>,
        actions: Py<PyArray1<u16>>,
        profile_name: &str,
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
        py.allow_threads(|| {
            self.pool.choose_heuristic_public_profile_actions_into(
                &indices,
                actions_slice,
                profile_name,
            )
        })
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

