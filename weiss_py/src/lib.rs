use std::sync::Arc;

use numpy::ndarray::{Array1, Array2, ArrayViewMut, Dimension};
use numpy::{Element, PyArray, PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyType};

use weiss_core::config::{ErrorPolicy, ObservationVisibility};
use weiss_core::encode::{
    ACTION_ENCODING_VERSION, ACTION_SPACE_SIZE, OBS_ENCODING_VERSION, OBS_LEN, PASS_ACTION_ID,
    SPEC_HASH,
};
use weiss_core::legal::ActionDesc;
use weiss_core::pool::{BatchOutDebug, BatchOutMinimal};
use weiss_core::{CardDb, CurriculumConfig, DebugConfig, EnvConfig, EnvPool, RewardConfig};

fn parse_reward_config(reward_json: Option<String>) -> PyResult<RewardConfig> {
    if let Some(json) = reward_json {
        serde_json::from_str::<RewardConfig>(&json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("reward_json parse error: {e}"))
        })
    } else {
        Ok(RewardConfig::default())
    }
}

fn parse_curriculum_config(curriculum_json: Option<String>) -> PyResult<CurriculumConfig> {
    if let Some(json) = curriculum_json {
        serde_json::from_str::<CurriculumConfig>(&json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "curriculum_json parse error: {e}"
            ))
        })
    } else {
        Ok(CurriculumConfig {
            enable_visibility_policies: true,
            ..Default::default()
        })
    }
}

fn parse_error_policy(error_policy: Option<String>) -> PyResult<ErrorPolicy> {
    if let Some(policy) = error_policy {
        match policy.to_lowercase().as_str() {
            "strict" => Ok(ErrorPolicy::Strict),
            "lenient_terminate" | "lenient" => Ok(ErrorPolicy::LenientTerminate),
            "lenient_noop" => Ok(ErrorPolicy::LenientNoop),
            other => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "error_policy must be one of strict, lenient_terminate, lenient_noop (got {other})"
            ))),
        }
    } else {
        Ok(ErrorPolicy::LenientTerminate)
    }
}

fn parse_observation_visibility(
    observation_visibility: Option<String>,
) -> PyResult<ObservationVisibility> {
    if let Some(mode) = observation_visibility {
        match mode.to_lowercase().as_str() {
            "public" => Ok(ObservationVisibility::Public),
            "full" => Ok(ObservationVisibility::Full),
            other => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "observation_visibility must be public or full (got {other})"
            ))),
        }
    } else {
        Ok(ObservationVisibility::Public)
    }
}

fn build_debug_config(
    fingerprint_every_n: Option<u32>,
    event_ring_capacity: Option<usize>,
) -> DebugConfig {
    DebugConfig {
        fingerprint_every_n: fingerprint_every_n.unwrap_or(0),
        event_ring_capacity: event_ring_capacity.unwrap_or(0),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_env_config(
    db_path: String,
    deck_lists: Vec<Vec<u32>>,
    deck_ids: Option<Vec<u32>>,
    max_decisions: u32,
    max_ticks: u32,
    reward: RewardConfig,
    error_policy: ErrorPolicy,
    observation_visibility: ObservationVisibility,
) -> PyResult<(Arc<CardDb>, EnvConfig)> {
    let db = CardDb::load(db_path).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Card DB load failed: {e}"))
    })?;
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
    let config = EnvConfig {
        deck_lists: [deck_lists[0].clone(), deck_lists[1].clone()],
        deck_ids: [deck_ids_vec[0], deck_ids_vec[1]],
        max_decisions,
        max_ticks,
        reward,
        error_policy,
        observation_visibility,
        end_condition_policy: Default::default(),
    };
    Ok((Arc::new(db), config))
}

fn array_mut<'py, T, D>(py: Python<'py>, arr: &'py Py<PyArray<T, D>>) -> ArrayViewMut<'py, T, D>
where
    D: Dimension,
    T: Element,
{
    unsafe { arr.bind(py).as_array_mut() }
}

