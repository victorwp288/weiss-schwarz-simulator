#[pyclass(name = "BatchOutTrajectory")]
struct PyBatchOutTrajectory {
    steps: usize,
    obs: Py<PyArray3<i32>>,
    masks: Py<PyArray3<u8>>,
    rewards: Py<PyArray2<f32>>,
    terminated: Py<PyArray2<bool>>,
    truncated: Py<PyArray2<bool>>,
    actor: Py<PyArray2<i8>>,
    decision_kind: Py<PyArray2<i8>>,
    decision_id: Py<PyArray2<u32>>,
    engine_status: Py<PyArray2<u8>>,
    spec_hash: Py<PyArray2<u64>>,
    main_move_action: Py<PyArray2<bool>>,
    main_pass_action: Py<PyArray2<bool>>,
    actions: Py<PyArray2<u32>>,
}

#[pymethods]
impl PyBatchOutTrajectory {
    #[new]
    fn new(py: Python<'_>, steps: usize, num_envs: usize) -> PyResult<Self> {
        if steps == 0 || num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps and num_envs must be > 0",
            ));
        }
        let env_steps = steps.checked_mul(num_envs).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "trajectory size overflow (steps * num_envs)",
            )
        })?;
        let _ = env_steps.checked_mul(OBS_LEN).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "obs size overflow (steps * num_envs * obs_len)",
            )
        })?;
        let _ = env_steps.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "masks size overflow (steps * num_envs * action_space)",
            )
        })?;
        let obs = Array3::<i32>::zeros((steps, num_envs, OBS_LEN));
        let masks = Array3::<u8>::zeros((steps, num_envs, ACTION_SPACE_SIZE));
        let rewards = Array2::<f32>::zeros((steps, num_envs));
        let terminated = Array2::<bool>::from_elem((steps, num_envs), false);
        let truncated = Array2::<bool>::from_elem((steps, num_envs), false);
        let actor = Array2::<i8>::zeros((steps, num_envs));
        let decision_kind = Array2::<i8>::zeros((steps, num_envs));
        let decision_id = Array2::<u32>::zeros((steps, num_envs));
        let engine_status = Array2::<u8>::zeros((steps, num_envs));
        let spec_hash = Array2::<u64>::from_elem((steps, num_envs), SPEC_HASH);
        let main_move_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let main_pass_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let actions = Array2::<u32>::zeros((steps, num_envs));
        Ok(Self {
            steps,
            obs: PyArray3::from_owned_array(py, obs).unbind(),
            masks: PyArray3::from_owned_array(py, masks).unbind(),
            rewards: PyArray2::from_owned_array(py, rewards).unbind(),
            terminated: PyArray2::from_owned_array(py, terminated).unbind(),
            truncated: PyArray2::from_owned_array(py, truncated).unbind(),
            actor: PyArray2::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray2::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray2::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray2::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray2::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray2::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray2::from_owned_array(py, main_pass_action).unbind(),
            actions: PyArray2::from_owned_array(py, actions).unbind(),
        })
    }

    #[getter]
    fn steps(&self) -> usize {
        self.steps
    }
    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray3<i32>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn masks(&self, py: Python<'_>) -> Py<PyArray3<u8>> {
        self.masks.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray2<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray2<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_pass_action.clone_ref(py)
    }
    #[getter]
    fn actions(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.actions.clone_ref(py)
    }
}

#[pyclass(name = "BatchOutTrajectoryI16")]
struct PyBatchOutTrajectoryI16 {
    steps: usize,
    obs: Py<PyArray3<i16>>,
    masks: Py<PyArray3<u8>>,
    rewards: Py<PyArray2<f32>>,
    terminated: Py<PyArray2<bool>>,
    truncated: Py<PyArray2<bool>>,
    actor: Py<PyArray2<i8>>,
    decision_kind: Py<PyArray2<i8>>,
    decision_id: Py<PyArray2<u32>>,
    engine_status: Py<PyArray2<u8>>,
    spec_hash: Py<PyArray2<u64>>,
    main_move_action: Py<PyArray2<bool>>,
    main_pass_action: Py<PyArray2<bool>>,
    actions: Py<PyArray2<u32>>,
}

