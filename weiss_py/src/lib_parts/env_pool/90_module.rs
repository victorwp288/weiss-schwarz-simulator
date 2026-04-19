#[pymodule]
fn weiss_sim(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("OBS_LEN", OBS_LEN)?;
    m.add("ACTION_SPACE_SIZE", ACTION_SPACE_SIZE)?;
    m.add("ACTION_META_WIDTH", ACTION_META_WIDTH)?;
    m.add("ACTION_META_UNUSED", ACTION_META_UNUSED)?;
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
    m.add_function(wrap_pyfunction!(decode_factorized_action_id_py, m)?)?;
    m.add_function(wrap_pyfunction!(encode_factorized_action_py, m)?)?;
    m.add_function(wrap_pyfunction!(build_info_py, m)?)?;
    m.add_function(wrap_pyfunction!(export_card_table_json_py, m)?)?;
    Ok(())
}
