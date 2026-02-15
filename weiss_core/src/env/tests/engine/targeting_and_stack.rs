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
            source_ref: None,
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
            source_ref: None,
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
        card_ids: Vec::new(),
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
        vec![1, 0]
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
        card_ids: Vec::new(),
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
        card_ids: Vec::new(),
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
    let mut env = GameEnv::new_or_panic(
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
        card_ids: Vec::new(),
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
        card_ids: Vec::new(),
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
                source_ref: None,
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

