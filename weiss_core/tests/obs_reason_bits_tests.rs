use std::sync::Arc;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType};
use weiss_core::encode::{
    encode_observation, OBS_REASON_BASE, OBS_REASON_IN_MAIN, OBS_REASON_LEN, OBS_REASON_NO_COLOR,
    OBS_REASON_NO_HAND, OBS_REASON_NO_STOCK,
};
use weiss_core::env::GameEnv;
use weiss_core::legal::{Decision, DecisionKind};
use weiss_core::state::{CardInstance, Phase};

fn make_db() -> Arc<CardDb> {
    let mut cards = Vec::new();
    for id in 1..=13u32 {
        let (color, level, cost) = match id {
            1 => (CardColor::Red, 1, 2),
            2 => (CardColor::Blue, 1, 0),
            _ => (CardColor::Red, 0, 0),
        };
        cards.push(CardStatic {
            id,
            card_set: None,
            card_type: CardType::Character,
            color,
            level,
            cost,
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
    for id in 1..=12u32 {
        deck.extend(std::iter::repeat_n(id, 4));
    }
    deck.extend(std::iter::repeat_n(13u32, 2));
    assert_eq!(deck.len(), 50);
    deck
}

fn make_config(deck: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck.clone(), deck],
        deck_ids: [91, 92],
        max_decisions: 10,
        max_ticks: 100,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

#[test]
fn reason_bits_ignore_opponent_hidden_zones() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut env = GameEnv::new(db, config, curriculum, 42, Default::default(), None, 0);

    env.state.turn.phase = Phase::Main;
    let decision = Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    };

    env.state.players[1].hand.clear();
    env.state.players[1].hand.push(CardInstance::new(1, 1, 100));
    let mut obs_a = vec![0; weiss_core::encode::OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        Some(&decision),
        None,
        None,
        ObservationVisibility::Public,
        &mut obs_a,
    );

    env.state.players[1].hand.clear();
    env.state.players[1].hand.push(CardInstance::new(2, 1, 200));
    let mut obs_b = vec![0; weiss_core::encode::OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        Some(&decision),
        None,
        None,
        ObservationVisibility::Public,
        &mut obs_b,
    );

    assert_eq!(
        &obs_a[OBS_REASON_BASE..OBS_REASON_BASE + OBS_REASON_LEN],
        &obs_b[OBS_REASON_BASE..OBS_REASON_BASE + OBS_REASON_LEN]
    );
}

#[test]
fn reason_bits_reflect_resource_blocks_in_main() {
    let db = make_db();
    let config = make_config(make_deck());
    let curriculum = CurriculumConfig::default();
    let mut env = GameEnv::new(db, config, curriculum, 7, Default::default(), None, 0);

    let player = 0usize;
    env.state.players[player].hand.clear();
    env.state.players[player]
        .hand
        .push(CardInstance::new(1, 0, 10));
    env.state.players[player]
        .hand
        .push(CardInstance::new(2, 0, 11));
    env.state.players[player].stock.clear();
    env.state.players[player].level.clear();
    env.state.players[player]
        .level
        .push(CardInstance::new(3, 0, 12));
    env.state.players[player].clock.clear();

    env.state.turn.phase = Phase::Main;
    let decision = Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    };
    let mut obs = vec![0; weiss_core::encode::OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        Some(&decision),
        None,
        None,
        ObservationVisibility::Public,
        &mut obs,
    );
    let reasons = &obs[OBS_REASON_BASE..OBS_REASON_BASE + OBS_REASON_LEN];
    assert_eq!(reasons[OBS_REASON_IN_MAIN], 1);
    assert_eq!(reasons[OBS_REASON_NO_STOCK], 1);
    assert_eq!(reasons[OBS_REASON_NO_COLOR], 1);
    assert_eq!(reasons[OBS_REASON_NO_HAND], 0);
}
