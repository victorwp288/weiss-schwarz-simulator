use super::*;
use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
    SimultaneousLossPolicy,
};
use crate::db::{
    AbilityCost, AbilityDef, AbilityKind, AbilityTiming, CardColor, CardDb, CardId, CardStatic,
    CardType, EffectTemplate,
};
use crate::effects::{EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec};
use crate::encode::{
    encode_observation, OBS_CONTEXT_BASE, OBS_CONTEXT_CHOICE_ACTIVE, OBS_CONTEXT_ENCORE_PENDING,
    OBS_CONTEXT_LEN, OBS_CONTEXT_PRIORITY_WINDOW, OBS_CONTEXT_STACK_NONEMPTY, OBS_HEADER_LEN,
    OBS_LEN, OBS_REVEAL_BASE, OBS_REVEAL_LEN, PER_PLAYER_BLOCK_LEN, PER_PLAYER_CLIMAX_TOP,
    PER_PLAYER_CLOCK_TOP, PER_PLAYER_COUNTS, PER_PLAYER_DECK, PER_PLAYER_HAND, PER_PLAYER_LEVEL,
    PER_PLAYER_RESOLUTION_TOP, PER_PLAYER_STAGE, PER_PLAYER_STOCK_TOP, PER_PLAYER_WAITING_TOP,
};
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::fingerprint::{events_fingerprint, state_fingerprint};
use crate::replay::ReplayConfig;
use crate::replay::ReplayEvent;
use crate::state::{
    CardInstance, ChoiceReason, ChoiceState, ChoiceZone, CostStepKind, EncoreRequest,
    PendingTargetEffect, PriorityState, StackItem, StageSlot, StageStatus, TargetSelectionState,
    TargetSide, TargetSlotFilter, TargetSpec, TargetZone, TerminalResult, TimingWindow,
    TriggerEffect, REVEAL_HISTORY_LEN,
};
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
            clone.id = base.id + CLONE_OFFSET * idx as u32;
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
        let clone_id = base_id + CLONE_OFFSET * *idx;
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
    GameEnv::new(
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
    GameEnv::new(
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

#[test]
fn stack_group_ordering_stable() {
    let mut env = make_env();
    let spec_a = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 2, 0, 0),
        kind: EffectKind::Draw { count: 1 },
        target: None,
        optional: false,
    };
    let spec_b = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::Draw { count: 1 },
        target: None,
        optional: false,
    };
    let item_a = StackItem {
        id: 2,
        controller: 0,
        source_id: 2,
        effect_id: spec_a.id,
        payload: EffectPayload {
            spec: spec_a,
            targets: Vec::new(),
        },
    };
    let item_b = StackItem {
        id: 1,
        controller: 0,
        source_id: 1,
        effect_id: spec_b.id,
        payload: EffectPayload {
            spec: spec_b,
            targets: Vec::new(),
        },
    };
    env.enqueue_stack_items(vec![item_a, item_b]);
    let order = env.state.turn.stack_order.as_ref().expect("stack order");
    assert_eq!(order.items[0].source_id, 1);
    assert_eq!(order.items[1].source_id, 2);
}

