mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, ChoiceReason, ChoiceZone, DamageType};

fn choose_counter(env: &mut GameEnv) {
    if let Some(choice) = env.state.turn.choice.as_ref() {
        if choice.reason == ChoiceReason::PriorityActionSelect {
            let idx = choice
                .options
                .iter()
                .position(|opt| opt.zone == ChoiceZone::PriorityCounter)
                .expect("counter option");
            env.apply_action(ActionDesc::ChoiceSelect { index: idx as u8 })
                .unwrap();
        }
    }
}

#[test]
fn effect_damage_canceled_by_counter() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC, CARD_COUNTER_CANCEL]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 10, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_EFFECT_ATTACK)],
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
        vec![CARD_COUNTER_CANCEL],
        vec![],
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    choose_counter(&mut env);

    let effect_modified = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::DamageModified {
                damage_type: DamageType::Effect,
                canceled: true,
                modified: 0,
                ..
            }
        )
    });
    let effect_committed = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::DamageCommitted {
                damage_type: DamageType::Effect,
                ..
            }
        )
    });
    assert!(effect_modified);
    assert!(!effect_committed);
    assert_eq!(env.state.players[1].clock.len(), 1);
    env.validate_state().unwrap();
}

#[test]
fn effect_damage_reduced_then_applied() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC, CARD_COUNTER_REDUCE]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 11, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_EFFECT_ATTACK)],
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
        vec![CARD_COUNTER_REDUCE],
        vec![],
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    choose_counter(&mut env);

    let effect_modified = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::DamageModified {
                damage_type: DamageType::Effect,
                canceled: false,
                modified: 1,
                ..
            }
        )
    });
    let effect_committed = env
        .replay_events
        .iter()
        .filter(|e| {
            matches!(
                e,
                ReplayEvent::DamageCommitted {
                    damage_type: DamageType::Effect,
                    ..
                }
            )
        })
        .count();
    assert!(effect_modified);
    assert_eq!(effect_committed, 1);
    assert_eq!(env.state.players[1].clock.len(), 2);
    env.validate_state().unwrap();
}

#[test]
fn effect_damage_multiple_reductions_apply_in_order() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC, CARD_COUNTER_DOUBLE_REDUCE]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 27, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_EFFECT_ATTACK)],
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
        vec![CARD_COUNTER_DOUBLE_REDUCE],
        vec![],
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    choose_counter(&mut env);

    let effect_event_id = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::DamageIntent {
                event_id,
                damage_type: DamageType::Effect,
                ..
            } = e
            {
                Some(*event_id)
            } else {
                None
            }
        })
        .unwrap();

    let applied: Vec<(i32, i32)> = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::DamageModifierApplied {
                event_id,
                before_amount,
                after_amount,
                ..
            } = e
            {
                if *event_id == effect_event_id {
                    return Some((*before_amount, *after_amount));
                }
            }
            None
        })
        .collect();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0], (2, 1));
    assert_eq!(applied[1], (1, 0));
    env.validate_state().unwrap();
}

#[test]
fn battle_damage_vs_effect_damage_flags() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        enable_counters: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 12, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_EFFECT_ATTACK)],
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
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();

    let mut has_effect = false;
    let mut has_battle = false;
    for event in &env.replay_events {
        if let ReplayEvent::DamageCommitted { damage_type, .. } = event {
            match damage_type {
                DamageType::Effect => has_effect = true,
                DamageType::Battle => has_battle = true,
            }
        }
    }
    assert!(has_effect);
    assert!(has_battle);
    env.validate_state().unwrap();
}

#[test]
fn reversal_cause_is_recorded_correctly() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        enable_counters: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 13, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_HIGH_POWER)],
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
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();

    let battle_event_id = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::DamageCommitted {
                event_id,
                damage_type: DamageType::Battle,
                ..
            } = e
            {
                Some(*event_id)
            } else {
                None
            }
        })
        .next_back()
        .unwrap();

    let reversal = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ReversalCommitted {
                cause_damage_event, ..
            } = e
            {
                Some(*cause_damage_event)
            } else {
                None
            }
        })
        .unwrap();

    assert_eq!(reversal, Some(battle_event_id));
    env.validate_state().unwrap();
}

#[test]
fn multiple_instances_damage_same_step_ordering() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_MULTI_EFFECT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        enable_counters: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 14, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_MULTI_EFFECT_ATTACK)],
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
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();

    let intents: Vec<DamageType> = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::DamageIntent { damage_type, .. } = e {
                Some(*damage_type)
            } else {
                None
            }
        })
        .collect();
    assert!(intents.len() >= 3);
    assert_eq!(
        &intents[0..3],
        &[DamageType::Effect, DamageType::Effect, DamageType::Battle]
    );
    env.validate_state().unwrap();
}
