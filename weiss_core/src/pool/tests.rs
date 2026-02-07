use super::*;
use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardColor, CardDb, CardStatic, CardType};
use crate::encode::ACTION_SPACE_SIZE;
use crate::env::DebugConfig;
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
