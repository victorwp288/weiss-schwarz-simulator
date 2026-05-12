use super::*;
use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardColor, CardDb, CardStatic, CardType};
use crate::encode::{
    action_meta_for_id, ACTION_META_UNUSED, ACTION_META_WIDTH, ACTION_SPACE_SIZE, CHOICE_BASE,
    LEGAL_ACTION_CONTEXT_UNUSED, LEGAL_ACTION_CONTEXT_V1_WIDTH, OBS_LEN, SPEC_HASH,
};
use crate::env::{DebugConfig, EngineErrorCode, FaultSource, GameEnv};
use crate::error::{ConfigError, EnvError, StateError};
use crate::legal::{Decision, DecisionKind};
use crate::replay::ReplayConfig;
use crate::state::{ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, GameState};
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

fn assert_packed_legal_output_matches_direct_export(
    pool: &mut EnvPool,
    ids: &[u16],
    offsets: &[u32],
    meta: &[u16],
) {
    let num_envs = pool.envs.len();
    let mut expected_ids = vec![0u16; num_envs * ACTION_SPACE_SIZE];
    let mut expected_offsets = vec![0u32; num_envs + 1];
    let total = pool
        .legal_action_ids_batch_into(&mut expected_ids, &mut expected_offsets)
        .expect("direct legal ids");
    assert_eq!(offsets, expected_offsets.as_slice());
    assert_eq!(&ids[..total], &expected_ids[..total]);

    let mut expected_meta =
        vec![ACTION_META_UNUSED; num_envs * ACTION_SPACE_SIZE * ACTION_META_WIDTH];
    let meta_total = pool
        .legal_action_meta_batch_into(&mut expected_meta)
        .expect("direct legal action meta");
    assert_eq!(meta_total, total);
    let used_meta_len = total * ACTION_META_WIDTH;
    assert_eq!(&meta[..used_meta_len], &expected_meta[..used_meta_len]);
}

struct I16LegalIdTestBuffers {
    obs: Vec<i16>,
    legal_ids: Vec<u16>,
    legal_action_meta: Vec<u16>,
    legal_offsets: Vec<u32>,
    rewards: Vec<f32>,
    terminated: Vec<bool>,
    truncated: Vec<bool>,
    actor: Vec<i8>,
    decision_kind: Vec<i8>,
    decision_id: Vec<u32>,
    engine_status: Vec<u8>,
    spec_hash: Vec<u64>,
    main_move_action: Vec<bool>,
    main_pass_action: Vec<bool>,
}

impl I16LegalIdTestBuffers {
    fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0i16; num_envs * OBS_LEN],
            legal_ids: vec![0u16; num_envs * ACTION_SPACE_SIZE],
            legal_action_meta: vec![
                ACTION_META_UNUSED;
                num_envs * ACTION_SPACE_SIZE * ACTION_META_WIDTH
            ],
            legal_offsets: vec![0u32; num_envs + 1],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![0; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
            main_move_action: vec![false; num_envs],
            main_pass_action: vec![false; num_envs],
        }
    }

    fn view_mut(&mut self) -> BatchOutMinimalI16LegalIds<'_> {
        BatchOutMinimalI16LegalIds {
            obs: &mut self.obs,
            legal_ids: &mut self.legal_ids,
            legal_action_meta: &mut self.legal_action_meta,
            legal_offsets: &mut self.legal_offsets,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
            main_move_action: &mut self.main_move_action,
            main_pass_action: &mut self.main_pass_action,
        }
    }

    fn assert_matches_direct_export(&self, pool: &mut EnvPool) {
        assert_packed_legal_output_matches_direct_export(
            pool,
            &self.legal_ids,
            &self.legal_offsets,
            &self.legal_action_meta,
        );
    }
}

