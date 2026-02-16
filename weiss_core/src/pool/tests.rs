use super::*;
use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardColor, CardDb, CardStatic, CardType};
use crate::encode::{ACTION_SPACE_SIZE, SPEC_HASH};
use crate::env::{DebugConfig, EngineErrorCode, FaultSource, GameEnv};
use crate::error::{ConfigError, EnvError, StateError};
use crate::replay::ReplayConfig;
use crate::state::GameState;
use std::sync::Arc;

fn make_db() -> Arc<CardDb> {
    let mut cards = Vec::new();
    for id in 1..=13u32 {
        cards.push(CardStatic {
            id,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        });
    }
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_deck() -> Vec<u32> {
    let mut deck = Vec::new();
    for id in 1..=12u32 {
        deck.extend(std::iter::repeat_n(id, 4));
    }
    deck.extend(std::iter::repeat_n(13u32, 2));
    assert_eq!(deck.len(), 50);
    deck
}

fn make_config(deck: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck.clone(), deck],
        deck_ids: [1, 2],
        max_decisions: 10,
        max_ticks: 100,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn first_legal_actions(pool: &EnvPool) -> Vec<u32> {
    let mut actions = vec![0u32; pool.envs.len()];
    pool.first_legal_action_ids_into(&mut actions)
        .expect("first legal actions");
    actions
}

fn replay_config_for_test(label: &str) -> ReplayConfig {
    let mut replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: std::env::temp_dir().join(format!(
            "weiss-replay-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        )),
        ..ReplayConfig::default()
    };
    replay_config.rebuild_cache();
    replay_config
}

#[test]
fn thread_pool_is_per_env_pool() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let pool = EnvPool::new_debug(
        2,
        db,
        config,
        curriculum,
        7,
        Some(2),
        DebugConfig::default(),
    )
    .expect("pool");
    assert_eq!(pool.envs.len(), 2);
    assert!(pool.thread_pool.is_some());
    assert_eq!(pool.thread_pool.as_ref().unwrap().current_num_threads(), 2);
    assert_eq!(pool.effective_num_threads(), 2);
}

#[test]
fn effective_num_threads_is_one_when_serial() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let pool = EnvPool::new_debug(2, db, config, curriculum, 8, None, DebugConfig::default())
        .expect("pool");
    assert_eq!(pool.effective_num_threads(), 1);
}

#[test]
fn reset_indices_with_masks_matches_action_masks() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(2, db, config, curriculum, 11, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    let _ = pool.reset_into(&mut out.view_mut());

    let mut reset_out = BatchOutMinimalBuffers::new(pool.envs.len());
    let _ = pool.reset_indices_into(&[0], &mut reset_out.view_mut());
    let masks_snapshot = reset_out.masks.clone();
    let masks = pool.action_masks_batch().expect("masks");
    assert_eq!(
        masks_snapshot.as_slice(),
        masks.as_slice(),
        "mask scratch must match action_masks_batch"
    );
}

#[test]
fn legal_action_ids_match_action_masks() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(2, db, config, curriculum, 13, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    let _ = pool.reset_into(&mut out.view_mut());

    let num_envs = pool.envs.len();
    let mut ids = vec![0u16; num_envs * ACTION_SPACE_SIZE];
    let mut offsets = vec![0u32; num_envs + 1];
    let total = pool
        .legal_action_ids_batch_into(&mut ids, &mut offsets)
        .expect("ids");
    assert!(total <= ids.len());

    for env_idx in 0..num_envs {
        let start = offsets[env_idx] as usize;
        let end = offsets[env_idx + 1] as usize;
        let mask_offset = env_idx * ACTION_SPACE_SIZE;
        let mask = &out.masks[mask_offset..mask_offset + ACTION_SPACE_SIZE];
        let mut expected = Vec::new();
        for (action_id, &value) in mask.iter().enumerate() {
            if value != 0 {
                expected.push(action_id as u16);
            }
        }
        assert_eq!(&ids[start..end], expected.as_slice());
    }
}

