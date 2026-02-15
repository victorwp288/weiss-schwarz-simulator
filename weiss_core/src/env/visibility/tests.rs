use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardColor, CardDb, CardStatic, CardType};
use crate::env::GameEnv;
use crate::events::Event;
use crate::legal::ActionDesc;
use crate::replay::{ReplayConfig, ReplayEvent};
use crate::state::{ChoiceOptionRef, ChoiceReason, ChoiceZone};
use std::sync::Arc;

fn make_db() -> Arc<CardDb> {
    let mut cards = Vec::new();
    for id in 1..=13 {
        cards.push(CardStatic {
            id,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        });
    }
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_deck() -> Vec<u32> {
    let mut deck = Vec::new();
    for id in 1..=13u32 {
        for _ in 0..4 {
            deck.push(id);
        }
    }
    deck.truncate(50);
    deck
}

fn make_env() -> GameEnv {
    let db = make_db();
    let deck = make_deck();
    let config = EnvConfig {
        deck_lists: [deck.clone(), deck],
        deck_ids: [1, 2],
        max_decisions: 100,
        max_ticks: 1000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    let curriculum = CurriculumConfig {
        enable_visibility_policies: true,
        ..Default::default()
    };
    GameEnv::new_or_panic(db, config, curriculum, 9, ReplayConfig::default(), None, 0)
}

#[test]
fn sanitize_draw_hides_card_ids_in_public() {
    let env = make_env();
    let ctx = env.replay_visibility_context();
    let event = Event::Draw { player: 0, card: 7 };
    let sanitized = env.sanitize_event_for_viewer(&event, ctx);
    match sanitized {
        ReplayEvent::Draw { card, .. } => assert_eq!(card, 0),
        _ => panic!("unexpected replay event"),
    }
}

#[test]
fn sanitize_choice_option_hides_hidden_zone() {
    let mut env = make_env();
    let ctx = env.replay_visibility_context();
    let option = ChoiceOptionRef {
        card_id: 5,
        instance_id: 123,
        zone: ChoiceZone::Hand,
        index: Some(0),
        target_slot: None,
    };
    let sanitized =
        env.sanitize_choice_option_for_event(ChoiceReason::PriorityActionSelect, 0, ctx, &option);
    assert_eq!(sanitized.card_id, 0);
    assert_eq!(sanitized.instance_id, 0);
    assert!(sanitized.index.is_none());

    env.mark_instance_revealed(&[0, 1], 123);
    let revealed =
        env.sanitize_choice_option_for_event(ChoiceReason::PriorityActionSelect, 0, ctx, &option);
    assert_eq!(revealed.card_id, 5);
    assert_eq!(revealed.instance_id, 0);
}

#[test]
fn sanitize_choice_option_strips_instance_id_in_public_replay() {
    let env = make_env();
    let ctx = env.replay_visibility_context();
    let option = ChoiceOptionRef {
        card_id: 7,
        instance_id: 4242,
        zone: ChoiceZone::Stage,
        index: Some(0),
        target_slot: None,
    };
    let sanitized =
        env.sanitize_choice_option_for_event(ChoiceReason::PriorityActionSelect, 0, ctx, &option);
    assert_eq!(sanitized.card_id, 7);
    assert_eq!(sanitized.instance_id, 0);
    assert_eq!(sanitized.index, Some(0));
}

#[test]
fn sanitize_action_masks_hidden_indices() {
    let env = make_env();
    let ctx = env.replay_visibility_context();
    let action = ActionDesc::MulliganSelect { hand_index: 3 };
    let masked = env.sanitize_action_for_viewer(&action, 0, ctx);
    match masked {
        ActionDesc::MulliganSelect { hand_index } => assert_eq!(hand_index, u8::MAX),
        _ => panic!("unexpected masked action"),
    }
}
