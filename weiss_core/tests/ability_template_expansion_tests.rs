use std::sync::Arc;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{
    AbilityCost, AbilityDef, AbilityKind, AbilityTemplate, AbilityTiming, CardColor, CardDb,
    CardStatic, CardType, EffectTemplate, TargetTemplate,
};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::state::{AttackType, CardInstance, ChoiceReason, Phase, StageSlot, StageStatus};

const CARD_TEMPLATE_STOCK_CHARGE: u32 = 20;
const CARD_TEMPLATE_MILL_TOP: u32 = 21;
const CARD_TEMPLATE_HEAL: u32 = 22;
const CARD_TEMPLATE_ON_REVERSE_DRAW: u32 = 23;

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
    cards[0].abilities = vec![AbilityTemplate::AutoOnPlaySalvage {
        count: 1,
        optional: false,
        card_type: Some(CardType::Character),
    }];
    cards[1].abilities = vec![AbilityTemplate::AutoOnPlaySearchDeckTop {
        count: 2,
        optional: false,
        card_type: Some(CardType::Character),
    }];
    cards[2].abilities = vec![AbilityTemplate::AutoOnPlayRevealDeckTop { count: 2 }];
    cards[3].abilities = vec![
        AbilityTemplate::ActivatedTargetedPower {
            amount: 500,
            count: 1,
            target: TargetTemplate::SelfStage,
        },
        AbilityTemplate::ActivatedPaidTargetedPower {
            cost: 1,
            amount: 1000,
            count: 1,
            target: TargetTemplate::SelfStage,
        },
    ];
    cards[9].ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::OnPlay),
        effects: vec![EffectTemplate::StockCharge { count: 2 }],
        targets: vec![],
        cost: AbilityCost::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_limit: None,
    }];
    cards.push(CardStatic {
        id: CARD_TEMPLATE_STOCK_CHARGE,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Blue,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![AbilityTemplate::AutoOnPlayStockCharge { count: 2 }],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    });
    cards.push(CardStatic {
        id: CARD_TEMPLATE_MILL_TOP,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Green,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![AbilityTemplate::AutoOnPlayMillTop { count: 2 }],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    });
    cards.push(CardStatic {
        id: CARD_TEMPLATE_HEAL,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Yellow,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![AbilityTemplate::AutoOnPlayHeal { count: 1 }],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    });
    cards.push(CardStatic {
        id: CARD_TEMPLATE_ON_REVERSE_DRAW,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: vec![],
        traits: vec![],
        abilities: vec![AbilityTemplate::AutoOnReverseDraw { count: 1 }],
        ability_defs: vec![],
        counter_timing: false,
        raw_text: None,
    });
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_deck_list() -> Vec<u32> {
    let mut deck = Vec::new();
    for id in 1..=13u32 {
        for _ in 0..4 {
            deck.push(id);
        }
    }
    deck.truncate(50);
    deck
}

fn make_deck_with_card(card_id: u32) -> Vec<u32> {
    let mut deck = make_deck_list();
    for offset in 0..4 {
        let slot = deck.len() - 1 - offset;
        deck[slot] = card_id;
    }
    deck
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [1, 2],
        max_decisions: 200,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}

#[allow(clippy::too_many_arguments)]
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
        slot_state.card = Some(make_instance(card, owner, 9, slot));
        slot_state.status = StageStatus::Stand;
        p.stage[slot] = slot_state;
    }
}

fn set_main_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player,
        kind: DecisionKind::Main,
        focus_slot: None,
    });
}

fn set_attack_decision(env: &mut GameEnv, player: u8) {
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

#[test]
fn auto_on_play_salvage_moves_waiting_room_card() {
    let db = make_db();
    let deck = make_deck_list();
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        1,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![1],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![5],
        vec![],
        vec![],
    );

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    assert!(env.state.players[0].hand.iter().any(|c| c.id == 5));
    assert!(!env.state.players[0].waiting_room.iter().any(|c| c.id == 5));
}