struct I16LegalIdNoMetaTestBuffers {
    obs: Vec<i16>,
    legal_ids: Vec<u16>,
    legal_offsets: Vec<u32>,
    rewards: Vec<f32>,
    terminated: Vec<bool>,
    truncated: Vec<bool>,
    actor: Vec<i8>,
    decision_kind: Vec<i8>,
    decision_id: Vec<u32>,
    engine_status: Vec<u8>,
    spec_hash: Vec<u64>,
    main_move_action: Vec<bool>,
    main_pass_action: Vec<bool>,
}

impl I16LegalIdNoMetaTestBuffers {
    fn new(num_envs: usize) -> Self {
        Self {
            obs: vec![0i16; num_envs * OBS_LEN],
            legal_ids: vec![0u16; num_envs * ACTION_SPACE_SIZE],
            legal_offsets: vec![0u32; num_envs + 1],
            rewards: vec![0.0; num_envs],
            terminated: vec![false; num_envs],
            truncated: vec![false; num_envs],
            actor: vec![0; num_envs],
            decision_kind: vec![0; num_envs],
            decision_id: vec![0; num_envs],
            engine_status: vec![0; num_envs],
            spec_hash: vec![SPEC_HASH; num_envs],
            main_move_action: vec![false; num_envs],
            main_pass_action: vec![false; num_envs],
        }
    }

    fn view_mut(&mut self) -> BatchOutMinimalI16LegalIdsNoMeta<'_> {
        BatchOutMinimalI16LegalIdsNoMeta {
            obs: &mut self.obs,
            legal_ids: &mut self.legal_ids,
            legal_offsets: &mut self.legal_offsets,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            spec_hash: &mut self.spec_hash,
            main_move_action: &mut self.main_move_action,
            main_pass_action: &mut self.main_pass_action,
        }
    }

    fn assert_matches_meta_layout(&self, expected: &I16LegalIdTestBuffers) {
        assert_eq!(self.obs, expected.obs);
        assert_eq!(self.legal_ids, expected.legal_ids);
        assert_eq!(self.legal_offsets, expected.legal_offsets);
        assert_eq!(self.rewards, expected.rewards);
        assert_eq!(self.terminated, expected.terminated);
        assert_eq!(self.truncated, expected.truncated);
        assert_eq!(self.actor, expected.actor);
        assert_eq!(self.decision_kind, expected.decision_kind);
        assert_eq!(self.decision_id, expected.decision_id);
        assert_eq!(self.engine_status, expected.engine_status);
        assert_eq!(self.spec_hash, expected.spec_hash);
        assert_eq!(self.main_move_action, expected.main_move_action);
        assert_eq!(self.main_pass_action, expected.main_pass_action);
    }
}

struct I16LegalIdTrajectoryTestBuffers {
    obs: Vec<i16>,
    legal_ids: Vec<u16>,
    legal_action_meta: Vec<u16>,
    legal_offsets: Vec<u32>,
    rewards: Vec<f32>,
    terminated: Vec<bool>,
    truncated: Vec<bool>,
    actor: Vec<i8>,
    decision_kind: Vec<i8>,
    decision_id: Vec<u32>,
    engine_status: Vec<u8>,
    episode_seed: Vec<u64>,
    spec_hash: Vec<u64>,
    main_move_action: Vec<bool>,
    main_pass_action: Vec<bool>,
    actions: Vec<u32>,
}

impl I16LegalIdTrajectoryTestBuffers {
    fn new(steps: usize, num_envs: usize) -> Self {
        let env_steps = steps * num_envs;
        Self {
            obs: vec![0i16; env_steps * OBS_LEN],
            legal_ids: vec![0u16; env_steps * ACTION_SPACE_SIZE],
            legal_action_meta: vec![
                ACTION_META_UNUSED;
                env_steps * ACTION_SPACE_SIZE * ACTION_META_WIDTH
            ],
            legal_offsets: vec![0u32; steps * (num_envs + 1)],
            rewards: vec![0.0; env_steps],
            terminated: vec![false; env_steps],
            truncated: vec![false; env_steps],
            actor: vec![0; env_steps],
            decision_kind: vec![0; env_steps],
            decision_id: vec![0; env_steps],
            engine_status: vec![0; env_steps],
            episode_seed: vec![0; env_steps],
            spec_hash: vec![SPEC_HASH; env_steps],
            main_move_action: vec![false; env_steps],
            main_pass_action: vec![false; env_steps],
            actions: vec![0; env_steps],
        }
    }

