use std::sync::Arc;

use weiss_core::config::{CurriculumConfig, EnvConfig, RewardConfig, ObservationVisibility, ErrorPolicy};
use weiss_core::db::{CardDb, CardStatic, CardType, CardColor, TriggerIcon, AbilityTemplate};
use weiss_core::env::GameEnv;
use weiss_core::legal::{Decision, DecisionKind, ActionDesc};
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, ChoiceReason, DamageType, ModifierDuration, ModifierKind, PendingTrigger, Phase, StageSlot, StageStatus, TriggerEffect};
use weiss_core::replay::ReplayConfig;

const CARD_BASIC: u32 = 1;
const CARD_EFFECT_ATTACK: u32 = 3;
const CARD_COUNTER_CANCEL: u32 = 4;
const CARD_COUNTER_REDUCE: u32 = 5;
const CARD_CLIMAX: u32 = 6;
const CARD_TRIGGER_MULTI: u32 = 7;
const CARD_END_DRAW: u32 = 8;
const CARD_EVENT_DAMAGE: u32 = 9;
const CARD_MULTI_EFFECT_ATTACK: u32 = 10;
const CARD_HIGH_POWER: u32 = 11;
const CARD_END_DRAW_DOUBLE: u32 = 12;
const CARD_TRIGGER_GATE: u32 = 13;
const CARD_TRIGGER_BOUNCE: u32 = 14;
const CARD_TRIGGER_TREASURE: u32 = 15;
const CARD_TRIGGER_STANDBY: u32 = 16;
const CARD_CANNOT_ATTACK: u32 = 17;
const CARD_COUNTER_DOUBLE_REDUCE: u32 = 18;

fn enable_validate() {
    std::env::set_var("WEISS_VALIDATE_STATE", "1");
}

fn replay_config() -> ReplayConfig {
    ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: std::env::temp_dir(),
        compress: false,
        include_trigger_card_id: true,
    }
}

fn make_db() -> Arc<CardDb> {
    let cards = vec![
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
            abilities: vec![AbilityTemplate::AutoOnAttackDealDamage { amount: 2, cancelable: true }],
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
            counter_timing: true,
            raw_text: None,
        },
        CardStatic {
            id: CARD_CLIMAX,
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
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_MULTI,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Soul, TriggerIcon::Draw],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_END_DRAW,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::AutoEndPhaseDraw { count: 1 }],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_EVENT_DAMAGE,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::EventDealDamage { amount: 2, cancelable: true }],
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
                AbilityTemplate::AutoOnAttackDealDamage { amount: 1, cancelable: true },
                AbilityTemplate::AutoOnAttackDealDamage { amount: 1, cancelable: true },
            ],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_HIGH_POWER,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 9000,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_END_DRAW_DOUBLE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![
                AbilityTemplate::AutoEndPhaseDraw { count: 1 },
                AbilityTemplate::AutoEndPhaseDraw { count: 1 },
            ],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_GATE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Gate],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_BOUNCE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Bounce],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_TREASURE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Treasure],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TRIGGER_STANDBY,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Standby],
            traits: vec![],
            abilities: vec![],
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
            counter_timing: true,
            raw_text: None,
        },
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [100, 101],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
    }
}

fn build_deck_list(size: usize, extras: &[u32]) -> Vec<u32> {
    let mut deck = extras.to_vec();
    while deck.len() < size {
        deck.push(CARD_BASIC);
    }
    deck
}

fn setup_player_state(
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

    for &card in &hand { consume(card, "hand"); }
    for &card in &stock { consume(card, "stock"); }
    for &card in &deck_top { consume(card, "deck_top"); }
    for &card in &clock { consume(card, "clock"); }
    for &card in &level { consume(card, "level"); }
    for &card in &waiting_room { consume(card, "waiting_room"); }
    for &card in &memory { consume(card, "memory"); }
    for &card in &climax { consume(card, "climax"); }
    for &(_, card) in &stage_cards { consume(card, "stage"); }

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

    let p = &mut env.state.players[player];
    p.hand = hand;
    p.stock = stock;
    p.clock = clock;
    p.level = level;
    p.waiting_room = waiting_room;
    p.memory = memory;
    p.climax = climax;
    p.deck = deck;
    p.stage = [StageSlot::empty(), StageSlot::empty(), StageSlot::empty(), StageSlot::empty(), StageSlot::empty()];
    for (slot, card) in stage_cards {
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(card);
        slot_state.status = StageStatus::Stand;
        p.stage[slot] = slot_state;
    }
}