#[test]
fn target_candidate_ordering_by_zone() {
    let mut env = make_env();
    let p = 0usize;
    let owner = p as u8;
    let mut next_id = 1u32;
    env.state.players[p].hand = vec![
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
        make_instance(1, owner, &mut next_id),
    ];
    env.state.players[p].waiting_room = vec![
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
        make_instance(1, owner, &mut next_id),
    ];
    env.state.players[p].clock = vec![
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
    ];
    env.state.players[p].level = vec![
        make_instance(2, owner, &mut next_id),
        make_instance(1, owner, &mut next_id),
    ];
    env.state.players[p].stock = vec![
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
        make_instance(1, owner, &mut next_id),
    ];
    env.state.players[p].memory = vec![make_instance(1, owner, &mut next_id)];
    env.state.players[p].climax = vec![make_instance(2, owner, &mut next_id)];
    env.state.players[p].resolution = vec![
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
    ];
    env.state.players[p].deck = vec![
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
        make_instance(1, owner, &mut next_id),
        make_instance(2, owner, &mut next_id),
    ];
    env.state.players[p].stage = [
        {
            let mut s = StageSlot::empty();
            s.card = Some(make_instance(1, owner, &mut next_id));
            s
        },
        {
            let mut s = StageSlot::empty();
            s.card = Some(make_instance(2, owner, &mut next_id));
            s
        },
        StageSlot::empty(),
        StageSlot::empty(),
        StageSlot::empty(),
    ];

    let spec = |zone| TargetSpec {
        zone,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 3,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };

    let stage = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Stage), &[]);
    assert_eq!(
        stage.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1]
    );

    let waiting = enumerate_targets_for_test(&env, owner, &spec(TargetZone::WaitingRoom), &[]);
    assert_eq!(
        waiting.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let hand = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Hand), &[]);
    assert_eq!(
        hand.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let deck = enumerate_targets_for_test(&env, owner, &spec(TargetZone::DeckTop), &[]);
    assert_eq!(
        deck.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );

    let clock = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Clock), &[]);
    assert_eq!(
        clock.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1]
    );

    let level = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Level), &[]);
    assert_eq!(
        level.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1]
    );

    let stock = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Stock), &[]);
    assert_eq!(
        stock.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let memory = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Memory), &[]);
    assert_eq!(memory.iter().map(|t| t.index).collect::<Vec<_>>(), vec![0]);

    let climax = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Climax), &[]);
    assert_eq!(climax.iter().map(|t| t.index).collect::<Vec<_>>(), vec![0]);

    let resolution = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Resolution), &[]);
    assert_eq!(
        resolution.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn target_slot_filters_back_row_and_specific_slot() {
    let mut env = make_env();
    let owner = 0u8;
    let mut next_id = 1u32;
    env.state.players[0].stage = [
        {
            let mut s = StageSlot::empty();
            s.card = Some(make_instance(1, owner, &mut next_id));
            s
        },
        {
            let mut s = StageSlot::empty();
            s.card = Some(make_instance(2, owner, &mut next_id));
            s
        },
        StageSlot::empty(),
        {
            let mut s = StageSlot::empty();
            s.card = Some(make_instance(1, owner, &mut next_id));
            s
        },
        {
            let mut s = StageSlot::empty();
            s.card = Some(make_instance(2, owner, &mut next_id));
            s
        },
    ];

    let back_row = TargetSpec {
        zone: TargetZone::Stage,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::BackRow,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 2,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let back_targets = enumerate_targets_for_test(&env, owner, &back_row, &[]);
    assert_eq!(
        back_targets.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![3, 4]
    );

    let specific = TargetSpec {
        zone: TargetZone::Stage,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::SpecificSlot(1),
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let specific_targets = enumerate_targets_for_test(&env, owner, &specific, &[]);
    assert_eq!(
        specific_targets.iter().map(|t| t.index).collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn target_filters_apply_deterministically() {
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
            traits: vec![10],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 2,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 2,
            cost: 1,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![20],
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
            level: 1,
            cost: 2,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![10],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    let mut cards = cards;
    add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db build"));
    let config = EnvConfig {
        deck_lists: [
            legalize_deck(vec![1, 2, 3], &[1, 2, 3]),
            legalize_deck(vec![1, 2, 3], &[1, 2, 3]),
        ],
        deck_ids: [1, 2],
        max_decisions: 200,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::LenientTerminate,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        7,
        ReplayConfig::default(),
        None,
        0,
    );
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    env.state.players[0].waiting_room = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
        make_instance(3, 0, &mut next_id),
    ];
    let spec = TargetSpec {
        zone: TargetZone::WaitingRoom,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::Any,
        card_type: Some(CardType::Character),
        card_trait: Some(10),
        level_max: Some(1),
        cost_max: Some(1),
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let targets = enumerate_targets_for_test(&env, 0, &spec, &[]);
    let ids: Vec<u32> = targets.iter().map(|t| t.card_id).collect();
    assert_eq!(ids, vec![1]);
}

#[test]
fn target_selection_uses_snapshot_candidates() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let top = make_instance(1, 0, &mut next_id);
    let below = make_instance(2, 0, &mut next_id);
    env.state.players[0].deck = vec![below, top];

    let spec = TargetSpec {
        zone: TargetZone::DeckTop,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: Some(2),
        source_only: false,
        reveal_to_controller: false,
    };
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::MoveToHand,
        target: Some(spec.clone()),
        optional: false,
    };
    env.start_target_selection(
        0,
        1,
        spec,
        PendingTargetEffect::EffectPending {
            instance_id: 1,
            payload: EffectPayload {
                spec: effect_spec,
                targets: Vec::new(),
            },
        },
        false,
    );
    let before = env
        .state
        .turn
        .target_selection
        .as_ref()
        .expect("selection")
        .candidates
        .clone();
    env.state.players[0].deck.reverse();
    let after = env
        .state
        .turn
        .target_selection
        .as_ref()
        .expect("selection")
        .candidates
        .clone();
    assert_eq!(before, after);
}

#[test]
fn move_to_waiting_room_from_stage_removes_card() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(card);
    env.state.players[0].stage[0].status = StageStatus::Stand;

    let spec = TargetSpec {
        zone: TargetZone::Stage,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::SpecificSlot(0),
        card_type: Some(CardType::Character),
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let target = enumerate_targets_for_test(&env, 0, &spec, &[])
        .into_iter()
        .next()
        .expect("stage target");
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::MoveToWaitingRoom,
        target: Some(spec),
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: vec![target],
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[0].stage[0].card.is_none());
    assert!(env.state.players[0]
        .waiting_room
        .iter()
        .any(|c| c.instance_id == card.instance_id));
}

#[test]
fn move_to_stock_from_deck_top_moves_top_card() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let top = make_instance(1, 0, &mut next_id);
    let below = make_instance(2, 0, &mut next_id);
    env.state.players[0].deck = vec![below, top];
    env.state.players[0].stock.clear();

    let spec = TargetSpec {
        zone: TargetZone::DeckTop,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: Some(1),
        source_only: false,
        reveal_to_controller: false,
    };
    let target = enumerate_targets_for_test(&env, 0, &spec, &[])
        .into_iter()
        .next()
        .expect("deck target");
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 2, 0, 0),
        kind: EffectKind::MoveToStock,
        target: Some(spec),
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: vec![target],
    };
    env.resolve_effect_payload(0, 2, &payload);

    assert_eq!(env.state.players[0].deck.len(), 1);
    assert_eq!(env.state.players[0].stock.len(), 1);
    assert_eq!(env.state.players[0].stock[0].instance_id, top.instance_id);
}

