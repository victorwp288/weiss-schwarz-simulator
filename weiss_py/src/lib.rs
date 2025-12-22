use std::sync::Arc;

use numpy::ndarray::Array2;
use numpy::{PyArray1, PyArray2};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::Bound;

use weiss_core::config::{ErrorPolicy, ObservationVisibility};
use weiss_core::encode::{
    ACTION_ENCODING_VERSION, ACTION_SPACE_SIZE, OBS_ENCODING_VERSION, OBS_LEN,
};
use weiss_core::legal::ActionDesc;
use weiss_core::replay::ReplayConfig;
use weiss_core::{CardDb, CurriculumConfig, EnvConfig, EnvPool, RewardConfig};

type StepBatchResultPy = (
    Py<PyArray2<i32>>,
    Py<PyArray1<f32>>,
    Py<PyArray1<bool>>,
    Py<PyArray1<bool>>,
    Py<PyList>,
);

type StepBatchFastResultPy = (
    Py<PyArray2<i32>>,
    Py<PyArray1<f32>>,
    Py<PyArray1<bool>>,
    Py<PyArray1<bool>>,
    Py<PyArray1<i8>>,
    Py<PyArray1<i8>>,
    Py<PyArray1<i8>>,
    Py<PyArray1<bool>>,
    Py<PyArray1<bool>>,
);

#[pyclass(name = "EnvPool")]
pub struct PyEnvPool {
    pool: EnvPool,
}

