#[pyclass(name = "BatchOutMinimal")]
struct PyBatchOutMinimal {
    obs: Py<PyArray2<i32>>,
    masks: Py<PyArray2<u8>>,
    rewards: Py<PyArray1<f32>>,
    terminated: Py<PyArray1<bool>>,
    truncated: Py<PyArray1<bool>>,
    actor: Py<PyArray1<i8>>,
    decision_kind: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
    main_move_action: Py<PyArray1<bool>>,
    main_pass_action: Py<PyArray1<bool>>,
}

#[pyclass(name = "BatchOutMinimalI16")]
struct PyBatchOutMinimalI16 {
    obs: Py<PyArray2<i16>>,
    masks: Py<PyArray2<u8>>,
    rewards: Py<PyArray1<f32>>,
    terminated: Py<PyArray1<bool>>,
    truncated: Py<PyArray1<bool>>,
    actor: Py<PyArray1<i8>>,
    decision_kind: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
    main_move_action: Py<PyArray1<bool>>,
    main_pass_action: Py<PyArray1<bool>>,
}

#[pymethods]
impl PyBatchOutMinimalI16 {
    #[new]
    fn new(py: Python<'_>, num_envs: usize) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let obs = Array2::<i16>::zeros((num_envs, OBS_LEN));
        let masks = Array2::<u8>::zeros((num_envs, ACTION_SPACE_SIZE));
        let rewards = Array1::<f32>::zeros(num_envs);
        let terminated = Array1::<bool>::from_elem(num_envs, false);
        let truncated = Array1::<bool>::from_elem(num_envs, false);
        let actor = Array1::<i8>::zeros(num_envs);
        let decision_kind = Array1::<i8>::zeros(num_envs);
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
        let main_move_action = Array1::<bool>::from_elem(num_envs, false);
        let main_pass_action = Array1::<bool>::from_elem(num_envs, false);
        Ok(Self {
            obs: PyArray2::from_owned_array(py, obs).unbind(),
            masks: PyArray2::from_owned_array(py, masks).unbind(),
            rewards: PyArray1::from_owned_array(py, rewards).unbind(),
            terminated: PyArray1::from_owned_array(py, terminated).unbind(),
            truncated: PyArray1::from_owned_array(py, truncated).unbind(),
            actor: PyArray1::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray1::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray1::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray1::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray1::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray1::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray1::from_owned_array(py, main_pass_action).unbind(),
        })
    }

    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray2<i16>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn masks(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.masks.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray1<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray1<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_pass_action.clone_ref(py)
    }
}

#[pyclass(name = "BatchOutMinimalI16LegalIds")]
struct PyBatchOutMinimalI16LegalIds {
    obs: Py<PyArray2<i16>>,
    legal_ids: Py<PyArray1<u16>>,
    legal_action_meta: Py<PyArray2<u16>>,
    legal_offsets: Py<PyArray1<u32>>,
    rewards: Py<PyArray1<f32>>,
    terminated: Py<PyArray1<bool>>,
    truncated: Py<PyArray1<bool>>,
    actor: Py<PyArray1<i8>>,
    decision_kind: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
    main_move_action: Py<PyArray1<bool>>,
    main_pass_action: Py<PyArray1<bool>>,
}

#[pymethods]
impl PyBatchOutMinimalI16LegalIds {
    #[new]
    fn new(py: Python<'_>, num_envs: usize) -> PyResult<Self> {
        ensure_action_space_u16()?;
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let obs = Array2::<i16>::zeros((num_envs, OBS_LEN));
        let legal_ids_len = num_envs.checked_mul(ACTION_SPACE_SIZE).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_ids size overflow (num_envs * action_space)",
            )
        })?;
        let legal_ids = Array1::<u16>::zeros(legal_ids_len);
        let legal_action_meta =
            Array2::<u16>::from_elem((legal_ids_len, ACTION_META_WIDTH), ACTION_META_UNUSED);
        let legal_offsets_len = num_envs.checked_add(1).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "legal_offsets size overflow (num_envs + 1)",
            )
        })?;
        let legal_offsets = Array1::<u32>::zeros(legal_offsets_len);
        let rewards = Array1::<f32>::zeros(num_envs);
        let terminated = Array1::<bool>::from_elem(num_envs, false);
        let truncated = Array1::<bool>::from_elem(num_envs, false);
        let actor = Array1::<i8>::zeros(num_envs);
        let decision_kind = Array1::<i8>::zeros(num_envs);
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
        let main_move_action = Array1::<bool>::from_elem(num_envs, false);
        let main_pass_action = Array1::<bool>::from_elem(num_envs, false);
        Ok(Self {
            obs: PyArray2::from_owned_array(py, obs).unbind(),
            legal_ids: PyArray1::from_owned_array(py, legal_ids).unbind(),
            legal_action_meta: PyArray2::from_owned_array(py, legal_action_meta).unbind(),
            legal_offsets: PyArray1::from_owned_array(py, legal_offsets).unbind(),
            rewards: PyArray1::from_owned_array(py, rewards).unbind(),
            terminated: PyArray1::from_owned_array(py, terminated).unbind(),
            truncated: PyArray1::from_owned_array(py, truncated).unbind(),
            actor: PyArray1::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray1::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray1::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray1::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray1::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray1::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray1::from_owned_array(py, main_pass_action).unbind(),
        })
    }

    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray2<i16>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn legal_ids(&self, py: Python<'_>) -> Py<PyArray1<u16>> {
        self.legal_ids.clone_ref(py)
    }
    #[getter]
    fn legal_action_meta(&self, py: Python<'_>) -> Py<PyArray2<u16>> {
        self.legal_action_meta.clone_ref(py)
    }
    #[getter]
    fn legal_offsets(&self, py: Python<'_>) -> Py<PyArray1<u32>> {
        self.legal_offsets.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray1<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray1<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_pass_action.clone_ref(py)
    }
}