fn force_attack_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Attack;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.decision = Some(Decision { player, kind: DecisionKind::AttackDeclaration, focus_slot: None });
}

fn slot_power_from_obs(obs: &[i32], player_block: usize, slot: usize) -> i32 {
    let base = weiss_core::encode::OBS_HEADER_LEN + player_block * weiss_core::encode::PER_PLAYER_BLOCK_LEN;
    let offset = base + weiss_core::encode::PER_PLAYER_COUNTS + slot * weiss_core::encode::PER_STAGE_SLOT + 3;
    obs[offset]
}

#[test]
fn effect_damage_canceled_by_counter() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC, CARD_COUNTER_CANCEL]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 10, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_EFFECT_ATTACK)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![CARD_COUNTER_CANCEL], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Counter);
    env.apply_action(ActionDesc::CounterPlay { hand_index: 0 }).unwrap();

    let effect_modified = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::DamageModified { damage_type: DamageType::Effect, canceled: true, modified: 0, .. }
    ));
    let effect_committed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::DamageCommitted { damage_type: DamageType::Effect, .. }
    ));
    assert!(effect_modified);
    assert!(!effect_committed);
    assert_eq!(env.state.players[1].clock.len(), 1);
    env.validate_state().unwrap();
}

#[test]
fn effect_damage_reduced_then_applied() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC, CARD_COUNTER_REDUCE]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 11, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_EFFECT_ATTACK)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![CARD_COUNTER_REDUCE], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    env.apply_action(ActionDesc::CounterPlay { hand_index: 0 }).unwrap();

    let effect_modified = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::DamageModified { damage_type: DamageType::Effect, canceled: false, modified: 1, .. }
    ));
    let effect_committed = env.replay_events.iter().filter(|e| matches!(e, ReplayEvent::DamageCommitted { damage_type: DamageType::Effect, .. })).count();
    assert!(effect_modified);
    assert_eq!(effect_committed, 1);
    assert_eq!(env.state.players[1].clock.len(), 2);
    env.validate_state().unwrap();
}

#[test]
fn effect_damage_multiple_reductions_apply_in_order() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC, CARD_COUNTER_DOUBLE_REDUCE]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 27, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_EFFECT_ATTACK)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![CARD_COUNTER_DOUBLE_REDUCE], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    env.apply_action(ActionDesc::CounterPlay { hand_index: 0 }).unwrap();

    let effect_event_id = env.replay_events.iter().find_map(|e| {
        if let ReplayEvent::DamageIntent { event_id, damage_type: DamageType::Effect, .. } = e {
            Some(*event_id)
        } else {
            None
        }
    }).unwrap();

    let applied: Vec<(i32, i32)> = env.replay_events.iter().filter_map(|e| {
        if let ReplayEvent::DamageModifierApplied { event_id, before_amount, after_amount, .. } = e {
            if *event_id == effect_event_id {
                return Some((*before_amount, *after_amount));
            }
        }
        None
    }).collect();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0], (2, 1));
    assert_eq!(applied[1], (1, 0));
    env.validate_state().unwrap();
}

#[test]
fn battle_damage_vs_effect_damage_flags() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 12, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_EFFECT_ATTACK)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();

    let mut has_effect = false;
    let mut has_battle = false;
    for event in &env.replay_events {
        if let ReplayEvent::DamageCommitted { damage_type, .. } = event {
            match damage_type {
                DamageType::Effect => has_effect = true,
                DamageType::Battle => has_battle = true,
            }
        }
    }
    assert!(has_effect);
    assert!(has_battle);
    env.validate_state().unwrap();
}

#[test]
fn reversal_cause_is_recorded_correctly() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 13, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_HIGH_POWER)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();

    let battle_event_id = env.replay_events.iter().filter_map(|e| {
        if let ReplayEvent::DamageCommitted { event_id, damage_type: DamageType::Battle, .. } = e {
            Some(*event_id)
        } else {
            None
        }
    }).last().unwrap();

    let reversal = env.replay_events.iter().find_map(|e| {
        if let ReplayEvent::ReversalCommitted { cause_damage_event, .. } = e {
            Some(*cause_damage_event)
        } else {
            None
        }
    }).unwrap();

    assert_eq!(reversal, Some(battle_event_id));
    env.validate_state().unwrap();
}

