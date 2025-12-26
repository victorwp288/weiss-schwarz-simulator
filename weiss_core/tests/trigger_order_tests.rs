mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, DecisionKind};
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, PendingTrigger, TriggerEffect};

#[test]
fn trigger_orders_when_both_players_trigger() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_END_DRAW]);
    let deck_b = build_deck_list(20, &[CARD_END_DRAW]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        20,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(3, CARD_END_DRAW)],
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
        vec![(3, CARD_END_DRAW)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Pass).unwrap();

    let triggers: Vec<u8> = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::TriggerResolved { player, effect, .. } = e {
                if matches!(effect, TriggerEffect::AutoAbility { .. }) {
                    return Some(*player);
                }
            }
            None
        })
        .collect();
    assert_eq!(triggers, vec![0, 1]);
    env.validate_state().unwrap();
}

#[test]
fn trigger_order_active_resolves_before_opponent_order() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        23,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
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

    env.state.turn.pending_triggers = vec![
        PendingTrigger {
            id: 1,
            group_id: 42,
            player: 0,
            source_card: CARD_BASIC,
            effect: TriggerEffect::Draw,
            effect_id: None,
        },
        PendingTrigger {
            id: 2,
            group_id: 42,
            player: 1,
            source_card: CARD_BASIC,
            effect: TriggerEffect::Draw,
            effect_id: None,
        },
        PendingTrigger {
            id: 3,
            group_id: 42,
            player: 1,
            source_card: CARD_BASIC,
            effect: TriggerEffect::Soul,
            effect_id: None,
        },
    ];
    env.state.turn.next_trigger_id = 4;
    env.state.turn.next_trigger_group_id = 43;
    env.state.turn.trigger_order = None;

    env.apply_action(ActionDesc::Pass).unwrap();

    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::TriggerOrder
    );
    assert_eq!(env.decision.as_ref().unwrap().player, 1);
    let resolved_players: Vec<u8> = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::TriggerResolved { player, .. } = e {
                Some(*player)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(resolved_players, vec![0]);
    assert_eq!(env.state.turn.pending_triggers.len(), 2);
    assert!(env
        .state
        .turn
        .pending_triggers
        .iter()
        .all(|t| t.player == 1));
    env.validate_state().unwrap();
}

#[test]
fn player_orders_own_simultaneous_triggers_decision() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC, CARD_TRIGGER_MULTI]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: true,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 21, replay_config(), None, 0);

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
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::TriggerOrder
    );

    env.apply_action(ActionDesc::TriggerOrder { index: 1 })
        .unwrap();

    let resolved: Vec<TriggerEffect> = env
        .replay_events
        .iter()
        .filter_map(|e| {
            if let ReplayEvent::TriggerResolved { effect, .. } = e {
                Some(*effect)
            } else {
                None
            }
        })
        .collect();
    assert!(resolved.len() >= 2);
    assert_eq!(resolved[0], TriggerEffect::Draw);
    env.validate_state().unwrap();
}

#[test]
fn trigger_source_leaves_play_last_known_info() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_END_DRAW, CARD_END_DRAW_DOUBLE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        22,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(0, CARD_END_DRAW), (3, CARD_END_DRAW_DOUBLE)],
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
    force_attack_decision(&mut env, 0);
    env.state.turn.pending_triggers.clear();
    env.state.turn.pending_triggers_sorted = true;
    env.state.turn.trigger_order = Some(weiss_core::state::TriggerOrderState {
        group_id: 1,
        player: 0,
        choices: vec![1],
    });
    env.state.turn.pending_triggers.push(PendingTrigger {
        id: 1,
        group_id: 1,
        player: 0,
        source_card: CARD_END_DRAW_DOUBLE,
        effect: TriggerEffect::AutoAbility { ability_index: 0 },
        effect_id: None,
    });
    env.state.turn.pending_triggers_sorted = false;
    env.state.turn.next_trigger_id = 2;
    env.decision = Some(weiss_core::legal::Decision {
        player: 0,
        kind: DecisionKind::TriggerOrder,
        focus_slot: None,
    });

    let card = env.state.players[0].stage[3].card.take().unwrap();
    env.state.players[0].waiting_room.push(card);

    env.apply_action(ActionDesc::TriggerOrder { index: 0 })
        .unwrap();

    let canceled = env
        .replay_events
        .iter()
        .any(|e| matches!(e, ReplayEvent::TriggerCanceled { .. }));
    assert!(!canceled);
    env.validate_state().unwrap();
}