    fn view_mut(&mut self) -> BatchOutTrajectoryI16LegalIds<'_> {
        BatchOutTrajectoryI16LegalIds {
            obs: &mut self.obs,
            legal_ids: &mut self.legal_ids,
            legal_action_meta: &mut self.legal_action_meta,
            legal_offsets: &mut self.legal_offsets,
            rewards: &mut self.rewards,
            terminated: &mut self.terminated,
            truncated: &mut self.truncated,
            actor: &mut self.actor,
            decision_kind: &mut self.decision_kind,
            decision_id: &mut self.decision_id,
            engine_status: &mut self.engine_status,
            episode_seed: &mut self.episode_seed,
            spec_hash: &mut self.spec_hash,
            main_move_action: &mut self.main_move_action,
            main_pass_action: &mut self.main_pass_action,
            actions: &mut self.actions,
        }
    }

    fn assert_eq_to(&self, other: &Self) {
        assert_eq!(self.obs, other.obs);
        assert_eq!(self.legal_ids, other.legal_ids);
        assert_eq!(self.legal_action_meta, other.legal_action_meta);
        assert_eq!(self.legal_offsets, other.legal_offsets);
        assert_eq!(self.rewards, other.rewards);
        assert_eq!(self.terminated, other.terminated);
        assert_eq!(self.truncated, other.truncated);
        assert_eq!(self.actor, other.actor);
        assert_eq!(self.decision_kind, other.decision_kind);
        assert_eq!(self.decision_id, other.decision_id);
        assert_eq!(self.engine_status, other.engine_status);
        assert_eq!(self.episode_seed, other.episode_seed);
        assert_eq!(self.spec_hash, other.spec_hash);
        assert_eq!(self.main_move_action, other.main_move_action);
        assert_eq!(self.main_pass_action, other.main_pass_action);
        assert_eq!(self.actions, other.actions);
    }
}

#[test]
fn i16_legal_id_outputs_match_direct_export_after_reset_and_step() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(4, db, config, curriculum, 14, None, DebugConfig::default())
        .expect("pool");
    pool.set_output_mask_enabled(false);
    let num_envs = pool.envs.len();
    let mut buffers = I16LegalIdTestBuffers::new(num_envs);

    pool.reset_into_i16_legal_ids(&mut buffers.view_mut())
        .expect("reset i16 legal ids");
    buffers.assert_matches_direct_export(&mut pool);

    let mut actions = vec![0u32; num_envs];
    for (env_index, action) in actions.iter_mut().enumerate() {
        let start = buffers.legal_offsets[env_index] as usize;
        let end = buffers.legal_offsets[env_index + 1] as usize;
        *action = if start == end {
            0
        } else {
            u32::from(buffers.legal_ids[start])
        };
    }
    pool.step_into_i16_legal_ids(&actions, &mut buffers.view_mut())
        .expect("step i16 legal ids");
    buffers.assert_matches_direct_export(&mut pool);
}

#[test]
fn i16_legal_id_outputs_match_direct_export_after_reset_variants() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(4, db, config, curriculum, 15, None, DebugConfig::default())
        .expect("pool");
    pool.set_output_mask_enabled(false);
    let mut buffers = I16LegalIdTestBuffers::new(pool.envs.len());

    pool.reset_into_i16_legal_ids(&mut buffers.view_mut())
        .expect("initial reset i16 legal ids");
    buffers.assert_matches_direct_export(&mut pool);

    pool.reset_indices_into_i16_legal_ids(&[1, 3], &mut buffers.view_mut())
        .expect("reset indexed i16 legal ids");
    buffers.assert_matches_direct_export(&mut pool);

    let done_mask = [true, false, true, false];
    pool.reset_done_into_i16_legal_ids(&done_mask, &mut buffers.view_mut())
        .expect("reset done i16 legal ids");
    buffers.assert_matches_direct_export(&mut pool);

    pool.reset_indices_with_episode_seeds_into_i16_legal_ids(
        &[0, 2],
        &[1_001, 1_002],
        &mut buffers.view_mut(),
    )
    .expect("reset indexed seeded i16 legal ids");
    buffers.assert_matches_direct_export(&mut pool);
}