#[test]
fn multiple_instances_damage_same_step_ordering() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_MULTI_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 14, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_MULTI_EFFECT_ATTACK)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();

    let intents: Vec<DamageType> = env.replay_events.iter().filter_map(|e| {
        if let ReplayEvent::DamageIntent { damage_type, .. } = e {
            Some(*damage_type)
        } else {
            None
        }
    }).collect();
    assert!(intents.len() >= 3);
    assert_eq!(&intents[0..3], &[DamageType::Effect, DamageType::Effect, DamageType::Battle]);
    env.validate_state().unwrap();
}

#[test]
fn cannot_attack_when_rested() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 15, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.state.players[0].stage[0].status = StageStatus::Rest;
    force_attack_decision(&mut env, 0);

    assert!(env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).is_err());
    env.validate_state().unwrap();
}

#[test]
fn cannot_attack_with_cannot_attack_status() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 160, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.state.players[0].stage[0].cannot_attack = true;
    force_attack_decision(&mut env, 0);

    assert!(env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).is_err());
    env.validate_state().unwrap();
}

#[test]
fn cannot_attack_from_ability_template() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CANNOT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 16, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_CANNOT_ATTACK)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.state.turn.phase = Phase::Climax;
    env.state.turn.active_player = 0;
    env.decision = Some(Decision { player: 0, kind: DecisionKind::Climax, focus_slot: None });

    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let has_attack = env.last_legal_actions.iter().any(|a| matches!(a, ActionDesc::Attack { slot: 0, .. }));
    assert!(!has_attack);
    assert!(env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).is_err());
    env.validate_state().unwrap();
}

#[test]
fn attack_target_must_be_legal_lane() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 17, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    assert!(env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).is_err());
    env.validate_state().unwrap();
}

#[test]
fn attack_cost_must_be_payable() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 18, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.state.players[0].stage[0].attack_cost = 2;
    force_attack_decision(&mut env, 0);
    assert!(env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).is_err());

    let card = env.state.players[0].deck.pop().unwrap();
    env.state.players[0].stock.push(card);
    let card = env.state.players[0].deck.pop().unwrap();
    env.state.players[0].stock.push(card);
    env.state.players[0].stage[0].attack_cost = 2;
    let stock_before = env.state.players[0].stock.len();
    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    let stock_after = env.state.players[0].stock.len();
    assert_eq!(stock_after, stock_before + 1 - 2);
    env.validate_state().unwrap();
}

#[test]
fn cannot_declare_attack_twice_if_once_per_turn() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 19, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::AttackDeclaration);
    assert!(env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).is_err());
    env.validate_state().unwrap();
}

#[test]
fn trigger_orders_when_both_players_trigger() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_END_DRAW]);
    let deck_b = build_deck_list(20, &[CARD_END_DRAW]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 20, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(3, CARD_END_DRAW)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(3, CARD_END_DRAW)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::AttackPass).unwrap();

    let triggers: Vec<u8> = env.replay_events.iter().filter_map(|e| {
        if let ReplayEvent::TriggerResolved { player, effect, .. } = e {
            if matches!(effect, TriggerEffect::EndPhaseDraw { .. }) {
                return Some(*player);
            }
        }
        None
    }).collect();
    assert_eq!(triggers, vec![0, 1]);
    env.validate_state().unwrap();
}

#[test]
fn trigger_order_active_resolves_before_opponent_order() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 23, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.state.turn.pending_triggers = vec![
        PendingTrigger { id: 1, group_id: 42, player: 0, source_card: CARD_BASIC, effect: TriggerEffect::Draw },
        PendingTrigger { id: 2, group_id: 42, player: 1, source_card: CARD_BASIC, effect: TriggerEffect::Draw },
        PendingTrigger { id: 3, group_id: 42, player: 1, source_card: CARD_BASIC, effect: TriggerEffect::Soul },
    ];
    env.state.turn.next_trigger_id = 4;
    env.state.turn.next_trigger_group_id = 43;
    env.state.turn.trigger_order = None;

    env.apply_action(ActionDesc::AttackPass).unwrap();

    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::TriggerOrder);
    assert_eq!(env.decision.as_ref().unwrap().player, 1);
    let resolved_players: Vec<u8> = env.replay_events.iter().filter_map(|e| {
        if let ReplayEvent::TriggerResolved { player, .. } = e {
            Some(*player)
        } else {
            None
        }
    }).collect();
    assert_eq!(resolved_players, vec![0]);
    assert_eq!(env.state.turn.pending_triggers.len(), 2);
    assert!(env.state.turn.pending_triggers.iter().all(|t| t.player == 1));
    env.validate_state().unwrap();
}

