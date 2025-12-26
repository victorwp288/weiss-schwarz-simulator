use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use weiss_core::config::{CurriculumConfig, ObservationVisibility};
use weiss_core::db::WSDB_SCHEMA_VERSION;
use weiss_core::encode::{
    ACTION_ENCODING_VERSION, ACTION_SPACE_SIZE, CHOICE_COUNT, OBS_ENCODING_VERSION,
};
use weiss_core::fingerprint::FINGERPRINT_ALGO;
use weiss_core::replay::REPLAY_SCHEMA_VERSION;

fn invariants_doc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("invariants.md")
}

fn parse_invariants_doc() -> BTreeMap<String, String> {
    let content = fs::read_to_string(invariants_doc_path())
        .expect("docs/invariants.md must be readable for tests");
    let mut map = BTreeMap::new();
    for raw in content.lines() {
        let mut line = raw.trim();
        if line.starts_with('-') {
            line = line.trim_start_matches('-').trim();
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            continue;
        }
        if key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

fn visibility_to_str(value: ObservationVisibility) -> &'static str {
    match value {
        ObservationVisibility::Public => "public",
        ObservationVisibility::Full => "full",
    }
}

#[test]
fn invariants_doc_matches_code() {
    let inv = parse_invariants_doc();
    let curriculum = CurriculumConfig::default();

    assert_eq!(
        inv.get("action_space_size"),
        Some(&ACTION_SPACE_SIZE.to_string())
    );
    assert_eq!(inv.get("choice_page_size"), Some(&CHOICE_COUNT.to_string()));
    assert_eq!(
        inv.get("action_encoding_version"),
        Some(&ACTION_ENCODING_VERSION.to_string())
    );
    assert_eq!(
        inv.get("obs_encoding_version"),
        Some(&OBS_ENCODING_VERSION.to_string())
    );
    assert_eq!(
        inv.get("replay_schema_version"),
        Some(&REPLAY_SCHEMA_VERSION.to_string())
    );
    assert_eq!(
        inv.get("wsdb_schema_version"),
        Some(&WSDB_SCHEMA_VERSION.to_string())
    );
    assert_eq!(
        inv.get("fingerprint_algo"),
        Some(&FINGERPRINT_ALGO.to_string())
    );
    assert_eq!(
        inv.get("observation_visibility_default"),
        Some(&visibility_to_str(ObservationVisibility::default()).to_string())
    );
    assert_eq!(
        inv.get("visibility_policies_default"),
        Some(&curriculum.enable_visibility_policies.to_string())
    );
    assert_eq!(
        inv.get("priority_windows_default"),
        Some(&curriculum.enable_priority_windows.to_string())
    );
    assert_eq!(
        inv.get("refresh_penalty_default"),
        Some(&curriculum.enable_refresh_penalty.to_string())
    );
    assert_eq!(
        inv.get("replay_sanitization_requires_visibility_policies"),
        Some(&"true".to_string())
    );
    assert_eq!(
        inv.get("replay_sanitization_requires_public_visibility"),
        Some(&"true".to_string())
    );
}
