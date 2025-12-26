use std::sync::{Arc, OnceLock};

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TargetTemplate};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{CardInstance, ChoiceReason, ChoiceZone, Phase, StageSlot, StageStatus};

const CARD_BASIC: u32 = 1;
const CARD_TARGET_OPP_FRONT: u32 = 30;
const CARD_TARGET_OPP_STAGE: u32 = 33;
const CARD_TARGET_OPP_BACK: u32 = 34;
const CARD_TARGET_WR_MULTI: u32 = 31;
const CARD_TARGET_WR_TRUNC: u32 = 32;

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}

fn enable_validate() {
    static VALIDATE_ONCE: OnceLock<()> = OnceLock::new();
    VALIDATE_ONCE.get_or_init(|| {
        std::env::set_var("WEISS_VALIDATE_STATE", "1");
    });
}

fn replay_config() -> ReplayConfig {
    ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: std::env::temp_dir(),
        compress: false,
        include_trigger_card_id: true,
        ..Default::default()
    }
}

fn make_db() -> Arc<CardDb> {
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
            id: CARD_TARGET_OPP_FRONT,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedPower {
                amount: 500,
                count: 1,
                target: TargetTemplate::OppFrontRow,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TARGET_OPP_STAGE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedPower {
                amount: 500,
                count: 1,
                target: TargetTemplate::OppStage,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TARGET_OPP_BACK,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedPower {
                amount: 500,
                count: 1,
                target: TargetTemplate::OppBackRow,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TARGET_WR_MULTI,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedMoveToHand {
                count: 2,
                target: TargetTemplate::SelfWaitingRoom,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_TARGET_WR_TRUNC,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedMoveToHand {
                count: 1,
                target: TargetTemplate::SelfWaitingRoom,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
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
        deck_ids: [200, 201],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn build_deck_list(size: usize, extras: &[u32]) -> Vec<u32> {
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
        slot_state.card = Some(make_instance(card, owner, 4, slot));
        slot_state.status = StageStatus::Stand;
        p.stage[slot] = slot_state;
    }
}

fn force_main_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.target_selection = None;
    env.state.turn.priority = None;
    env.state.turn.stack.clear();
    env.state.turn.pending_stack_groups.clear();
    env.state.turn.stack_order = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.state.turn.main_passed = false;
    env.decision = Some(Decision {
        player,
        kind: DecisionKind::Main,
        focus_slot: None,
    });
}

fn choose_priority_activation(env: &mut GameEnv) {
    if let Some(choice) = env.state.turn.choice.as_ref() {
        if choice.reason == ChoiceReason::PriorityActionSelect {
            let idx = choice
                .options
                .iter()
                .enumerate()
                .filter(|(_, opt)| opt.zone == ChoiceZone::PriorityAct)
                .min_by_key(|(_, opt)| {
                    (
                        opt.index.unwrap_or(u8::MAX),
                        opt.target_slot.unwrap_or(u8::MAX),
                    )
                })
                .map(|(idx, _)| idx)
                .expect("priority activation");
            env.apply_action(ActionDesc::ChoiceSelect { index: idx as u8 })
                .unwrap();
        }
    }
}

#[test]
fn target_opponent_front_row_ordering() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TARGET_OPP_FRONT]);
    let deck_b = build_deck_list(
        20,
        &[
            CARD_BASIC,
            CARD_BASIC + deck_support::CLONE_OFFSET,
            CARD_BASIC + deck_support::CLONE_OFFSET * 2,
            CARD_BASIC + deck_support::CLONE_OFFSET * 3,
            CARD_BASIC + deck_support::CLONE_OFFSET * 4,
        ],
    );
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 99, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_TARGET_OPP_FRONT)],
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
        vec![
            (0, CARD_BASIC),
            (1, CARD_BASIC),
            (2, CARD_BASIC),
            (3, CARD_BASIC),
        ],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let presented = env
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } => Some(options.clone()),
            _ => None,
        })
        .expect("target selection choice");

    let indices: Vec<u8> = presented
        .iter()
        .map(|opt| opt.reference.index.unwrap())
        .collect();
    assert_eq!(indices, vec![0, 1, 2]);
    assert!(presented
        .iter()
        .all(|opt| opt.reference.zone == ChoiceZone::Stage));

    env.apply_action(ActionDesc::ChoiceSelect { index: 1 })
        .unwrap();

    let applied = env.state.modifiers.iter().any(|m| {
        m.source == CARD_TARGET_OPP_FRONT
            && m.target_player == 1
            && m.target_slot == 1
            && m.magnitude == 500
    });
    assert!(applied);
}