#[test]
fn engine_error_reset_count_tracks_auto_resets() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(2, db, config, curriculum, 9, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());

    assert_eq!(pool.engine_error_reset_count(), 0);
    let codes = vec![1u8, 0u8];
    let reset = pool
        .auto_reset_on_error_codes_into(&codes, &mut out.view_mut())
        .expect("auto reset");
    assert_eq!(reset, 1);
    assert_eq!(pool.engine_error_reset_count(), 1);

    pool.reset_engine_error_reset_count();
    assert_eq!(pool.engine_error_reset_count(), 0);
}

#[test]
fn strict_pool_step_panic_isolated_to_single_env() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(2, db, config, curriculum, 17, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[0].obs_buf.clear();
    let actions = first_legal_actions(&pool);
    pool.step_into(&actions, &mut out.view_mut()).expect("step");

    assert_eq!(out.engine_status[0], EngineErrorCode::Panic as u8);
    assert!(out.truncated[0]);
    assert!(!out.terminated[0]);
    assert_eq!(out.engine_status[1], EngineErrorCode::None as u8);
}

#[test]
fn strict_pool_step_panic_isolated_to_single_env_with_thread_pool() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(
        4,
        db,
        config,
        curriculum,
        1701,
        Some(2),
        DebugConfig::default(),
    )
    .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[2].obs_buf.clear();
    let actions = first_legal_actions(&pool);
    pool.step_into(&actions, &mut out.view_mut()).expect("step");

    assert_eq!(out.engine_status[2], EngineErrorCode::Panic as u8);
    for idx in [0usize, 1, 3] {
        assert_eq!(out.engine_status[idx], EngineErrorCode::None as u8);
    }
}

#[test]
fn strict_pool_faults_do_not_abort_batch() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(2, db, config, curriculum, 99, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    assert_eq!(pool.error_policy, ErrorPolicy::Strict);
    pool.envs[1].obs_buf.clear();
    let actions = first_legal_actions(&pool);
    let result = pool.step_into(&actions, &mut out.view_mut());
    assert!(result.is_ok());
    assert_eq!(out.engine_status[1], EngineErrorCode::Panic as u8);
    assert_eq!(out.engine_status[0], EngineErrorCode::None as u8);
}

#[test]
fn step_panic_recovery_preserves_replay_sampling_config() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(
        1,
        db,
        config,
        curriculum,
        1099,
        None,
        DebugConfig::default(),
    )
    .expect("pool");
    let replay_config = replay_config_for_test("step-panic");
    pool.enable_replay_sampling(replay_config.clone())
        .expect("enable replay");
    assert!(pool.envs[0].replay_writer.is_some());

    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");
    pool.envs[0].obs_buf.clear();
    let actions = first_legal_actions(&pool);
    pool.step_into(&actions, &mut out.view_mut()).expect("step");

    assert_eq!(out.engine_status[0], EngineErrorCode::Panic as u8);
    assert!(pool.envs[0].replay_writer.is_some());
    assert_eq!(pool.envs[0].replay_config.enabled, replay_config.enabled);
    assert_eq!(
        pool.envs[0].replay_config.sample_rate,
        replay_config.sample_rate
    );
    assert_eq!(
        pool.envs[0].replay_config.sample_threshold,
        replay_config.sample_threshold
    );
    assert_eq!(pool.envs[0].replay_config.out_dir, replay_config.out_dir);
}

#[test]
fn reset_panic_isolated_to_single_env() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(2, db, config, curriculum, 123, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[1].player_block_cache_self[0].clear();
    pool.reset_indices_into(&[1], &mut out.view_mut())
        .expect("reset indices");

    assert_eq!(out.engine_status[1], EngineErrorCode::ResetPanic as u8);
    assert!(out.truncated[1]);
    assert!(!out.terminated[1]);
    assert_eq!(out.engine_status[0], EngineErrorCode::None as u8);
}