#[pymethods]
impl PyBatchOutTrajectoryI16 {
    #[new]
    fn new(py: Python<'_>, steps: usize, num_envs: usize) -> PyResult<Self> {
        if steps == 0 || num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps and num_envs must be > 0",
            ));
        }
        let env_steps = steps.checked_mul(num_envs).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "trajectory size overflow (steps * num_envs)",
            )
        })?;
        let _ = env_steps.checked_mul(OBS_LEN).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "obs size overflow (steps * num_envs * obs_len)",
            )
        })?;
        let _ = env_steps.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "masks size overflow (steps * num_envs * action_space)",
            )
        })?;
        let obs = Array3::<i16>::zeros((steps, num_envs, OBS_LEN));
        let masks = Array3::<u8>::zeros((steps, num_envs, ACTION_SPACE_SIZE));
        let rewards = Array2::<f32>::zeros((steps, num_envs));
        let terminated = Array2::<bool>::from_elem((steps, num_envs), false);
        let truncated = Array2::<bool>::from_elem((steps, num_envs), false);
        let actor = Array2::<i8>::zeros((steps, num_envs));
        let decision_kind = Array2::<i8>::zeros((steps, num_envs));
        let decision_id = Array2::<u32>::zeros((steps, num_envs));
        let engine_status = Array2::<u8>::zeros((steps, num_envs));
        let spec_hash = Array2::<u64>::from_elem((steps, num_envs), SPEC_HASH);
        let main_move_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let main_pass_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let actions = Array2::<u32>::zeros((steps, num_envs));
        Ok(Self {
            steps,
            obs: PyArray3::from_owned_array(py, obs).unbind(),
            masks: PyArray3::from_owned_array(py, masks).unbind(),
            rewards: PyArray2::from_owned_array(py, rewards).unbind(),
            terminated: PyArray2::from_owned_array(py, terminated).unbind(),
            truncated: PyArray2::from_owned_array(py, truncated).unbind(),
            actor: PyArray2::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray2::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray2::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray2::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray2::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray2::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray2::from_owned_array(py, main_pass_action).unbind(),
            actions: PyArray2::from_owned_array(py, actions).unbind(),
        })
    }

    #[getter]
    fn steps(&self) -> usize {
        self.steps
    }
    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray3<i16>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn masks(&self, py: Python<'_>) -> Py<PyArray3<u8>> {
        self.masks.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray2<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray2<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_pass_action.clone_ref(py)
    }
    #[getter]
    fn actions(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.actions.clone_ref(py)
    }
}

#[pyclass(name = "BatchOutTrajectoryI16LegalIds")]
struct PyBatchOutTrajectoryI16LegalIds {
    steps: usize,
    obs: Py<PyArray3<i16>>,
    legal_ids: Py<PyArray2<u16>>,
    legal_action_meta: Py<PyArray3<u16>>,
    legal_offsets: Py<PyArray2<u32>>,
    rewards: Py<PyArray2<f32>>,
    terminated: Py<PyArray2<bool>>,
    truncated: Py<PyArray2<bool>>,
    actor: Py<PyArray2<i8>>,
    decision_kind: Py<PyArray2<i8>>,
    decision_id: Py<PyArray2<u32>>,
    engine_status: Py<PyArray2<u8>>,
    episode_seed: Py<PyArray2<u64>>,
    spec_hash: Py<PyArray2<u64>>,
    main_move_action: Py<PyArray2<bool>>,
    main_pass_action: Py<PyArray2<bool>>,
    actions: Py<PyArray2<u32>>,
}

#[pymethods]
impl PyBatchOutTrajectoryI16LegalIds {
    #[new]
    fn new(py: Python<'_>, steps: usize, num_envs: usize) -> PyResult<Self> {
        ensure_action_space_u16()?;
        if steps == 0 || num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps and num_envs must be > 0",
            ));
        }
        let env_steps = steps.checked_mul(num_envs).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "trajectory size overflow (steps * num_envs)",
            )
        })?;
        let _ = env_steps.checked_mul(OBS_LEN).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "obs size overflow (steps * num_envs * obs_len)",
            )
        })?;
        let obs = Array3::<i16>::zeros((steps, num_envs, OBS_LEN));
        let legal_ids_len = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_ids size overflow (num_envs * action_space)",
            )
        })?;
        let _ = steps.checked_mul(legal_ids_len).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_ids size overflow (steps * num_envs * action_space)",
            )
        })?;
        let legal_ids = Array2::<u16>::zeros((steps, legal_ids_len));
        let legal_action_meta = Array3::<u16>::from_elem(
            (steps, legal_ids_len, ACTION_META_WIDTH),
            ACTION_META_UNUSED,
        );
        let legal_offsets_len = num_envs.checked_add(1).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_offsets size overflow (num_envs + 1)",
            )
        })?;
        let _ = steps.checked_mul(legal_offsets_len).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_offsets size overflow (steps * (num_envs + 1))",
            )
        })?;
        let legal_offsets = Array2::<u32>::zeros((steps, legal_offsets_len));
        let rewards = Array2::<f32>::zeros((steps, num_envs));
        let terminated = Array2::<bool>::from_elem((steps, num_envs), false);
        let truncated = Array2::<bool>::from_elem((steps, num_envs), false);
        let actor = Array2::<i8>::zeros((steps, num_envs));
        let decision_kind = Array2::<i8>::zeros((steps, num_envs));
        let decision_id = Array2::<u32>::zeros((steps, num_envs));
        let engine_status = Array2::<u8>::zeros((steps, num_envs));
        let episode_seed = Array2::<u64>::zeros((steps, num_envs));
        let spec_hash = Array2::<u64>::from_elem((steps, num_envs), SPEC_HASH);
        let main_move_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let main_pass_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let actions = Array2::<u32>::zeros((steps, num_envs));
        Ok(Self {
            steps,
            obs: PyArray3::from_owned_array(py, obs).unbind(),
            legal_ids: PyArray2::from_owned_array(py, legal_ids).unbind(),
            legal_action_meta: PyArray3::from_owned_array(py, legal_action_meta).unbind(),
            legal_offsets: PyArray2::from_owned_array(py, legal_offsets).unbind(),
            rewards: PyArray2::from_owned_array(py, rewards).unbind(),
            terminated: PyArray2::from_owned_array(py, terminated).unbind(),
            truncated: PyArray2::from_owned_array(py, truncated).unbind(),
            actor: PyArray2::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray2::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray2::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray2::from_owned_array(py, engine_status).unbind(),
            episode_seed: PyArray2::from_owned_array(py, episode_seed).unbind(),
            spec_hash: PyArray2::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray2::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray2::from_owned_array(py, main_pass_action).unbind(),
            actions: PyArray2::from_owned_array(py, actions).unbind(),
        })
    }

    #[getter]
    fn steps(&self) -> usize {
        self.steps
    }
    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray3<i16>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn legal_ids(&self, py: Python<'_>) -> Py<PyArray2<u16>> {
        self.legal_ids.clone_ref(py)
    }
    #[getter]
    fn legal_action_meta(&self, py: Python<'_>) -> Py<PyArray3<u16>> {
        self.legal_action_meta.clone_ref(py)
    }
    #[getter]
    fn legal_offsets(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.legal_offsets.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray2<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn episode_seed(&self, py: Python<'_>) -> Py<PyArray2<u64>> {
        self.episode_seed.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray2<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_pass_action.clone_ref(py)
    }
    #[getter]
    fn actions(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.actions.clone_ref(py)
    }
}

