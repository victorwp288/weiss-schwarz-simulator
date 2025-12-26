use std::sync::{Arc, OnceLock};

#[path = "deck_support.rs"]
mod deck_support;
#[path = "replay_bundle_support.rs"]
mod replay_bundle_support;

use proptest::prelude::*;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::encode::{action_id_for, MAX_DECK};
use weiss_core::env::GameEnv;
use weiss_core::fingerprint::{events_fingerprint, state_fingerprint};
use weiss_core::util::Rng64;

fn make_db() -> Arc<CardDb> {
    let mut cards = vec![
        CardStatic {
            id: 1,
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
        },
        CardStatic {
            id: 2,
            card_set: None,
            card_type: CardType::Climax,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_env(seed: u64) -> GameEnv {
    let db = make_db();
    let pool = [1];
    let deck_a = deck_support::legalize_deck(vec![1; 50], &pool);
    let deck_b = deck_support::legalize_deck(vec![1; 50], &pool);
    let config = EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [1, 2],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        seed,
        Default::default(),
        None,
        0,
    )
}

fn enable_validate() {
    static VALIDATE_ONCE: OnceLock<()> = OnceLock::new();
    VALIDATE_ONCE.get_or_init(|| {
        std::env::set_var("WEISS_VALIDATE_STATE", "1");
    });
}

fn total_cards(env: &GameEnv, player: usize) -> usize {
    let p = &env.state.players[player];
    let stage_count = p.stage.iter().filter(|c| c.card.is_some()).count();
    p.deck.len()
        + p.hand.len()
        + p.waiting_room.len()
        + p.clock.len()
        + p.level.len()
        + p.stock.len()
        + p.memory.len()
        + p.climax.len()
        + stage_count
}

proptest! {
    #[test]
    fn proptest_invariants(seed in any::<u64>()) {
        enable_validate();
        let mut env = make_env(seed);
        let mut rng = Rng64::new(seed ^ 0x1234_5678);
        let mut action_ids = Vec::new();
        for _ in 0..80 {
            if env.state.terminal.is_some() {
                break;
            }
            let decision = env.decision.clone().expect("decision should exist");
            let actions = weiss_core::legal::legal_actions(&env.state, &decision, &env.db, &env.curriculum);
            let idx = rng.gen_range(actions.len());
            if let Some(action_id) = action_id_for(&actions[idx]) {
                action_ids.push(action_id as u32);
            }
            env.apply_action(actions[idx].clone()).unwrap();
            env.validate_state().unwrap();
            let total_a = total_cards(&env, 0);
            let total_b = total_cards(&env, 1);
            if total_a != MAX_DECK || total_b != MAX_DECK {
                let state_hash = state_fingerprint(&env.state);
                let events_hash = events_fingerprint(env.canonical_events());
                replay_bundle_support::maybe_dump_failure_bundle(
                    "proptest_invariants",
                    seed,
                    &env.config,
                    &env.curriculum,
                    &action_ids,
                    state_hash,
                    events_hash,
                );
            }
            prop_assert_eq!(total_a, MAX_DECK);
            prop_assert_eq!(total_b, MAX_DECK);
        }
    }

    #[test]
    fn proptest_determinism(seed in any::<u64>()) {
        enable_validate();
        let mut env_a = make_env(seed);
        let mut env_b = make_env(seed);
        let mut rng = Rng64::new(seed ^ 0xBEEF_BEEF);
        let mut action_ids = Vec::new();
        for _ in 0..80 {
            if env_a.state.terminal.is_some() || env_b.state.terminal.is_some() {
                break;
            }
            let decision = env_a.decision.clone().expect("decision should exist");
            let actions = weiss_core::legal::legal_actions(&env_a.state, &decision, &env_a.db, &env_a.curriculum);
            let idx = rng.gen_range(actions.len());
            let action = actions[idx].clone();
            if let Some(action_id) = action_id_for(&action) {
                action_ids.push(action_id as u32);
            }
            env_a.apply_action(action.clone()).unwrap();
            env_b.apply_action(action).unwrap();
            let hash_a = state_fingerprint(&env_a.state);
            let hash_b = state_fingerprint(&env_b.state);
            if hash_a != hash_b {
                let events_hash_a = events_fingerprint(env_a.canonical_events());
                let events_hash_b = events_fingerprint(env_b.canonical_events());
                replay_bundle_support::maybe_dump_failure_bundle(
                    "proptest_determinism_a",
                    seed,
                    &env_a.config,
                    &env_a.curriculum,
                    &action_ids,
                    hash_a,
                    events_hash_a,
                );
                replay_bundle_support::maybe_dump_failure_bundle(
                    "proptest_determinism_b",
                    seed,
                    &env_b.config,
                    &env_b.curriculum,
                    &action_ids,
                    hash_b,
                    events_hash_b,
                );
            }
            prop_assert_eq!(hash_a, hash_b);
        }
    }
}

#[test]
fn fuzz_invariants_fixed_seed() {
    enable_validate();
    let seed = 2025;
    let mut env = make_env(seed);
    let mut rng = Rng64::new(seed ^ 0xDEADBEEF);
    for _ in 0..8000 {
        if env.state.terminal.is_some() {
            break;
        }
        let decision = env.decision.clone().expect("decision should exist");
        let actions =
            weiss_core::legal::legal_actions(&env.state, &decision, &env.db, &env.curriculum);
        let idx = rng.gen_range(actions.len());
        env.apply_action(actions[idx].clone()).unwrap();
        env.validate_state().unwrap();
    }
}

#[test]
fn determinism_events_fixed_seed() {
    enable_validate();
    let seed = 4242;
    let mut env_a = make_env(seed);
    let mut env_b = make_env(seed);
    let mut rng = Rng64::new(seed ^ 0xA11C_EE55);
    let mut action_ids = Vec::new();
    for _ in 0..200 {
        if env_a.state.terminal.is_some() || env_b.state.terminal.is_some() {
            break;
        }
        let decision = env_a.decision.clone().expect("decision should exist");
        let actions =
            weiss_core::legal::legal_actions(&env_a.state, &decision, &env_a.db, &env_a.curriculum);
        let idx = rng.gen_range(actions.len());
        let action = actions[idx].clone();
        if let Some(action_id) = action_id_for(&action) {
            action_ids.push(action_id as u32);
        }
        env_a.apply_action(action.clone()).unwrap();
        env_b.apply_action(action).unwrap();
        let hash_a = state_fingerprint(&env_a.state);
        let hash_b = state_fingerprint(&env_b.state);
        if hash_a != hash_b {
            let events_hash_a = events_fingerprint(env_a.canonical_events());
            let events_hash_b = events_fingerprint(env_b.canonical_events());
            replay_bundle_support::maybe_dump_failure_bundle(
                "determinism_events_a",
                seed,
                &env_a.config,
                &env_a.curriculum,
                &action_ids,
                hash_a,
                events_hash_a,
            );
            replay_bundle_support::maybe_dump_failure_bundle(
                "determinism_events_b",
                seed,
                &env_b.config,
                &env_b.curriculum,
                &action_ids,
                hash_b,
                events_hash_b,
            );
        }
        assert_eq!(hash_a, hash_b);
    }
    let events_hash_a = events_fingerprint(env_a.canonical_events());
    let events_hash_b = events_fingerprint(env_b.canonical_events());
    if events_hash_a != events_hash_b {
        replay_bundle_support::maybe_dump_failure_bundle(
            "determinism_events_final_a",
            seed,
            &env_a.config,
            &env_a.curriculum,
            &action_ids,
            state_fingerprint(&env_a.state),
            events_hash_a,
        );
        replay_bundle_support::maybe_dump_failure_bundle(
            "determinism_events_final_b",
            seed,
            &env_b.config,
            &env_b.curriculum,
            &action_ids,
            state_fingerprint(&env_b.state),
            events_hash_b,
        );
    }
    assert_eq!(events_hash_a, events_hash_b);
}
