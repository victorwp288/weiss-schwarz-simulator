mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, DecisionKind};
use weiss_core::state::AttackType;

#[test]
fn full_turn_cycle_golden() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        42,
        Default::default(),
        None,
        0,
    );
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Clock);
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Main);
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Climax);
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::AttackDeclaration
    );
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.state.turn.active_player, 1);
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Clock);
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Main);
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Climax);
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::AttackDeclaration
    );
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::AttackDeclaration
    );
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.state.turn.active_player, 0);
}