fn action_desc_to_pydict(py: Python<'_>, action: &ActionDesc) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    match action {
        ActionDesc::MulliganConfirm => {
            dict.set_item("kind", "mulligan_confirm")?;
        }
        ActionDesc::MulliganSelect { hand_index } => {
            dict.set_item("kind", "mulligan_select")?;
            dict.set_item("hand_index", hand_index)?;
        }
        ActionDesc::Pass => {
            dict.set_item("kind", "pass")?;
        }
        ActionDesc::Clock { hand_index } => {
            dict.set_item("kind", "clock")?;
            dict.set_item("hand_index", hand_index)?;
        }
        ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => {
            dict.set_item("kind", "main_play_character")?;
            dict.set_item("hand_index", hand_index)?;
            dict.set_item("stage_slot", stage_slot)?;
        }
        ActionDesc::MainPlayEvent { hand_index } => {
            dict.set_item("kind", "main_play_event")?;
            dict.set_item("hand_index", hand_index)?;
        }
        ActionDesc::MainMove { from_slot, to_slot } => {
            dict.set_item("kind", "main_move")?;
            dict.set_item("from_slot", from_slot)?;
            dict.set_item("to_slot", to_slot)?;
        }
        ActionDesc::MainActivateAbility {
            slot,
            ability_index,
        } => {
            dict.set_item("kind", "main_activate_ability")?;
            dict.set_item("slot", slot)?;
            dict.set_item("ability_index", ability_index)?;
        }
        ActionDesc::ClimaxPlay { hand_index } => {
            dict.set_item("kind", "climax_play")?;
            dict.set_item("hand_index", hand_index)?;
        }
        ActionDesc::Attack { slot, attack_type } => {
            dict.set_item("kind", "attack")?;
            dict.set_item("slot", slot)?;
            dict.set_item("attack_type", format!("{:?}", attack_type))?;
        }
        ActionDesc::CounterPlay { hand_index } => {
            dict.set_item("kind", "counter_play")?;
            dict.set_item("hand_index", hand_index)?;
        }
        ActionDesc::LevelUp { index } => {
            dict.set_item("kind", "level_up")?;
            dict.set_item("index", index)?;
        }
        ActionDesc::EncorePay { slot } => {
            dict.set_item("kind", "encore_pay")?;
            dict.set_item("slot", slot)?;
        }
        ActionDesc::EncoreDecline { slot } => {
            dict.set_item("kind", "encore_decline")?;
            dict.set_item("slot", slot)?;
        }
        ActionDesc::TriggerOrder { index } => {
            dict.set_item("kind", "trigger_order")?;
            dict.set_item("index", index)?;
        }
        ActionDesc::ChoiceSelect { index } => {
            dict.set_item("kind", "choice_select")?;
            dict.set_item("index", index)?;
        }
        ActionDesc::ChoicePrevPage => {
            dict.set_item("kind", "choice_prev_page")?;
        }
        ActionDesc::ChoiceNextPage => {
            dict.set_item("kind", "choice_next_page")?;
        }
        ActionDesc::Concede => {
            dict.set_item("kind", "concede")?;
        }
    }
    Ok(dict.into())
}

