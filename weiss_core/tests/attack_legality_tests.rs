mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::state::{AttackType, Phase, StageStatus};

#[test]
fn cannot_attack_when_rested() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        15,
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
    env.state.players[0].stage[0].status = StageStatus::Rest;
    force_attack_decision(&mut env, 0);

    assert!(env
        .apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Direct
        })
        .is_err());
    env.validate_state().unwrap();
}

#[test]
fn cannot_attack_with_cannot_attack_status() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        160,
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
    env.state.players[0].stage[0].cannot_attack = true;
    force_attack_decision(&mut env, 0);

    assert!(env
        .apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Direct
        })
        .is_err());
    env.validate_state().unwrap();
}

#[test]
fn cannot_attack_from_ability_template() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CANNOT_ATTACK]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        16,
        replay_config(),
        None,
        0,
    );

    setup_player_state(
        &mut env,
        0,
        vec![CARD_CANNOT_ATTACK],
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
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });

    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    let has_attack = env
        .legal_actions()
        .iter()
        .any(|a| matches!(a, ActionDesc::Attack { slot: 0, .. }));
    assert!(!has_attack);
    assert!(env
        .apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Frontal
        })
        .is_err());
    env.validate_state().unwrap();
}

#[test]
fn attack_target_must_be_legal_lane() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        17,
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
    force_attack_decision(&mut env, 0);

    assert!(env
        .apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Frontal
        })
        .is_err());
    env.validate_state().unwrap();
}

#[test]
fn attack_cost_must_be_payable() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        18,
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
    env.state.players[0].stage[0].attack_cost = 2;
    force_attack_decision(&mut env, 0);
    assert!(env
        .apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Direct
        })
        .is_err());

    let card = env.state.players[0].deck.pop().unwrap();
    env.state.players[0].stock.push(card);
    let card = env.state.players[0].deck.pop().unwrap();
    env.state.players[0].stock.push(card);
    env.state.players[0].stage[0].attack_cost = 2;
    let stock_before = env.state.players[0].stock.len();
    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let stock_after = env.state.players[0].stock.len();
    assert_eq!(stock_after, stock_before + 1 - 2);
    env.validate_state().unwrap();
}

#[test]
fn cannot_declare_attack_twice_if_once_per_turn() {
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
    let mut env = GameEnv::new(db, config, curriculum, 19, replay_config(), None, 0);

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
        DecisionKind::AttackDeclaration
    );
    assert!(env
        .apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Frontal
        })
        .is_err());
    env.validate_state().unwrap();
}
