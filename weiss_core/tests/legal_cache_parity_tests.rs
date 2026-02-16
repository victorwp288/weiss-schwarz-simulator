mod combat_support;

use combat_support::*;
use weiss_core::config::CurriculumConfig;
use weiss_core::encode::action_id_for;
use weiss_core::env::{legal_action_ids_cached_into, GameEnv};
use weiss_core::legal::{legal_actions_cached, Decision, DecisionKind, LegalActionIds};
use weiss_core::state::{ModifierDuration, ModifierKind, Phase};

fn assert_action_descriptor_and_id_parity(env: &GameEnv) {
    let decision = env.decision.as_ref().expect("decision");
    let actions = legal_actions_cached(&env.state, decision, &env.db, &env.curriculum, None);
    let mut ids = LegalActionIds::new();
    legal_action_ids_cached_into(
        &env.state,
        decision,
        &env.db,
        &env.curriculum,
        None,
        &mut ids,
    );
    let action_ids: Vec<u16> = actions
        .iter()
        .map(|action| action_id_for(action).expect("encode action id") as u16)
        .collect();
    assert_eq!(ids.as_slice(), action_ids.as_slice());
}

#[test]
fn legal_cache_ids_match_descriptors_with_and_without_modifiers() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_BASIC, CARD_EVENT_DAMAGE]);
    let deck_b = build_deck_list(20, &[CARD_BASIC]);
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(
        db,
        config,
        CurriculumConfig::default(),
        332,
        replay_config(),
        None,
        0,
    );
    setup_player_state(
        &mut env,
        0,
        vec![CARD_BASIC, CARD_EVENT_DAMAGE],
        vec![],
        vec![(0, CARD_BASIC), (1, CARD_BASIC)],
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

    assert_action_descriptor_and_id_parity(&env);

    env.add_modifier(
        CARD_BASIC,
        0,
        0,
        ModifierKind::CannotPlayEventsFromHand,
        1,
        ModifierDuration::UntilEndOfTurn,
    )
    .expect("add CannotPlayEventsFromHand");
    env.add_modifier(
        CARD_BASIC,
        0,
        0,
        ModifierKind::CannotMoveStagePosition,
        1,
        ModifierDuration::UntilEndOfTurn,
    )
    .expect("add CannotMoveStagePosition");

    assert_action_descriptor_and_id_parity(&env);
    env.validate_state().expect("state valid");
}
