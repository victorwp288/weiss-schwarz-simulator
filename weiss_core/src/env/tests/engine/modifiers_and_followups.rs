#[test]
fn continuous_facing_opponent_add_soul_applies_to_mirrored_slot() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Continuous,
        timing: None,
        effects: vec![EffectTemplate::FacingOpponentAddSoul { amount: -1 }],
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
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[1].stage[0].card = Some(make_instance(2, 1, &mut next_id));

    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();

    assert!(env.state.modifiers.iter().any(|modifier| {
        modifier.source == 1
            && modifier.target_player == 1
            && modifier.target_slot == 0
            && modifier.target_card == 2
            && modifier.kind == ModifierKind::Soul
            && modifier.magnitude == -1
    }));
}

#[test]
fn cannot_become_reverse_modifier_prevents_battle_reversal() {
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
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[1].stage[0].card = Some(make_instance(2, 1, &mut next_id));
    let _ = env.add_modifier(
        1,
        1,
        0,
        ModifierKind::CannotBecomeReverse,
        1,
        ModifierDuration::WhileOnStage,
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

    assert_ne!(env.state.players[1].stage[0].status, StageStatus::Reverse);
}

#[test]
fn cannot_frontal_attack_modifier_blocks_frontal_attack() {
    let mut env = make_env();
    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[1].stage[0].card = Some(make_instance(2, 1, &mut next_id));
    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::CannotFrontalAttack,
        1,
        ModifierDuration::WhileOnStage,
    );

    let frontal =
        crate::legal::can_declare_attack(&env.state, 0, 0, AttackType::Frontal, &env.curriculum);
    let side =
        crate::legal::can_declare_attack(&env.state, 0, 0, AttackType::Side, &env.curriculum);

    assert!(frontal.is_err());
    assert!(side.is_ok());
}

#[test]
fn cannot_be_chosen_modifier_filters_opponent_stage_targets() {
    let mut env = make_env();
    let mut next_id = 1u32;
    env.state.players[1].stage[0].card = Some(make_instance(2, 1, &mut next_id));
    let _ = env.add_modifier(
        2,
        1,
        0,
        ModifierKind::CannotBeChosenByOpponentEffects,
        1,
        ModifierDuration::WhileOnStage,
    );
    let spec = TargetSpec {
        zone: TargetZone::Stage,
        side: TargetSide::Opponent,
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

    let targets = enumerate_targets_for_test(&env, 0, &spec, &[]);
    assert!(targets.is_empty());
}

#[test]
fn cannot_play_events_modifier_blocks_main_event_actions() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let event_id = cards
        .iter_mut()
        .find(|card| card.id == 2)
        .map(|card| {
            card.card_type = CardType::Event;
            card.level = 0;
            card.cost = 0;
            card.color = CardColor::Colorless;
            card.id
        })
        .expect("card 2 present in test db");
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    env.state.players[0].hand = vec![make_instance(event_id, 0, &mut next_id)];
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));

    let decision = crate::legal::Decision {
        kind: crate::legal::DecisionKind::Main,
        player: 0,
        focus_slot: None,
    };
    let baseline = crate::legal::legal_actions_cached(
        &env.state,
        &decision,
        &env.db,
        &env.curriculum,
        env.curriculum.allowed_card_sets_cache.as_ref(),
    );
    assert!(baseline
        .iter()
        .any(|action| matches!(action, crate::legal::ActionDesc::MainPlayEvent { .. })));

    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::CannotPlayEventsFromHand,
        1,
        ModifierDuration::WhileOnStage,
    );
    let blocked = crate::legal::legal_actions_cached(
        &env.state,
        &decision,
        &env.db,
        &env.curriculum,
        env.curriculum.allowed_card_sets_cache.as_ref(),
    );
    assert!(!blocked
        .iter()
        .any(|action| matches!(action, crate::legal::ActionDesc::MainPlayEvent { .. })));
}

