mod engine_support;

use engine_support::*;
use weiss_core::config::ErrorPolicy;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;

#[test]
fn illegal_mainplay_lenient_noop_no_hand_leak() {
    let db = make_db();
    let deck_a = vec![9; 50];
    let deck_b = vec![9; 50];
    let mut config = make_config(deck_a, deck_b);
    config.error_policy = ErrorPolicy::LenientNoop;
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        7,
        Default::default(),
        None,
        0,
    );
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    let hand_len = env.state.players[env.state.turn.active_player as usize]
        .hand
        .len();
    let _ = env
        .apply_action(ActionDesc::MainPlayCharacter {
            hand_index: 0,
            stage_slot: 0,
        })
        .unwrap();
    let hand_after = env.state.players[env.state.turn.active_player as usize]
        .hand
        .len();
    assert_eq!(hand_len, hand_after);
}

#[test]
fn illegal_mainplay_lenient_terminate_no_hand_leak() {
    let db = make_db();
    let deck_a = vec![9; 50];
    let deck_b = vec![9; 50];
    let mut config = make_config(deck_a, deck_b);
    config.error_policy = ErrorPolicy::LenientTerminate;
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        9,
        Default::default(),
        None,
        0,
    );
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    let hand_len = env.state.players[env.state.turn.active_player as usize]
        .hand
        .len();
    let outcome = env
        .apply_action(ActionDesc::MainPlayCharacter {
            hand_index: 0,
            stage_slot: 0,
        })
        .unwrap();
    let hand_after = env.state.players[env.state.turn.active_player as usize]
        .hand
        .len();
    assert!(outcome.terminated);
    assert_eq!(hand_len, hand_after);
}