#[pyclass(name = "BatchOutTrajectoryNoMask")]
struct PyBatchOutTrajectoryNoMask {
    steps: usize,
    obs: Py<PyArray3<i32>>,
    rewards: Py<PyArray2<f32>>,
    terminated: Py<PyArray2<bool>>,
    truncated: Py<PyArray2<bool>>,
    actor: Py<PyArray2<i8>>,
    decision_kind: Py<PyArray2<i8>>,
    decision_id: Py<PyArray2<u32>>,
    engine_status: Py<PyArray2<u8>>,
    spec_hash: Py<PyArray2<u64>>,
    main_move_action: Py<PyArray2<bool>>,
    main_pass_action: Py<PyArray2<bool>>,
    actions: Py<PyArray2<u32>>,
}

#[pymethods]
impl PyBatchOutTrajectoryNoMask {
    #[new]
    fn new(py: Python<'_>, steps: usize, num_envs: usize) -> PyResult<Self> {
        if steps == 0 || num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps and num_envs must be > 0",
            ));
        }
        let env_steps = steps.checked_mul(num_envs).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "trajectory size overflow (steps * num_envs)",
            )
        })?;
        let _ = env_steps.checked_mul(OBS_LEN).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "obs size overflow (steps * num_envs * obs_len)",
            )
        })?;
        let obs = Array3::<i32>::zeros((steps, num_envs, OBS_LEN));
        let rewards = Array2::<f32>::zeros((steps, num_envs));
        let terminated = Array2::<bool>::from_elem((steps, num_envs), false);
        let truncated = Array2::<bool>::from_elem((steps, num_envs), false);
        let actor = Array2::<i8>::zeros((steps, num_envs));
        let decision_kind = Array2::<i8>::zeros((steps, num_envs));
        let decision_id = Array2::<u32>::zeros((steps, num_envs));
        let engine_status = Array2::<u8>::zeros((steps, num_envs));
        let spec_hash = Array2::<u64>::from_elem((steps, num_envs), SPEC_HASH);
        let main_move_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let main_pass_action = Array2::<bool>::from_elem((steps, num_envs), false);
        let actions = Array2::<u32>::zeros((steps, num_envs));
        Ok(Self {
            steps,
            obs: PyArray3::from_owned_array(py, obs).unbind(),
            rewards: PyArray2::from_owned_array(py, rewards).unbind(),
            terminated: PyArray2::from_owned_array(py, terminated).unbind(),
            truncated: PyArray2::from_owned_array(py, truncated).unbind(),
            actor: PyArray2::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray2::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray2::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray2::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray2::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray2::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray2::from_owned_array(py, main_pass_action).unbind(),
            actions: PyArray2::from_owned_array(py, actions).unbind(),
        })
    }

    #[getter]
    fn steps(&self) -> usize {
        self.steps
    }
    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray3<i32>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray2<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray2<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray2<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray2<bool>> {
        self.main_pass_action.clone_ref(py)
    }
    #[getter]
    fn actions(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.actions.clone_ref(py)
    }
}

