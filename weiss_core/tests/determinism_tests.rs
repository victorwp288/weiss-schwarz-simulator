use std::sync::Arc;

#[path = "deck_support.rs"]
mod deck_support;
#[path = "replay_bundle_support.rs"]
mod replay_bundle_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::env::GameEnv;
use weiss_core::fingerprint::{events_fingerprint, state_fingerprint};
use weiss_core::legal::DecisionKind;
use weiss_core::replay::{ReplayConfig, ReplayEvent};

const CARD_BASIC: u32 = 1;

fn make_db() -> Arc<CardDb> {
    let mut cards = vec![CardStatic {
        id: CARD_BASIC,
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
    }];
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    let pool = [CARD_BASIC];
    EnvConfig {
        deck_lists: [
            deck_support::legalize_deck(deck_a, &pool),
            deck_support::legalize_deck(deck_b, &pool),
        ],
        deck_ids: [100, 101],
        max_decisions: 100,
        max_ticks: 5000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn run_episode(
    env: &mut GameEnv,
    max_steps: usize,
) -> (Vec<DecisionKind>, Vec<Vec<u8>>, Vec<u32>, u64, u64) {
    let mut kinds = Vec::new();
    let mut masks = Vec::new();
    let mut action_ids = Vec::new();
    for _ in 0..max_steps {
        if env.state.terminal.is_some() {
            break;
        }
        let decision = env.decision.as_ref().expect("decision");
        kinds.push(decision.kind);
        let mask = env.action_mask();
        masks.push(mask.to_vec());
        let action_id = mask.iter().position(|v| *v == 1).expect("legal action");
        action_ids.push(action_id as u32);
        env.apply_action_id(action_id).expect("apply action");
    }
    let state_hash = state_fingerprint(&env.state);
    let replay_hash = events_fingerprint(env.canonical_events());
    (kinds, masks, action_ids, state_hash, replay_hash)
}

#[test]
fn determinism_default_config() {
    let db = make_db();
    let deck_a = vec![CARD_BASIC; 50];
    let deck_b = vec![CARD_BASIC; 50];
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };

    let mut env_a = GameEnv::new(
        db.clone(),
        config.clone(),
        CurriculumConfig::default(),
        123,
        replay_config.clone(),
        None,
        0,
    );
    let mut env_b = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        123,
        replay_config,
        None,
        0,
    );

    let (kinds_a, masks_a, actions_a, hash_a, replay_hash_a) = run_episode(&mut env_a, 40);
    let (kinds_b, masks_b, actions_b, hash_b, replay_hash_b) = run_episode(&mut env_b, 40);

    if kinds_a != kinds_b
        || masks_a != masks_b
        || hash_a != hash_b
        || replay_hash_a != replay_hash_b
    {
        replay_bundle_support::maybe_dump_failure_bundle(
            "determinism_default_a",
            123,
            &env_a.config,
            &env_a.curriculum,
            &actions_a,
            hash_a,
            replay_hash_a,
        );
        replay_bundle_support::maybe_dump_failure_bundle(
            "determinism_default_b",
            123,
            &env_b.config,
            &env_b.curriculum,
            &actions_b,
            hash_b,
            replay_hash_b,
        );
    }

    assert_eq!(kinds_a, kinds_b);
    assert_eq!(masks_a, masks_b);
    assert_eq!(hash_a, hash_b);
    assert_eq!(replay_hash_a, replay_hash_b);
}

#[test]
fn determinism_with_flags_enabled_and_window_events_gated() {
    let db = make_db();
    let deck_a = vec![CARD_BASIC; 50];
    let deck_b = vec![CARD_BASIC; 50];
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };

    let curriculum_off = CurriculumConfig {
        enable_priority_windows: false,
        ..Default::default()
    };
    let mut env_off = GameEnv::new(
        db.clone(),
        config.clone(),
        curriculum_off,
        456,
        replay_config.clone(),
        None,
        0,
    );
    let (_kinds_off, _masks_off, _actions_off, _hash_off, _replay_hash_off) =
        run_episode(&mut env_off, 40);
    let off_has_climax_window = env_off.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::TimingWindowEntered {
                window: weiss_core::state::TimingWindow::ClimaxWindow,
                ..
            }
        )
    });
    assert!(!off_has_climax_window);

    let curriculum_on = CurriculumConfig {
        enable_priority_windows: true,
        enable_visibility_policies: true,
        ..Default::default()
    };
    let mut env_on_a = GameEnv::new(
        db.clone(),
        config.clone(),
        curriculum_on.clone(),
        456,
        replay_config.clone(),
        None,
        0,
    );
    let mut env_on_b = GameEnv::new(db, config, curriculum_on, 456, replay_config, None, 0);

    let (kinds_a, masks_a, actions_a, hash_a, replay_hash_a) = run_episode(&mut env_on_a, 40);
    let (kinds_b, masks_b, actions_b, hash_b, replay_hash_b) = run_episode(&mut env_on_b, 40);

    if kinds_a != kinds_b
        || masks_a != masks_b
        || hash_a != hash_b
        || replay_hash_a != replay_hash_b
    {
        replay_bundle_support::maybe_dump_failure_bundle(
            "determinism_flags_a",
            456,
            &env_on_a.config,
            &env_on_a.curriculum,
            &actions_a,
            hash_a,
            replay_hash_a,
        );
        replay_bundle_support::maybe_dump_failure_bundle(
            "determinism_flags_b",
            456,
            &env_on_b.config,
            &env_on_b.curriculum,
            &actions_b,
            hash_b,
            replay_hash_b,
        );
    }

    assert_eq!(kinds_a, kinds_b);
    assert_eq!(masks_a, masks_b);
    assert_eq!(hash_a, hash_b);
    assert_eq!(replay_hash_a, replay_hash_b);

    let on_has_climax_window = env_on_a.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::TimingWindowEntered {
                window: weiss_core::state::TimingWindow::ClimaxWindow,
                ..
            }
        )
    });
    assert!(on_has_climax_window);
}
