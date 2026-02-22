#[cfg(panic = "abort")]
compile_error!(
    "weiss_py requires panic=unwind so per-env panic containment in EnvPool can trap unwinds"
);

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Error as AnyhowError;
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
use weiss_core::{
    CardDb, ConfigError, CurriculumConfig, DebugConfig, EndConditionPolicy, EnvConfig, EnvError,
    EnvPool, RewardConfig, StateError,
};

const DEFAULT_WSDB_BYTES: &[u8] = include_bytes!("../default_cards.wsdb");

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
            "raise" => Ok(ErrorPolicy::Strict),
            "replace" => Ok(ErrorPolicy::LenientNoop),
            "terminate" => Ok(ErrorPolicy::LenientTerminate),
            other => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "error_policy must be one of raise, replace, terminate (got {other})"
            ))),
        }
    } else {
        Ok(ErrorPolicy::LenientNoop)
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

fn parse_end_condition_policy(
    end_condition_policy_json: Option<String>,
) -> PyResult<EndConditionPolicy> {
    if let Some(json) = end_condition_policy_json {
        serde_json::from_str::<EndConditionPolicy>(&json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "end_condition_policy_json parse error: {e}"
            ))
        })
    } else {
        Ok(EndConditionPolicy::default())
    }
}

fn config_error_to_issue_dict<'py>(
    py: Python<'py>,
    err: ConfigError,
) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    match err {
        ConfigError::DeckLength {
            player,
            got,
            expected,
        } => {
            dict.set_item("kind", "deck_length")?;
            dict.set_item("player", player)?;
            dict.set_item("got", got)?;
            dict.set_item("expected", expected)?;
        }
        ConfigError::UnknownCardId { player, card_id } => {
            dict.set_item("kind", "unknown_card_id")?;
            dict.set_item("player", player)?;
            dict.set_item("card_id", card_id)?;
        }
        ConfigError::ClimaxCount { player, got, max } => {
            dict.set_item("kind", "climax_count")?;
            dict.set_item("player", player)?;
            dict.set_item("got", got)?;
            dict.set_item("max", max)?;
        }
        ConfigError::CardCopyCount {
            player,
            card_id,
            got,
            max,
        } => {
            dict.set_item("kind", "card_copy_count")?;
            dict.set_item("player", player)?;
            dict.set_item("card_id", card_id)?;
            dict.set_item("got", got)?;
            dict.set_item("max", max)?;
        }
    }
    Ok(dict.unbind())
}

fn map_pool_init_error(err: AnyhowError) -> PyErr {
    if let Some(env_err) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<EnvError>())
    {
        return PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "EnvPool init failed: {env_err}"
        ));
    }
    if let Some(cfg_err) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<ConfigError>())
    {
        return PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "EnvPool init failed: {cfg_err}"
        ));
    }
    if let Some(state_err) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<StateError>())
    {
        return PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "EnvPool init failed: {state_err}"
        ));
    }
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("EnvPool init failed: {err}"))
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

fn resolve_num_threads(num_envs: usize, requested_num_threads: Option<usize>) -> Option<usize> {
    requested_num_threads
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .map(|threads| threads.min(num_envs.max(1)))
}

fn load_card_db(db_path: Option<String>) -> PyResult<CardDb> {
    let db_path = db_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty());
    match db_path {
        Some(path) => CardDb::load(path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Card DB load failed for '{path}': {e}"
            ))
        }),
        None => CardDb::from_wsdb_bytes(DEFAULT_WSDB_BYTES).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Bundled Card DB load failed: {e}"
            ))
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_env_config(
    db_path: Option<String>,
    deck_lists: Vec<Vec<u32>>,
    deck_ids: Option<Vec<u32>>,
    max_decisions: u32,
    max_ticks: u32,
    reward: RewardConfig,
    error_policy: ErrorPolicy,
    observation_visibility: ObservationVisibility,
    end_condition_policy: EndConditionPolicy,
) -> PyResult<(Arc<CardDb>, EnvConfig)> {
    let db = load_card_db(db_path)?;
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
        end_condition_policy,
    };
    Ok((Arc::new(db), config))
}

fn array_mut<'py, T, D>(py: Python<'py>, arr: &'py Py<PyArray<T, D>>) -> ArrayViewMut<'py, T, D>
where
    D: Dimension,
    T: Element,
{
    // All callers validate shape and contiguity first; this helper centralizes the only mutable
    // ndarray view cast used by the Python boundary write-into paths.
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
    // Keep dimension checks explicit here so Python-side buffer contract errors stay actionable.
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
