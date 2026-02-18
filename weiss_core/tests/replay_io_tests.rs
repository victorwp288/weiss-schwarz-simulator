mod engine_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::fingerprint::{events_fingerprint, state_fingerprint};
use weiss_core::legal::ActionDesc;
use weiss_core::replay::{read_replay_file, ReplayConfig, ReplayVisibilityMode, ReplayWriter};

fn replay_files_in(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|s| s == "wsr").unwrap_or(false) {
            files.push(path);
        }
    }
    files
}

fn wait_for_replay_files(dir: &Path, timeout: Duration) -> Vec<PathBuf> {
    let start = Instant::now();
    loop {
        let files = replay_files_in(dir);
        if !files.is_empty() || start.elapsed() >= timeout {
            return files;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn replay_roundtrip_headers() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("roundtrip");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        include_trigger_card_id: true,
        ..Default::default()
    };
    let writer = ReplayWriter::new(&replay_config).unwrap();
    let mut env = GameEnv::new_or_panic(
        db,
        config,
        default_curriculum(),
        21,
        replay_config.clone(),
        Some(writer),
        0,
    );
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.finish_episode_replay();
    let files = wait_for_replay_files(&replay_dir, Duration::from_secs(2));
    assert!(!files.is_empty());
    let data = read_replay_file(&files[0]).unwrap();
    assert_eq!(
        data.header.obs_version,
        weiss_core::encode::OBS_ENCODING_VERSION
    );
    assert_eq!(
        data.header.action_version,
        weiss_core::encode::ACTION_ENCODING_VERSION
    );
    assert_eq!(data.body.action_ids.len(), data.body.actions.len());
}

#[test]
fn replay_actions_reproduce_state_and_events() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        visibility_mode: ReplayVisibilityMode::Full,
        ..Default::default()
    };
    let mut env_a = GameEnv::new_or_panic(
        db.clone(),
        config.clone(),
        default_curriculum(),
        99,
        replay_config.clone(),
        None,
        0,
    );
    for _ in 0..40 {
        if env_a.state.terminal.is_some() {
            break;
        }
        let action = env_a
            .legal_actions()
            .first()
            .cloned()
            .expect("legal action");
        env_a.apply_action(action).unwrap();
    }

    let actions = env_a.replay_actions_raw.clone();
    let expected_state = state_fingerprint(&env_a.state);
    let expected_events = events_fingerprint(env_a.canonical_events());

    let mut env_b =
        GameEnv::new_or_panic(db, config, default_curriculum(), 99, replay_config, None, 0);
    for action in actions {
        if env_b.state.terminal.is_some() {
            break;
        }
        env_b.apply_action(action).unwrap();
    }

    assert_eq!(state_fingerprint(&env_b.state), expected_state);
    assert_eq!(
        events_fingerprint(env_b.canonical_events()),
        expected_events
    );
}

#[test]
fn replay_roundtrip_full_visibility_reproduces_state() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("roundtrip_full");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        include_trigger_card_id: true,
        visibility_mode: ReplayVisibilityMode::Full,
        ..Default::default()
    };
    let writer = ReplayWriter::new(&replay_config).unwrap();
    let mut env = GameEnv::new_or_panic(
        db.clone(),
        config.clone(),
        default_curriculum(),
        77,
        replay_config.clone(),
        Some(writer),
        0,
    );
    for _ in 0..60 {
        if env.state.terminal.is_some() {
            break;
        }
        let action = env
            .legal_actions()
            .first()
            .cloned()
            .unwrap_or(ActionDesc::Pass);
        env.apply_action(action).unwrap();
    }
    let expected_state = state_fingerprint(&env.state);
    let expected_actions = env.replay_actions_raw.clone();
    env.finish_episode_replay();
    let files = wait_for_replay_files(&replay_dir, Duration::from_secs(2));
    assert!(!files.is_empty());
    let data = read_replay_file(&files[0]).unwrap();
    assert_eq!(data.body.actions, expected_actions);
    assert_eq!(data.body.action_ids.len(), data.body.actions.len());
    let recorded_state = data
        .body
        .final_state
        .as_ref()
        .expect("final_state")
        .state_hash;
    assert_eq!(recorded_state, expected_state);
    let mut replay_env = GameEnv::new_or_panic(
        db,
        config,
        default_curriculum(),
        0,
        ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            visibility_mode: ReplayVisibilityMode::Full,
            ..Default::default()
        },
        None,
        0,
    );
    replay_env.reset_with_episode_seed(data.header.episode_seed);
    for action in data.body.actions.iter() {
        if replay_env.state.terminal.is_some() {
            break;
        }
        replay_env.apply_action(action.clone()).unwrap();
    }
    assert_eq!(state_fingerprint(&replay_env.state), expected_state);
    if let Some(events) = data.body.events.as_ref() {
        assert_eq!(
            events_fingerprint(events),
            events_fingerprint(replay_env.canonical_events())
        );
    }
}

#[test]
fn replay_store_actions_false_records_events_only() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("roundtrip_events_only");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        visibility_mode: ReplayVisibilityMode::Full,
        store_actions: false,
        ..Default::default()
    };
    let writer = ReplayWriter::new(&replay_config).unwrap();
    let mut env = GameEnv::new_or_panic(
        db,
        config,
        default_curriculum(),
        33,
        replay_config,
        Some(writer),
        0,
    );
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.finish_episode_replay();

    let files = wait_for_replay_files(&replay_dir, Duration::from_secs(2));
    assert!(!files.is_empty());
    let read_start = Instant::now();
    let data = loop {
        match read_replay_file(&files[0]) {
            Ok(data) => break data,
            Err(_) if read_start.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("failed to read replay file: {err}"),
        }
    };
    assert!(data.body.actions.is_empty());
    assert!(data.body.action_ids.is_empty());
    assert!(data
        .body
        .events
        .as_ref()
        .is_some_and(|events| !events.is_empty()));
}
