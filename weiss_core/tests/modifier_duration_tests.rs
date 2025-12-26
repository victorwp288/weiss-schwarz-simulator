mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{AttackType, ModifierDuration, ModifierKind};

#[test]
fn end_of_turn_expirations_remove_modifiers() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        24,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(3, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    env.state.players[0].stage[3].power_mod_turn = 1000;
    env.state.players[0].stage[3].cannot_attack = true;
    env.state.players[0].stage[3].attack_cost = 2;
    force_attack_decision(&mut env, 0);

    env.apply_action(ActionDesc::Pass).unwrap();

    let slot = &env.state.players[0].stage[3];
    assert_eq!(slot.power_mod_turn, 0);
    assert!(!slot.cannot_attack);
    assert_eq!(slot.attack_cost, 0);
    env.validate_state().unwrap();
}

#[test]
fn modifier_until_end_of_turn_expires() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        enable_counters: false,
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
    env.add_modifier(
        CARD_BASIC,
        0,
        0,
        ModifierKind::Power,
        1000,
        ModifierDuration::UntilEndOfTurn,
    );

    let mut obs = vec![0; weiss_core::encode::OBS_LEN];
    weiss_core::encode::encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs,
    );
    assert_eq!(slot_power_from_obs(&obs, 0, 0), 1500);

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();

    assert!(env.state.modifiers.is_empty());
    let removed = env
        .replay_events
        .iter()
        .filter(|e| matches!(e, ReplayEvent::ModifierRemoved { .. }))
        .count();
    assert!(removed >= 1);
    env.validate_state().unwrap();
}

#[test]
fn modifier_while_on_stage_removed_on_leave() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_HIGH_POWER]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        enable_counters: false,
        enable_encore: false,
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
    env.add_modifier(
        CARD_BASIC,
        0,
        0,
        ModifierKind::Power,
        500,
        ModifierDuration::WhileOnStage,
    );

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();

    assert!(env.state.modifiers.is_empty());
    let removed = env
        .replay_events
        .iter()
        .filter(|e| matches!(e, ReplayEvent::ModifierRemoved { .. }))
        .count();
    assert!(removed >= 1);
    env.validate_state().unwrap();
}
