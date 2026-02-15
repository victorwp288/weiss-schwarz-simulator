use super::GameEnv;
use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardColor, CardDb, CardId, CardStatic, CardType};
use crate::effects::{EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec};
use crate::replay::ReplayConfig;
use crate::state::{CardInstance, StackItem, TargetRef, TargetSpec};
use std::collections::HashMap;
use std::sync::Arc;

const CLONE_OFFSET: u32 = 1000;
const CLONE_GROUPS: usize = 12;
const MAX_COPIES: usize = 4;

fn make_instance(id: CardId, owner: u8, next_id: &mut u32) -> CardInstance {
    let instance = CardInstance::new(id, owner, *next_id);
    *next_id = next_id.wrapping_add(1);
    instance
}

fn add_clone_cards(cards: &mut Vec<CardStatic>) {
    let base_cards = cards.clone();
    for base in base_cards {
        for idx in 1..=CLONE_GROUPS {
            let mut clone = base.clone();
            let idx_u32 = u32::try_from(idx).expect("clone group index overflow");
            let delta = CLONE_OFFSET
                .checked_mul(idx_u32)
                .expect("clone id multiplier overflow");
            clone.id = base.id.checked_add(delta).expect("clone id overflow");
            cards.push(clone);
        }
    }
}

fn legalize_deck(mut deck: Vec<u32>, filler_pool: &[u32]) -> Vec<u32> {
    let max_deck = crate::encode::MAX_DECK;
    if deck.len() > max_deck {
        panic!("deck length {} exceeds MAX_DECK {}", deck.len(), max_deck);
    }
    if filler_pool.is_empty() {
        panic!("filler pool empty");
    }
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut next_clone: HashMap<u32, u32> = HashMap::new();
    for card_id in &mut deck {
        *card_id = assign_id(*card_id, &mut counts, &mut next_clone);
    }
    let mut filler_iter = filler_pool.iter().copied().cycle();
    while deck.len() < max_deck {
        let base = filler_iter.next().expect("filler");
        let card_id = assign_id(base, &mut counts, &mut next_clone);
        deck.push(card_id);
    }
    deck
}

fn assign_id(
    base_id: u32,
    counts: &mut HashMap<u32, usize>,
    next_clone: &mut HashMap<u32, u32>,
) -> u32 {
    let count = counts.entry(base_id).or_insert(0);
    if *count < MAX_COPIES {
        *count += 1;
        return base_id;
    }
    loop {
        let idx = next_clone.entry(base_id).or_insert(1);
        if *idx > CLONE_GROUPS as u32 {
            panic!(
                "not enough clone ids for base {} (needed clone group {})",
                base_id, idx
            );
        }
        let delta = CLONE_OFFSET
            .checked_mul(*idx)
            .expect("clone id multiplier overflow");
        let clone_id = base_id.checked_add(delta).expect("clone id overflow");
        let clone_count = counts.entry(clone_id).or_insert(0);
        if *clone_count < MAX_COPIES {
            *clone_count += 1;
            return clone_id;
        }
        *idx += 1;
    }
}

fn make_env() -> GameEnv {
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
    ];
    add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db"));
    let config = EnvConfig {
        deck_lists: [
            legalize_deck(vec![1; 50], &[1]),
            legalize_deck(vec![2; 50], &[2]),
        ],
        deck_ids: [1, 2],
        max_decisions: 100,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    GameEnv::new_or_panic(
        db,
        config,
        CurriculumConfig::default(),
        1,
        Default::default(),
        None,
        0,
    )
}

fn make_noop_stack_item(id: u32) -> StackItem {
    let spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::TriggerIcon {
            icon: crate::db::TriggerIcon::Soul,
        },
        target: None,
        optional: false,
    };
    StackItem {
        id,
        controller: 0,
        source_id: 1,
        effect_id: spec.id,
        payload: EffectPayload {
            spec,
            targets: Vec::new(),
            source_ref: None,
        },
    }
}

fn make_env_with_replay(replay_config: ReplayConfig) -> GameEnv {
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
    ];
    add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db"));
    let config = EnvConfig {
        deck_lists: [
            legalize_deck(vec![1; 50], &[1]),
            legalize_deck(vec![2; 50], &[2]),
        ],
        deck_ids: [1, 2],
        max_decisions: 100,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    GameEnv::new_or_panic(
        db,
        config,
        CurriculumConfig::default(),
        2,
        replay_config,
        None,
        0,
    )
}

fn enumerate_targets_for_test(
    env: &GameEnv,
    controller: u8,
    spec: &TargetSpec,
    selected: &[TargetRef],
) -> Vec<TargetRef> {
    let mut out = Vec::new();
    GameEnv::enumerate_target_candidates_into(
        &env.state,
        &env.db,
        &env.curriculum,
        controller,
        spec,
        selected,
        &mut out,
    );
    out
}

mod choice;
mod engine;
mod observation;
mod replay;
