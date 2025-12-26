mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::effects::EffectKind;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, ChoiceReason, Phase, TimingWindow};

#[test]
fn counter_priority_autoplays_single_counter() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_COUNTER_REDUCE]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 40, replay_config(), None, 0);

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
    assert_eq!(
        presented.0[0].reference.zone,
        weiss_core::state::ChoiceZone::PriorityCounter
    );
    assert_eq!(
        presented.0[1].reference.zone,
        weiss_core::state::ChoiceZone::PriorityPass
    );

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    let pushed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackPushed { item } if matches!(item.payload.spec.kind, EffectKind::CounterDamageReduce { .. }) && item.source_id == CARD_COUNTER_REDUCE
    ));
    let resolved = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackResolved { item } if matches!(item.payload.spec.kind, EffectKind::CounterDamageReduce { .. }) && item.source_id == CARD_COUNTER_REDUCE
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
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 41, replay_config(), None, 0);

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
    let id0 = if ref0.instance_id != 0 {
        ref0.instance_id
    } else {
        ref0.card_id
    };
    let id1 = if ref1.instance_id != 0 {
        ref1.instance_id
    } else {
        ref1.card_id
    };
    let option_id_0 = (id0 as u64) << 32 | (12u64 << 24);
    let option_id_1 = (id1 as u64) << 32 | (12u64 << 24) | (1u64 << 8);
    assert_eq!(options[0].option_id, option_id_0);
    assert_eq!(options[1].option_id, option_id_1);
    assert_eq!(ref2.zone, weiss_core::state::ChoiceZone::PriorityPass);

    env.apply_action(ActionDesc::ChoiceSelect { index: 1 })
        .unwrap();
    let pushed = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::StackPushed { item } if matches!(item.payload.spec.kind, EffectKind::CounterDamageCancel) && item.source_id == CARD_COUNTER_CANCEL
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
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig {
            enable_priority_windows: true,
            ..Default::default()
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
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig {
            enable_priority_windows: true,
            ..Default::default()
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
