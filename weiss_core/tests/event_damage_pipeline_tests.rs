mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::ReplayEvent;
use weiss_core::state::{DamageType, Phase};

#[test]
fn effect_damage_from_event_uses_pipeline() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_EVENT_DAMAGE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let curriculum = CurriculumConfig {
        enable_triggers: false,
        ..Default::default()
    };
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 26, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![CARD_EVENT_DAMAGE],
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

    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });

    env.apply_action(ActionDesc::MainPlayEvent { hand_index: 0 })
        .unwrap();
    let effect_intent = env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::DamageIntent {
                damage_type: DamageType::Effect,
                ..
            }
        )
    });
    assert!(effect_intent);
    env.validate_state().unwrap();
}