#[test]
fn auto_on_play_search_deck_top_limits_candidates() {
    let db = make_db();
    let deck = make_deck_list();
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        2,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![2],
        vec![],
        vec![],
        vec![7, 8],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let top_before: Vec<u32> = env.state.players[0]
        .deck
        .iter()
        .rev()
        .take(2)
        .map(|c| c.id)
        .collect();
    let deck_len_before = env.state.players[0].deck.len();

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    let choice = env.state.turn.choice.as_ref().expect("choice present");
    assert_eq!(choice.reason, ChoiceReason::TargetSelect);
    assert_eq!(choice.total_candidates, 2);

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    assert_eq!(env.state.players[0].deck.len(), deck_len_before - 1);
    let hand_ids: Vec<u32> = env.state.players[0].hand.iter().map(|c| c.id).collect();
    assert!(hand_ids.iter().any(|id| top_before.contains(id)));
}

#[test]
fn auto_on_play_reveal_deck_top_logs_reveal_events() {
    let db = make_db();
    let deck = make_deck_list();
    let config = make_config(deck.clone(), deck);
    let replay_config = weiss_core::replay::ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        3,
        replay_config,
        None,
        0,
    );
    env.recording = true;
    env.replay_events.clear();

    setup_player_state(
        &mut env,
        0,
        vec![3],
        vec![],
        vec![],
        vec![9, 10],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    let reveals = env.replay_events.iter().filter(|e| {
        matches!(
            e,
            weiss_core::replay::ReplayEvent::Reveal {
                reason: weiss_core::events::RevealReason::AbilityEffect,
                ..
            }
        )
    });
    assert!(reveals.count() >= 1);
}

#[test]
fn paid_activated_ability_requires_stock() {
    let db = make_db();
    let deck = make_deck_list();
    let config = make_config(deck.clone(), deck);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        enable_activated_abilities: true,
        priority_autopick_single_action: false,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 4, Default::default(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, 4)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    env.curriculum.enable_priority_windows = true;
    env.curriculum.priority_allow_pass = true;
    env.curriculum.strict_priority_mode = false;
    env.curriculum.priority_autopick_single_action = false;
    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::Pass).unwrap();

    assert!(env.state.turn.priority.is_some(), "priority window missing");
    let choice = env.state.turn.choice.as_ref().expect("priority choice");
    let mut has_free = false;
    let mut has_paid = false;
    for opt in &choice.options {
        if opt.zone == weiss_core::state::ChoiceZone::PriorityAct {
            if opt.target_slot == Some(0) {
                has_free = true;
            }
            if opt.target_slot == Some(1) {
                has_paid = true;
            }
        }
    }
    assert!(has_free);
    assert!(!has_paid);

    let mut env = GameEnv::new(
        env.db.clone(),
        make_config(make_deck_list(), make_deck_list()),
        CurriculumConfig {
            enable_priority_windows: true,
            enable_activated_abilities: true,
            priority_autopick_single_action: false,
            ..Default::default()
        },
        5,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![1],
        vec![(0, 4)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    env.curriculum.enable_priority_windows = true;
    env.curriculum.priority_allow_pass = true;
    env.curriculum.strict_priority_mode = false;
    env.curriculum.priority_autopick_single_action = false;
    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::Pass).unwrap();

    assert!(env.state.turn.priority.is_some(), "priority window missing");
    let choice = env.state.turn.choice.as_ref().expect("priority choice");
    let mut has_free = false;
    let mut has_paid = false;
    for opt in &choice.options {
        if opt.zone == weiss_core::state::ChoiceZone::PriorityAct {
            if opt.target_slot == Some(0) {
                has_free = true;
            }
            if opt.target_slot == Some(1) {
                has_paid = true;
            }
        }
    }
    assert!(has_free);
    assert!(has_paid);
}

