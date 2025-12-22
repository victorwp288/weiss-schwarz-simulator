mod engine_support;

use std::fs;
use std::thread::sleep;
use std::time::Duration;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::{read_replay_file, ReplayConfig, ReplayWriter};

#[test]
fn replay_roundtrip_headers() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![1; 20];
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("roundtrip");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        include_trigger_card_id: true,
    };
    let writer = ReplayWriter::new(&replay_config).unwrap();
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        21,
        replay_config.clone(),
        Some(writer),
    );
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    env.apply_action(ActionDesc::AttackPass).unwrap();
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