#[test]
fn move_to_clock_from_hand_moves_card() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].hand = vec![card];
    env.state.players[0].clock.clear();

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let target = enumerate_targets_for_test(&env, 0, &spec, &[])
        .into_iter()
        .next()
        .expect("hand target");
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 3, 0, 0),
        kind: EffectKind::MoveToClock,
        target: Some(spec),
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: vec![target],
    };
    env.resolve_effect_payload(0, 3, &payload);

    assert!(env.state.players[0].hand.is_empty());
    assert_eq!(env.state.players[0].clock.len(), 1);
    assert_eq!(env.state.players[0].clock[0].instance_id, card.instance_id);
}

#[test]
fn rest_and_stand_target_updates_stage_status() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(card);
    env.state.players[0].stage[0].status = StageStatus::Stand;

    let spec = TargetSpec {
        zone: TargetZone::Stage,
        side: TargetSide::SelfSide,
        slot_filter: TargetSlotFilter::SpecificSlot(0),
        card_type: Some(CardType::Character),
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let target = enumerate_targets_for_test(&env, 0, &spec, &[])
        .into_iter()
        .next()
        .expect("stage target");
    let rest_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 4, 0, 0),
        kind: EffectKind::RestTarget,
        target: Some(spec.clone()),
        optional: false,
    };
    let rest_payload = EffectPayload {
        spec: rest_spec,
        targets: vec![target],
    };
    env.resolve_effect_payload(0, 4, &rest_payload);
    assert_eq!(env.state.players[0].stage[0].status, StageStatus::Rest);

    let target = enumerate_targets_for_test(&env, 0, &spec, &[])
        .into_iter()
        .next()
        .expect("stage target");
    let stand_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 5, 0, 0),
        kind: EffectKind::StandTarget,
        target: Some(spec),
        optional: false,
    };
    let stand_payload = EffectPayload {
        spec: stand_spec,
        targets: vec![target],
    };
    env.resolve_effect_payload(0, 5, &stand_payload);
    assert_eq!(env.state.players[0].stage[0].status, StageStatus::Stand);
}

#[test]
fn activated_ability_costs_apply_in_order() {
    let ability_def = AbilityDef {
        kind: AbilityKind::Activated,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        targets: Vec::new(),
        cost: AbilityCost {
            stock: 1,
            rest_self: true,
            rest_other: 0,
            discard_from_hand: 1,
            clock_from_hand: 0,
            clock_from_deck_top: 0,
            reveal_from_hand: 1,
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_limit: None,
    };
    let card = CardStatic {
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
        ability_defs: vec![ability_def],
        counter_timing: false,
        raw_text: None,
    };
    let mut cards = vec![card];
    add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db"));
    let deck = legalize_deck(vec![1u32; 50], &[1]);
    let config = EnvConfig {
        deck_lists: [deck.clone(), deck],
        deck_ids: [1, 2],
        max_decisions: 50,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        5,
        ReplayConfig::default(),
        None,
        0,
    );
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    let stage_card = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.players[0].stage[0].status = StageStatus::Stand;
    env.state.players[0].stock = vec![make_instance(1, 0, &mut next_id)];
    let hand_a = make_instance(1, 0, &mut next_id);
    let hand_b = make_instance(1, 0, &mut next_id);
    let hand_c = make_instance(1, 0, &mut next_id);
    env.state.players[0].hand = vec![hand_a, hand_b, hand_c];

    let pending = env
        .queue_activated_ability_stack_item(0, 0, 0)
        .expect("activate ability");
    assert!(pending);
    assert_eq!(env.state.players[0].stage[0].status, StageStatus::Rest);
    assert!(env.state.players[0].stock.is_empty());
    let pending_cost = env.state.turn.pending_cost.as_ref().expect("pending cost");
    assert_eq!(
        pending_cost.current_step,
        Some(CostStepKind::DiscardFromHand)
    );

    let choice = env.state.turn.choice.take().expect("cost choice");
    assert_eq!(choice.reason, ChoiceReason::CostPayment);
    let option = choice.options[0];
    env.recycle_choice_options(choice.options);
    env.apply_choice_effect(choice.reason, choice.player, option, choice.pending_trigger);

    let pending_cost = env.state.turn.pending_cost.as_ref().expect("pending cost");
    assert_eq!(
        pending_cost.current_step,
        Some(CostStepKind::RevealFromHand)
    );
    assert_eq!(env.state.players[0].hand.len(), 2);

    let choice = env.state.turn.choice.take().expect("reveal choice");
    let option = choice.options[0];
    env.recycle_choice_options(choice.options);
    env.apply_choice_effect(choice.reason, choice.player, option, choice.pending_trigger);

    assert!(env.state.turn.pending_cost.is_none());
    assert!(!env.state.turn.stack.is_empty());
}

#[test]
fn random_discard_is_deterministic() {
    let mut env_a = make_env();
    let mut env_b = make_env();
    let _ = env_a.reset_no_copy();
    let _ = env_b.reset_no_copy();
    let mut next_id = 1u32;
    let cards = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
        make_instance(3, 0, &mut next_id),
    ];
    env_a.state.players[0].hand = cards.clone();
    env_b.state.players[0].hand = cards;
    env_a.state.players[0].waiting_room.clear();
    env_b.state.players[0].waiting_room.clear();

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::RandomDiscardFromHand {
            target: TargetSide::SelfSide,
            count: 2,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec.clone(),
        targets: Vec::new(),
    };
    env_a.resolve_effect_payload(0, 1, &payload);
    env_b.resolve_effect_payload(0, 1, &payload);

    let hand_a: Vec<u32> = env_a.state.players[0]
        .hand
        .iter()
        .map(|c| c.instance_id)
        .collect();
    let hand_b: Vec<u32> = env_b.state.players[0]
        .hand
        .iter()
        .map(|c| c.instance_id)
        .collect();
    assert_eq!(hand_a, hand_b);
    let wr_a: Vec<u32> = env_a.state.players[0]
        .waiting_room
        .iter()
        .map(|c| c.instance_id)
        .collect();
    let wr_b: Vec<u32> = env_b.state.players[0]
        .waiting_room
        .iter()
        .map(|c| c.instance_id)
        .collect();
    assert_eq!(wr_a, wr_b);
}

