use std::sync::Arc;

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::encode::{action_id_for, build_action_mask, CHOICE_COUNT};
use weiss_core::env::GameEnv;
use weiss_core::legal::{legal_actions, ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone};

fn build_db(card_count: u32) -> Arc<CardDb> {
    let mut cards = (1..=card_count)
        .map(|id| CardStatic {
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
        })
        .collect();
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config() -> EnvConfig {
    let pool = [1];
    EnvConfig {
        deck_lists: [
            deck_support::legalize_deck(vec![1; 50], &pool),
            deck_support::legalize_deck(vec![1; 50], &pool),
        ],
        deck_ids: [1, 2],
        max_decisions: 50,
        max_ticks: 10_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn install_choice(env: &mut GameEnv, total: usize) {
    let options = (0..total)
        .map(|idx| ChoiceOptionRef {
            card_id: (idx + 1) as u32,
            instance_id: (idx + 1) as u32,
            zone: ChoiceZone::WaitingRoom,
            index: Some(idx as u8),
            target_slot: None,
        })
        .collect::<Vec<_>>();
    env.state.turn.choice = Some(ChoiceState {
        id: 1,
        reason: ChoiceReason::TriggerTreasureSelect,
        player: 0,
        options,
        total_candidates: total as u16,
        page_start: 0,
        pending_trigger: None,
    });
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Choice,
        focus_slot: None,
    });
}

fn action_mask_for_choice(env: &GameEnv) -> Vec<u8> {
    let decision = env.decision.as_ref().expect("decision");
    let actions = legal_actions(&env.state, decision, &env.db, &env.curriculum);
    let (mask, _lookup) = build_action_mask(&actions);
    mask
}

#[test]
fn choice_paging_navigates_and_selects_deterministically() {
    let db = build_db(40);
    let config = make_config();
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };

    let mut env_a = GameEnv::new(
        db.clone(),
        config.clone(),
        CurriculumConfig::default(),
        777,
        replay_config.clone(),
        None,
        0,
    );
    let mut env_b = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        777,
        replay_config,
        None,
        0,
    );

    let total = CHOICE_COUNT + 4;
    install_choice(&mut env_a, total);
    install_choice(&mut env_b, total);

    let next_id = action_id_for(&ActionDesc::ChoiceNextPage).expect("next id");
    let prev_id = action_id_for(&ActionDesc::ChoicePrevPage).expect("prev id");
    let mask = action_mask_for_choice(&env_a);
    assert_eq!(mask[next_id], 1);
    assert_eq!(mask[prev_id], 0);

    env_a.apply_action(ActionDesc::ChoiceNextPage).unwrap();
    env_b.apply_action(ActionDesc::ChoiceNextPage).unwrap();
    let page_changed = env_a.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::ChoicePageChanged { from_start, to_start, .. }
                if *from_start == 0 && *to_start == CHOICE_COUNT as u16
        )
    });
    assert!(page_changed);

    let page_start_a = env_a.state.turn.choice.as_ref().expect("choice").page_start;
    let page_start_b = env_b.state.turn.choice.as_ref().expect("choice").page_start;
    assert_eq!(page_start_a, CHOICE_COUNT as u16);
    assert_eq!(page_start_a, page_start_b);
    let mask = action_mask_for_choice(&env_a);
    assert_eq!(mask[prev_id], 1);

    env_a
        .apply_action(ActionDesc::ChoiceSelect { index: 2 })
        .unwrap();
    env_b
        .apply_action(ActionDesc::ChoiceSelect { index: 2 })
        .unwrap();

    let selected_a = env_a
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoiceMade { option, .. } => Some(option.card_id),
            _ => None,
        })
        .expect("choice made");
    let selected_b = env_b
        .replay_events
        .iter()
        .rev()
        .find_map(|e| match e {
            ReplayEvent::ChoiceMade { option, .. } => Some(option.card_id),
            _ => None,
        })
        .expect("choice made");
    assert_eq!(selected_a, selected_b);
    assert_eq!(selected_a, (CHOICE_COUNT + 3) as u32);
}
