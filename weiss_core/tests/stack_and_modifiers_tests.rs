use std::sync::{Arc, OnceLock};

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TargetTemplate};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{
    CardInstance, ChoiceReason, ModifierDuration, Phase, StageSlot, StageStatus, TargetZone,
};

const CARD_BASIC: u32 = 1;
const CARD_ACT_TARGET_POWER: u32 = 40;
const CARD_CONTINUOUS_SELF_BOUNCE: u32 = 41;

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
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.stock = stock
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.clock = clock
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.level = level
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.waiting_room = waiting_room
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.memory = memory
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.climax = climax
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
        .collect();
    p.deck = deck
        .into_iter()
        .map(|id| CardInstance::new(id, owner))
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
        slot_state.card = Some(CardInstance::new(card, owner));
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

#[test]
fn activated_targeting_resolves_via_stack() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_ACT_TARGET_POWER]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig::default();
    let mut env = GameEnv::new(db, config, curriculum, 120, replay_config(), None);

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

    env.apply_action(ActionDesc::MainPass).unwrap();

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
    let curriculum = CurriculumConfig::default();
    let mut env = GameEnv::new(db, config, curriculum, 121, replay_config(), None);

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

    env.apply_action(ActionDesc::MainPass).unwrap();

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