#[test]
fn reset_panic_isolated_to_single_env_with_thread_pool() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(
        4,
        db,
        config,
        curriculum,
        1702,
        Some(2),
        DebugConfig::default(),
    )
    .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[3].player_block_cache_self[0].clear();
    pool.reset_indices_into(&[3], &mut out.view_mut())
        .expect("reset indices");

    assert_eq!(out.engine_status[3], EngineErrorCode::ResetPanic as u8);
    for idx in [0usize, 1, 2] {
        assert_eq!(out.engine_status[idx], EngineErrorCode::None as u8);
    }
}

#[test]
fn reset_panic_recovery_preserves_replay_sampling_config() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(
        1,
        db,
        config,
        curriculum,
        1123,
        None,
        DebugConfig::default(),
    )
    .expect("pool");
    let replay_config = replay_config_for_test("reset-panic");
    pool.enable_replay_sampling(replay_config.clone())
        .expect("enable replay");
    assert!(pool.envs[0].replay_writer.is_some());

    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");
    pool.envs[0].player_block_cache_self[0].clear();
    pool.reset_indices_into(&[0], &mut out.view_mut())
        .expect("reset indices");

    assert_eq!(out.engine_status[0], EngineErrorCode::ResetPanic as u8);
    assert!(pool.envs[0].replay_writer.is_some());
    assert_eq!(pool.envs[0].replay_config.enabled, replay_config.enabled);
    assert_eq!(
        pool.envs[0].replay_config.sample_rate,
        replay_config.sample_rate
    );
    assert_eq!(
        pool.envs[0].replay_config.sample_threshold,
        replay_config.sample_threshold
    );
    assert_eq!(pool.envs[0].replay_config.out_dir, replay_config.out_dir);
}

#[test]
fn reset_error_is_split_from_reset_panic() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(1, db, config, curriculum, 223, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[0].config.deck_lists[0].pop();
    pool.reset_into(&mut out.view_mut())
        .expect("reset with invalid deck");

    assert_eq!(out.engine_status[0], EngineErrorCode::ResetError as u8);
    assert_ne!(out.engine_status[0], EngineErrorCode::ResetPanic as u8);
    assert!(out.truncated[0]);
}

#[test]
fn reset_revalidates_unknown_card_ids() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(1, db, config, curriculum, 224, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[0].config.deck_lists[0][0] = 999_999;
    pool.reset_into(&mut out.view_mut())
        .expect("reset with unknown card id");

    assert_eq!(out.engine_status[0], EngineErrorCode::ResetError as u8);
    assert!(out.truncated[0]);
}

#[test]
fn fault_latch_is_sticky_until_reset() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(1, db, config, curriculum, 77, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");

    pool.envs[0].obs_buf.clear();
    let actions = first_legal_actions(&pool);
    pool.step_into(&actions, &mut out.view_mut())
        .expect("first step");
    let first_status = out.engine_status[0];
    let first_actor = out.actor[0];
    let first_fingerprint = pool.envs[0]
        .fault_record()
        .expect("fault record")
        .fingerprint;

    pool.step_into(&actions, &mut out.view_mut())
        .expect("second step");
    assert_eq!(out.engine_status[0], first_status);
    assert_eq!(out.actor[0], first_actor);
    assert!(out.truncated[0]);
    assert!(!out.terminated[0]);
    assert_eq!(out.rewards[0], 0.0);
    assert_eq!(
        pool.envs[0]
            .fault_record()
            .expect("fault record")
            .fingerprint,
        first_fingerprint
    );

    pool.reset_into(&mut out.view_mut())
        .expect("reset clears fault");
    assert_eq!(out.engine_status[0], EngineErrorCode::None as u8);
}