#[test]
fn random_mill_is_deterministic() {
    let mut env_a = make_env();
    let mut env_b = make_env();
    let _ = env_a.reset_no_copy();
    let _ = env_b.reset_no_copy();
    let mut next_id = 1u32;
    let deck = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
        make_instance(3, 0, &mut next_id),
    ];
    env_a.state.players[0].deck = deck.clone();
    env_b.state.players[0].deck = deck;
    env_a.state.players[0].waiting_room.clear();
    env_b.state.players[0].waiting_room.clear();

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 2, 0, 0),
        kind: EffectKind::RandomMill {
            target: TargetSide::SelfSide,
            count: 2,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: Vec::new(),
    };
    env_a.resolve_effect_payload(0, 2, &payload);
    env_b.resolve_effect_payload(0, 2, &payload);

    let deck_a: Vec<u32> = env_a.state.players[0]
        .deck
        .iter()
        .map(|c| c.instance_id)
        .collect();
    let deck_b: Vec<u32> = env_b.state.players[0]
        .deck
        .iter()
        .map(|c| c.instance_id)
        .collect();
    assert_eq!(deck_a, deck_b);
    let wr_a: Vec<u32> = env_a.state.players[0]
        .waiting_room
        .iter()
        .map(|c| c.instance_id)
        .collect();
    let wr_b: Vec<u32> = env_b.state.players[0]
        .waiting_room
        .iter()
        .map(|c| c.instance_id)
        .collect();
    assert_eq!(wr_a, wr_b);
}

#[test]
fn heal_moves_clock_to_waiting_room() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].clock = vec![card];
    env.state.players[0].waiting_room.clear();

    let target = TargetRef {
        player: 0,
        zone: TargetZone::Clock,
        index: 0,
        card_id: card.id,
        instance_id: card.instance_id,
    };
    let spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 10, 0, 0),
        kind: EffectKind::Heal,
        target: Some(TargetSpec {
            side: TargetSide::SelfSide,
            zone: TargetZone::Clock,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            card_trait: None,
            level_max: None,
            cost_max: None,
            count: 1,
            limit: None,
            source_only: false,
            reveal_to_controller: false,
        }),
        optional: false,
    };
    let payload = EffectPayload {
        spec,
        targets: vec![target],
    };
    env.resolve_effect_payload(0, 10, &payload);
    assert!(env.state.players[0].clock.is_empty());
    assert_eq!(env.state.players[0].waiting_room.len(), 1);
    assert_eq!(
        env.state.players[0].waiting_room[0].instance_id,
        card.instance_id
    );
}

#[test]
fn mill_top_moves_cards_to_waiting_room_in_order() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let a = make_instance(1, 0, &mut next_id);
    let b = make_instance(2, 0, &mut next_id);
    let c = make_instance(1, 0, &mut next_id);
    env.state.players[0].deck = vec![a, b, c];
    env.state.players[0].waiting_room.clear();

    let spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 11, 0, 0),
        kind: EffectKind::MillTop {
            target: TargetSide::SelfSide,
            count: 2,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec,
        targets: Vec::new(),
    };
    env.resolve_effect_payload(0, 11, &payload);
    assert_eq!(env.state.players[0].waiting_room.len(), 2);
    assert_eq!(
        env.state.players[0].waiting_room[0].instance_id,
        c.instance_id
    );
    assert_eq!(
        env.state.players[0].waiting_room[1].instance_id,
        b.instance_id
    );
}

