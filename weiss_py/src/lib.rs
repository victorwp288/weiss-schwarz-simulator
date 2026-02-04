use std::path::PathBuf;
use std::sync::Arc;

use numpy::ndarray::{Array1, Array2, Array3, ArrayViewMut, Dimension};
use numpy::{
    Element, PyArray, PyArray1, PyArray2, PyArray3, PyArrayMethods, PyReadonlyArray1,
    PyReadonlyArray2, PyUntypedArrayMethods,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyType};

use weiss_core::config::{ErrorPolicy, ObservationVisibility};
use weiss_core::encode::{
    action_spec_json as action_spec_json_core, decode_action_id as decode_action_id_core,
    observation_spec_json as observation_spec_json_core, ActionParamValue, ACTION_ENCODING_VERSION,
    ACTION_SPACE_SIZE, ACTOR_NONE, DECISION_KIND_NONE, OBS_ENCODING_VERSION, OBS_LEN,
    PASS_ACTION_ID, POLICY_VERSION, SPEC_HASH,
};
use weiss_core::legal::ActionDesc;
use weiss_core::pool::{
    BatchOutDebug, BatchOutMinimal, BatchOutMinimalI16, BatchOutMinimalI16LegalIds,
    BatchOutMinimalNoMask, BatchOutTrajectory, BatchOutTrajectoryI16,
    BatchOutTrajectoryI16LegalIds, BatchOutTrajectoryNoMask,
};
use weiss_core::replay::{ReplayConfig, ReplayVisibilityMode};
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

fn parse_replay_visibility_mode(visibility_mode: Option<String>) -> PyResult<ReplayVisibilityMode> {
    if let Some(mode) = visibility_mode {
        match mode.to_lowercase().as_str() {
            "full" => Ok(ReplayVisibilityMode::Full),
            "public" => Ok(ReplayVisibilityMode::Public),
            other => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "replay visibility must be full or public (got {other})"
            ))),
        }
    } else {
        Ok(ReplayVisibilityMode::Public)
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
    if let Err(err) = reward.validate_zero_sum() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "reward config must be zero-sum for terminal outcomes: {err}"
        )));
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

fn ensure_first_dim<'py, T, D>(
    py: Python<'py>,
    name: &str,
    arr: &'py Py<PyArray<T, D>>,
    expected: usize,
    expected_second: Option<usize>,
) -> PyResult<()>
where
    D: Dimension,
    T: Element,
{
    let shape = arr.bind(py).shape();
    let first_dim = shape.first().copied().unwrap_or(0);
    if first_dim != expected {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!(
                "{name} output buffer first dimension must match expected size (got {first_dim}, expected {expected})"
            ),
        ));
    }
    if let Some(expected_second) = expected_second {
        let second_dim = shape.get(1).copied().unwrap_or(0);
        if second_dim != expected_second {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!(
                    "{name} output buffer second dimension must match expected size (got {second_dim}, expected {expected_second})"
                ),
            ));
        }
    }
    Ok(())
}

fn ensure_len(name: &str, len: usize, expected: usize) -> PyResult<()> {
    if len != expected {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "{name} length must match num_envs (got {len}, expected {expected})"
        )));
    }
    Ok(())
}

fn ensure_third_dim<'py, T, D>(
    py: Python<'py>,
    name: &str,
    arr: &'py Py<PyArray<T, D>>,
    expected: usize,
) -> PyResult<()>
where
    D: Dimension,
    T: Element,
{
    let shape = arr.bind(py).shape();
    let third_dim = shape.get(2).copied().unwrap_or(0);
    if third_dim != expected {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!(
                "{name} output buffer third dimension must match expected size (got {third_dim}, expected {expected})"
            ),
        ));
    }
    Ok(())
}

fn validate_indices_and_seeds(
    indices: &[usize],
    episode_seeds: &[u64],
    num_envs: usize,
) -> PyResult<()> {
    if indices.len() != episode_seeds.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "indices length must match episode_seeds length (got {}, expected {})",
            indices.len(),
            episode_seeds.len()
        )));
    }
    for (pos, &idx) in indices.iter().enumerate() {
        if idx >= num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "indices[{pos}] out of bounds (got {idx}, max {})",
                num_envs.saturating_sub(1)
            )));
        }
    }
    Ok(())
}