#[pymethods]
impl PyBatchOutMinimal {
    #[new]
    fn new(py: Python<'_>, num_envs: usize) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let obs = Array2::<i32>::zeros((num_envs, OBS_LEN));
        let masks = Array2::<u8>::zeros((num_envs, ACTION_SPACE_SIZE));
        let rewards = Array1::<f32>::zeros(num_envs);
        let terminated = Array1::<bool>::from_elem(num_envs, false);
        let truncated = Array1::<bool>::from_elem(num_envs, false);
        let actor = Array1::<i8>::zeros(num_envs);
        let decision_kind = Array1::<i8>::zeros(num_envs);
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
        let main_move_action = Array1::<bool>::from_elem(num_envs, false);
        let main_pass_action = Array1::<bool>::from_elem(num_envs, false);
        Ok(Self {
            obs: PyArray2::from_owned_array(py, obs).unbind(),
            masks: PyArray2::from_owned_array(py, masks).unbind(),
            rewards: PyArray1::from_owned_array(py, rewards).unbind(),
            terminated: PyArray1::from_owned_array(py, terminated).unbind(),
            truncated: PyArray1::from_owned_array(py, truncated).unbind(),
            actor: PyArray1::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray1::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray1::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray1::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray1::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray1::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray1::from_owned_array(py, main_pass_action).unbind(),
        })
    }

    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray2<i32>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn masks(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.masks.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray1<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray1<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_pass_action.clone_ref(py)
    }
}

#[pyclass(name = "BatchOutMinimalNoMask")]
struct PyBatchOutMinimalNoMask {
    obs: Py<PyArray2<i32>>,
    rewards: Py<PyArray1<f32>>,
    terminated: Py<PyArray1<bool>>,
    truncated: Py<PyArray1<bool>>,
    actor: Py<PyArray1<i8>>,
    decision_kind: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
    main_move_action: Py<PyArray1<bool>>,
    main_pass_action: Py<PyArray1<bool>>,
}

#[pymethods]
impl PyBatchOutMinimalNoMask {
    #[new]
    fn new(py: Python<'_>, num_envs: usize) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let obs = Array2::<i32>::zeros((num_envs, OBS_LEN));
        let rewards = Array1::<f32>::zeros(num_envs);
        let terminated = Array1::<bool>::from_elem(num_envs, false);
        let truncated = Array1::<bool>::from_elem(num_envs, false);
        let actor = Array1::<i8>::zeros(num_envs);
        let decision_kind = Array1::<i8>::zeros(num_envs);
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
        let main_move_action = Array1::<bool>::from_elem(num_envs, false);
        let main_pass_action = Array1::<bool>::from_elem(num_envs, false);
        Ok(Self {
            obs: PyArray2::from_owned_array(py, obs).unbind(),
            rewards: PyArray1::from_owned_array(py, rewards).unbind(),
            terminated: PyArray1::from_owned_array(py, terminated).unbind(),
            truncated: PyArray1::from_owned_array(py, truncated).unbind(),
            actor: PyArray1::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray1::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray1::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray1::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray1::from_owned_array(py, spec_hash).unbind(),
            main_move_action: PyArray1::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray1::from_owned_array(py, main_pass_action).unbind(),
        })
    }

    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray2<i32>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray1<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray1<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_pass_action.clone_ref(py)
    }
}

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

#[pyclass(name = "BatchOutDebug")]
struct PyBatchOutDebug {
    obs: Py<PyArray2<i32>>,
    masks: Py<PyArray2<u8>>,
    rewards: Py<PyArray1<f32>>,
    terminated: Py<PyArray1<bool>>,
    truncated: Py<PyArray1<bool>>,
    actor: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
    decision_kind: Py<PyArray1<i8>>,
    main_move_action: Py<PyArray1<bool>>,
    main_pass_action: Py<PyArray1<bool>>,
    state_fingerprint: Py<PyArray1<u64>>,
    events_fingerprint: Py<PyArray1<u64>>,
    mask_fingerprint: Py<PyArray1<u64>>,
    event_counts: Py<PyArray1<u16>>,
    event_codes: Py<PyArray2<u32>>,
}