#[test]
fn target_opponent_stage_ordering() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TARGET_OPP_STAGE]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 101, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_TARGET_OPP_STAGE)],
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
        vec![
            (0, CARD_BASIC),
            (1, CARD_BASIC + deck_support::CLONE_OFFSET),
            (2, CARD_BASIC + deck_support::CLONE_OFFSET * 2),
            (3, CARD_BASIC + deck_support::CLONE_OFFSET * 3),
            (4, CARD_BASIC + deck_support::CLONE_OFFSET * 4),
        ],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let presented = env
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } => Some(options.clone()),
            _ => None,
        })
        .expect("target selection choice");

    let indices: Vec<u8> = presented
        .iter()
        .map(|opt| opt.reference.index.unwrap())
        .collect();
    assert_eq!(indices, vec![0, 1, 2, 3, 4]);
    assert!(presented
        .iter()
        .all(|opt| opt.reference.zone == ChoiceZone::Stage));
}

#[test]
fn target_opponent_back_row_ordering() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TARGET_OPP_BACK]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 102, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_TARGET_OPP_BACK)],
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
        vec![(1, CARD_BASIC), (3, CARD_BASIC), (4, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let presented = env
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } => Some(options.clone()),
            _ => None,
        })
        .expect("target selection choice");

    let indices: Vec<u8> = presented
        .iter()
        .map(|opt| opt.reference.index.unwrap())
        .collect();
    assert_eq!(indices, vec![3, 4]);
    assert!(presented
        .iter()
        .all(|opt| opt.reference.zone == ChoiceZone::Stage));
}

#[test]
fn multi_target_selection_no_duplicates() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TARGET_WR_MULTI]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 100, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_TARGET_WR_MULTI)],
        vec![],
        vec![],
        vec![],
        vec![CARD_BASIC, CARD_BASIC, CARD_BASIC],
        vec![],
        vec![],
    );
    setup_player_state(
        &mut env,
        1,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);
    env.apply_action(ActionDesc::ChoiceSelect { index: 1 })
        .unwrap();

    let second_presented = env
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } => Some(options.clone()),
            _ => None,
        })
        .expect("second target choice");

    let indices: Vec<u8> = second_presented
        .iter()
        .map(|opt| opt.reference.index.unwrap())
        .collect();
    assert_eq!(indices, vec![0, 2]);

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    assert_eq!(env.state.players[0].hand.len(), 2);
    assert_eq!(env.state.players[0].waiting_room.len(), 1);
}

#[test]
fn target_choice_truncation_metadata() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(30, &[CARD_TARGET_WR_TRUNC]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 101, replay_config(), None, 0);

    let waiting_room: Vec<u32> = env.config.deck_lists[0]
        .iter()
        .copied()
        .filter(|id| *id != CARD_TARGET_WR_TRUNC)
        .take(18)
        .collect();
    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_TARGET_WR_TRUNC)],
        vec![],
        vec![],
        vec![],
        waiting_room,
        vec![],
        vec![],
    );
    setup_player_state(
        &mut env,
        1,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let (options, total) = env
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                total_candidates,
                ..
            } => Some((options.clone(), *total_candidates)),
            _ => None,
        })
        .expect("truncation choice");

    assert_eq!(total, 18);
    assert_eq!(options.len(), 16);
    let indices: Vec<u8> = options
        .iter()
        .map(|opt| opt.reference.index.unwrap())
        .collect();
    assert_eq!(indices, (0u8..16u8).collect::<Vec<u8>>());
}
