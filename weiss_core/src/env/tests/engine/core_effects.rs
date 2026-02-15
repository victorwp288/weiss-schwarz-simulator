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
        card_ids: Vec::new(),
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
        source_ref: None,
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
        card_ids: Vec::new(),
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
        source_ref: None,
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
        card_ids: Vec::new(),
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
        source_ref: None,
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
        card_ids: Vec::new(),
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
        source_ref: None,
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
        source_ref: None,
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
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost {
            stock: 1,
            rest_self: true,
            rest_other: 0,
            sacrifice_from_stage: 0,
            discard_from_hand: 1,
            clock_from_hand: 0,
            clock_from_deck_top: 0,
            reveal_from_hand: 1,
            move_self_to_waiting_room: false,
            return_self_to_hand: false,
        },
        conditions: Default::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
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
    let mut env = GameEnv::new_or_panic(
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
fn clock_from_deck_top_cost_aborts_when_draw_registers_loss() {
    let ability_def = AbilityDef {
        kind: AbilityKind::Activated,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost {
            stock: 0,
            rest_self: false,
            rest_other: 0,
            sacrifice_from_stage: 0,
            discard_from_hand: 0,
            clock_from_hand: 0,
            clock_from_deck_top: 1,
            reveal_from_hand: 0,
            move_self_to_waiting_room: false,
            return_self_to_hand: false,
        },
        conditions: Default::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
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
    let mut env = GameEnv::new_or_panic(
        db,
        config,
        CurriculumConfig::default(),
        9,
        ReplayConfig::default(),
        None,
        0,
    );
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(source);
    env.state.players[0].stage[0].status = StageStatus::Stand;
    env.state.players[0].deck.clear();
    env.state.players[0].waiting_room.clear();
    env.state.players[0].clock.clear();

    // Keep depth at zero to exercise the draw path that can register terminal loss.
    env.state.turn.cost_payment_depth = 0;
    env.state.turn.pending_cost = Some(CostPaymentState {
        controller: 0,
        source_id: source.id,
        source_instance_id: source.instance_id,
        source_slot: Some(0),
        ability_index: 0,
        remaining: AbilityCost {
            stock: 0,
            rest_self: false,
            rest_other: 0,
            sacrifice_from_stage: 0,
            discard_from_hand: 0,
            clock_from_hand: 0,
            clock_from_deck_top: 1,
            reveal_from_hand: 0,
            move_self_to_waiting_room: false,
            return_self_to_hand: false,
        },
        current_step: None,
        outcome: CostPaymentOutcome::ResolveAbility,
    });

    env.start_cost_choice();

    assert!(matches!(
        env.state.terminal,
        Some(TerminalResult::Win { winner: 1 })
    ));
    assert!(env.state.turn.pending_cost.is_none());
    assert_eq!(env.state.turn.cost_payment_depth, 0);
    assert!(
        env.state.turn.stack.is_empty(),
        "cost abort should prevent ability resolution"
    );
    assert!(env.state.players[0].clock.is_empty());
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
        source_ref: None,
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
        source_ref: None,
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
            card_ids: Vec::new(),
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
        source_ref: None,
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
        source_ref: None,
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
    let payload = EffectPayload {
        spec,
        targets,
        source_ref: None,
    };
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
    let _ = env.reset_no_copy();
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
        source_ref: None,
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
        source_ref: None,
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
    let mut env = GameEnv::new_or_panic(
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
