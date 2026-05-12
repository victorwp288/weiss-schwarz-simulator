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

#[pyclass(name = "BatchOutMinimalI16LegalIdsNoMeta")]
struct PyBatchOutMinimalI16LegalIdsNoMeta {
    obs: Py<PyArray2<i16>>,
    legal_ids: Py<PyArray1<u16>>,
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
impl PyBatchOutMinimalI16LegalIdsNoMeta {
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