#[test]
fn i16_legal_id_nometa_outputs_match_meta_layout() {
    let db = make_db();
    let mut config = make_config(make_deck());
    config.max_decisions = 100_000;
    config.max_ticks = 1_000_000;
    let curriculum = CurriculumConfig::default();
    let mut pool_meta = EnvPool::new_debug(
        4,
        db.clone(),
        config.clone(),
        curriculum.clone(),
        151,
        None,
        DebugConfig::default(),
    )
    .expect("meta pool");
    let mut pool_nometa =
        EnvPool::new_debug(4, db, config, curriculum, 151, None, DebugConfig::default())
            .expect("nometa pool");
    pool_meta.set_output_mask_enabled(false);
    pool_nometa.set_output_mask_enabled(false);

    let mut expected = I16LegalIdTestBuffers::new(pool_meta.envs.len());
    let mut actual = I16LegalIdNoMetaTestBuffers::new(pool_nometa.envs.len());
    pool_meta
        .reset_into_i16_legal_ids(&mut expected.view_mut())
        .expect("reset meta");
    pool_nometa
        .reset_into_i16_legal_ids_nometa(&mut actual.view_mut())
        .expect("reset nometa");
    actual.assert_matches_meta_layout(&expected);

    let mut actions = vec![0u32; pool_meta.envs.len()];
    for (env_index, action) in actions.iter_mut().enumerate() {
        let start = expected.legal_offsets[env_index] as usize;
        *action = u32::from(expected.legal_ids[start]);
    }
    pool_meta
        .step_into_i16_legal_ids(&actions, &mut expected.view_mut())
        .expect("step meta");
    pool_nometa
        .step_into_i16_legal_ids_nometa(&actions, &mut actual.view_mut())
        .expect("step nometa");
    actual.assert_matches_meta_layout(&expected);

    let done = [false, true, false, true];
    pool_meta
        .reset_done_into_i16_legal_ids(&done, &mut expected.view_mut())
        .expect("reset done meta");
    pool_nometa
        .reset_done_into_i16_legal_ids_nometa(&done, &mut actual.view_mut())
        .expect("reset done nometa");
    actual.assert_matches_meta_layout(&expected);
}

#[test]
fn legal_action_context_v1_matches_packed_legal_ids() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(4, db, config, curriculum, 172, None, DebugConfig::default())
        .expect("pool");
    pool.set_output_mask_enabled(false);
    let mut out = I16LegalIdNoMetaTestBuffers::new(pool.envs.len());
    pool.reset_into_i16_legal_ids_nometa(&mut out.view_mut())
        .expect("reset nometa");

    let mut ids = vec![0u16; pool.envs.len() * ACTION_SPACE_SIZE];
    let mut offsets = vec![0u32; pool.envs.len() + 1];
    let ids_count = pool
        .legal_action_ids_batch_into(&mut ids, &mut offsets)
        .expect("legal ids");
    let mut context = vec![
        LEGAL_ACTION_CONTEXT_UNUSED;
        pool.envs.len() * ACTION_SPACE_SIZE * LEGAL_ACTION_CONTEXT_V1_WIDTH
    ];
    let context_count = pool
        .legal_action_context_v1_batch_into(&mut context)
        .expect("legal context");
    assert_eq!(ids_count, context_count);
    assert_eq!(
        offsets.last().copied().unwrap_or_default() as usize,
        ids_count
    );

    for (row_index, &action_id) in ids.iter().take(ids_count).enumerate() {
        let context_offset = row_index * LEGAL_ACTION_CONTEXT_V1_WIDTH;
        let row = &context[context_offset..context_offset + LEGAL_ACTION_CONTEXT_V1_WIDTH];
        let meta = action_meta_for_id(action_id as usize).expect("action meta");
        assert_eq!(row[0], i32::from(meta[0]));
        assert!(row[4] >= 0, "decision_kind should be populated");
        assert!(row[5] == 0 || row[5] == 1, "actor should be a seat");
    }
}