#[pymethods]
impl PyEnvPool {
    /// Create a new environment pool.
    #[new]
    #[pyo3(signature = (num_envs, db_path, deck_lists, deck_ids=None, max_decisions=10_000, max_ticks=100_000, seed=0, curriculum_json=None, reward_json=None, error_policy=None, observation_visibility=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
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
    ) -> PyResult<Self> {
        if num_envs == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "num_envs must be > 0",
            ));
        }
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
        let reward = if let Some(json) = reward_json {
            serde_json::from_str::<RewardConfig>(&json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "reward_json parse error: {e}"
                ))
            })?
        } else {
            RewardConfig::default()
        };
        let curriculum = if let Some(json) = curriculum_json {
            serde_json::from_str::<CurriculumConfig>(&json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "curriculum_json parse error: {e}"
                ))
            })?
        } else {
            CurriculumConfig::default()
        };
        let error_policy = if let Some(policy) = error_policy {
            match policy.to_lowercase().as_str() {
                "strict" => ErrorPolicy::Strict,
                "lenient_terminate" | "lenient" => ErrorPolicy::LenientTerminate,
                "lenient_noop" => ErrorPolicy::LenientNoop,
                other => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "error_policy must be one of strict, lenient_terminate, lenient_noop (got {other})"
                    )));
                }
            }
        } else {
            ErrorPolicy::LenientTerminate
        };
        let observation_visibility = if let Some(mode) = observation_visibility {
            match mode.to_lowercase().as_str() {
                "public" => ObservationVisibility::Public,
                "full" => ObservationVisibility::Full,
                other => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "observation_visibility must be public or full (got {other})"
                    )));
                }
            }
        } else {
            ObservationVisibility::Public
        };
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
        let pool = EnvPool::new(num_envs, Arc::new(db), config, curriculum, seed);
        Ok(Self { pool })
    }

    fn reset_all<'py>(&mut self, py: Python<'py>) -> PyResult<Py<PyArray2<i32>>> {
        let result = self.pool.reset_all();
        let num_envs = self.pool.envs.len();
        let obs = Array2::from_shape_vec((num_envs, OBS_LEN), result.obs).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to shape obs array")
        })?;
        Ok(PyArray2::from_owned_array_bound(py, obs).unbind())
    }

    fn reset_indices<'py>(
        &mut self,
        py: Python<'py>,
        indices: Vec<usize>,
    ) -> PyResult<Py<PyArray2<i32>>> {
        let result = self.pool.reset_indices(&indices);
        let num_envs = self.pool.envs.len();
        let obs = Array2::from_shape_vec((num_envs, OBS_LEN), result.obs).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to shape obs array")
        })?;
        Ok(PyArray2::from_owned_array_bound(py, obs).unbind())
    }

    /// Step all envs once. Info dict includes `actor`, the observation/reward perspective for this transition.
    fn step_batch<'py>(
        &mut self,
        py: Python<'py>,
        actions: Vec<u32>,
    ) -> PyResult<StepBatchResultPy> {
        let result = py
            .allow_threads(|| self.pool.step_batch(&actions))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("step_batch failed: {e}"))
            })?;

        let num_envs = self.pool.envs.len();
        let obs = Array2::from_shape_vec((num_envs, OBS_LEN), result.obs).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to shape obs array")
        })?;
        let obs = PyArray2::from_owned_array_bound(py, obs).unbind();
        let rewards = PyArray1::from_vec_bound(py, result.rewards).unbind();
        let terminated = PyArray1::from_iter_bound(py, result.terminated).unbind();
        let truncated = PyArray1::from_iter_bound(py, result.truncated).unbind();

        let infos = PyList::empty_bound(py);
        for info in result.infos {
            let dict = PyDict::new_bound(py);
            dict.set_item("obs_version", info.obs_version)?;
            dict.set_item("action_version", info.action_version)?;
            dict.set_item("decision_kind", info.decision_kind)?;
            dict.set_item("current_player", info.current_player)?;
            dict.set_item("actor", info.actor)?;
            dict.set_item("decision_count", info.decision_count)?;
            dict.set_item("tick_count", info.tick_count)?;
            dict.set_item("terminal", format!("{:?}", info.terminal))?;
            dict.set_item("illegal_action", info.illegal_action)?;
            dict.set_item("engine_error", info.engine_error)?;
            infos.append(dict)?;
        }

        Ok((obs, rewards, terminated, truncated, infos.unbind()))
    }

    /// Fast step: returns arrays only. `actor` is the observation/reward perspective per env.
    fn step_batch_fast<'py>(
        &mut self,
        py: Python<'py>,
        actions: Vec<u32>,
    ) -> PyResult<StepBatchFastResultPy> {
        let result = py
            .allow_threads(|| self.pool.step_batch(&actions))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("step_batch failed: {e}"))
            })?;

        let num_envs = self.pool.envs.len();
        let obs = Array2::from_shape_vec((num_envs, OBS_LEN), result.obs).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to shape obs array")
        })?;
        let obs = PyArray2::from_owned_array_bound(py, obs).unbind();
        let rewards = PyArray1::from_vec_bound(py, result.rewards).unbind();
        let terminated = PyArray1::from_iter_bound(py, result.terminated).unbind();
        let truncated = PyArray1::from_iter_bound(py, result.truncated).unbind();

        let mut current_player = Vec::with_capacity(result.infos.len());
        let mut decision_kind = Vec::with_capacity(result.infos.len());
        let mut actor = Vec::with_capacity(result.infos.len());
        let mut illegal_action = Vec::with_capacity(result.infos.len());
        let mut engine_error = Vec::with_capacity(result.infos.len());
        for info in result.infos {
            current_player.push(info.current_player);
            decision_kind.push(info.decision_kind);
            actor.push(info.actor);
            illegal_action.push(info.illegal_action);
            engine_error.push(info.engine_error);
        }
        let current_player = PyArray1::from_vec_bound(py, current_player).unbind();
        let decision_kind = PyArray1::from_vec_bound(py, decision_kind).unbind();
        let actor = PyArray1::from_vec_bound(py, actor).unbind();
        let illegal_action = PyArray1::from_iter_bound(py, illegal_action).unbind();
        let engine_error = PyArray1::from_iter_bound(py, engine_error).unbind();

        Ok((
            obs,
            rewards,
            terminated,
            truncated,
            current_player,
            decision_kind,
            actor,
            illegal_action,
            engine_error,
        ))
    }

    fn action_masks_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray2<u8>>> {
        let masks = self.pool.action_masks_batch();
        let num_envs = self.pool.envs.len();
        let mask_array =
            Array2::from_shape_vec((num_envs, ACTION_SPACE_SIZE), masks).map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to shape mask array")
            })?;
        Ok(PyArray2::from_owned_array_bound(py, mask_array).unbind())
    }

    fn legal_actions_batch(&self) -> PyResult<Vec<Vec<PyObject>>> {
        let actions = self.pool.legal_actions_batch();
        Python::with_gil(|py| {
            let mut out = Vec::with_capacity(actions.len());
            for list in actions {
                let mut py_list = Vec::with_capacity(list.len());
                for action in list {
                    let dict = PyDict::new_bound(py);
                    match action {
                        ActionDesc::MulliganKeep => {
                            dict.set_item("kind", "mulligan_keep")?;
                        }
                        ActionDesc::MulliganAll => {
                            dict.set_item("kind", "mulligan_all")?;
                        }
                        ActionDesc::ClockPass => {
                            dict.set_item("kind", "clock_pass")?;
                        }
                        ActionDesc::Clock { hand_index } => {
                            dict.set_item("kind", "clock")?;
                            dict.set_item("hand_index", hand_index)?;
                        }
                        ActionDesc::MainPass => {
                            dict.set_item("kind", "main_pass")?;
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
                        ActionDesc::ClimaxPass => {
                            dict.set_item("kind", "climax_pass")?;
                        }
                        ActionDesc::ClimaxPlay { hand_index } => {
                            dict.set_item("kind", "climax_play")?;
                            dict.set_item("hand_index", hand_index)?;
                        }
                        ActionDesc::AttackPass => {
                            dict.set_item("kind", "attack_pass")?;
                        }
                        ActionDesc::Attack { slot, attack_type } => {
                            dict.set_item("kind", "attack")?;
                            dict.set_item("slot", slot)?;
                            dict.set_item("attack_type", format!("{:?}", attack_type))?;
                        }
                        ActionDesc::CounterPass => {
                            dict.set_item("kind", "counter_pass")?;
                        }
                        ActionDesc::CounterPlay { hand_index } => {
                            dict.set_item("kind", "counter_play")?;
                            dict.set_item("hand_index", hand_index)?;
                        }
                        ActionDesc::LevelUp { index } => {
                            dict.set_item("kind", "level_up")?;
                            dict.set_item("index", index)?;
                        }
                        ActionDesc::EncoreYes => {
                            dict.set_item("kind", "encore_yes")?;
                        }
                        ActionDesc::EncoreNo => {
                            dict.set_item("kind", "encore_no")?;
                        }
                        ActionDesc::TriggerOrder { index } => {
                            dict.set_item("kind", "trigger_order")?;
                            dict.set_item("index", index)?;
                        }
                        ActionDesc::ChoiceSelect { index } => {
                            dict.set_item("kind", "choice_select")?;
                            dict.set_item("index", index)?;
                        }
                    }
                    py_list.push(dict.into_py(py));
                }
                out.push(py_list);
            }
            Ok(out)
        })
    }

    fn get_current_player_batch<'py>(&self, py: Python<'py>) -> PyResult<Py<PyArray1<i8>>> {
        let players = self.pool.get_current_player_batch();
        Ok(PyArray1::from_vec_bound(py, players).unbind())
    }

    fn render_ansi(&self, env_index: usize, perspective: u8) -> PyResult<String> {
        Ok(self.pool.render_ansi(env_index, perspective))
    }

    fn set_curriculum(&mut self, curriculum_json: String) -> PyResult<()> {
        let curriculum: CurriculumConfig = serde_json::from_str(&curriculum_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "curriculum_json parse error: {e}"
            ))
        })?;
        self.pool.set_curriculum(curriculum);
        Ok(())
    }

    #[pyo3(signature = (enabled, sample_rate, out_dir, compress=false, include_trigger_card_id=false))]
    fn enable_replay_sampling(
        &mut self,
        enabled: bool,
        sample_rate: f32,
        out_dir: String,
        compress: bool,
        include_trigger_card_id: bool,
    ) -> PyResult<()> {
        let config = ReplayConfig {
            enabled,
            sample_rate,
            out_dir: out_dir.into(),
            compress,
            include_trigger_card_id,
        };
        self.pool.enable_replay_sampling(config).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "enable_replay_sampling failed: {e}"
            ))
        })?;
        Ok(())
    }

    #[getter]
    fn action_space(&self) -> usize {
        self.pool.action_space
    }

    #[getter]
    fn obs_len(&self) -> usize {
        OBS_LEN
    }
}

#[pymodule]
fn weiss_sim(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEnvPool>()?;
    m.add("__version__", "0.1.0")?;
    m.add("OBS_ENCODING_VERSION", OBS_ENCODING_VERSION)?;
    m.add("ACTION_ENCODING_VERSION", ACTION_ENCODING_VERSION)?;
    Ok(())
}