#[pymethods]
impl PyBatchOutDebug {
    #[new]
    fn new(py: Python<'_>, num_envs: usize, event_capacity: usize) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let obs = Array2::<i32>::zeros((num_envs, OBS_LEN));
        let masks = Array2::<u8>::zeros((num_envs, ACTION_SPACE_SIZE));
        let rewards = Array1::<f32>::zeros(num_envs);
        let terminated = Array1::<bool>::from_elem(num_envs, false);
        let truncated = Array1::<bool>::from_elem(num_envs, false);
        let actor = Array1::<i8>::zeros(num_envs);
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
        let decision_kind = Array1::<i8>::zeros(num_envs);
        let main_move_action = Array1::<bool>::from_elem(num_envs, false);
        let main_pass_action = Array1::<bool>::from_elem(num_envs, false);
        let state_fingerprint = Array1::<u64>::zeros(num_envs);
        let events_fingerprint = Array1::<u64>::zeros(num_envs);
        let mask_fingerprint = Array1::<u64>::zeros(num_envs);
        let event_counts = Array1::<u16>::zeros(num_envs);
        let event_codes = Array2::<u32>::zeros((num_envs, event_capacity));
        Ok(Self {
            obs: PyArray2::from_owned_array(py, obs).unbind(),
            masks: PyArray2::from_owned_array(py, masks).unbind(),
            rewards: PyArray1::from_owned_array(py, rewards).unbind(),
            terminated: PyArray1::from_owned_array(py, terminated).unbind(),
            truncated: PyArray1::from_owned_array(py, truncated).unbind(),
            actor: PyArray1::from_owned_array(py, actor).unbind(),
            decision_id: PyArray1::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray1::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray1::from_owned_array(py, spec_hash).unbind(),
            decision_kind: PyArray1::from_owned_array(py, decision_kind).unbind(),
            main_move_action: PyArray1::from_owned_array(py, main_move_action).unbind(),
            main_pass_action: PyArray1::from_owned_array(py, main_pass_action).unbind(),
            state_fingerprint: PyArray1::from_owned_array(py, state_fingerprint).unbind(),
            events_fingerprint: PyArray1::from_owned_array(py, events_fingerprint).unbind(),
            mask_fingerprint: PyArray1::from_owned_array(py, mask_fingerprint).unbind(),
            event_counts: PyArray1::from_owned_array(py, event_counts).unbind(),
            event_codes: PyArray2::from_owned_array(py, event_codes).unbind(),
        })
    }

    #[getter]
    fn obs(&self, py: Python<'_>) -> Py<PyArray2<i32>> {
        self.obs.clone_ref(py)
    }
    #[getter]
    fn masks(&self, py: Python<'_>) -> Py<PyArray2<u8>> {
        self.masks.clone_ref(py)
    }
    #[getter]
    fn rewards(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.rewards.clone_ref(py)
    }
    #[getter]
    fn terminated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.terminated.clone_ref(py)
    }
    #[getter]
    fn truncated(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.truncated.clone_ref(py)
    }
    #[getter]
    fn actor(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.actor.clone_ref(py)
    }
    #[getter]
    fn decision_id(&self, py: Python<'_>) -> Py<PyArray1<u32>> {
        self.decision_id.clone_ref(py)
    }
    #[getter]
    fn engine_status(&self, py: Python<'_>) -> Py<PyArray1<u8>> {
        self.engine_status.clone_ref(py)
    }
    #[getter]
    fn spec_hash(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.spec_hash.clone_ref(py)
    }
    #[getter]
    fn decision_kind(&self, py: Python<'_>) -> Py<PyArray1<i8>> {
        self.decision_kind.clone_ref(py)
    }
    #[getter]
    fn main_move_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_move_action.clone_ref(py)
    }
    #[getter]
    fn main_pass_action(&self, py: Python<'_>) -> Py<PyArray1<bool>> {
        self.main_pass_action.clone_ref(py)
    }
    #[getter]
    fn state_fingerprint(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.state_fingerprint.clone_ref(py)
    }
    #[getter]
    fn events_fingerprint(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.events_fingerprint.clone_ref(py)
    }
    #[getter]
    fn mask_fingerprint(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.mask_fingerprint.clone_ref(py)
    }
    #[getter]
    fn event_counts(&self, py: Python<'_>) -> Py<PyArray1<u16>> {
        self.event_counts.clone_ref(py)
    }
    #[getter]
    fn event_codes(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.event_codes.clone_ref(py)
    }
}

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
