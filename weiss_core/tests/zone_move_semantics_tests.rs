mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::events::Zone;
use weiss_core::legal::ActionDesc;
use weiss_core::replay::ReplayEvent;
use weiss_core::state::AttackType;

#[test]
fn reversed_stage_cleanup_emits_zone_move() {
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC, CARD_HIGH_POWER]);
    let deck_b = build_deck_list(20, &[CARD_HIGH_POWER, CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_encore: false,
        enable_triggers: false,
        ..Default::default()
    };
    let replay_config = replay_config();
    let mut env = GameEnv::new(db, config, curriculum, 55, replay_config, None, 0);

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

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();

    let moved = env.replay_events.iter().any(|event| {
        matches!(
            event,
            ReplayEvent::ZoneMove {
                player,
                from: Zone::Stage,
                to: Zone::WaitingRoom,
                from_slot: Some(0),
                ..
            } if *player == 0
        )
    });
    assert!(moved);
}
