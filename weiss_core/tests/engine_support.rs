#![allow(dead_code)]

use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TriggerIcon};
use weiss_core::env::GameEnv;

pub fn make_db() -> Arc<CardDb> {
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
            card_type: CardType::Character,
            color: CardColor::Blue,
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
            id: 3,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 2,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 4,
            card_set: None,
            card_type: CardType::Climax,
            color: CardColor::Red,
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
        CardStatic {
            id: 5,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Soul],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 6,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::CounterBackup { power: 1000 }],
            ability_defs: vec![],
            counter_timing: true,
            raw_text: None,
        },
        CardStatic {
            id: 7,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 9000,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 8,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 1000,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 9,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 1,
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
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

pub fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [10, 11],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

pub fn default_curriculum() -> CurriculumConfig {
    CurriculumConfig::default()
}

pub fn temp_dir(label: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    dir.push(format!("ws_sim_{label}_{stamp}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn total_cards(env: &GameEnv, player: usize) -> usize {
    let p = &env.state.players[player];
    let mut total = p.hand.len()
        + p.deck.len()
        + p.stock.len()
        + p.waiting_room.len()
        + p.clock.len()
        + p.level.len()
        + p.memory.len()
        + p.climax.len();
    for slot in &p.stage {
        if slot.card.is_some() {
            total += 1;
        }
    }
    total
}