#[test]
fn swap_stage_slots_effect_swaps_cards() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let a = make_instance(1, 0, &mut next_id);
    let b = make_instance(2, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(a);
    env.state.players[0].stage[1].card = Some(b);

    let targets = vec![
        TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: a.id,
            instance_id: a.instance_id,
        },
        TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 1,
            card_id: b.id,
            instance_id: b.instance_id,
        },
    ];
    let spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 12, 0, 0),
        kind: EffectKind::SwapStageSlots,
        target: None,
        optional: false,
    };
    let payload = EffectPayload { spec, targets };
    env.resolve_effect_payload(0, 12, &payload);
    assert_eq!(
        env.state.players[0].stage[0].card.unwrap().instance_id,
        b.instance_id
    );
    assert_eq!(
        env.state.players[0].stage[1].card.unwrap().instance_id,
        a.instance_id
    );
}

#[test]
fn reveal_zone_top_logs_reveal_event() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.recording = true;
    env.replay_events.clear();
    let mut next_id = 1u32;
    let card = make_instance(1, 0, &mut next_id);
    env.state.players[0].hand = vec![card];

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 3, 0, 0),
        kind: EffectKind::RevealZoneTop {
            target: TargetSide::SelfSide,
            zone: TargetZone::Hand,
            count: 1,
            audience: RevealAudience::Public,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: Vec::new(),
    };
    env.resolve_effect_payload(0, 3, &payload);

    assert!(env.replay_events.iter().any(|event| match event {
        ReplayEvent::Reveal { card, .. } => *card == 1,
        _ => false,
    }));
}

#[test]
fn stock_charge_moves_cards_from_deck_to_stock() {
    let mut env = make_env();
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let top = make_instance(1, 0, &mut next_id);
    let next = make_instance(2, 0, &mut next_id);
    let bottom = make_instance(3, 0, &mut next_id);
    env.state.players[0].deck = vec![bottom, next, top];
    env.state.players[0].stock.clear();

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 6, 0, 0),
        kind: EffectKind::StockCharge { count: 2 },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: Vec::new(),
    };
    env.resolve_effect_payload(0, 6, &payload);

    assert_eq!(env.state.players[0].deck.len(), 1);
    assert_eq!(env.state.players[0].stock.len(), 2);
    let stock_ids: Vec<u32> = env.state.players[0].stock.iter().map(|c| c.id).collect();
    assert_eq!(stock_ids, vec![1, 2]);
}

#[test]
fn visibility_policy_masks_opponent_hidden_choices() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    let mut next_id = 1u32;
    env.state.players[1].hand = vec![
        make_instance(1, 1, &mut next_id),
        make_instance(2, 1, &mut next_id),
    ];

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::Opponent,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
        kind: EffectKind::MoveToHand,
        target: Some(spec.clone()),
        optional: false,
    };
    let candidates = enumerate_targets_for_test(&env, 0, &spec, &[]);
    env.state.turn.target_selection = Some(TargetSelectionState {
        controller: 0,
        source_id: 1,
        remaining: 1,
        spec,
        selected: Vec::new(),
        candidates,
        effect: PendingTargetEffect::EffectPending {
            instance_id: 1,
            payload: EffectPayload {
                spec: effect_spec,
                targets: Vec::new(),
            },
        },
        allow_skip: false,
    });
    env.present_target_choice();

    let (choice_id, options) = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                choice_id,
                options,
                ..
            } = e
            {
                Some((*choice_id, options))
            } else {
                None
            }
        })
        .expect("choice presented");
    assert!(options.iter().all(|opt| opt.reference.card_id == 0));
    assert!(options.iter().all(|opt| opt.reference.index.is_none()));
    assert!(options
        .iter()
        .all(|opt| opt.option_id >> 32 == choice_id as u64));
    let mut unique = std::collections::BTreeSet::new();
    for opt in options {
        assert!(unique.insert(opt.option_id));
    }

    env.replay_events.clear();
    env.state.turn.choice = None;
    let revealed = env.state.players[1].hand[1];
    env.reveal_card(
        1,
        &revealed,
        RevealReason::TriggerCheck,
        RevealAudience::Public,
    );
    env.present_target_choice();

    let options = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } = e
            {
                Some(options)
            } else {
                None
            }
        })
        .expect("choice presented");
    assert!(options.iter().any(|opt| opt.reference.card_id == 2));
    assert!(options.iter().any(|opt| opt.reference.card_id == 0));
}

#[test]
fn public_replay_masks_hidden_action_params() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.replay_actions.clear();

    env.log_action(
        1,
        ActionDesc::MainPlayCharacter {
            hand_index: 3,
            stage_slot: 2,
        },
    );

    let last = env.replay_actions.last().expect("action logged");
    match last {
        ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => {
            assert_eq!(*hand_index, u8::MAX);
            assert_eq!(*stage_slot, 2);
        }
        _ => panic!("unexpected action: {last:?}"),
    }
}

#[test]
fn public_observation_masks_opponent_last_action_params() {
    let mut env = make_env();
    env.curriculum.enable_visibility_policies = false;
    env.last_action_desc = Some(ActionDesc::MainPlayCharacter {
        hand_index: 4,
        stage_slot: 1,
    });
    env.last_action_player = Some(1);
    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );
    assert_eq!(obs[5], 6);
    assert_eq!(obs[6], -1);
    assert_eq!(obs[7], 1);
}

