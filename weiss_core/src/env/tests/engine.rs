use super::*;
use crate::config::*;
use crate::db::*;
use crate::effects::*;
use crate::env::{EngineErrorCode, CHECK_TIMING_QUIESCENCE_CAP};
use crate::events::*;
use crate::replay::ReplayEvent;
use crate::state::*;

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
fn action_cache_reuses_for_same_decision() {
    let mut env = make_env();
    env.advance_until_decision();
    env.update_action_cache();

    let decision_id = env.decision_id();
    let mask_before = env.action_mask().to_vec();
    let ids_before = env.action_ids_cache().to_vec();
    let bits_before = env.action_mask_bits().to_vec();

    env.update_action_cache();

    assert_eq!(env.decision_id(), decision_id);
    assert_eq!(env.action_mask(), mask_before.as_slice());
    assert_eq!(env.action_ids_cache(), ids_before.as_slice());
    assert_eq!(env.action_mask_bits(), bits_before.as_slice());
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

#[test]
fn terminal_rewards_are_zero_sum() {
    let mut env = make_env();
    env.state.terminal = Some(TerminalResult::Win { winner: 0 });
    let r0 = env.terminal_reward_for(0);
    let r1 = env.terminal_reward_for(1);
    assert!((r0 + r1).abs() < 1e-6);
    env.state.terminal = Some(TerminalResult::Draw);
    assert_eq!(env.terminal_reward_for(0), 0.0);
    assert_eq!(env.terminal_reward_for(1), 0.0);
}

#[test]
fn shaping_reward_is_antisymmetric() {
    let mut env = make_env();
    env.config.reward.enable_shaping = true;
    env.state.terminal = None;
    let delta = [2, 1];
    let r0 = env.compute_reward(0, &delta);
    let r1 = env.compute_reward(1, &delta);
    assert!((r0 + r1).abs() < 1e-6);
}

#[test]
fn terminal_and_timeout_flags_are_distinct() {
    let mut env = make_env();
    env.state.terminal = Some(TerminalResult::Timeout);
    let timeout_outcome = env.build_outcome_with_obs(0.0, true);
    assert!(timeout_outcome.truncated);
    assert!(!timeout_outcome.terminated);

    env.state.terminal = Some(TerminalResult::Win { winner: 0 });
    let win_outcome = env.build_outcome_with_obs(0.0, true);
    assert!(win_outcome.terminated);
    assert!(!win_outcome.truncated);
}