#[test]
fn fault_fingerprint_is_deterministic_for_same_seed() {
    fn run_once(seed: u64) -> (u8, u64, f32, bool) {
        let db = make_db();
        let config = make_config(make_deck());
        let curriculum = CurriculumConfig::default();
        let mut pool = EnvPool::new_debug(
            2,
            db,
            config,
            curriculum,
            seed,
            None,
            DebugConfig::default(),
        )
        .expect("pool");
        let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
        pool.reset_into(&mut out.view_mut()).expect("reset");
        let actions = first_legal_actions(&pool);
        pool.envs[1].obs_buf.clear();
        pool.step_into(&actions, &mut out.view_mut()).expect("step");
        let fingerprint = pool.envs[1]
            .fault_record()
            .expect("fault record")
            .fingerprint;
        (
            out.engine_status[1],
            fingerprint,
            out.rewards[1],
            out.truncated[1],
        )
    }

    let a = run_once(5150);
    let b = run_once(5150);
    assert_eq!(a, b);
    assert_eq!(a.0, EngineErrorCode::Panic as u8);
}

#[test]
fn step_writes_every_output_slot_on_mixed_success_and_fault() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(3, db, config, curriculum, 41, None, DebugConfig::default())
        .expect("pool");
    let mut out = BatchOutMinimalBuffers::new(pool.envs.len());
    pool.reset_into(&mut out.view_mut()).expect("reset");
    let actions = first_legal_actions(&pool);

    out.rewards.fill(f32::NAN);
    out.actor.fill(99);
    out.engine_status.fill(255);
    out.spec_hash.fill(0);
    out.masks.fill(7);
    pool.envs[1].obs_buf.clear();
    pool.step_into(&actions, &mut out.view_mut()).expect("step");

    for i in 0..pool.envs.len() {
        assert!(out.rewards[i].is_finite());
        assert_ne!(out.actor[i], 99);
        assert_ne!(out.engine_status[i], 255);
        assert_eq!(out.spec_hash[i], SPEC_HASH);
        let row = &out.masks[i * ACTION_SPACE_SIZE..(i + 1) * ACTION_SPACE_SIZE];
        assert!(row.iter().all(|&v| v == 0 || v == 1));
    }
}

#[test]
fn constructor_and_validation_return_typed_errors() {
    let db = make_db();
    let mut invalid_config = make_config(make_deck());
    invalid_config.deck_lists[0][0] = 999_999;
    let cfg_err = invalid_config
        .validate_with_db(&db)
        .expect_err("invalid card id should fail");
    assert!(matches!(
        cfg_err,
        ConfigError::UnknownCardId {
            player: 0,
            card_id: 999_999
        }
    ));

    let deck = make_deck();
    let state_err = GameState::new(deck.clone(), deck, 5, 2).expect_err("state error expected");
    assert!(matches!(
        state_err,
        StateError::InvalidStartingPlayer { got: 2 }
    ));

    let mut bad_env_config = make_config(make_deck());
    bad_env_config.deck_lists[0].pop();
    let env_err = GameEnv::new(
        db,
        bad_env_config,
        CurriculumConfig::default(),
        0,
        ReplayConfig::default(),
        None,
        0,
    );
    let env_err = match env_err {
        Ok(_) => panic!("env constructor should fail"),
        Err(err) => err,
    };
    assert!(matches!(
        env_err,
        EnvError::Config(ConfigError::DeckLength {
            player: 0,
            got: 49,
            expected: 50
        })
    ));
}

#[test]
fn latch_fault_keeps_fault_source_metadata() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut env =
        GameEnv::new_or_panic(db, config, curriculum, 17, ReplayConfig::default(), None, 0);
    let _ = env.latch_fault(
        EngineErrorCode::InvariantViolation,
        Some(0),
        FaultSource::Step,
        false,
    );
    let record = env.fault_record().expect("fault record");
    assert_eq!(record.code, EngineErrorCode::InvariantViolation);
    assert_eq!(record.source, FaultSource::Step);
}