#[test]
fn player_orders_own_simultaneous_triggers_decision() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC, CARD_TRIGGER_MULTI]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = true;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 21, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![CARD_TRIGGER_MULTI], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::TriggerOrder);

    env.apply_action(ActionDesc::TriggerOrder { index: 1 }).unwrap();

    let resolved: Vec<TriggerEffect> = env.replay_events.iter().filter_map(|e| {
        if let ReplayEvent::TriggerResolved { effect, .. } = e {
            Some(*effect)
        } else {
            None
        }
    }).collect();
    assert!(resolved.len() >= 2);
    assert_eq!(resolved[0], TriggerEffect::Draw);
    env.validate_state().unwrap();
}

#[test]
fn trigger_source_leaves_play_last_known_info() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_END_DRAW_DOUBLE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 22, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(3, CARD_END_DRAW_DOUBLE)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::AttackPass).unwrap();

    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::TriggerOrder);
    // Remove source card before resolving triggers.
    let card = env.state.players[0].stage[3].card.take().unwrap();
    env.state.players[0].waiting_room.push(card);

    env.apply_action(ActionDesc::TriggerOrder { index: 0 }).unwrap();

    let canceled = env.replay_events.iter().any(|e| matches!(e, ReplayEvent::TriggerCanceled { .. }));
    assert!(!canceled);
    env.validate_state().unwrap();
}

#[test]
fn trigger_gate_choice_skipped_no_candidates() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_GATE, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 30, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![CARD_TRIGGER_GATE], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();

    let skipped = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ChoiceSkipped { reason: ChoiceReason::TriggerGateSelect, .. }
    ));
    assert!(skipped);
    assert!(env.state.players[0].hand.is_empty());
    env.validate_state().unwrap();
}

#[test]
fn trigger_gate_choice_autopicked_single_candidate() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_GATE, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 31, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![CARD_TRIGGER_GATE], vec![], vec![], vec![CARD_BASIC], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();

    let autopicked = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ChoiceAutopicked { option, .. } if option.card_id == CARD_BASIC
    ));
    assert!(autopicked);
    assert_eq!(env.state.players[0].hand.len(), 1);
    assert!(env.state.players[0].hand.contains(&CARD_BASIC));
    env.validate_state().unwrap();
}

#[test]
fn trigger_gate_choice_manual_multiple_candidates() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_GATE, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 32, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![CARD_TRIGGER_GATE], vec![], vec![], vec![CARD_BASIC, CARD_HIGH_POWER], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 }).unwrap();

    let presented = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ChoicePresented { reason: ChoiceReason::TriggerGateSelect, total_candidates, .. } if *total_candidates == 2
    ));
    let made = env.replay_events.iter().any(|e| matches!(e, ReplayEvent::ChoiceMade { .. }));
    assert!(presented);
    assert!(made);
    assert!(env.state.players[0].hand.contains(&CARD_BASIC));
    env.validate_state().unwrap();
}

#[test]
fn trigger_bounce_choice_moves_stage_card() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_BOUNCE, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 33, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC), (1, CARD_HIGH_POWER)], vec![CARD_TRIGGER_BOUNCE], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    env.apply_action(ActionDesc::ChoiceSelect { index: 1 }).unwrap();
    assert!(env.state.players[0].hand.contains(&CARD_HIGH_POWER));
    assert!(env.state.players[0].stage[1].card.is_none());
    env.validate_state().unwrap();
}

#[test]
fn trigger_standby_autopick_moves_waiting_room_card() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_STANDBY, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 34, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC), (1, CARD_BASIC), (2, CARD_BASIC), (3, CARD_BASIC)], vec![CARD_TRIGGER_STANDBY], vec![], vec![], vec![CARD_HIGH_POWER], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();

    let autopicked = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ChoiceAutopicked { .. }
    ));
    assert!(autopicked);
    assert!(env.state.players[0].stage[4].card.is_some());
    assert!(env.state.players[0].stage[4].card == Some(CARD_HIGH_POWER));
    env.validate_state().unwrap();
}

#[test]
fn trigger_treasure_adds_stock() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_TREASURE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 35, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![CARD_TRIGGER_TREASURE, CARD_BASIC], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();

    assert_eq!(env.state.players[0].stock.len(), 2);
    let treasure_resolved = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::TriggerResolved { effect: TriggerEffect::Treasure, .. }
    ));
    assert!(treasure_resolved);
    env.validate_state().unwrap();
}

