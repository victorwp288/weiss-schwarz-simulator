use std::sync::{Arc, OnceLock};

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TargetTemplate};
use weiss_core::encode::MAX_DECK;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{
    CardInstance, ChoiceReason, ChoiceZone, ModifierDuration, Phase, StageSlot, StageStatus,
    TargetZone,
};

const CARD_BASIC: u32 = 1;
const CARD_ACT_TARGET_POWER: u32 = 40;
const CARD_CONTINUOUS_SELF_BOUNCE: u32 = 41;

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
            id: CARD_ACT_TARGET_POWER,
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
        CardStatic {
            id: CARD_CONTINUOUS_SELF_BOUNCE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![
                AbilityTemplate::ContinuousPower { amount: 1500 },
                AbilityTemplate::ActivatedTargetedMoveToHand {
                    count: 1,
                    target: TargetTemplate::SelfStage,
                },
            ],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [300, 301],
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
    if deck.len() > size {
        deck.truncate(size);
    }
    extend_with_filler(&mut deck, CARD_BASIC, size);
    extend_with_filler(&mut deck, CARD_BASIC, MAX_DECK);
    deck
}

fn extend_with_filler(deck: &mut Vec<u32>, base_id: u32, target_len: usize) {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut next_clone: HashMap<u32, u32> = HashMap::new();
    for &card_id in deck.iter() {
        *counts.entry(card_id).or_insert(0) += 1;
    }
    while deck.len() < target_len {
        let card_id = assign_id(base_id, &mut counts, &mut next_clone);
        deck.push(card_id);
    }
}

fn assign_id(
    base_id: u32,
    counts: &mut std::collections::HashMap<u32, usize>,
    next_clone: &mut std::collections::HashMap<u32, u32>,
) -> u32 {
    let count = counts.entry(base_id).or_insert(0);
    if *count < 4 {
        *count += 1;
        return base_id;
    }
    loop {
        let idx = next_clone.entry(base_id).or_insert(1);
        if *idx > deck_support::CLONE_GROUPS as u32 {
            panic!(
                "not enough clone ids for base {} (needed clone group {})",
                base_id, idx
            );
        }
        let clone_id = base_id + deck_support::CLONE_OFFSET * *idx;
        let clone_count = counts.entry(clone_id).or_insert(0);
        if *clone_count < 4 {
            *clone_count += 1;
            return clone_id;
        }
        *idx += 1;
    }
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
fn activated_targeting_resolves_via_stack() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_ACT_TARGET_POWER]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 120, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_ACT_TARGET_POWER), (1, CARD_BASIC)],
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
        .expect("target choice");
    assert_eq!(presented.len(), 2);

    env.apply_action(ActionDesc::ChoiceSelect { index: 1 })
        .unwrap();

    let stack_pushed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackPushed { item } if matches!(item.payload.spec.kind, weiss_core::effects::EffectKind::AddModifier { magnitude: 1000, duration: ModifierDuration::UntilEndOfTurn, .. })
            && item.payload.targets.len() == 1
            && item.payload.targets[0].zone == TargetZone::Stage
            && item.payload.targets[0].index == 1
    ));
    assert!(stack_pushed);

    let modifier_added = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ModifierAdded { source, target_slot, magnitude: 1000, duration: ModifierDuration::UntilEndOfTurn, .. }
        if *source == CARD_ACT_TARGET_POWER && *target_slot == 1
    ));
    assert!(modifier_added);
}

#[test]
fn continuous_modifier_applies_and_clears_on_leave() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CONTINUOUS_SELF_BOUNCE]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 121, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![CARD_CONTINUOUS_SELF_BOUNCE],
        vec![],
        vec![],
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

    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();

    let modifier_added = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ModifierAdded { source, target_slot, magnitude: 1500, duration: ModifierDuration::WhileOnStage, .. }
        if *source == CARD_CONTINUOUS_SELF_BOUNCE && *target_slot == 0
    ));
    assert!(modifier_added);

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let modifier_removed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ModifierRemoved { reason, .. } if matches!(reason, weiss_core::events::ModifierRemoveReason::TargetLeftStage)
    ));
    assert!(modifier_removed);
    assert!(env.state.players[0].stage[0].card.is_none());
    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|c| c.id == CARD_CONTINUOUS_SELF_BOUNCE));
}
