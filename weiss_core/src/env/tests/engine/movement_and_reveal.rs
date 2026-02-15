#[test]
fn move_to_marker_moves_waiting_room_target_under_source() {
    let mut env = make_env();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(source);
    let target = make_instance(2, 0, &mut next_id);
    env.state.players[0].waiting_room.push(target);
    let spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
        kind: EffectKind::MoveToMarker,
        target: Some(TargetSpec {
            zone: TargetZone::WaitingRoom,
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
        }),
        optional: false,
    };
    let payload = EffectPayload {
        spec,
        targets: vec![TargetRef {
            player: 0,
            zone: TargetZone::WaitingRoom,
            index: 0,
            card_id: target.id,
            instance_id: target.instance_id,
        }],
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };
    env.resolve_effect_payload(0, 1, &payload);
    assert!(env.state.players[0].waiting_room.is_empty());
    assert_eq!(env.state.players[0].stage[0].markers.len(), 1);
    assert_eq!(
        env.state.players[0].stage[0].markers[0].instance_id,
        target.instance_id
    );
}

#[test]
fn reveal_top_gate_moves_this_to_hand_on_success() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .level = 2;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));

    env.state.players[0].deck.clear();
    env.state.players[0].hand.clear();
    env.state.players[0].stage[0] = StageSlot::empty();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    let revealed = make_instance(2, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(source);
    env.state.players[0].deck.push(revealed);

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::RevealTopIfLevelAtLeastMoveThisToHand { min_level: 1 },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[0].stage[0].card.is_none());
    assert_eq!(env.state.players[0].hand.len(), 1);
    assert_eq!(env.state.players[0].hand[0].instance_id, source.instance_id);
    assert_eq!(env.state.players[0].deck.len(), 1);
    assert_eq!(
        env.state.players[0].deck[0].instance_id,
        revealed.instance_id
    );
}

#[test]
fn reveal_top_gate_rest_this_treats_climax_as_level_zero() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let top = cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present");
    top.card_type = CardType::Climax;
    top.level = 3;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));

    env.state.players[0].deck.clear();
    env.state.players[0].stage[0] = StageSlot::empty();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    let revealed = make_instance(2, 0, &mut next_id);
    let mut source_slot = StageSlot::empty();
    source_slot.card = Some(source);
    source_slot.status = StageStatus::Stand;
    env.state.players[0].stage[0] = source_slot;
    env.state.players[0].deck.push(revealed);

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::RevealTopIfLevelAtLeastRestThis { min_level: 1 },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert_eq!(env.state.players[0].stage[0].status, StageStatus::Stand);
    assert_eq!(env.state.players[0].deck.len(), 1);
    assert_eq!(
        env.state.players[0].deck[0].instance_id,
        revealed.instance_id
    );
}

#[test]
fn reveal_top_gate_moves_top_to_stock_on_success() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .level = 2;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));

    env.state.players[0].deck.clear();
    env.state.players[0].stock.clear();
    let mut next_id = 1u32;
    let revealed = make_instance(2, 0, &mut next_id);
    env.state.players[0].deck.push(revealed);

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::RevealTopIfLevelAtLeastMoveTopToStock { min_level: 1 },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: None,
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[0].deck.is_empty());
    assert_eq!(env.state.players[0].stock.len(), 1);
    assert_eq!(
        env.state.players[0].stock[0].instance_id,
        revealed.instance_id
    );
}

#[test]
fn move_to_memory_moves_stage_target_to_memory() {
    let mut env = make_env();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(source);
    env.state.players[0].memory.clear();
    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::MoveToMemory,
            target: Some(TargetSpec {
                zone: TargetZone::Stage,
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
            }),
            optional: false,
        },
        targets: vec![TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }],
        source_ref: None,
    };

    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[0].stage[0].card.is_none());
    assert_eq!(env.state.players[0].memory.len(), 1);
    assert_eq!(
        env.state.players[0].memory[0].instance_id,
        source.instance_id
    );
}

#[test]
fn battle_opponent_move_to_memory_if_moves_facing_character() {
    let mut env = make_env();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    let target = make_instance(2, 1, &mut next_id);
    env.state.players[0].stage[0].card = Some(source);
    env.state.players[1].stage[0].card = Some(target);
    env.state.players[1].memory.clear();

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::BattleOpponentMoveToMemoryIf {
                max_level: None,
                max_cost: None,
                level_gt_opponent_level: false,
            },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };

    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[1].stage[0].card.is_none());
    assert_eq!(env.state.players[1].memory.len(), 1);
    assert_eq!(
        env.state.players[1].memory[0].instance_id,
        target.instance_id
    );
}