#[pyclass(name = "BatchOutMinimal")]
struct PyBatchOutMinimal {
    obs: Py<PyArray2<i32>>,
    masks: Py<PyArray2<u8>>,
    rewards: Py<PyArray1<f32>>,
    terminated: Py<PyArray1<bool>>,
    truncated: Py<PyArray1<bool>>,
    actor: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
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
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
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
    state_fingerprint: Py<PyArray1<u64>>,
    events_fingerprint: Py<PyArray1<u64>>,
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
        let state_fingerprint = Array1::<u64>::zeros(num_envs);
        let events_fingerprint = Array1::<u64>::zeros(num_envs);
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
            state_fingerprint: PyArray1::from_owned_array(py, state_fingerprint).unbind(),
            events_fingerprint: PyArray1::from_owned_array(py, events_fingerprint).unbind(),
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
    fn state_fingerprint(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.state_fingerprint.clone_ref(py)
    }
    #[getter]
    fn events_fingerprint(&self, py: Python<'_>) -> Py<PyArray1<u64>> {
        self.events_fingerprint.clone_ref(py)
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

#[pyclass(name = "EnvPool")]
struct PyEnvPool {
    pool: EnvPool,
}

#[pymethods]
impl PyEnvPool {
    #[classmethod]
    #[pyo3(signature = (
        num_envs,
        db_path,
        deck_lists,
        deck_ids=None,
        max_decisions=2000,
        max_ticks=100_000,
        seed=0,
        curriculum_json=None,
        reward_json=None,
        num_threads=None,
        debug_fingerprint_every_n=0,
        debug_event_ring_capacity=0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new_rl_train(
        _cls: &Bound<'_, PyType>,
        num_envs: usize,
        db_path: String,
        deck_lists: Vec<Vec<u32>>,
        deck_ids: Option<Vec<u32>>,
        max_decisions: u32,
        max_ticks: u32,
        seed: u64,
        curriculum_json: Option<String>,
        reward_json: Option<String>,
        num_threads: Option<usize>,
        debug_fingerprint_every_n: u32,
        debug_event_ring_capacity: usize,
    ) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let reward = parse_reward_config(reward_json)?;
        let curriculum = parse_curriculum_config(curriculum_json)?;
        let (db, config) = build_env_config(
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            reward,
            ErrorPolicy::LenientTerminate,
            ObservationVisibility::Public,
        )?;
        let debug = build_debug_config(
            Some(debug_fingerprint_every_n),
            Some(debug_event_ring_capacity),
        );
        let pool =
            EnvPool::new_rl_train(num_envs, db, config, curriculum, seed, num_threads, debug)
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "EnvPool init failed: {e}"
                    ))
                })?;
        Ok(Self { pool })
    }

    #[classmethod]
    #[pyo3(signature = (
        num_envs,
        db_path,
        deck_lists,
        deck_ids=None,
        max_decisions=2000,
        max_ticks=100_000,
        seed=0,
        curriculum_json=None,
        reward_json=None,
        num_threads=None,
        debug_fingerprint_every_n=0,
        debug_event_ring_capacity=0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new_rl_eval(
        _cls: &Bound<'_, PyType>,
        num_envs: usize,
        db_path: String,
        deck_lists: Vec<Vec<u32>>,
        deck_ids: Option<Vec<u32>>,
        max_decisions: u32,
        max_ticks: u32,
        seed: u64,
        curriculum_json: Option<String>,
        reward_json: Option<String>,
        num_threads: Option<usize>,
        debug_fingerprint_every_n: u32,
        debug_event_ring_capacity: usize,
    ) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let reward = parse_reward_config(reward_json)?;
        let curriculum = parse_curriculum_config(curriculum_json)?;
        let (db, config) = build_env_config(
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            reward,
            ErrorPolicy::LenientTerminate,
            ObservationVisibility::Public,
        )?;
        let debug = build_debug_config(
            Some(debug_fingerprint_every_n),
            Some(debug_event_ring_capacity),
        );
        let pool = EnvPool::new_rl_eval(num_envs, db, config, curriculum, seed, num_threads, debug)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "EnvPool init failed: {e}"
                ))
            })?;
        Ok(Self { pool })
    }

    #[classmethod]
    #[pyo3(signature = (
        num_envs,
        db_path,
        deck_lists,
        deck_ids=None,
        max_decisions=2000,
        max_ticks=100_000,
        seed=0,
        curriculum_json=None,
        reward_json=None,
        error_policy=None,
        observation_visibility=None,
        num_threads=None,
        debug_fingerprint_every_n=0,
        debug_event_ring_capacity=0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new_debug(
        _cls: &Bound<'_, PyType>,
        num_envs: usize,
        db_path: String,
        deck_lists: Vec<Vec<u32>>,
        deck_ids: Option<Vec<u32>>,
        max_decisions: u32,
        max_ticks: u32,
        seed: u64,
        curriculum_json: Option<String>,
        reward_json: Option<String>,
        error_policy: Option<String>,
        observation_visibility: Option<String>,
        num_threads: Option<usize>,
        debug_fingerprint_every_n: u32,
        debug_event_ring_capacity: usize,
    ) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
        let reward = parse_reward_config(reward_json)?;
        let curriculum = parse_curriculum_config(curriculum_json)?;
        let error_policy = parse_error_policy(error_policy)?;
        let visibility = parse_observation_visibility(observation_visibility)?;
        let (db, config) = build_env_config(
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            reward,
            error_policy,
            visibility,
        )?;
        let debug = build_debug_config(
            Some(debug_fingerprint_every_n),
            Some(debug_event_ring_capacity),
        );
        let pool = EnvPool::new_debug(num_envs, db, config, curriculum, seed, num_threads, debug)
            .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("EnvPool init failed: {e}"))
        })?;
        Ok(Self { pool })
    }

    fn reset_into<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
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
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.reset_into(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
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
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.reset_indices_into(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
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
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.reset_done_into(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
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
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.step_into(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_debug_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutDebug>,
    ) -> PyResult<()> {
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
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
        let mut decision_kind = array_mut(py, &out.decision_kind);
        let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
        })?;
        let mut state_fingerprint = array_mut(py, &out.state_fingerprint);
        let state_fingerprint_slice = state_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("state_fingerprint not contiguous")
        })?;
        let mut events_fingerprint = array_mut(py, &out.events_fingerprint);
        let events_fingerprint_slice = events_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("events_fingerprint not contiguous")
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
                decision_id: decision_id_slice,
                engine_status: engine_status_slice,
                spec_hash: spec_hash_slice,
            },
            decision_kind: decision_kind_slice,
            state_fingerprint: state_fingerprint_slice,
            events_fingerprint: events_fingerprint_slice,
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
        let mut decision_kind = array_mut(py, &out.decision_kind);
        let decision_kind_slice = decision_kind.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("decision_kind not contiguous")
        })?;
        let mut state_fingerprint = array_mut(py, &out.state_fingerprint);
        let state_fingerprint_slice = state_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("state_fingerprint not contiguous")
        })?;
        let mut events_fingerprint = array_mut(py, &out.events_fingerprint);
        let events_fingerprint_slice = events_fingerprint.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("events_fingerprint not contiguous")
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
                decision_id: decision_id_slice,
                engine_status: engine_status_slice,
                spec_hash: spec_hash_slice,
            },
            decision_kind: decision_kind_slice,
            state_fingerprint: state_fingerprint_slice,
            events_fingerprint: events_fingerprint_slice,
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
        let codes = codes
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("codes not contiguous"))?;
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

    fn engine_error_reset_count(&self) -> u64 {
        self.pool.engine_error_reset_count()
    }

    fn reset_engine_error_reset_count(&mut self) {
        self.pool.reset_engine_error_reset_count();
    }

    fn action_lookup_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyList>> {
        let outer = PyList::empty(py);
        for env in &self.pool.envs {
            let inner = PyList::empty(py);
            for entry in env.action_lookup() {
                match entry {
                    Some(action) => inner.append(action_desc_to_pydict(py, action)?)?,
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
            let action = env
                .action_lookup()
                .get(*action_id as usize)
                .and_then(|a| a.clone());
            match action {
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
        &self,
        py: Python<'py>,
        ids: Py<PyArray1<u16>>,
        offsets: Py<PyArray1<u32>>,
    ) -> PyResult<usize> {
        let mut ids_arr = array_mut(py, &ids);
        let ids_slice = ids_arr
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("ids not contiguous"))?;
        let mut offsets_arr = array_mut(py, &offsets);
        let offsets_slice = offsets_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("offsets not contiguous")
        })?;
        py.allow_threads(|| {
            self.pool
                .legal_action_ids_batch_into(ids_slice, offsets_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn render_ansi(&self, env_index: usize, perspective: u8) -> String {
        self.pool.render_ansi(env_index, perspective)
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
}

#[pymodule]
fn weiss_sim(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("OBS_LEN", OBS_LEN)?;
    m.add("ACTION_SPACE_SIZE", ACTION_SPACE_SIZE)?;
    m.add("OBS_ENCODING_VERSION", OBS_ENCODING_VERSION)?;
    m.add("ACTION_ENCODING_VERSION", ACTION_ENCODING_VERSION)?;
    m.add("SPEC_HASH", SPEC_HASH)?;
    m.add("PASS_ACTION_ID", PASS_ACTION_ID)?;
    m.add_class::<PyEnvPool>()?;
    m.add_class::<PyBatchOutMinimal>()?;
    m.add_class::<PyBatchOutDebug>()?;
    Ok(())
}
