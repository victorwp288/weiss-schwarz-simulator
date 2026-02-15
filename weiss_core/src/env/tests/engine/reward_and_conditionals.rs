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

#[test]
fn continuous_conditional_turn_modifier_recomputes_by_turn() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    card.power = 1000;
    card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Continuous,
        timing: None,
        effects: vec![EffectTemplate::ConditionalAddPower {
            amount: 1500,
            turn: Some(ConditionTurn::SelfTurn),
            zone_count: None,
            require_source_marker: false,
            per_source_marker: false,
            per_zone_count: false,
            exclude_source: false,
            target_ids: Vec::new(),
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
    env.state.turn.active_player = 0;
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    let own_turn_power = env.compute_slot_power(0, 0);
    assert_eq!(own_turn_power, 2500);

    env.state.turn.active_player = 1;
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    let opp_turn_power = env.compute_slot_power(0, 0);
    assert_eq!(opp_turn_power, 1000);
}

#[test]
fn continuous_conditional_level_total_modifier_recomputes_by_level_zone() {
    let mut env = make_env();
    let mut cards = env.db.cards.clone();
    let source_card = cards
        .iter_mut()
        .find(|card| card.id == 1)
        .expect("card 1 present");
    source_card.power = 1000;
    source_card.ability_defs = vec![AbilityDef {
        kind: AbilityKind::Continuous,
        timing: None,
        effects: vec![EffectTemplate::ConditionalAddPower {
            amount: 2000,
            turn: None,
            zone_count: Some(ZoneCountCondition {
                side: TargetSide::SelfSide,
                zone: CountZone::LevelTotal,
                cmp: CountCmp::AtLeast,
                value: 2,
                card_ids: Vec::new(),
            }),
            require_source_marker: false,
            per_source_marker: false,
            per_zone_count: false,
            exclude_source: false,
            target_ids: Vec::new(),
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
    cards
        .iter_mut()
        .find(|card| card.id == 2)
        .expect("card 2 present")
        .level = 2;
    env.db = Arc::new(CardDb::new(cards).expect("db rebuild"));
    let mut next_id = 1u32;
    env.state.players[0].stage[0].card = Some(make_instance(1, 0, &mut next_id));
    env.state.players[0]
        .level
        .push(make_instance(2, 0, &mut next_id));
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    assert_eq!(env.compute_slot_power(0, 0), 3000);

    env.state.players[0].level.pop();
    env.mark_continuous_modifiers_dirty();
    env.refresh_continuous_modifiers_if_needed();
    assert_eq!(env.compute_slot_power(0, 0), 1000);
}