#[test]
fn cannot_play_backup_modifier_blocks_counter_priority_action() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let counter_id = cards
        .iter_mut()
        .find(|card| card.id == 2)
        .map(|card| {
            card.card_type = CardType::Character;
            card.counter_timing = true;
            card.level = 0;
            card.cost = 0;
            card.color = CardColor::Colorless;
            card.abilities = vec![AbilityTemplate::CounterBackup { power: 1000 }];
            card.ability_defs = vec![];
            card.id
        })
        .expect("card 2 present in test db");
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    env.state.players[0].hand = vec![make_instance(counter_id, 0, &mut next_id)];
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.turn.attack = Some(AttackContext {
        attacker_slot: 0,
        defender_slot: Some(0),
        attack_type: AttackType::Frontal,
        trigger_card: None,
        trigger_instance_id: None,
        trigger_checks_total: 1,
        trigger_checks_resolved: 0,
        damage: 1,
        counter_allowed: true,
        counter_played: false,
        counter_power: 0,
        damage_modifiers: Vec::new(),
        pending_shot_damage: 0,
        next_modifier_id: 1,
        last_damage_event_id: None,
        auto_trigger_enqueued: false,
        auto_damage_enqueued: false,
        battle_damage_applied: false,
        step: AttackStep::Counter,
        decl_window_done: false,
        trigger_window_done: false,
        damage_window_done: false,
    });
    env.enter_timing_window(TimingWindow::CounterWindow, 0);

    env.collect_priority_actions(0);
    assert!(env
        .scratch
        .priority_actions
        .iter()
        .any(|action| matches!(action, crate::legal::ActionDesc::CounterPlay { .. })));

    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::CannotPlayBackupFromHand,
        1,
        ModifierDuration::WhileOnStage,
    );
    env.collect_priority_actions(0);
    assert!(!env
        .scratch
        .priority_actions
        .iter()
        .any(|action| matches!(action, crate::legal::ActionDesc::CounterPlay { .. })));
}

#[test]
fn encore_stock_cost_modifier_allows_two_stock_encore() {
    let mut env = make_env();
    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[0].stage[0].status = StageStatus::Reverse;
    env.state.players[0].stock = vec![
        make_instance(2, 0, &mut next_id),
        make_instance(2, 0, &mut next_id),
    ];
    env.state.turn.encore_queue = vec![EncoreRequest { player: 0, slot: 0 }];
    let _ = env.add_modifier(
        1,
        0,
        0,
        ModifierKind::EncoreStockCost,
        2,
        ModifierDuration::WhileOnStage,
    );

    env.resolve_encore(0, 0, true).expect("encore resolves");

    assert!(env.state.players[0].stage[0].card.is_some());
    assert_eq!(env.state.players[0].stage[0].status, StageStatus::Rest);
    assert!(env.state.players[0].stock.is_empty());
}

#[test]
fn trigger_resolution_climax_condition_queues_only_on_climax_reveal() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present")
        .ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::TriggerResolution),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            trigger_check_revealed_climax: true,
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    let mut climax_card = cards
        .iter()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .clone();
    climax_card.id = 999;
    climax_card.card_type = CardType::Climax;
    cards.push(climax_card);
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    env.state.turn.active_player = 0;
    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[0].deck = vec![make_instance(2, 0, &mut next_id)];

    let mut ctx = AttackContext {
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
        step: AttackStep::Trigger,
        decl_window_done: false,
        trigger_window_done: false,
        damage_window_done: false,
    };
    env.state.turn.pending_triggers.clear();
    env.resolve_trigger_step(&mut ctx);
    assert!(env.state.turn.pending_triggers.is_empty());

    env.state.turn.pending_triggers.clear();
    env.state.players[0].deck = vec![make_instance(999, 0, &mut next_id)];
    ctx.trigger_card = None;
    ctx.trigger_instance_id = None;
    env.resolve_trigger_step(&mut ctx);
    assert!(env
        .state
        .turn
        .pending_triggers
        .iter()
        .any(|trigger| trigger.source_card == 1));
}