#[test]
fn public_observation_masks_hidden_zones_without_policies() {
    let mut env = make_env();
    env.curriculum.enable_visibility_policies = false;
    env.config.observation_visibility = ObservationVisibility::Public;

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );

    let opponent_base = OBS_HEADER_LEN + PER_PLAYER_BLOCK_LEN;
    let stock_start = opponent_base
        + PER_PLAYER_COUNTS
        + PER_PLAYER_STAGE
        + PER_PLAYER_CLIMAX_TOP
        + PER_PLAYER_LEVEL
        + PER_PLAYER_CLOCK_TOP
        + PER_PLAYER_WAITING_TOP
        + PER_PLAYER_RESOLUTION_TOP;
    let hand_start = stock_start + PER_PLAYER_STOCK_TOP;
    let deck_start = hand_start + PER_PLAYER_HAND;

    assert!(obs[stock_start..stock_start + PER_PLAYER_STOCK_TOP]
        .iter()
        .all(|v| *v == -1));
    assert!(obs[hand_start..hand_start + PER_PLAYER_HAND]
        .iter()
        .all(|v| *v == -1));
    assert!(obs[deck_start..deck_start + PER_PLAYER_DECK]
        .iter()
        .all(|v| *v == -1));

    let own_hand_start = OBS_HEADER_LEN
        + PER_PLAYER_COUNTS
        + PER_PLAYER_STAGE
        + PER_PLAYER_CLIMAX_TOP
        + PER_PLAYER_LEVEL
        + PER_PLAYER_CLOCK_TOP
        + PER_PLAYER_WAITING_TOP
        + PER_PLAYER_RESOLUTION_TOP
        + PER_PLAYER_STOCK_TOP;
    assert!(obs[own_hand_start..own_hand_start + PER_PLAYER_HAND]
        .iter()
        .any(|v| *v > 0));
}

#[test]
fn public_replay_masks_hidden_draws() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.recording = true;
    env.replay_events.clear();

    env.log_event(Event::Draw {
        player: 1,
        card: 99,
    });

    let last = env.replay_events.last().expect("draw event");
    match last {
        ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
        _ => panic!("unexpected event: {last:?}"),
    }
}

#[test]
fn reveal_history_updates_for_controller_only() {
    let mut env = make_env();
    let card = env.state.players[0]
        .deck
        .last()
        .cloned()
        .expect("deck card");
    env.reveal_card(
        0,
        &card,
        RevealReason::AbilityEffect,
        RevealAudience::ControllerOnly,
    );

    let mut obs_p0 = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs_p0,
    );
    let reveal_p0 = &obs_p0[OBS_REVEAL_BASE..OBS_REVEAL_BASE + OBS_REVEAL_LEN];
    assert_eq!(reveal_p0[0], card.id as i32);

    let mut obs_p1 = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        1,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs_p1,
    );
    let reveal_p1 = &obs_p1[OBS_REVEAL_BASE..OBS_REVEAL_BASE + OBS_REVEAL_LEN];
    assert!(reveal_p1.iter().all(|v| *v == 0));
}

#[test]
fn reveal_history_ring_buffer_keeps_latest_reveals() {
    let mut env = make_env();
    let mut revealed = Vec::new();
    let total = REVEAL_HISTORY_LEN + 2;
    for idx in 0..total {
        let card = env.state.players[0]
            .deck
            .get(idx)
            .cloned()
            .expect("deck card");
        env.reveal_card(
            0,
            &card,
            RevealReason::AbilityEffect,
            RevealAudience::ControllerOnly,
        );
        revealed.push(card.id);
    }
    let start = revealed.len() - REVEAL_HISTORY_LEN;
    let expected = &revealed[start..];

    let mut obs_p0 = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs_p0,
    );
    let reveal_p0 = &obs_p0[OBS_REVEAL_BASE..OBS_REVEAL_BASE + OBS_REVEAL_LEN];
    let expected_vals: Vec<i32> = expected.iter().map(|id| *id as i32).collect();
    assert_eq!(&reveal_p0[..expected_vals.len()], expected_vals.as_slice());
}

#[test]
fn observation_context_bits_reflect_turn_state() {
    let mut env = make_env();
    env.config.observation_visibility = ObservationVisibility::Public;
    env.advance_until_decision();
    env.update_action_cache();

    env.state.turn.priority = Some(PriorityState {
        holder: 0,
        passes: 0,
        window: TimingWindow::MainWindow,
        used_act_mask: 0,
    });
    env.state.turn.choice = Some(ChoiceState {
        id: 1,
        reason: ChoiceReason::EndPhaseDiscard,
        player: 0,
        options: Vec::new(),
        total_candidates: 0,
        page_start: 0,
        pending_trigger: None,
    });
    env.state.turn.stack.push(make_noop_stack_item(1));
    env.state
        .turn
        .encore_queue
        .push(EncoreRequest { player: 0, slot: 0 });

    let mut obs = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        env.last_perspective,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );
    let ctx = &obs[OBS_CONTEXT_BASE..OBS_CONTEXT_BASE + OBS_CONTEXT_LEN];
    assert_eq!(ctx[OBS_CONTEXT_PRIORITY_WINDOW], 1);
    assert_eq!(ctx[OBS_CONTEXT_CHOICE_ACTIVE], 1);
    assert_eq!(ctx[OBS_CONTEXT_STACK_NONEMPTY], 1);
    assert_eq!(ctx[OBS_CONTEXT_ENCORE_PENDING], 1);
}

