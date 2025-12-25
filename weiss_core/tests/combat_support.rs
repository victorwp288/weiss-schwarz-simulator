#![allow(dead_code)]

#[path = "deck_support.rs"]
mod deck_support;

use std::sync::{Arc, OnceLock};

use weiss_core::config::{EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig};
use weiss_core::db::{
    AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TargetTemplate, TriggerIcon,
};
use weiss_core::env::GameEnv;
use weiss_core::legal::{Decision, DecisionKind};
use weiss_core::replay::ReplayConfig;
use weiss_core::state::{CardInstance, Phase, StageSlot, StageStatus};

pub const CARD_BASIC: u32 = 1;
pub const CARD_EFFECT_ATTACK: u32 = 3;
pub const CARD_COUNTER_CANCEL: u32 = 4;
pub const CARD_COUNTER_REDUCE: u32 = 5;
pub const CARD_CLIMAX: u32 = 6;
pub const CARD_TRIGGER_MULTI: u32 = 7;
pub const CARD_END_DRAW: u32 = 8;
pub const CARD_EVENT_DAMAGE: u32 = 9;
pub const CARD_MULTI_EFFECT_ATTACK: u32 = 10;
pub const CARD_HIGH_POWER: u32 = 11;
pub const CARD_END_DRAW_DOUBLE: u32 = 12;
pub const CARD_TRIGGER_GATE: u32 = 13;
pub const CARD_TRIGGER_BOUNCE: u32 = 14;
pub const CARD_TRIGGER_TREASURE: u32 = 15;
pub const CARD_TRIGGER_STANDBY: u32 = 16;
pub const CARD_CANNOT_ATTACK: u32 = 17;
pub const CARD_COUNTER_DOUBLE_REDUCE: u32 = 18;

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}
pub const CARD_LEVEL_ONE: u32 = 19;
pub const CARD_LEVEL_TWO: u32 = 20;
pub const CARD_ACT_ABILITY: u32 = 21;

pub fn enable_validate() {
    static VALIDATE_ONCE: OnceLock<()> = OnceLock::new();
    VALIDATE_ONCE.get_or_init(|| {
        std::env::set_var("WEISS_VALIDATE_STATE", "1");
    });
}

pub fn replay_config() -> ReplayConfig {
    ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: std::env::temp_dir(),
        compress: false,
        include_trigger_card_id: true,
        ..Default::default()
    }
}