#[test]
fn damage_canceled_queues_damage_dealt_canceled_auto_ability() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let attacker = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    attacker.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::DamageDealtCanceled),
        effects: vec![EffectTemplate::MoveToHand],
        effect_optional: vec![false],
        targets: vec![TargetTemplate::This],
        cost: AbilityCost::default(),
        conditions: Default::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .card_type = CardType::Climax;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    let attacker_inst = make_instance(1, 0, &mut next_id);
    env.state.players[0].stage[0].card = Some(attacker_inst);
    env.state.players[1].stage[0].card = Some(make_instance(1, 1, &mut next_id));
    env.state.players[1].deck = vec![make_instance(2, 1, &mut next_id)];
    env.state.players[1].waiting_room.clear();

    env.state.turn.active_player = 0;
    let mut ctx = AttackContext {
        attacker_slot: 0,
        defender_slot: Some(0),
        attack_type: AttackType::Frontal,
        trigger_card: None,
        trigger_instance_id: None,
        trigger_checks_total: 1,
        trigger_checks_resolved: 1,
        damage: 1,
        counter_allowed: false,
        counter_played: false,
        counter_power: 0,
        damage_modifiers: Vec::new(),
        pending_shot_damage: 0,
        next_modifier_id: 1,
        last_damage_event_id: None,
        auto_trigger_enqueued: true,
        auto_damage_enqueued: true,
        battle_damage_applied: false,
        step: AttackStep::Damage,
        decl_window_done: true,
        trigger_window_done: true,
        damage_window_done: false,
    };

    let pause = env.resolve_damage_step(&mut ctx);
    assert!(pause);
    assert_eq!(env.state.turn.stack.len(), 1);
    assert!(matches!(
        env.state.turn.stack[0].payload.spec.kind,
        EffectKind::MoveToHand
    ));

    env.curriculum.enable_priority_windows = false;
    env.decision = None;
    env.resolve_quiescence_until_decision();

    assert!(env.state.players[0].stage[0].card.is_none());
    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|card| card.instance_id == attacker_inst.instance_id));
}

#[test]
fn damage_canceled_queues_damage_received_canceled_auto_ability() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let defender = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    defender.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::DamageReceivedCanceled),
        effects: vec![EffectTemplate::MoveToStock],
        effect_optional: vec![false],
        targets: vec![TargetTemplate::This],
        cost: AbilityCost::default(),
        conditions: Default::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .card_type = CardType::Climax;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(2, 0, &mut next_id));
    let defender_inst = make_instance(1, 1, &mut next_id);
    env.state.players[1].stage[0].card = Some(defender_inst);
    env.state.players[1].deck = vec![make_instance(2, 1, &mut next_id)];
    env.state.players[1].stock.clear();

    env.state.turn.active_player = 0;
    let mut ctx = AttackContext {
        attacker_slot: 0,
        defender_slot: Some(0),
        attack_type: AttackType::Frontal,
        trigger_card: None,
        trigger_instance_id: None,
        trigger_checks_total: 1,
        trigger_checks_resolved: 1,
        damage: 1,
        counter_allowed: false,
        counter_played: false,
        counter_power: 0,
        damage_modifiers: Vec::new(),
        pending_shot_damage: 0,
        next_modifier_id: 1,
        last_damage_event_id: None,
        auto_trigger_enqueued: true,
        auto_damage_enqueued: true,
        battle_damage_applied: false,
        step: AttackStep::Damage,
        decl_window_done: true,
        trigger_window_done: true,
        damage_window_done: false,
    };

    let pause = env.resolve_damage_step(&mut ctx);
    assert!(pause);
    assert_eq!(env.state.turn.stack.len(), 1);
    assert!(matches!(
        env.state.turn.stack[0].payload.spec.kind,
        EffectKind::MoveToStock
    ));

    env.curriculum.enable_priority_windows = false;
    env.decision = None;
    env.resolve_quiescence_until_decision();

    assert!(env.state.players[1].stage[0].card.is_none());
    assert!(env.state.players[1]
        .stock
        .iter()
        .any(|card| card.instance_id == defender_inst.instance_id));
}

