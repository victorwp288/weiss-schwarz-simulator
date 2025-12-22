mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::{ReplayConfig, ReplayEvent, ReplayWriter};
use weiss_core::state::AttackType;

#[test]
fn trigger_moves_card_to_stock_and_logs() {
    let db = make_db();
    let deck_a = vec![5; 20];
    let deck_b = vec![5; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = true;
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("trigger");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        include_trigger_card_id: true,
    };
    let writer = Some(ReplayWriter::new(&replay_config).unwrap());
    let mut env = GameEnv::new(db, config, curriculum, 13, replay_config.clone(), writer);
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
    let attacker = env.state.turn.active_player as usize;
    let stock_before = env.state.players[attacker].stock.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let stock_after = env.state.players[attacker].stock.len();
    assert_eq!(stock_after - stock_before, 1);
    assert!(env
        .replay_events
        .iter()
        .any(|e| matches!(e, ReplayEvent::Trigger { .. })));
}

#[test]
fn refresh_penalty_applied() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![1; 20];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        14,
        Default::default(),
        None,
    );
    let active = env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[active].deck);
    env.state.players[active].waiting_room = deck;
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    for _ in 0..12 {
        if env.state.players[active].clock.len() == 1 {
            break;
        }
        let action = env
            .last_legal_actions
            .get(0)
            .cloned()
            .expect("legal action");
        env.apply_action(action).unwrap();
    }
    assert_eq!(env.state.players[active].clock.len(), 1);
    assert!(!env.state.players[active].deck.is_empty());
}
