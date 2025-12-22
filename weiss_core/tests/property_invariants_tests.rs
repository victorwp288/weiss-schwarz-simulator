use std::sync::{Arc, OnceLock};

use proptest::prelude::*;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::env::GameEnv;
use weiss_core::util::hash_value;
use weiss_core::util::Rng64;

fn make_db() -> Arc<CardDb> {
    let cards = vec![
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
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_env(seed: u64) -> GameEnv {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![1; 20];
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
        for _ in 0..80 {
            if env.state.terminal.is_some() {
                break;
            }
            let decision = env.decision.clone().expect("decision should exist");
            let actions = weiss_core::legal::legal_actions(&env.state, &decision, &env.db, &env.curriculum);
            let idx = rng.gen_range(actions.len());
            env.apply_action(actions[idx].clone()).unwrap();
            env.validate_state().unwrap();
            prop_assert_eq!(total_cards(&env, 0), 20);
            prop_assert_eq!(total_cards(&env, 1), 20);
        }
    }

    #[test]
    fn proptest_determinism(seed in any::<u64>()) {
        enable_validate();
        let mut env_a = make_env(seed);
        let mut env_b = make_env(seed);
        let mut rng = Rng64::new(seed ^ 0xBEEF_BEEF);
        for _ in 0..80 {
            if env_a.state.terminal.is_some() || env_b.state.terminal.is_some() {
                break;
            }
            let decision = env_a.decision.clone().expect("decision should exist");
            let actions = weiss_core::legal::legal_actions(&env_a.state, &decision, &env_a.db, &env_a.curriculum);
            let idx = rng.gen_range(actions.len());
            let action = actions[idx].clone();
            env_a.apply_action(action.clone()).unwrap();
            env_b.apply_action(action).unwrap();
            prop_assert_eq!(hash_value(&env_a.state), hash_value(&env_b.state));
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