fn ensure_first_two_dims<'py, T, D>(
    py: Python<'py>,
    name: &str,
    arr: &'py Py<PyArray<T, D>>,
    expected_first: usize,
    expected_second: usize,
) -> PyResult<()>
where
    D: Dimension,
    T: Element,
{
    ensure_first_dim(py, name, arr, expected_first, Some(expected_second))
}

fn ensure_action_space_u16() -> PyResult<()> {
    if ACTION_SPACE_SIZE > u16::MAX as usize {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!(
                "action space size {ACTION_SPACE_SIZE} exceeds u16 max 65535; u16 legal-ids outputs are not supported"
            ),
        ));
    }
    Ok(())
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

fn action_id_desc_to_pydict(
    py: Python<'_>,
    desc: &weiss_core::encode::ActionIdDesc,
) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    dict.set_item("family", desc.family)?;
    let params = PyList::empty(py);
    for param in &desc.params {
        let entry = PyDict::new(py);
        entry.set_item("name", param.name)?;
        match param.value {
            ActionParamValue::Int(value) => entry.set_item("value", value)?,
            ActionParamValue::Str(value) => entry.set_item("value", value)?,
        }
        params.append(entry)?;
    }
    dict.set_item("params", params)?;
    Ok(dict.into())
}

#[pyfunction(name = "observation_spec_json")]
fn observation_spec_json_py() -> String {
    observation_spec_json_core()
}

#[pyfunction(name = "action_spec_json")]
fn action_spec_json_py() -> String {
    action_spec_json_core()
}

#[pyfunction(name = "decode_action_id")]
fn decode_action_id_py(py: Python<'_>, action_id: u32) -> PyResult<PyObject> {
    let Some(desc) = decode_action_id_core(action_id as usize) else {
        return Ok(py.None());
    };
    action_id_desc_to_pydict(py, &desc)
}

#[pyfunction(name = "build_info")]
fn build_info_py(py: Python<'_>) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    let profile = option_env!("BUILD_PROFILE")
        .map(|value| value.to_string())
        .unwrap_or_else(|| {
            if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            }
        });
    dict.set_item("profile", profile)?;
    dict.set_item("debug_assertions", cfg!(debug_assertions))?;
    let opt_level = option_env!("BUILD_OPT_LEVEL")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    dict.set_item("opt_level", opt_level)?;
    let target = option_env!("BUILD_TARGET")
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS));
    dict.set_item("target", target)?;
    let validate_env = std::env::var("WEISS_VALIDATE_STATE").ok().as_deref() == Some("1");
    let validate_default = cfg!(debug_assertions) || validate_env;
    dict.set_item("validate_state_default", validate_default)?;
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
    decision_kind: Py<PyArray1<i8>>,
    decision_id: Py<PyArray1<u32>>,
    engine_status: Py<PyArray1<u8>>,
    spec_hash: Py<PyArray1<u64>>,
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
}

#[pyclass(name = "BatchOutMinimalI16LegalIds")]
struct PyBatchOutMinimalI16LegalIds {
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
        let legal_ids = Array1::<u16>::zeros(num_envs * ACTION_SPACE_SIZE);
        let legal_offsets = Array1::<u32>::zeros(num_envs + 1);
        let rewards = Array1::<f32>::zeros(num_envs);
        let terminated = Array1::<bool>::from_elem(num_envs, false);
        let truncated = Array1::<bool>::from_elem(num_envs, false);
        let actor = Array1::<i8>::zeros(num_envs);
        let decision_kind = Array1::<i8>::zeros(num_envs);
        let decision_id = Array1::<u32>::zeros(num_envs);
        let engine_status = Array1::<u8>::zeros(num_envs);
        let spec_hash = Array1::<u64>::from_elem(num_envs, SPEC_HASH);
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
    fn actions(&self, py: Python<'_>) -> Py<PyArray2<u32>> {
        self.actions.clone_ref(py)
    }
}