#[test]
fn action_cache_reuses_for_same_decision() {
    let mut env = make_env();
    env.advance_until_decision();
    env.update_action_cache();

    let decision_id = env.decision_id();
    let mask_before = env.action_mask().to_vec();
    let lookup_before = env.action_lookup().to_vec();

    env.update_action_cache();

    assert_eq!(env.decision_id(), decision_id);
    assert_eq!(env.action_mask(), mask_before.as_slice());
    assert_eq!(env.action_lookup(), lookup_before.as_slice());
}

#[test]
fn rule_actions_remove_non_character_from_stage() {
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
            card_type: CardType::Event,
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
    ];
    add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db"));
    let config = EnvConfig {
        deck_lists: [
            legalize_deck(vec![1; 50], &[1]),
            legalize_deck(vec![1; 50], &[1]),
        ],
        deck_ids: [1, 2],
        max_decisions: 50,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        3,
        ReplayConfig::default(),
        None,
        0,
    );
    let _ = env.reset_no_copy();
    let mut next_id = 1u32;
    let event_card = make_instance(2, 0, &mut next_id);
    env.place_card_on_stage(0, event_card, 0, StageStatus::Stand, Zone::Hand, None);
    env.advance_until_decision();
    assert!(env.state.players[0].stage[0].card.is_none());
    assert!(env.state.players[0].waiting_room.iter().any(|c| c.id == 2));
}

#[test]
fn trigger_group_ordering_is_stable_and_grouped_event_logged() {
    let mut env = make_env();
    env.recording = true;
    env.canonical_events.clear();
    env.replay_events.clear();

    let effects = vec![
        TriggerEffect::Bounce,
        TriggerEffect::Soul,
        TriggerEffect::Draw,
    ];
    env.queue_trigger_group(0, 1, effects);

    let pending: Vec<TriggerEffect> = env
        .state
        .turn
        .pending_triggers
        .iter()
        .map(|t| t.effect)
        .collect();
    assert_eq!(
        pending,
        vec![
            TriggerEffect::Soul,
            TriggerEffect::Draw,
            TriggerEffect::Bounce
        ]
    );

    let grouped = env
        .canonical_events
        .iter()
        .find_map(|event| match event {
            Event::TriggerGrouped {
                group_id,
                trigger_ids,
            } => Some((*group_id, trigger_ids.clone())),
            _ => None,
        })
        .expect("TriggerGrouped event");
    let pending_ids: Vec<u32> = env
        .state
        .turn
        .pending_triggers
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(grouped.1, pending_ids);
}

#[test]
fn trigger_quiescence_cap_sets_timeout_and_error_code() {
    let mut env = make_env();
    env.curriculum.enable_priority_windows = false;
    env.decision = None;
    env.state.turn.choice = None;
    env.state.turn.priority = None;
    env.state.turn.stack_order = None;
    env.state.turn.pending_triggers.clear();

    let cap = CHECK_TIMING_QUIESCENCE_CAP;
    let mut stack = Vec::with_capacity(cap as usize + 1);
    for id in 0..=cap {
        stack.push(make_noop_stack_item(id + 1));
    }
    env.state.turn.stack = stack;

    env.resolve_quiescence_until_decision();

    assert_eq!(env.state.terminal, Some(TerminalResult::Timeout));
    assert!(env.last_engine_error);
    assert_eq!(
        env.last_engine_error_code,
        EngineErrorCode::TriggerQuiescenceCap
    );
}

#[test]
fn trigger_pipeline_resolves_under_load_without_quiescence_cap() {
    let mut env = make_env();
    env.curriculum.enable_priority_windows = false;
    env.curriculum.enable_triggers = true;
    env.decision = None;
    env.state.turn.choice = None;
    env.state.turn.priority = None;
    env.state.turn.stack_order = None;
    env.state.turn.pending_triggers.clear();
    env.state.turn.pending_triggers_sorted = true;

    for _ in 0..32 {
        env.queue_trigger_group(0, 1, vec![TriggerEffect::Soul]);
    }

    env.resolve_quiescence_until_decision();

    assert!(env.state.terminal.is_none());
    assert!(!env.last_engine_error);
    assert!(env.state.turn.pending_triggers.is_empty());
    assert!(env.state.turn.stack.is_empty());
}

