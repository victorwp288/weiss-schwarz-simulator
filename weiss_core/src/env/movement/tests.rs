use super::super::GameEnv;
use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardColor, CardDb, CardStatic, CardType};
use crate::events::Zone;
use crate::replay::ReplayConfig;
use crate::state::{CardInstance, ModifierDuration, ModifierKind, StageStatus};
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

fn make_env() -> GameEnv {
    let db = make_db();
    let deck = make_deck();
    let config = EnvConfig {
        deck_lists: [deck.clone(), deck],
        deck_ids: [12, 13],
        max_decisions: 10,
        max_ticks: 100,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        11,
        ReplayConfig::default(),
        None,
        0,
    )
}

#[test]
fn cached_slot_power_matches_uncached() {
    let mut env = make_env();
    let card = CardInstance::new(1, 0, 1);
    env.place_card_on_stage(0, card, 0, StageStatus::Stand, Zone::Hand, None);
    env.add_modifier(
        1,
        0,
        0,
        ModifierKind::Power,
        1000,
        ModifierDuration::UntilEndOfTurn,
    );

    let cached = env.compute_slot_power(0, 0);
    let uncached = env.compute_slot_power_uncached(0, 0);
    assert_eq!(cached, uncached);

    env.remove_modifiers_for_slot(0, 0);
    let cached_after = env.compute_slot_power(0, 0);
    let uncached_after = env.compute_slot_power_uncached(0, 0);
    assert_eq!(cached_after, uncached_after);
}