#[test]
fn battle_step_modifier_moves_reversed_battle_opponent_to_memory() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present")
        .power = 3000;
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .power = 500;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    let attacker = make_instance(1, 0, &mut next_id);
    let defender = make_instance(2, 1, &mut next_id);
    env.state.players[0].stage[0].card = Some(attacker);
    env.state.players[1].stage[0].card = Some(defender);
    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::BattleOpponentMoveToMemoryOnReverse,
        1,
        ModifierDuration::UntilEndOfTurn,
    );

    let ctx = AttackContext {
        attacker_slot: 0,
        defender_slot: Some(0),
        attack_type: AttackType::Frontal,
        trigger_card: None,
        trigger_instance_id: None,
        trigger_checks_total: 1,
        trigger_checks_resolved: 0,
        damage: 1,
        counter_allowed: false,
        counter_played: false,
        counter_power: 0,
        damage_modifiers: Vec::new(),
        pending_shot_damage: 0,
        next_modifier_id: 1,
        last_damage_event_id: None,
        auto_trigger_enqueued: false,
        auto_damage_enqueued: false,
        battle_damage_applied: false,
        step: AttackStep::Battle,
        decl_window_done: false,
        trigger_window_done: false,
        damage_window_done: false,
    };

    env.resolve_battle_step(&ctx);

    assert!(env.state.players[1].stage[0].card.is_none());
    assert_eq!(env.state.players[1].memory.len(), 1);
    assert_eq!(
        env.state.players[1].memory[0].instance_id,
        defender.instance_id
    );
}

#[test]
fn battle_opponent_reverse_timing_queues_attacker_auto_ability() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let attacker_card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    attacker_card.power = 3000;
    attacker_card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BattleOpponentReverse),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: Default::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    let defender_card = cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present");
    defender_card.power = 500;
    defender_card.ability_defs.clear();
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));

    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[1].stage[0].card = Some(make_instance(2, 1, &mut next_id));
    env.state.turn.pending_triggers.clear();
    env.state.turn.pending_triggers_sorted = true;
    env.state.turn.active_player = 0;

    let ctx = AttackContext {
        attacker_slot: 0,
        defender_slot: Some(0),
        attack_type: AttackType::Frontal,
        trigger_card: None,
        trigger_instance_id: None,
        trigger_checks_total: 1,
        trigger_checks_resolved: 0,
        damage: 1,
        counter_allowed: false,
        counter_played: false,
        counter_power: 0,
        damage_modifiers: Vec::new(),
        pending_shot_damage: 0,
        next_modifier_id: 1,
        last_damage_event_id: None,
        auto_trigger_enqueued: false,
        auto_damage_enqueued: false,
        battle_damage_applied: false,
        step: AttackStep::Battle,
        decl_window_done: false,
        trigger_window_done: false,
        damage_window_done: false,
    };
    env.resolve_battle_step(&ctx);

    assert!(env
        .state
        .turn
        .pending_triggers
        .iter()
        .any(|trigger| trigger.source_card == 1));
}

#[test]
fn battle_opponent_stock_swap_moves_bottom_stock_to_waiting_room() {
    let mut env = make_env();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    let target = make_instance(2, 1, &mut next_id);
    let bottom_stock = make_instance(3, 1, &mut next_id);

    env.state.players[0].stage[0].card = Some(source);
    env.state.players[1].stage[0].card = Some(target);
    env.state.players[1].stock.push(bottom_stock);

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
                max_level: Some(0),
                max_cost: None,
                level_gt_opponent_level: false,
            },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[1].stage[0].card.is_none());
    assert_eq!(env.state.players[1].stock.len(), 1);
    assert_eq!(
        env.state.players[1].stock[0].instance_id,
        target.instance_id
    );
    assert_eq!(env.state.players[1].waiting_room.len(), 1);
    assert_eq!(
        env.state.players[1].waiting_room[0].instance_id,
        bottom_stock.instance_id
    );
}

