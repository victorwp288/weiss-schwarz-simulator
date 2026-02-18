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
fn ability_def_climax_gate_filters_timing_trigger_queue() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            requires_approx_effects: false,
            climax_area: Some(AbilityDefClimaxAreaCondition {
                side: TargetSide::SelfSide,
                card_ids: vec![1],
            }),
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.players[0].climax.clear();
    env.state.turn.pending_triggers.clear();

    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert!(env.state.turn.pending_triggers.is_empty());

    let climax_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("climax source");
    let climax_card = env.state.players[0].deck.remove(climax_idx);
    env.state.players[0].climax.push(climax_card);
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert_eq!(env.state.turn.pending_triggers.len(), 2);
    assert!(matches!(
        env.state.turn.pending_triggers[0].effect,
        TriggerEffect::AutoAbility { .. }
    ));
}

#[test]
fn ability_def_climax_gate_rechecked_at_trigger_resolve() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            requires_approx_effects: false,
            climax_area: Some(AbilityDefClimaxAreaCondition {
                side: TargetSide::SelfSide,
                card_ids: vec![1],
            }),
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    let climax_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("climax source");
    let climax_card = env.state.players[0].deck.remove(climax_idx);
    env.state.players[0].climax.push(climax_card);
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    let trigger = env
        .state
        .turn
        .pending_triggers
        .pop()
        .expect("queued auto trigger");

    if let Some(climax_card) = env.state.players[0].climax.pop() {
        env.state.players[0].waiting_room.push(climax_card);
    }
    let resolved = env.resolve_trigger(trigger).expect("resolve trigger");
    assert!(!resolved);
    assert!(env.state.turn.stack.is_empty());
}

#[test]
fn climax_auto_cost_uses_no_synthetic_source_slot_for_rest_other() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.card_type = CardType::Climax;
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost {
            rest_other: 1,
            ..Default::default()
        },
        conditions: AbilityDefConditions::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();

    let climax_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("climax source");
    let climax_card = env.state.players[0].deck.remove(climax_idx);
    env.state.players[0].climax.push(climax_card);

    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id != 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.players[0].stage[0].status = StageStatus::Stand;
    let second_stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id != 1)
        .expect("second stage source");
    let second_stage_card = env.state.players[0].deck.remove(second_stage_idx);
    env.state.players[0].stage[1].card = Some(second_stage_card);
    env.state.players[0].stage[1].status = StageStatus::Stand;

    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    let trigger = env
        .state
        .turn
        .pending_triggers
        .pop()
        .expect("queued auto trigger");
    let resolved = env.resolve_trigger(trigger).expect("resolve trigger");
    assert!(resolved);

    let choice = env.state.turn.choice.as_ref().expect("cost payment choice");
    assert_eq!(choice.reason, ChoiceReason::CostPayment);
    assert!(choice
        .options
        .iter()
        .any(|option| option.zone == ChoiceZone::Stage && option.index == Some(0)));
    assert!(env
        .state
        .turn
        .pending_cost
        .as_ref()
        .is_some_and(|cost| cost.source_slot.is_none()));

    let choice = env
        .state
        .turn
        .choice
        .take()
        .expect("cost payment choice should remain active");
    let option = choice.options[0];
    env.apply_choice_effect(choice.reason, choice.player, option, choice.pending_trigger);
    assert_eq!(env.state.players[0].stage[0].status, StageStatus::Rest);
}

#[test]
fn ability_def_requires_approx_effects_respects_curriculum_gate() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            requires_approx_effects: true,
            climax_area: None,
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.turn.pending_triggers.clear();

    env.curriculum.enable_approx_effects = false;
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert!(env.state.turn.pending_triggers.is_empty());

    env.curriculum.enable_approx_effects = true;
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert_eq!(env.state.turn.pending_triggers.len(), 1);
}

#[test]
fn continuous_ability_requires_approx_effects_respects_curriculum_gate() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Continuous,
        timing: None,
        effects: vec![EffectTemplate::AddPower {
            amount: 500,
            duration_turn: false,
        }],
        effect_optional: Vec::new(),
        targets: vec![TargetTemplate::This],
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            requires_approx_effects: true,
            climax_area: None,
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);

    env.curriculum.enable_approx_effects = false;
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    assert!(!env
        .state
        .modifiers
        .iter()
        .any(|modifier| modifier.source == 1
            && modifier.target_player == 0
            && modifier.target_slot == 0));

    env.curriculum.enable_approx_effects = true;
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    assert!(env
        .state
        .modifiers
        .iter()
        .any(|modifier| modifier.source == 1
            && modifier.target_player == 0
            && modifier.target_slot == 0));
}