#[test]
fn damage_not_canceled_queues_damage_received_not_canceled_auto_ability() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let defender = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    defender.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::DamageReceivedNotCanceled),
        effects: vec![EffectTemplate::AddPower {
            amount: 2000,
            duration_turn: true,
        }],
        effect_optional: vec![false],
        targets: vec![TargetTemplate::This],
        cost: AbilityCost::default(),
        conditions: Default::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(2, 0, &mut next_id));
    let defender_inst = make_instance(1, 1, &mut next_id);
    env.state.players[1].stage[0].card = Some(defender_inst);
    env.state.players[1].deck = vec![make_instance(2, 1, &mut next_id)];

    env.state.turn.active_player = 0;
    let mut ctx = AttackContext {
        attacker_slot: 0,
        defender_slot: Some(0),
        attack_type: AttackType::Frontal,
        trigger_card: None,
        trigger_instance_id: None,
        trigger_checks_total: 1,
        trigger_checks_resolved: 1,
        damage: 1,
        counter_allowed: false,
        counter_played: false,
        counter_power: 0,
        damage_modifiers: Vec::new(),
        pending_shot_damage: 0,
        next_modifier_id: 1,
        last_damage_event_id: None,
        auto_trigger_enqueued: true,
        auto_damage_enqueued: true,
        battle_damage_applied: false,
        step: AttackStep::Damage,
        decl_window_done: true,
        trigger_window_done: true,
        damage_window_done: false,
    };

    let pause = env.resolve_damage_step(&mut ctx);
    assert!(pause);
    assert_eq!(env.state.turn.stack.len(), 1);
    assert!(matches!(
        env.state.turn.stack[0].payload.spec.kind,
        EffectKind::AddModifier {
            kind: ModifierKind::Power,
            ..
        }
    ));

    env.curriculum.enable_priority_windows = false;
    env.decision = None;
    env.resolve_quiescence_until_decision();

    assert!(env.state.players[1].stage[0].card.is_some());
}

#[test]
fn other_attack_declaration_auto_adds_power_to_source() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let attacker_family_ids: Vec<u32> = cards
        .iter()
        .filter_map(|card| {
            if card.id % 1000 == 1 {
                Some(card.id)
            } else {
                None
            }
        })
        .collect();
    for source in cards.iter_mut().filter(|card| card.id % 1000 == 1) {
        source.ability_defs = vec![AbilityDef {
            kind: AbilityKind::Auto,
            timing: Some(AbilityTiming::OtherAttackDeclaration),
            effects: vec![EffectTemplate::AddPowerIfOtherAttackerMatches {
                amount: 1000,
                duration_turn: true,
                attacker_card_ids: attacker_family_ids.clone(),
            }],
            effect_optional: vec![],
            targets: vec![],
            cost: AbilityCost::default(),
            conditions: Default::default(),
            target_card_type: None,
            target_trait: None,
            target_level_max: None,
            target_cost_max: None,
            target_card_ids: Vec::new(),
            target_limit: None,
        }];
    }
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let attacker = env.state.players[0].hand.pop().expect("attacker from hand");
    let source_inst = env.state.players[0].hand.pop().expect("source from hand");
    let defender = env.state.players[1].hand.pop().expect("defender from hand");
    env.state.players[0].stage[0].card = Some(attacker);
    env.state.players[0].stage[1].card = Some(source_inst);
    env.state.players[1].stage[0].card = Some(defender);
    env.state.turn.phase = Phase::Attack;
    env.state.turn.active_player = 0;

    env.curriculum.enable_priority_windows = false;
    env.declare_attack(0, 0, AttackType::Frontal)
        .expect("declare attack");
    env.resolve_attack_pipeline();
    assert!(!env.state.turn.stack.is_empty());

    env.decision = None;
    env.resolve_quiescence_until_decision();

    assert!(env.state.modifiers.iter().any(|modifier| {
        modifier.target_player == 0
            && modifier.target_slot == 1
            && modifier.kind == ModifierKind::Power
            && modifier.magnitude == 1000
            && modifier.duration == ModifierDuration::UntilEndOfTurn
    }));
}

#[test]
fn reset_stock_from_deck_top_moves_all_stock_and_refills() {
    let mut env = make_env();
    let _ = env.reset_no_copy();

    let s0 = env.state.players[1].hand.pop().expect("stock card 0");
    let s1 = env.state.players[1].hand.pop().expect("stock card 1");
    env.state.players[1].stock = vec![s0, s1];
    let wr_before = env.state.players[1].waiting_room.len();

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::ResetStockFromDeckTop {
            target: TargetSide::Opponent,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: Vec::new(),
        source_ref: None,
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert_eq!(env.state.players[1].stock.len(), 2);
    assert_eq!(env.state.players[1].waiting_room.len(), wr_before + 2);
}

#[test]
fn heal_if_source_played_from_hand_this_turn_requires_flag() {
    let mut env = make_env();
    let _ = env.reset_no_copy();

    let source = env.state.players[0].hand.pop().expect("source");
    env.place_card_on_stage(0, source, 0, StageStatus::Stand, Zone::Hand, None);
    let source_ref = TargetRef {
        player: 0,
        zone: TargetZone::Stage,
        index: 0,
        card_id: source.id,
        instance_id: source.instance_id,
    };

    let clock_card = env.state.players[0].hand.pop().expect("clock source");
    env.move_card_between_zones(0, clock_card, Zone::Hand, Zone::Clock, None, None);
    let clock_before = env.state.players[0].clock.len();
    let wr_before = env.state.players[0].waiting_room.len();

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, source.id, 0, 0),
        kind: EffectKind::HealIfSourcePlayedFromHandThisTurn,
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: Vec::new(),
        source_ref: Some(source_ref),
    };
    env.resolve_effect_payload(0, source.id, &payload);

    assert_eq!(env.state.players[0].clock.len(), clock_before - 1);
    assert_eq!(env.state.players[0].waiting_room.len(), wr_before + 1);
}