#[test]
fn battle_opponent_clock_swap_moves_top_clock_to_waiting_room_first() {
    let mut env = make_env();
    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    let target = make_instance(2, 1, &mut next_id);
    let top_clock = make_instance(3, 1, &mut next_id);

    env.state.players[0].stage[0].card = Some(source);
    env.state.players[1].stage[0].card = Some(target);
    env.state.players[1].clock.push(top_clock);

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
                max_level: Some(0),
                max_cost: None,
                level_gt_opponent_level: false,
            },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert!(env.state.players[1].stage[0].card.is_none());
    assert_eq!(env.state.players[1].clock.len(), 1);
    assert_eq!(
        env.state.players[1].clock[0].instance_id,
        target.instance_id
    );
    assert_eq!(env.state.players[1].waiting_room.len(), 1);
    assert_eq!(
        env.state.players[1].waiting_room[0].instance_id,
        top_clock.instance_id
    );
}

#[test]
fn battle_opponent_top_deck_to_stock_if_respects_effective_level() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .level = 1;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    let source = make_instance(1, 0, &mut next_id);
    let target = make_instance(2, 1, &mut next_id);
    env.state.players[0].stage[0].card = Some(source);
    env.state.players[1].stage[0].card = Some(target);
    env.state.players[0].deck = vec![make_instance(3, 0, &mut next_id)];

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::BattleOpponentTopDeckToStockIf { min_level: 2 },
            target: None,
            optional: true,
        },
        targets: Vec::new(),
        source_ref: Some(TargetRef {
            player: 0,
            zone: TargetZone::Stage,
            index: 0,
            card_id: source.id,
            instance_id: source.instance_id,
        }),
    };

    env.resolve_effect_payload(0, 1, &payload);
    assert_eq!(env.state.players[0].stock.len(), 0);

    let _ = env.add_modifier(
        1,
        1,
        0,
        ModifierKind::Level,
        1,
        ModifierDuration::WhileOnStage,
    );
    env.resolve_effect_payload(0, 1, &payload);
    assert_eq!(env.state.players[0].stock.len(), 1);
}

#[test]
fn recycle_waiting_room_to_deck_shuffle_moves_all_cards() {
    let mut env = make_env();
    let mut next_id = 1u32;
    env.state.players[0].deck = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
    ];
    env.state.players[0].waiting_room = vec![
        make_instance(3, 0, &mut next_id),
        make_instance(4, 0, &mut next_id),
        make_instance(5, 0, &mut next_id),
    ];

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::RecycleWaitingRoomToDeckShuffle,
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: None,
    };

    env.resolve_effect_payload(0, 1, &payload);

    assert_eq!(env.state.players[0].waiting_room.len(), 0);
    assert_eq!(env.state.players[0].deck.len(), 5);
}

#[test]
fn search_top_deck_level_at_least_mill_rest_moves_first_eligible_to_hand() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present")
        .level = 0;
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .level = 1;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    env.state.players[0].hand.clear();
    env.state.players[0].waiting_room.clear();

    let mut next_id = 1u32;
    env.state.players[0].deck = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
        make_instance(1, 0, &mut next_id),
    ];

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::SearchTopDeckToHandLevelAtLeastMillRest {
                look_count: 3,
                choose_count: 1,
                min_level: 1,
            },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: None,
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert_eq!(env.state.players[0].hand.len(), 1);
    assert_eq!(env.state.players[0].hand[0].id, 2);
    assert_eq!(env.state.players[0].waiting_room.len(), 2);
    assert!(env.state.players[0].deck.is_empty());
}

#[test]
fn reveal_top_and_salvage_by_revealed_level_uses_top_card_level() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present")
        .level = 3;
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .level = 2;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    env.state.players[0].hand.clear();

    let mut next_id = 1u32;
    env.state.players[0].deck = vec![make_instance(2, 0, &mut next_id)];
    env.state.players[0].waiting_room = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
    ];

    let payload = EffectPayload {
        spec: EffectSpec {
            id: EffectId::new(EffectSourceKind::Auto, 1, 0, 0),
            kind: EffectKind::RevealTopAndSalvageByRevealedLevel {
                count: 1,
                climax_level: 0,
            },
            target: None,
            optional: false,
        },
        targets: Vec::new(),
        source_ref: None,
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert_eq!(env.state.players[0].hand.len(), 1);
    assert_eq!(env.state.players[0].hand[0].id, 2);
    assert_eq!(env.state.players[0].waiting_room.len(), 1);
    assert_eq!(env.state.players[0].waiting_room[0].id, 1);
    assert_eq!(env.state.players[0].deck.len(), 1);
    assert_eq!(env.state.players[0].deck[0].id, 2);
}

