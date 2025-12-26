use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use weiss_core::config::{CurriculumConfig, EnvConfig};
use weiss_core::fingerprint::config_fingerprint;

pub fn maybe_dump_failure_bundle(
    label: &str,
    seed: u64,
    config: &EnvConfig,
    curriculum: &CurriculumConfig,
    action_ids: &[u32],
    state_hash: u64,
    events_hash: u64,
) {
    if std::env::var("WEISS_DUMP_FAILURE_BUNDLE").is_err() {
        return;
    }
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.push(format!("ws_failure_{label}_{seed}_{stamp}"));
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path: PathBuf = dir.join("bundle.txt");
    let mut file = match File::create(&path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let config_hash = config_fingerprint(config, curriculum);
    let _ = writeln!(file, "label: {label}");
    let _ = writeln!(file, "seed: {seed}");
    let _ = writeln!(file, "config_hash: {config_hash}");
    let _ = writeln!(file, "state_hash: {state_hash}");
    let _ = writeln!(file, "events_hash: {events_hash}");
    let _ = writeln!(file, "action_ids: {}", format_action_ids(action_ids));
}

fn format_action_ids(action_ids: &[u32]) -> String {
    if action_ids.is_empty() {
        return String::from("[]");
    }
    let mut out = String::from("[");
    for (i, id) in action_ids.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&id.to_string());
    }
    out.push(']');
    out
}