#[test]
fn continuous_middle_center_add_soul_applies_only_in_middle_slot() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    for card in cards.iter_mut().filter(|card| card.id % 1000 == 1) {
        card.ability_defs = vec![AbilityDef {
            kind: AbilityKind::Continuous,
            timing: None,
            effects: vec![EffectTemplate::AddSoulIfMiddleCenter { amount: 1 }],
            effect_optional: vec![],
            targets: vec![],
            cost: AbilityCost::default(),
            conditions: Default::default(),
            target_card_type: None,
            target_trait: None,
            target_level_max: None,
            target_cost_max: None,
            target_card_ids: Vec::new(),
            target_limit: None,
        }];
    }
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let source = env.state.players[0].hand.pop().expect("source");
    env.place_card_on_stage(0, source, 1, StageStatus::Stand, Zone::Hand, None);
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    assert!(env.state.modifiers.iter().any(|modifier| {
        modifier.target_player == 0
            && modifier.target_slot == 1
            && modifier.kind == ModifierKind::Soul
            && modifier.magnitude == 1
            && modifier.layer == ModifierLayer::Continuous
    }));
}

#[test]
fn cannot_use_auto_encore_for_player_blocks_encore_payment() {
    let mut env = make_env();
    let _ = env.reset_no_copy();

    let mut next_id = 1_000_000u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[0].stage[0].status = StageStatus::Reverse;
    env.state.players[0].stock = vec![
        make_instance(1, 0, &mut next_id),
        make_instance(1, 0, &mut next_id),
        make_instance(1, 0, &mut next_id),
    ];
    env.state
        .turn
        .encore_queue
        .push(EncoreRequest { player: 0, slot: 0 });

    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::CannotUseAutoEncoreForPlayer {
            target: TargetSide::SelfSide,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: Vec::new(),
        source_ref: None,
    };
    env.resolve_effect_payload(0, 1, &payload);

    let err = env
        .resolve_encore(0, 0, true)
        .expect_err("encore should be blocked");
    assert!(err.to_string().contains("AUTO Encore is unavailable"));
}

#[test]
fn move_waiting_room_card_to_source_slot_places_selected_card() {
    let mut env = make_env();
    let _ = env.reset_no_copy();

    let target = env.state.players[0].hand.pop().expect("target card");
    env.move_card_between_zones(0, target, Zone::Hand, Zone::WaitingRoom, None, None);
    let wr_index = env.state.players[0]
        .waiting_room
        .iter()
        .position(|card| card.instance_id == target.instance_id)
        .expect("target in waiting room");
    let wr_index_u8 = u8::try_from(wr_index).expect("wr index fits");
    let target_ref = TargetRef {
        player: 0,
        zone: TargetZone::WaitingRoom,
        index: wr_index_u8,
        card_id: target.id,
        instance_id: target.instance_id,
    };
    let source_ref = TargetRef {
        player: 0,
        zone: TargetZone::Stage,
        index: 0,
        card_id: 1,
        instance_id: 999_999,
    };
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
        kind: EffectKind::MoveWaitingRoomCardToSourceSlot,
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec: effect_spec,
        targets: vec![target_ref],
        source_ref: Some(source_ref),
    };
    env.resolve_effect_payload(0, 1, &payload);

    assert_eq!(
        env.state.players[0].stage[0].card.map(|c| c.instance_id),
        Some(target.instance_id)
    );
    assert!(!env.state.players[0]
        .waiting_room
        .iter()
        .any(|card| card.instance_id == target.instance_id));
}