#[test]
fn ability_def_turn_condition_filters_timing_trigger_queue() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            turn: Some(ConditionTurn::OpponentTurn),
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.turn.pending_triggers.clear();

    env.state.turn.active_player = 0;
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert!(env.state.turn.pending_triggers.is_empty());

    env.state.turn.active_player = 1;
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert_eq!(env.state.turn.pending_triggers.len(), 1);
}

#[test]
fn ability_def_turn_condition_rechecked_at_trigger_resolve() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            turn: Some(ConditionTurn::SelfTurn),
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.turn.active_player = 0;
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    let trigger = env
        .state
        .turn
        .pending_triggers
        .pop()
        .expect("queued auto trigger");

    env.state.turn.active_player = 1;
    let resolved = env.resolve_trigger(trigger).expect("resolve trigger");
    assert!(!resolved);
    assert!(env.state.turn.stack.is_empty());
}

#[test]
fn ability_def_self_memory_at_most_filters_timing_trigger_queue() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            self_memory_at_most: Some(0),
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.turn.pending_triggers.clear();

    env.state.players[0].memory.clear();
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert_eq!(env.state.turn.pending_triggers.len(), 1);

    let mut next_id = 2000u32;
    env.state.players[0]
        .memory
        .push(make_instance(2, 0, &mut next_id));
    env.state.turn.pending_triggers.clear();
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert!(env.state.turn.pending_triggers.is_empty());
}

#[test]
fn ability_def_self_memory_card_ids_any_filters_timing_trigger_queue() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            self_memory_card_ids_any: vec![2],
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    let stage_idx = env.state.players[0]
        .deck
        .iter()
        .position(|card| card.id == 1)
        .expect("stage source");
    let stage_card = env.state.players[0].deck.remove(stage_idx);
    env.state.players[0].stage[0].card = Some(stage_card);
    env.state.turn.pending_triggers.clear();

    env.state.players[0].memory.clear();
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert!(env.state.turn.pending_triggers.is_empty());

    let mut next_id = 3000u32;
    env.state.players[0]
        .memory
        .push(make_instance(2, 0, &mut next_id));
    env.state.turn.pending_triggers.clear();
    env.queue_timing_triggers(AbilityTiming::BeginMainPhase);
    assert_eq!(env.state.turn.pending_triggers.len(), 1);
}

#[test]
fn use_act_timing_trigger_queues_after_activated_ability_use() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let trigger_card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    trigger_card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Auto,
        timing: Some(AbilityTiming::UseAct),
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
    let activated_card = cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present");
    activated_card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Activated,
        timing: None,
        effects: vec![EffectTemplate::AddPower {
            amount: 500,
            duration_turn: true,
        }],
        effect_optional: Vec::new(),
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

    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[0].stage[1].card = Some(make_instance(2, 0, &mut next_id));
    env.state.turn.pending_triggers.clear();
    env.state.turn.pending_triggers_sorted = true;

    let paused = env
        .queue_activated_ability_stack_item(0, 1, 0)
        .expect("activate ability");
    assert!(!paused);
    assert!(env
        .state
        .turn
        .pending_triggers
        .iter()
        .any(|trigger| trigger.source_card == 1));
}

#[test]
fn hand_level_delta_condition_reduces_play_requirement() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.level = 1;
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Continuous,
        timing: None,
        effects: vec![],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            hand_level_delta: -1,
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    env.state.players[0].level.clear();

    let static_card = env.db.get(1).expect("card 1");
    assert!(env.meets_level_requirement(0, static_card));
}

#[test]
fn hand_ignore_color_requirement_condition_bypasses_color_gate() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.level = 1;
    card.color = CardColor::Blue;
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Continuous,
        timing: None,
        effects: vec![],
        effect_optional: Vec::new(),
        targets: Vec::new(),
        cost: AbilityCost::default(),
        conditions: AbilityDefConditions {
            source_rule_id: None,
            ignore_color_requirement: true,
            ..Default::default()
        },
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_card_ids: Vec::new(),
        target_limit: None,
    }];
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let _ = env.reset_no_copy();
    env.state.players[0].level.clear();
    env.state.players[0].clock.clear();
    env.curriculum.enforce_color_requirement = true;

    let static_card = env.db.get(1).expect("card 1");
    assert!(env.meets_color_requirement(0, static_card));
}
