mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::events::Event;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::ReplayConfig;
use weiss_core::state::TerminalResult;

#[test]
fn concede_is_always_legal_and_ends_immediately() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut curriculum = default_curriculum();
    curriculum.allow_concede = true;
    let mut env = GameEnv::new(db, config, curriculum, 77, replay_config, None, 0);

    assert!(env
        .legal_actions()
        .iter()
        .any(|a| matches!(a, ActionDesc::Concede)));

    let conceding_player = env.decision.as_ref().expect("decision").player;
    env.apply_action(ActionDesc::Concede).unwrap();

    assert!(matches!(
        env.state.terminal,
        Some(TerminalResult::Win { winner }) if winner == 1 - conceding_player
    ));
    assert!(env.decision.is_none());
    assert!(env
        .replay_events
        .iter()
        .any(|e| matches!(e, Event::Concede { player } if *player == conceding_player)));
}