#[test]
fn reveal_then_move_zone_is_logged_and_correct() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_MULTI]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 23, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![CARD_TRIGGER_MULTI], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();

    let reveal_index = env.replay_events.iter().position(|e| matches!(e, ReplayEvent::Reveal { card, .. } if *card == CARD_TRIGGER_MULTI)).unwrap();
    let trigger_index = env.replay_events.iter().position(|e| matches!(e, ReplayEvent::TriggerQueued { .. }));
    if let Some(trigger_index) = trigger_index {
        assert!(reveal_index < trigger_index);
    }
    assert!(env.state.players[0].stock.contains(&CARD_TRIGGER_MULTI));
    env.validate_state().unwrap();
}

#[test]
fn end_of_turn_expirations_remove_modifiers() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 24, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(3, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.state.players[0].stage[3].power_mod_turn = 1000;
    env.state.players[0].stage[3].cannot_attack = true;
    env.state.players[0].stage[3].attack_cost = 2;
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::AttackPass).unwrap();

    let slot = &env.state.players[0].stage[3];
    assert_eq!(slot.power_mod_turn, 0);
    assert!(!slot.cannot_attack);
    assert_eq!(slot.attack_cost, 0);
    env.validate_state().unwrap();
}

#[test]
fn modifier_until_end_of_turn_expires() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 40, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.add_modifier(CARD_BASIC, 0, 0, ModifierKind::Power, 1000, ModifierDuration::UntilEndOfTurn);

    let mut obs = vec![0; weiss_core::encode::OBS_LEN];
    weiss_core::encode::encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.config.observation_visibility,
        &mut obs,
    );
    assert_eq!(slot_power_from_obs(&obs, 0, 0), 1500);

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    env.apply_action(ActionDesc::AttackPass).unwrap();

    assert!(env.state.modifiers.is_empty());
    let removed = env.replay_events.iter().filter(|e| matches!(e, ReplayEvent::ModifierRemoved { .. })).count();
    assert!(removed >= 1);
    env.validate_state().unwrap();
}

#[test]
fn modifier_while_on_stage_removed_on_leave() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_HIGH_POWER]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    curriculum.enable_encore = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 41, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(0, CARD_BASIC)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![(0, CARD_HIGH_POWER)], vec![], vec![], vec![], vec![], vec![], vec![]);
    env.add_modifier(CARD_BASIC, 0, 0, ModifierKind::Power, 500, ModifierDuration::WhileOnStage);

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    env.apply_action(ActionDesc::AttackPass).unwrap();

    assert!(env.state.modifiers.is_empty());
    let removed = env.replay_events.iter().filter(|e| matches!(e, ReplayEvent::ModifierRemoved { .. })).count();
    assert!(removed >= 1);
    env.validate_state().unwrap();
}

#[test]
fn end_of_turn_triggers_fire_then_state_stabilizes() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_END_DRAW]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, CurriculumConfig::default(), 25, replay_config(), None);

    setup_player_state(&mut env, 0, vec![], vec![], vec![(3, CARD_END_DRAW)], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    force_attack_decision(&mut env, 0);

    let hand_before = env.state.players[0].hand.len();
    env.apply_action(ActionDesc::AttackPass).unwrap();

    let hand_after = env.state.players[0].hand.len();
    assert_eq!(hand_after, hand_before + 1);
    assert!(env.state.turn.pending_triggers.is_empty());
    assert!(env.state.turn.trigger_order.is_none());
    assert!(!env.state.turn.end_phase_pending);
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Clock);
    env.validate_state().unwrap();
}

#[test]
fn effect_damage_from_event_uses_pipeline() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EVENT_DAMAGE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 26, replay_config(), None);

    setup_player_state(&mut env, 0, vec![CARD_EVENT_DAMAGE], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    setup_player_state(&mut env, 1, vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]);

    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.decision = Some(Decision { player: 0, kind: DecisionKind::Main, focus_slot: None });

    env.apply_action(ActionDesc::MainPlayEvent { hand_index: 0 }).unwrap();
    let effect_intent = env.replay_events.iter().any(|e| matches!(e, ReplayEvent::DamageIntent { damage_type: DamageType::Effect, .. }));
    assert!(effect_intent);
    env.validate_state().unwrap();
}