#[pyclass(name = "BatchOutTrajectoryI16LegalIds")]
struct PyBatchOutTrajectoryI16LegalIds {
    steps: usize,
    obs: Py<PyArray3<i16>>,
    legal_ids: Py<PyArray2<u16>>,
    legal_offsets: Py<PyArray2<u32>>,
    rewards: Py<PyArray2<f32>>,
    terminated: Py<PyArray2<bool>>,
    truncated: Py<PyArray2<bool>>,
    actor: Py<PyArray2<i8>>,
    decision_kind: Py<PyArray2<i8>>,
    decision_id: Py<PyArray2<u32>>,
    engine_status: Py<PyArray2<u8>>,
    spec_hash: Py<PyArray2<u64>>,
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
        let obs = Array3::<i16>::zeros((steps, num_envs, OBS_LEN));
        let legal_ids = Array2::<u16>::zeros((steps, num_envs * ACTION_SPACE_SIZE));
        let legal_offsets = Array2::<u32>::zeros((steps, num_envs + 1));
        let rewards = Array2::<f32>::zeros((steps, num_envs));
        let terminated = Array2::<bool>::from_elem((steps, num_envs), false);
        let truncated = Array2::<bool>::from_elem((steps, num_envs), false);
        let actor = Array2::<i8>::zeros((steps, num_envs));
        let decision_kind = Array2::<i8>::zeros((steps, num_envs));
        let decision_id = Array2::<u32>::zeros((steps, num_envs));
        let engine_status = Array2::<u8>::zeros((steps, num_envs));
        let spec_hash = Array2::<u64>::from_elem((steps, num_envs), SPEC_HASH);
        let actions = Array2::<u32>::zeros((steps, num_envs));
        Ok(Self {
            steps,
            obs: PyArray3::from_owned_array(py, obs).unbind(),
            legal_ids: PyArray2::from_owned_array(py, legal_ids).unbind(),
            legal_offsets: PyArray2::from_owned_array(py, legal_offsets).unbind(),
            rewards: PyArray2::from_owned_array(py, rewards).unbind(),
            terminated: PyArray2::from_owned_array(py, terminated).unbind(),
            truncated: PyArray2::from_owned_array(py, truncated).unbind(),
            actor: PyArray2::from_owned_array(py, actor).unbind(),
            decision_kind: PyArray2::from_owned_array(py, decision_kind).unbind(),
            decision_id: PyArray2::from_owned_array(py, decision_id).unbind(),
            engine_status: PyArray2::from_owned_array(py, engine_status).unbind(),
            spec_hash: PyArray2::from_owned_array(py, spec_hash).unbind(),
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
        let obs = Array3::<i32>::zeros((steps, num_envs, OBS_LEN));
        let rewards = Array2::<f32>::zeros((steps, num_envs));
        let terminated = Array2::<bool>::from_elem((steps, num_envs), false);
        let truncated = Array2::<bool>::from_elem((steps, num_envs), false);
        let actor = Array2::<i8>::zeros((steps, num_envs));
        let decision_kind = Array2::<i8>::zeros((steps, num_envs));
        let decision_id = Array2::<u32>::zeros((steps, num_envs));
        let engine_status = Array2::<u8>::zeros((steps, num_envs));
        let spec_hash = Array2::<u64>::from_elem((steps, num_envs), SPEC_HASH);
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
        error_policy=None,
        num_threads=None,
        output_masks=true,
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
        error_policy: Option<String>,
        num_threads: Option<usize>,
        output_masks: bool,
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
        let (db, config) = build_env_config(
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            reward,
            error_policy,
            ObservationVisibility::Public,
        )?;
        let debug = build_debug_config(
            Some(debug_fingerprint_every_n),
            Some(debug_event_ring_capacity),
        );
        let mut pool =
            EnvPool::new_rl_train(num_envs, db, config, curriculum, seed, num_threads, debug)
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "EnvPool init failed: {e}"
                    ))
                })?;
        pool.set_output_mask_enabled(output_masks);
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
        num_threads=None,
        output_masks=true,
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
        error_policy: Option<String>,
        num_threads: Option<usize>,
        output_masks: bool,
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
        let (db, config) = build_env_config(
            db_path,
            deck_lists,
            deck_ids,
            max_decisions,
            max_ticks,
            reward,
            error_policy,
            ObservationVisibility::Public,
        )?;
        let debug = build_debug_config(
            Some(debug_fingerprint_every_n),
            Some(debug_event_ring_capacity),
        );
        let mut pool =
            EnvPool::new_rl_eval(num_envs, db, config, curriculum, seed, num_threads, debug)
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "EnvPool init failed: {e}"
                    ))
                })?;
        pool.set_output_mask_enabled(output_masks);
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
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_into(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_into_i16(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.reset_into_i16_legal_ids(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_into_nomask(&mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_indices_into(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_indices_into_i16(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| {
            self.pool
                .reset_indices_into_i16_legal_ids(&indices, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_indices_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_indices_into_nomask(&indices, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_done_into(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_done_into_i16(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.reset_done_into_i16_legal_ids(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn reset_done_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        done_mask: PyReadonlyArray1<bool>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let done = done_mask.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("done_mask not contiguous")
        })?;
        ensure_len("done_mask", done.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.reset_done_into_nomask(done, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_into(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_into_i16(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| self.pool.step_into_i16_legal_ids(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        actions: PyReadonlyArray1<u32>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let actions = actions.as_slice().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_into_nomask(actions, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| self.pool.step_first_legal_into(actions_slice, &mut out_min))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_first_legal_into_i16(actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| {
            self.pool
                .step_first_legal_into_i16_legal_ids(actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_first_legal_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_first_legal_into_nomask(actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_sample_legal_action_ids_uniform_into(seeds, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool.step_sample_legal_action_ids_uniform_into_i16(
                seeds,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| {
            self.pool
                .step_sample_legal_action_ids_uniform_into_i16_legal_ids(
                    seeds,
                    actions_slice,
                    &mut out_min,
                )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_legal_action_ids_uniform_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        ensure_len("seeds", seeds.len(), num_envs)?;
        let mut actions_arr = array_mut(py, &actions);
        let actions_slice = actions_arr.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        ensure_len("actions", actions_slice.len(), num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool.step_sample_legal_action_ids_uniform_into_nomask(
                seeds,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectory>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectory.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectory {
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
            actions: actions_slice,
        };
        py.allow_threads(|| self.pool.rollout_first_legal_into(steps, &mut out_traj))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectoryI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16 {
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
            actions: actions_slice,
        };
        py.allow_threads(|| self.pool.rollout_first_legal_into_i16(steps, &mut out_traj))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectoryI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_legal_ids_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16LegalIds.steps",
            ));
        }
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_first_legal_into_i16_legal_ids(steps, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_first_legal_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        out: PyRef<'py, PyBatchOutTrajectoryNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_nomask_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryNoMask.steps",
            ));
        }
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryNoMask {
            obs: obs_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_first_legal_into_nomask(steps, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectory>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectory.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectory {
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectoryI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16 {
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
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_i16(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectoryI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_i16_legal_ids_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryI16LegalIds.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_i16_legal_ids(
                    steps,
                    seeds,
                    &mut out_traj,
                )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn rollout_sample_legal_action_ids_uniform_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        steps: usize,
        seeds: PyReadonlyArray1<u64>,
        out: PyRef<'py, PyBatchOutTrajectoryNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_trajectory_nomask_dims(py, &out, steps, num_envs)?;
        if steps != out.steps {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "steps must match BatchOutTrajectoryNoMask.steps",
            ));
        }
        let seeds = seeds
            .as_slice()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("seeds not contiguous"))?;
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
        let mut actions = array_mut(py, &out.actions);
        let actions_slice = actions.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("actions not contiguous")
        })?;
        let mut out_traj = BatchOutTrajectoryNoMask {
            obs: obs_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
            actions: actions_slice,
        };
        py.allow_threads(|| {
            self.pool
                .rollout_sample_legal_action_ids_uniform_into_nomask(steps, seeds, &mut out_traj)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

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
                decision_kind: decision_kind_slice,
                decision_id: decision_id_slice,
                engine_status: engine_status_slice,
                spec_hash: spec_hash_slice,
            },
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
                decision_kind: decision_kind_slice,
                decision_id: decision_id_slice,
                engine_status: engine_status_slice,
                spec_hash: spec_hash_slice,
            },
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
        };
        py.allow_threads(|| {
            self.pool
                .auto_reset_on_error_codes_into_nomask(codes, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn engine_error_reset_count(&self) -> u64 {
        self.pool.engine_error_reset_count()
    }

    fn reset_engine_error_reset_count(&mut self) {
        self.pool.reset_engine_error_reset_count();
    }

    fn set_error_policy(&mut self, error_policy: String) -> PyResult<()> {
        let policy = parse_error_policy(Some(error_policy))?;
        self.pool.set_error_policy(policy);
        Ok(())
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
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
        py.allow_threads(|| {
            self.pool
                .select_actions_from_logits_into(logits, actions_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
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
        py.allow_threads(|| {
            self.pool
                .sample_actions_from_logits_into(logits, seeds, actions_slice)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
    }

    fn step_select_from_logits_into<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_select_from_logits_into(logits, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_select_from_logits_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_select_from_logits_into_i16(logits, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_select_from_logits_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_select_from_logits_into_nomask(logits, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_select_from_logits_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
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
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| {
            self.pool.step_select_from_logits_into_i16_legal_ids(
                logits,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_from_logits_into<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimal>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_sample_from_logits_into(logits, seeds, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_from_logits_into_i16<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool
                .step_sample_from_logits_into_i16(logits, seeds, actions_slice, &mut out_min)
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_from_logits_into_nomask<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalNoMask>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_nomask_dims(py, &out, num_envs)?;
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
        };
        py.allow_threads(|| {
            self.pool.step_sample_from_logits_into_nomask(
                logits,
                seeds,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    }

    fn step_sample_from_logits_into_i16_legal_ids<'py>(
        &mut self,
        py: Python<'py>,
        logits: PyReadonlyArray2<f32>,
        seeds: PyReadonlyArray1<u64>,
        actions: Py<PyArray1<u32>>,
        out: PyRef<'py, PyBatchOutMinimalI16LegalIds>,
    ) -> PyResult<()> {
        let num_envs = self.pool.envs.len();
        ensure_batch_out_minimal_i16_legal_ids_dims(py, &out, num_envs)?;
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
        let mut obs = array_mut(py, &out.obs);
        let obs_slice = obs
            .as_slice_mut()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("obs not contiguous"))?;
        let mut legal_ids = array_mut(py, &out.legal_ids);
        let legal_ids_slice = legal_ids.as_slice_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("legal_ids not contiguous")
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
        let mut out_min = BatchOutMinimalI16LegalIds {
            obs: obs_slice,
            legal_ids: legal_ids_slice,
            legal_offsets: legal_offsets_slice,
            rewards: rewards_slice,
            terminated: terminated_slice,
            truncated: truncated_slice,
            actor: actor_slice,
            decision_kind: decision_kind_slice,
            decision_id: decision_id_slice,
            engine_status: engine_status_slice,
            spec_hash: spec_hash_slice,
        };
        py.allow_threads(|| {
            self.pool.step_sample_from_logits_into_i16_legal_ids(
                logits,
                seeds,
                actions_slice,
                &mut out_min,
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
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
    m.add("POLICY_VERSION", POLICY_VERSION)?;
    m.add("SPEC_HASH", SPEC_HASH)?;
    m.add("PASS_ACTION_ID", PASS_ACTION_ID)?;
    m.add("ACTOR_NONE", ACTOR_NONE)?;
    m.add("DECISION_KIND_NONE", DECISION_KIND_NONE)?;
    m.add_class::<PyEnvPool>()?;
    m.add_class::<PyBatchOutMinimal>()?;
    m.add_class::<PyBatchOutMinimalI16>()?;
    m.add_class::<PyBatchOutMinimalI16LegalIds>()?;
    m.add_class::<PyBatchOutMinimalNoMask>()?;
    m.add_class::<PyBatchOutTrajectory>()?;
    m.add_class::<PyBatchOutTrajectoryI16>()?;
    m.add_class::<PyBatchOutTrajectoryI16LegalIds>()?;
    m.add_class::<PyBatchOutTrajectoryNoMask>()?;
    m.add_class::<PyBatchOutDebug>()?;
    m.add_function(wrap_pyfunction!(observation_spec_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(action_spec_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(decode_action_id_py, m)?)?;
    m.add_function(wrap_pyfunction!(build_info_py, m)?)?;
    Ok(())
}
