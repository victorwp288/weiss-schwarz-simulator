mod engine_support;

use std::fs;
use std::thread::sleep;
use std::time::Duration;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::fingerprint::{events_fingerprint, state_fingerprint};
use weiss_core::legal::ActionDesc;
use weiss_core::replay::{read_replay_file, ReplayConfig, ReplayWriter};

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
    let mut env = GameEnv::new(
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
    sleep(Duration::from_millis(50));

    let mut files = Vec::new();
    for entry in fs::read_dir(replay_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|s| s == "wsr").unwrap_or(false) {
            files.push(path);
        }
    }
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
        ..Default::default()
    };
    let mut env_a = GameEnv::new(
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

    let actions = env_a.replay_actions.clone();
    let expected_state = state_fingerprint(&env_a.state);
    let expected_events = events_fingerprint(env_a.canonical_events());

    let mut env_b = GameEnv::new(db, config, default_curriculum(), 99, replay_config, None, 0);
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