pub fn make_db() -> Arc<CardDb> {
    let mut cards = vec![
        CardStatic {
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
        },
        CardStatic {
            id: CARD_EFFECT_ATTACK,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::AutoOnAttackDealDamage {
                amount: 2,
                cancelable: true,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_COUNTER_CANCEL,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::CounterDamageCancel],
            ability_defs: vec![],
            counter_timing: true,
            raw_text: None,
        },
        CardStatic {
            id: CARD_COUNTER_REDUCE,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::CounterDamageReduce { amount: 1 }],
            ability_defs: vec![],
            counter_timing: true,
            raw_text: None,
        },
        CardStatic {
            id: CARD_CLIMAX,
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
        CardStatic {
            id: CARD_TRIGGER_MULTI,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Soul, TriggerIcon::Draw],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_END_DRAW,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::AutoEndPhaseDraw { count: 1 }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_EVENT_DAMAGE,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::EventDealDamage {
                amount: 1,
                cancelable: true,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_MULTI_EFFECT_ATTACK,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![
                AbilityTemplate::AutoOnAttackDealDamage {
                    amount: 1,
                    cancelable: true,
                },
                AbilityTemplate::AutoOnAttackDealDamage {
                    amount: 1,
                    cancelable: true,
                },
            ],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_HIGH_POWER,
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
            id: CARD_END_DRAW_DOUBLE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::AutoEndPhaseDraw { count: 2 }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_GATE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Gate],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_BOUNCE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Bounce],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_TREASURE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Treasure],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_STANDBY,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Standby],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_CANNOT_ATTACK,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ContinuousCannotAttack],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_COUNTER_DOUBLE_REDUCE,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![
                AbilityTemplate::CounterDamageReduce { amount: 1 },
                AbilityTemplate::CounterDamageReduce { amount: 1 },
            ],
            ability_defs: vec![],
            counter_timing: true,
            raw_text: None,
        },
        CardStatic {
            id: CARD_LEVEL_ONE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 1,
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
            id: CARD_LEVEL_TWO,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 2,
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
            id: CARD_ACT_ABILITY,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedPower {
                amount: 1000,
                count: 1,
                target: TargetTemplate::SelfStage,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

pub fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    let pool = [CARD_BASIC];
    EnvConfig {
        deck_lists: [
            deck_support::legalize_deck(deck_a, &pool),
            deck_support::legalize_deck(deck_b, &pool),
        ],
        deck_ids: [100, 101],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

pub fn build_deck_list(size: usize, extras: &[u32]) -> Vec<u32> {
    let mut deck = extras.to_vec();
    while deck.len() < size {
        deck.push(CARD_BASIC);
    }
    pad_deck(deck, CARD_BASIC)
}

fn pad_deck(deck: Vec<u32>, filler: u32) -> Vec<u32> {
    let pool = [filler];
    deck_support::legalize_deck(deck, &pool)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_player_state(
    env: &mut GameEnv,
    player: usize,
    hand: Vec<u32>,
    stock: Vec<u32>,
    stage_cards: Vec<(usize, u32)>,
    deck_top: Vec<u32>,
    clock: Vec<u32>,
    level: Vec<u32>,
    waiting_room: Vec<u32>,
    memory: Vec<u32>,
    climax: Vec<u32>,
) {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, i32> = HashMap::new();
    for &card in &env.config.deck_lists[player] {
        *counts.entry(card).or_insert(0) += 1;
    }
    let mut consume = |card: u32, zone: &str| {
        let entry = counts.entry(card).or_insert(0);
        *entry -= 1;
        if *entry < 0 {
            panic!("card {card} overused in {zone}");
        }
    };

    for &card in &hand {
        consume(card, "hand");
    }
    for &card in &stock {
        consume(card, "stock");
    }
    for &card in &deck_top {
        consume(card, "deck_top");
    }
    for &card in &clock {
        consume(card, "clock");
    }
    for &card in &level {
        consume(card, "level");
    }
    for &card in &waiting_room {
        consume(card, "waiting_room");
    }
    for &card in &memory {
        consume(card, "memory");
    }
    for &card in &climax {
        consume(card, "climax");
    }
    for &(_, card) in &stage_cards {
        consume(card, "stage");
    }

    let mut remaining = Vec::new();
    for (card, count) in counts {
        if count < 0 {
            panic!("card {card} negative count");
        }
        for _ in 0..count {
            remaining.push(card);
        }
    }

    let mut deck = remaining;
    let mut top = deck_top;
    top.reverse();
    deck.extend(top);

    let owner = player as u8;
    let p = &mut env.state.players[player];
    p.hand = hand
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 1, idx))
        .collect();
    p.stock = stock
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 2, idx))
        .collect();
    p.clock = clock
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 3, idx))
        .collect();
    p.level = level
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 4, idx))
        .collect();
    p.waiting_room = waiting_room
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 5, idx))
        .collect();
    p.memory = memory
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 6, idx))
        .collect();
    p.climax = climax
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 7, idx))
        .collect();
    p.deck = deck
        .into_iter()
        .enumerate()
        .map(|(idx, id)| make_instance(id, owner, 8, idx))
        .collect();
    p.stage = [
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
    ];
    for (slot, card) in stage_cards {
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(make_instance(card, owner, 4, slot));
        slot_state.status = StageStatus::Stand;
        p.stage[slot] = slot_state;
    }
}

pub fn force_attack_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Attack;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.turn_number = 1;
    env.state.turn.attack_subphase_count = 0;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.priority = None;
    env.state.turn.stack.clear();
    env.state.turn.pending_stack_groups.clear();
    env.state.turn.stack_order = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.state.turn.main_passed = false;
    env.decision = Some(Decision {
        player,
        kind: DecisionKind::AttackDeclaration,
        focus_slot: None,
    });
}

pub fn slot_power_from_obs(obs: &[i32], player_block: usize, slot: usize) -> i32 {
    let base = weiss_core::encode::OBS_HEADER_LEN
        + player_block * weiss_core::encode::PER_PLAYER_BLOCK_LEN;
    let offset = base
        + weiss_core::encode::PER_PLAYER_COUNTS
        + slot * weiss_core::encode::PER_STAGE_SLOT
        + 3;
    obs[offset]
}
