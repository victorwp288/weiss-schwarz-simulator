mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::events::{RevealAudience, RevealReason};
use weiss_core::legal::{ActionDesc, DecisionKind};
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, ChoiceReason, ChoiceZone, StageStatus};

#[test]
fn trigger_gate_choice_skipped_no_candidates() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_GATE, CARD_HIGH_POWER, CARD_CLIMAX]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        30,
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
        vec![CARD_TRIGGER_GATE],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    let skipped = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::ChoiceSkipped {
                reason: ChoiceReason::TargetSelect,
                ..
            }
        )
    });
    assert!(skipped);
    assert!(env.state.players[0].hand.is_empty());
    env.validate_state().unwrap();
}

#[test]
fn trigger_gate_choice_autopicked_single_candidate() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(
        20,
        &[CARD_TRIGGER_GATE, CARD_HIGH_POWER, CARD_CLIMAX, CARD_CLIMAX],
    );
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        31,
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
        vec![CARD_TRIGGER_GATE],
        vec![],
        vec![],
        vec![CARD_CLIMAX],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    let presented = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ChoicePresented { reason: ChoiceReason::TargetSelect, total_candidates, .. } if *total_candidates == 2
    ));
    let autopicked = env
        .replay_events
        .iter()
        .any(|e| matches!(e, ReplayEvent::ChoiceAutopicked { .. }));
    assert!(presented);
    assert!(!autopicked);

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();
    assert_eq!(env.state.players[0].hand.len(), 1);
    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|c| c.id == CARD_CLIMAX));
    env.validate_state().unwrap();
}

#[test]
fn trigger_gate_choice_manual_multiple_candidates() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(
        20,
        &[CARD_TRIGGER_GATE, CARD_HIGH_POWER, CARD_CLIMAX, CARD_CLIMAX],
    );
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        32,
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
        vec![CARD_TRIGGER_GATE],
        vec![],
        vec![],
        vec![CARD_CLIMAX, CARD_CLIMAX],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    let presented = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ChoicePresented { reason: ChoiceReason::TargetSelect, total_candidates, .. } if *total_candidates == 3
    ));
    let made = env
        .replay_events
        .iter()
        .any(|e| matches!(e, ReplayEvent::ChoiceMade { .. }));
    assert!(presented);
    assert!(made);
    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|c| c.id == CARD_CLIMAX));
    env.validate_state().unwrap();
}

#[test]
fn trigger_bounce_choice_moves_stage_card() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_BOUNCE, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        33,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_BASIC), (1, CARD_HIGH_POWER)],
        vec![CARD_TRIGGER_BOUNCE],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    env.apply_action(ActionDesc::ChoiceSelect { index: 1 })
        .unwrap();
    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|c| c.id == CARD_HIGH_POWER));
    assert!(env.state.players[0].stage[1].card.is_none());
    env.validate_state().unwrap();
}

#[test]
fn trigger_standby_choice_skipped_no_candidates() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_STANDBY, CARD_LEVEL_TWO]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        34,
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
        vec![CARD_TRIGGER_STANDBY],
        vec![],
        vec![],
        vec![CARD_LEVEL_TWO],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    let skipped = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::ChoiceSkipped {
                reason: ChoiceReason::TriggerStandbySelect,
                ..
            }
        )
    });
    assert!(skipped);
    assert!(env.state.players[0]
        .waiting_room
        .iter()
        .any(|c| c.id == CARD_LEVEL_TWO));
    assert!(!env.state.players[0]
        .stage
        .iter()
        .any(|slot| slot.card.map(|c| c.id) == Some(CARD_LEVEL_TWO)));
    env.validate_state().unwrap();
}

#[test]
fn trigger_standby_autopick_single_candidate() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_STANDBY, CARD_LEVEL_ONE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        35,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![
            (0, CARD_BASIC),
            (1, CARD_BASIC),
            (2, CARD_BASIC),
            (3, CARD_BASIC),
        ],
        vec![CARD_TRIGGER_STANDBY],
        vec![],
        vec![],
        vec![CARD_LEVEL_ONE],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    let presented = env
        .replay_events
        .iter()
        .any(|e| matches!(e,
            ReplayEvent::ChoicePresented { reason: ChoiceReason::TriggerStandbySelect, total_candidates, .. } if *total_candidates == 6
        ));
    assert!(presented);

    env.apply_action(ActionDesc::ChoiceSelect { index: 4 })
        .unwrap();
    let moved = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ZoneMove { card, from: weiss_core::events::Zone::WaitingRoom, to: weiss_core::events::Zone::Stage, to_slot: Some(4), .. } if *card == CARD_LEVEL_ONE
    ));
    assert!(moved);
    assert_eq!(
        env.state.players[0].stage[4].card.map(|c| c.id),
        Some(CARD_LEVEL_ONE)
    );
    assert_eq!(env.state.players[0].stage[4].status, StageStatus::Rest);
    env.validate_state().unwrap();
}

