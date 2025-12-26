use std::sync::{Arc, OnceLock};

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{
    AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TargetTemplate, TriggerIcon,
};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{
    AttackType, CardInstance, ChoiceReason, ChoiceZone, Phase, StageSlot, StageStatus,
};

const CARD_BASIC: u32 = 1;
const CARD_CONTROL_CHANGE: u32 = 50;
const CARD_BOUNCE: u32 = 51;

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
            id: CARD_CONTROL_CHANGE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedChangeController {
                count: 1,
                target: TargetTemplate::OppFrontRow,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_BOUNCE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
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
        deck_ids: [400, 401],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
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
        slot_state.card = Some(make_instance(card, owner, 9, slot));
        slot_state.status = StageStatus::Stand;
        p.stage[slot] = slot_state;
    }
}

fn force_main_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Main;
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

fn force_attack_decision(env: &mut GameEnv, player: u8) {
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
        kind: DecisionKind::AttackDeclaration,
        focus_slot: None,
    });
}

#[test]
fn control_change_allows_attack() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CONTROL_CHANGE]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 130, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(1, CARD_CONTROL_CHANGE)],
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
        vec![(0, CARD_BASIC)],
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

    let moved = env.state.players[0].stage[0].card.expect("controlled card");
    assert_eq!(moved.id, CARD_BASIC);
    assert_eq!(moved.owner, 1);
    assert_eq!(moved.controller, 0);
    assert!(env.state.players[1].stage[0].card.is_none());

    let control_logged = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ControlChanged { card, owner, from_controller, to_controller, from_slot: 0, to_slot: 0 }
        if *card == CARD_BASIC && *owner == 1 && *from_controller == 1 && *to_controller == 0
    ));
    assert!(control_logged);

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
}

#[test]
fn control_persists_after_leaving_stage() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CONTROL_CHANGE, CARD_BOUNCE]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 131, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(1, CARD_CONTROL_CHANGE), (2, CARD_BASIC)],
        vec![CARD_BOUNCE],
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
        vec![(0, CARD_BASIC)],
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
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Climax);
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Attack {
        slot: 2,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);
    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    let controlled_in_hand = env.state.players[0]
        .hand
        .iter()
        .find(|c| c.id == CARD_BASIC && c.owner == 1)
        .map(|c| c.controller)
        .unwrap_or(1);
    assert_eq!(controlled_in_hand, 0);
}
