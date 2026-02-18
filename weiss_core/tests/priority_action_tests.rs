mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::effects::EffectKind;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, ChoiceReason, Phase, TimingWindow};

fn pass_enabled_priority_curriculum() -> CurriculumConfig {
    CurriculumConfig {
        enable_priority_windows: true,
        strict_priority_mode: false,
        priority_autopick_single_action: false,
        ..Default::default()
    }
}

#[test]
fn counter_priority_autoplays_single_counter() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_COUNTER_REDUCE]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..pass_enabled_priority_curriculum()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(db, config, curriculum, 40, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
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

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    let presented = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::PriorityActionSelect,
                options,
                total_candidates,
                ..
            } = e
            {
                Some((options, total_candidates))
            } else {
                None
            }
        })
        .expect("priority choice presented");
    assert_eq!(*presented.1, 2);
    assert!(presented
        .0
        .iter()
        .any(|entry| entry.reference.zone == weiss_core::state::ChoiceZone::PriorityCounter));
    assert!(presented
        .0
        .iter()
        .any(|entry| entry.reference.zone == weiss_core::state::ChoiceZone::PriorityPass));
    let choice = env
        .state
        .turn
        .choice
        .as_ref()
        .expect("current priority choice");
    assert_eq!(choice.reason, ChoiceReason::PriorityActionSelect);
    let counter_index = choice
        .options
        .iter()
        .position(|entry| entry.card_id == CARD_COUNTER_REDUCE)
        .or_else(|| {
            choice
                .options
                .iter()
                .position(|entry| entry.zone == weiss_core::state::ChoiceZone::PriorityCounter)
        })
        .expect("counter option index");
    env.apply_action(ActionDesc::ChoiceSelect {
        index: counter_index as u8,
    })
    .unwrap();

    let pushed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackPushed { item } if matches!(item.payload.spec.kind, EffectKind::CounterDamageReduce { .. })
    ));
    let resolved = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackResolved { item } if matches!(item.payload.spec.kind, EffectKind::CounterDamageReduce { .. })
    ));
    assert!(pushed);
    assert!(resolved);
    env.validate_state().unwrap();
}

#[test]
fn counter_priority_choice_orders_by_hand_index() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_COUNTER_REDUCE, CARD_COUNTER_CANCEL]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..pass_enabled_priority_curriculum()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(db, config, curriculum, 41, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
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
    setup_player_state(
        &mut env,
        1,
        vec![CARD_COUNTER_REDUCE, CARD_COUNTER_CANCEL],
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
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    let (options, total) = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::PriorityActionSelect,
                options,
                total_candidates,
                ..
            } = e
            {
                Some((options, total_candidates))
            } else {
                None
            }
        })
        .expect("priority choice presented");
    assert_eq!(*total, 3);
    let ref0 = &options[0].reference;
    let ref1 = &options[1].reference;
    let ref2 = &options[2].reference;
    assert_eq!(ref0.zone, weiss_core::state::ChoiceZone::PriorityCounter);
    assert_eq!(ref1.zone, weiss_core::state::ChoiceZone::PriorityCounter);
    assert_eq!(ref2.zone, weiss_core::state::ChoiceZone::PriorityPass);

    let choice = env
        .state
        .turn
        .choice
        .as_ref()
        .expect("current priority choice");
    assert_eq!(choice.options.len(), 3);
    assert_eq!(
        choice.options[0].zone,
        weiss_core::state::ChoiceZone::PriorityCounter
    );
    assert_eq!(
        choice.options[1].zone,
        weiss_core::state::ChoiceZone::PriorityCounter
    );
    assert_eq!(choice.options[0].card_id, CARD_COUNTER_REDUCE);
    assert_eq!(choice.options[1].card_id, CARD_COUNTER_CANCEL);
    assert_eq!(
        choice.options[2].zone,
        weiss_core::state::ChoiceZone::PriorityPass
    );
    let cancel_index = choice
        .options
        .iter()
        .position(|entry| entry.card_id == CARD_COUNTER_CANCEL)
        .expect("cancel counter option");
    env.apply_action(ActionDesc::ChoiceSelect {
        index: cancel_index as u8,
    })
    .unwrap();
    let pushed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackPushed { item } if matches!(item.payload.spec.kind, EffectKind::CounterDamageCancel)
    ));
    assert!(pushed);
    env.validate_state().unwrap();
}

#[test]
fn main_priority_act_ability_pushes_and_resolves() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_ACT_ABILITY]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(
        db,
        config,
        CurriculumConfig {
            ..pass_enabled_priority_curriculum()
        },
        42,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_ACT_ABILITY)],
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
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });

    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    let presented = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::ChoicePresented {
                reason: ChoiceReason::PriorityActionSelect,
                ..
            }
        )
    });
    assert!(presented);

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    let entered = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::TimingWindowEntered {
                window: TimingWindow::MainWindow,
                ..
            }
        )
    });
    let pushed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackPushed { item } if matches!(item.payload.spec.kind, EffectKind::AddModifier { .. })
    ));
    let resolved = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackResolved { item } if matches!(item.payload.spec.kind, EffectKind::AddModifier { .. })
    ));
    let modifier_added = env.replay_events.iter().any(|e| {
        matches!(e,
            ReplayEvent::ModifierAdded { magnitude, .. } if *magnitude == 1000
        )
    });
    assert!(entered);
    assert!(pushed);
    assert!(resolved);
    assert!(modifier_added);
    env.validate_state().unwrap();
}

#[test]
fn main_priority_double_pass_ends_window() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(
        db,
        config,
        CurriculumConfig {
            ..pass_enabled_priority_curriculum()
        },
        43,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
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
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });

    env.apply_action(ActionDesc::Pass).unwrap();

    let passes: Vec<u8> = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::PriorityPassed {
                window: TimingWindow::MainWindow,
                pass_count,
                ..
            } = e
            {
                Some(*pass_count)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(passes, vec![1, 2]);
    assert!(env.state.turn.priority.is_none());
    env.validate_state().unwrap();
}
