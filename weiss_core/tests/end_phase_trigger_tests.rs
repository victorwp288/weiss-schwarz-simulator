mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, DecisionKind};

#[test]
fn end_of_turn_triggers_fire_then_state_stabilizes() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_END_DRAW]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        25,
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
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_attack_decision(&mut env, 0);

    let hand_before = env.state.players[0].hand.len();
    env.apply_action(ActionDesc::Pass).unwrap();

    let hand_after = env.state.players[0].hand.len();
    assert_eq!(hand_after, hand_before + 1);
    assert!(env.state.turn.pending_triggers.is_empty());
    assert!(env.state.turn.trigger_order.is_none());
    assert!(!env.state.turn.end_phase_pending);
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Clock);
    env.validate_state().unwrap();
}