#[test]
fn legal_action_context_v1_hides_opponent_hidden_choice_card() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut pool = EnvPool::new_debug(1, db, config, curriculum, 173, None, DebugConfig::default())
        .expect("pool");
    pool.set_output_mask_enabled(false);
    let mut out = I16LegalIdNoMetaTestBuffers::new(pool.envs.len());
    pool.reset_into_i16_legal_ids_nometa(&mut out.view_mut())
        .expect("reset nometa");

    let env = &mut pool.envs[0];
    let hidden = env.state.players[1].hand[0];
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Choice,
        focus_slot: None,
    });
    env.state.turn.choice = Some(ChoiceState {
        id: 77,
        reason: ChoiceReason::CostPayment,
        player: 1,
        options: vec![ChoiceOptionRef {
            card_id: hidden.id,
            instance_id: hidden.instance_id,
            zone: ChoiceZone::Hand,
            index: Some(0),
            target_slot: None,
        }],
        total_candidates: 1,
        page_start: 0,
        pending_trigger: None,
    });
    env.action_cache.clear();
    env.action_cache.last_action_ids.push(CHOICE_BASE as u16);

    let mut context =
        vec![LEGAL_ACTION_CONTEXT_UNUSED; ACTION_SPACE_SIZE * LEGAL_ACTION_CONTEXT_V1_WIDTH];
    let context_count = pool
        .legal_action_context_v1_batch_into(&mut context)
        .expect("legal context");
    assert_eq!(context_count, 1);
    let row = &context[..LEGAL_ACTION_CONTEXT_V1_WIDTH];
    assert_eq!(row[6], 1, "source zone should still identify hand");
    assert_eq!(
        row[7], LEGAL_ACTION_CONTEXT_UNUSED,
        "hidden opponent hand index should be hidden"
    );
    assert_eq!(
        row[8], LEGAL_ACTION_CONTEXT_UNUSED,
        "hidden opponent hand card id should be hidden"
    );
    assert_eq!(row[9], LEGAL_ACTION_CONTEXT_UNUSED);
    assert_eq!(row[10], LEGAL_ACTION_CONTEXT_UNUSED);
    assert_eq!(row[11], LEGAL_ACTION_CONTEXT_UNUSED);
}