#[test]
fn trigger_standby_choice_orders_candidates_and_replaces_when_full() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_STANDBY, CARD_LEVEL_ONE, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        36,
        replay_config(),
        None,
        0,
    );

    let mut fillers: Vec<u32> = env.config.deck_lists[0]
        .iter()
        .copied()
        .filter(|id| *id != CARD_TRIGGER_STANDBY && *id != CARD_LEVEL_ONE && *id != CARD_HIGH_POWER)
        .collect();
    fillers.resize(6, CARD_BASIC);
    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![
            (0, fillers[0]),
            (1, CARD_HIGH_POWER),
            (2, fillers[1]),
            (3, fillers[2]),
            (4, fillers[3]),
        ],
        vec![CARD_TRIGGER_STANDBY],
        vec![],
        vec![],
        vec![fillers[4], CARD_LEVEL_ONE],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    let presented = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TriggerStandbySelect,
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
        .expect("standby choice presented");
    assert_eq!(*presented.1, 11);
    let option0 = &presented.0[0].reference;
    assert_eq!(option0.card_id % 1000, CARD_BASIC);
    assert_eq!(option0.zone, ChoiceZone::WaitingRoom);
    assert_eq!(option0.index, Some(0));
    assert_eq!(option0.target_slot, Some(0));
    let option5 = &presented.0[5].reference;
    assert_eq!(option5.card_id % 1000, CARD_LEVEL_ONE);
    assert_eq!(option5.zone, ChoiceZone::WaitingRoom);
    assert_eq!(option5.index, Some(1));
    assert_eq!(option5.target_slot, Some(0));

    let last_option = &presented.0[presented.0.len() - 1].reference;
    assert_eq!(last_option.zone, ChoiceZone::Skip);

    env.apply_action(ActionDesc::ChoiceSelect { index: 6 })
        .unwrap();
    assert_eq!(
        env.state.players[0].stage[1].card.map(|c| c.id % 1000),
        Some(CARD_LEVEL_ONE)
    );
    assert_eq!(env.state.players[0].stage[1].status, StageStatus::Rest);
    assert!(env.state.players[0]
        .waiting_room
        .iter()
        .any(|c| c.id == CARD_HIGH_POWER));
    let replaced = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ZoneMove { card, from: weiss_core::events::Zone::Stage, to: weiss_core::events::Zone::WaitingRoom, from_slot: Some(1), .. } if *card == CARD_HIGH_POWER
    ));
    assert!(replaced);
    env.validate_state().unwrap();
}

#[test]
fn trigger_treasure_choice_stock_top_card() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_TREASURE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        37,
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
        vec![CARD_TRIGGER_TREASURE, CARD_BASIC],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);

    let reveal_ok = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::Reveal { card, reason: RevealReason::TriggerCheck, audience: RevealAudience::Public, .. } if *card == CARD_TRIGGER_TREASURE
    ));
    assert!(reveal_ok);

    let (options, _) = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TriggerTreasureSelect,
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
        .expect("treasure choice presented");
    assert_eq!(options.len(), 2);
    assert_ne!(options[0].option_id, options[1].option_id);
    assert!(matches!(options[0].reference.zone, ChoiceZone::DeckTop));
    assert!(matches!(options[1].reference.zone, ChoiceZone::DeckTop));

    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|c| c.id == CARD_TRIGGER_TREASURE));
    assert!(env.state.players[0]
        .stock
        .iter()
        .any(|c| c.id == CARD_BASIC));
    let moved_to_hand = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ZoneMove { card, from: weiss_core::events::Zone::Resolution, to: weiss_core::events::Zone::Hand, .. } if *card == CARD_TRIGGER_TREASURE
    ));
    let moved_to_stock = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ZoneMove { card, from: weiss_core::events::Zone::Deck, to: weiss_core::events::Zone::Stock, .. } if *card == CARD_BASIC
    ));
    assert!(moved_to_hand);
    assert!(moved_to_stock);
    env.validate_state().unwrap();
}

#[test]
fn trigger_treasure_choice_skip() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_TREASURE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        38,
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
        vec![CARD_TRIGGER_TREASURE, CARD_BASIC],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let skip_index = env
        .state
        .turn
        .choice
        .as_ref()
        .and_then(|choice| {
            choice.options.iter().enumerate().find_map(|(idx, opt)| {
                if opt.index == Some(1) {
                    Some(idx as u8)
                } else {
                    None
                }
            })
        })
        .expect("treasure skip option");
    env.apply_action(ActionDesc::ChoiceSelect { index: skip_index })
        .unwrap();

    assert!(env.state.players[0]
        .hand
        .iter()
        .any(|c| c.id == CARD_TRIGGER_TREASURE));
    assert!(env.state.players[0].stock.is_empty());
    let moved_to_stock = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::ZoneMove {
                card,
                from: weiss_core::events::Zone::Deck,
                to: weiss_core::events::Zone::Stock,
                ..
            } if *card == CARD_BASIC
        )
    });
    assert!(!moved_to_stock);
    env.validate_state().unwrap();
}

#[test]
fn reveal_then_move_zone_is_logged_and_correct() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_TRIGGER_MULTI]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 23, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_BASIC)],
        vec![CARD_TRIGGER_MULTI],
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
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    let reveal_index = env
        .replay_events
        .iter()
        .position(|e| matches!(e, ReplayEvent::Reveal { card, .. } if *card == CARD_TRIGGER_MULTI))
        .unwrap();
    let trigger_index = env
        .replay_events
        .iter()
        .position(|e| matches!(e, ReplayEvent::TriggerQueued { .. }));
    if let Some(trigger_index) = trigger_index {
        assert!(reveal_index < trigger_index);
    }
    assert!(env.state.players[0]
        .stock
        .iter()
        .any(|c| c.id == CARD_TRIGGER_MULTI));
    env.validate_state().unwrap();
}
