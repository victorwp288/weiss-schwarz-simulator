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
    reward_components: Py<PyArray2<f32>>,
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
        let reward_components = Array2::<f32>::zeros((num_envs, REWARD_COMPONENT_WIDTH));
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
            reward_components: PyArray2::from_owned_array(py, reward_components).unbind(),
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
    fn reward_components(&self, py: Python<'_>) -> Py<PyArray2<f32>> {
        self.reward_components.clone_ref(py)
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

