mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::state::AttackType;

#[test]
fn direct_attack_adds_soul() {
    let db = make_db();
    let deck_a = vec![3; 20];
    let deck_b = vec![3; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 11, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let defender = 1 - env.state.turn.active_player as usize;
    let clock_before = env.state.players[defender].clock.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let clock_after = env.state.players[defender].clock.len();
    assert_eq!(clock_after - clock_before, 3);
}

#[test]
fn side_attack_reduces_damage() {
    let db = make_db();
    let deck_a = vec![3; 20];
    let deck_b = vec![3; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 12, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let defender = 1 - env.state.turn.active_player as usize;
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].stage[0].card = Some(card);
    }
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].level.push(card);
    }
    let clock_before = env.state.players[defender].clock.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Side,
    })
    .unwrap();
    let clock_after = env.state.players[defender].clock.len();
    assert_eq!(clock_after - clock_before, 1);
}

#[test]
fn damage_cancel_on_climax() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![4; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 15, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let defender = 1 - env.state.turn.active_player as usize;
    let clock_before = env.state.players[defender].clock.len();
    let waiting_before = env.state.players[defender].waiting_room.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let clock_after = env.state.players[defender].clock.len();
    let waiting_after = env.state.players[defender].waiting_room.len();
    assert_eq!(clock_after, clock_before);
    assert!(waiting_after > waiting_before);
}