#[test]
fn ability_def_on_play_stock_charge_moves_cards() {
    let db = make_db();
    let deck = make_deck_list();
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        6,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![10],
        vec![],
        vec![],
        vec![1],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let top_before: Vec<u32> = env.state.players[0]
        .deck
        .iter()
        .rev()
        .take(2)
        .map(|c| c.id)
        .collect();
    let stock_len_before = env.state.players[0].stock.len();
    let deck_len_before = env.state.players[0].deck.len();

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    assert_eq!(env.state.players[0].stock.len(), stock_len_before + 2);
    assert_eq!(env.state.players[0].deck.len(), deck_len_before - 2);
    let stock_ids: Vec<u32> = env.state.players[0].stock.iter().map(|c| c.id).collect();
    assert_eq!(&stock_ids[stock_ids.len() - 2..], top_before.as_slice());
}

#[test]
fn auto_on_play_stock_charge_template_moves_cards() {
    let db = make_db();
    let deck = make_deck_with_card(CARD_TEMPLATE_STOCK_CHARGE);
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        7,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![CARD_TEMPLATE_STOCK_CHARGE],
        vec![],
        vec![],
        vec![1, 1],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let stock_len_before = env.state.players[0].stock.len();

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    assert_eq!(env.state.players[0].stock.len(), stock_len_before + 2);
    env.validate_state().unwrap();
}

#[test]
fn auto_on_play_mill_top_template_moves_cards() {
    let db = make_db();
    let deck = make_deck_with_card(CARD_TEMPLATE_MILL_TOP);
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        8,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![CARD_TEMPLATE_MILL_TOP],
        vec![],
        vec![],
        vec![2, 3],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    let wr_ids: Vec<u32> = env.state.players[0]
        .waiting_room
        .iter()
        .map(|c| c.id)
        .collect();
    assert!(wr_ids.contains(&2));
    assert!(wr_ids.contains(&3));
    env.validate_state().unwrap();
}

#[test]
fn auto_on_play_heal_template_moves_clock() {
    let db = make_db();
    let deck = make_deck_with_card(CARD_TEMPLATE_HEAL);
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        9,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![CARD_TEMPLATE_HEAL],
        vec![],
        vec![],
        vec![],
        vec![1],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let clock_before = env.state.players[0].clock.len();
    let wr_before = env.state.players[0].waiting_room.len();

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    assert_eq!(env.state.players[0].clock.len(), clock_before - 1);
    assert_eq!(env.state.players[0].waiting_room.len(), wr_before + 1);
    env.validate_state().unwrap();
}

#[test]
fn auto_on_reverse_draw_triggers_on_reversal() {
    let db = make_db();
    let deck = make_deck_with_card(CARD_TEMPLATE_ON_REVERSE_DRAW);
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        10,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, 1)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    setup_player_state(
        &mut env,
        1,
        vec![],
        vec![],
        vec![(0, CARD_TEMPLATE_ON_REVERSE_DRAW)],
        vec![1],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    env.curriculum.enable_counters = false;
    env.curriculum.enable_priority_windows = false;
    set_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();

    assert_eq!(env.state.players[1].hand.len(), 1);
    env.validate_state().unwrap();
}

#[test]
fn auto_on_reverse_draw_respects_curriculum_toggle() {
    let db = make_db();
    let deck = make_deck_with_card(CARD_TEMPLATE_ON_REVERSE_DRAW);
    let config = make_config(deck.clone(), deck);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig {
            enable_on_reverse_triggers: false,
            ..Default::default()
        },
        11,
        Default::default(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, 1)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    setup_player_state(
        &mut env,
        1,
        vec![],
        vec![],
        vec![(0, CARD_TEMPLATE_ON_REVERSE_DRAW)],
        vec![1],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );

    env.curriculum.enable_counters = false;
    env.curriculum.enable_priority_windows = false;
    set_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();

    assert_eq!(env.state.players[1].hand.len(), 0);
    env.validate_state().unwrap();
}