#[test]
fn public_replay_no_hidden_zone_leaks() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.recording = true;
    env.replay_events.clear();

    env.draw_to_hand(1, 1);

    let mut next_id = 1u32;
    env.state.players[1].hand.clear();
    env.state.players[1]
        .hand
        .push(make_instance(2, 1, &mut next_id));

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::Opponent,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
        kind: EffectKind::MoveToHand,
        target: Some(spec.clone()),
        optional: false,
    };
    let candidates = enumerate_targets_for_test(&env, 0, &spec, &[]);
    env.state.turn.target_selection = Some(TargetSelectionState {
        controller: 0,
        source_id: 1,
        remaining: 1,
        spec,
        selected: Vec::new(),
        candidates,
        effect: PendingTargetEffect::EffectPending {
            instance_id: 1,
            payload: EffectPayload {
                spec: effect_spec,
                targets: Vec::new(),
            },
        },
        allow_skip: false,
    });
    env.present_target_choice();

    for event in &env.replay_events {
        match event {
            ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
            ReplayEvent::ZoneMove {
                card,
                from,
                to,
                from_slot,
                to_slot,
                ..
            } => {
                let hidden_from = matches!(from, Zone::Deck | Zone::Hand | Zone::Stock);
                let hidden_to = matches!(to, Zone::Deck | Zone::Hand | Zone::Stock);
                if hidden_from && hidden_to {
                    assert_eq!(*card, 0);
                    assert_eq!(*from_slot, None);
                    assert_eq!(*to_slot, None);
                }
            }
            ReplayEvent::ChoicePresented { options, .. } => {
                for opt in options {
                    if matches!(
                        opt.reference.zone,
                        ChoiceZone::Hand | ChoiceZone::DeckTop | ChoiceZone::Stock
                    ) {
                        assert_eq!(opt.reference.card_id, 0);
                        assert_eq!(opt.reference.instance_id, 0);
                        assert!(opt.reference.index.is_none());
                    }
                }
            }
            _ => {}
        }
    }
}

#[test]
fn reveal_one_copy_does_not_unmask_duplicates() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.replay_events.clear();

    let mut next_id = 1u32;
    let first = make_instance(1, 1, &mut next_id);
    let second = make_instance(1, 1, &mut next_id);
    env.state.players[1].hand = vec![first, second];

    env.reveal_card(
        1,
        &first,
        RevealReason::TriggerCheck,
        RevealAudience::Public,
    );

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::Opponent,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
        kind: EffectKind::MoveToHand,
        target: Some(spec.clone()),
        optional: false,
    };
    let candidates = enumerate_targets_for_test(&env, 0, &spec, &[]);
    env.state.turn.target_selection = Some(TargetSelectionState {
        controller: 0,
        source_id: 1,
        remaining: 1,
        spec,
        selected: Vec::new(),
        candidates,
        effect: PendingTargetEffect::EffectPending {
            instance_id: 1,
            payload: EffectPayload {
                spec: effect_spec,
                targets: Vec::new(),
            },
        },
        allow_skip: false,
    });
    env.present_target_choice();

    let options = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } = e
            {
                Some(options)
            } else {
                None
            }
        })
        .expect("choice presented");
    let revealed = options
        .iter()
        .filter(|opt| opt.reference.card_id == 1)
        .count();
    let hidden = options
        .iter()
        .filter(|opt| opt.reference.card_id == 0)
        .count();
    assert_eq!(revealed, 1);
    assert_eq!(hidden, 1);
    assert!(options.iter().all(|opt| opt.reference.instance_id == 0));
}

#[test]
fn deterministic_replay_from_seed_and_actions() {
    let mut env_a = make_env();
    let _ = env_a.reset_no_copy();
    env_a.recording = true;
    env_a.replay_events.clear();
    env_a.canonical_events.clear();

    let mut actions: Vec<ActionDesc> = Vec::new();
    for _ in 0..40 {
        if env_a.state.terminal.is_some() {
            break;
        }
        let action = env_a.legal_actions().first().expect("legal action").clone();
        actions.push(action.clone());
        env_a.apply_action(action).expect("apply action");
    }
    let state_hash = state_fingerprint(&env_a.state);
    let events_hash = events_fingerprint(env_a.canonical_events());

    let mut env_b = make_env();
    let _ = env_b.reset_no_copy();
    env_b.recording = true;
    env_b.replay_events.clear();
    env_b.canonical_events.clear();

    for action in actions {
        if env_b.state.terminal.is_some() {
            break;
        }
        env_b.apply_action(action).expect("apply action");
    }
    assert_eq!(state_fingerprint(&env_b.state), state_hash);
    assert_eq!(events_fingerprint(env_b.canonical_events()), events_hash);
}

#[test]
fn alternate_end_conditions_simultaneous_loss_policies() {
    let mut env = make_env();
    env.curriculum.use_alternate_end_conditions = true;

    env.state.turn.active_player = 0;
    env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::Draw;
    env.config
        .end_condition_policy
        .allow_draw_on_simultaneous_loss = true;
    env.state.turn.pending_losses = [true, true];
    env.resolve_pending_losses();
    assert!(matches!(env.state.terminal, Some(TerminalResult::Draw)));

    env.state.terminal = None;
    env.state.turn.pending_losses = [true, true];
    env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::ActivePlayerWins;
    env.resolve_pending_losses();
    assert!(matches!(
        env.state.terminal,
        Some(TerminalResult::Win { winner: 0 })
    ));

    env.state.terminal = None;
    env.state.turn.pending_losses = [true, true];
    env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::NonActivePlayerWins;
    env.resolve_pending_losses();
    assert!(matches!(
        env.state.terminal,
        Some(TerminalResult::Win { winner: 1 })
    ));

    env.state.terminal = None;
    env.state.turn.pending_losses = [true, true];
    env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::Draw;
    env.config
        .end_condition_policy
        .allow_draw_on_simultaneous_loss = false;
    env.resolve_pending_losses();
    assert!(matches!(
        env.state.terminal,
        Some(TerminalResult::Win { winner: 0 })
    ));
}