#[test]
fn heuristic_public_rollout_matches_encoded_step_reference() {
    let db = make_db();
    let mut config = make_config(make_deck());
    config.max_decisions = 100_000;
    config.max_ticks = 1_000_000;
    let curriculum = CurriculumConfig::default();
    let mut actual_pool = EnvPool::new_debug(
        4,
        db.clone(),
        config.clone(),
        curriculum.clone(),
        16,
        None,
        DebugConfig::default(),
    )
    .expect("actual pool");
    let mut reference_pool =
        EnvPool::new_debug(4, db, config, curriculum, 16, None, DebugConfig::default())
            .expect("reference pool");
    actual_pool.set_output_mask_enabled(false);
    reference_pool.set_output_mask_enabled(false);

    let steps = 5;
    let num_envs = actual_pool.envs.len();
    let mut actual = I16LegalIdTrajectoryTestBuffers::new(steps, num_envs);
    let mut reference = I16LegalIdTrajectoryTestBuffers::new(steps, num_envs);
    actual_pool
        .rollout_heuristic_public_profile_into_i16_legal_ids(steps, &mut actual.view_mut(), "base")
        .expect("optimized rollout");

    let keep_flags = vec![false; num_envs];
    let env_indices: Vec<usize> = (0..num_envs).collect();
    let mut chosen_actions = vec![0u16; num_envs];
    let mut done_flags = vec![false; num_envs];
    let mut step_out = I16LegalIdTestBuffers::new(num_envs);

    for t in 0..steps {
        reference_pool
            .fill_outcomes_for_flags(&keep_flags)
            .expect("reference pre-step outcomes");

        let step_offset = t * num_envs;
        let obs_offset = step_offset * OBS_LEN;
        let ids_offset = step_offset * ACTION_SPACE_SIZE;
        let offsets_offset = t * (num_envs + 1);
        let meta_offset = ids_offset * ACTION_META_WIDTH;
        {
            let mut pre_step = BatchOutMinimalI16LegalIds {
                obs: &mut reference.obs[obs_offset..obs_offset + num_envs * OBS_LEN],
                legal_ids: &mut reference.legal_ids
                    [ids_offset..ids_offset + num_envs * ACTION_SPACE_SIZE],
                legal_action_meta: &mut reference.legal_action_meta
                    [meta_offset..meta_offset + num_envs * ACTION_SPACE_SIZE * ACTION_META_WIDTH],
                legal_offsets: &mut reference.legal_offsets
                    [offsets_offset..offsets_offset + num_envs + 1],
                rewards: &mut reference.rewards[step_offset..step_offset + num_envs],
                terminated: &mut reference.terminated[step_offset..step_offset + num_envs],
                truncated: &mut reference.truncated[step_offset..step_offset + num_envs],
                actor: &mut reference.actor[step_offset..step_offset + num_envs],
                decision_kind: &mut reference.decision_kind[step_offset..step_offset + num_envs],
                decision_id: &mut reference.decision_id[step_offset..step_offset + num_envs],
                engine_status: &mut reference.engine_status[step_offset..step_offset + num_envs],
                spec_hash: &mut reference.spec_hash[step_offset..step_offset + num_envs],
                main_move_action: &mut reference.main_move_action
                    [step_offset..step_offset + num_envs],
                main_pass_action: &mut reference.main_pass_action
                    [step_offset..step_offset + num_envs],
            };
            reference_pool
                .fill_minimal_out_i16_legal_ids(&reference_pool.outcomes_scratch, &mut pre_step)
                .expect("reference pre-step fill");
        }
        for (dst, env) in reference.episode_seed[step_offset..step_offset + num_envs]
            .iter_mut()
            .zip(reference_pool.envs.iter())
        {
            *dst = env.episode_seed;
        }

        reference_pool
            .choose_heuristic_public_profile_actions_into(&env_indices, &mut chosen_actions, "base")
            .expect("reference heuristic actions");
        for (dst, &action_id) in reference.actions[step_offset..step_offset + num_envs]
            .iter_mut()
            .zip(chosen_actions.iter())
        {
            *dst = u32::from(action_id);
        }

        reference_pool
            .step_into_i16_legal_ids(
                &reference.actions[step_offset..step_offset + num_envs],
                &mut step_out.view_mut(),
            )
            .expect("reference encoded step");
        for (env_index, done_flag) in done_flags.iter_mut().enumerate().take(num_envs) {
            let dst = step_offset + env_index;
            reference.rewards[dst] = step_out.rewards[env_index];
            reference.terminated[dst] = step_out.terminated[env_index];
            reference.truncated[dst] = step_out.truncated[env_index];
            reference.engine_status[dst] = step_out.engine_status[env_index];
            reference.main_move_action[dst] = step_out.main_move_action[env_index];
            reference.main_pass_action[dst] = step_out.main_pass_action[env_index];
            *done_flag = step_out.terminated[env_index] || step_out.truncated[env_index];
        }

        if done_flags.iter().any(|&done| done) {
            reference_pool
                .fill_outcomes_for_flags(&done_flags)
                .expect("reference auto-reset outcomes");
        }
    }

    actual.assert_eq_to(&reference);
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
