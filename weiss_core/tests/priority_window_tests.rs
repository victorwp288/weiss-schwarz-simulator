use std::sync::Arc;

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{AbilityTemplate, CardColor, CardDb, CardStatic, CardType};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::ReplayConfig;
use weiss_core::state::{CardInstance, Phase, StageSlot, StageStatus};

const CARD_BASIC: u32 = 1;
const CARD_ACT: u32 = 2;

fn make_instance(card_id: u32, owner: u8, zone_tag: u32, index: usize) -> CardInstance {
    let instance_id = ((owner as u32) << 24) | (zone_tag << 16) | (index as u32);
    CardInstance::new(card_id, owner, instance_id)
}

fn make_db() -> Arc<CardDb> {
    let mut cards = vec![
        CardStatic {
            id: CARD_BASIC,
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
        },
        CardStatic {
            id: CARD_ACT,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedTargetedPower {
                amount: 1000,
                count: 1,
                target: weiss_core::db::TargetTemplate::SelfStage,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    deck_support::add_clone_cards(&mut cards);
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    let pool = [CARD_BASIC, CARD_ACT];
    EnvConfig {
        deck_lists: [
            deck_support::legalize_deck(deck_a, &pool),
            deck_support::legalize_deck(deck_b, &pool),
        ],
        deck_ids: [200, 201],
        max_decisions: 100,
        max_ticks: 100,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn set_main_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player,
        kind: DecisionKind::Main,
        focus_slot: None,
    });
}

#[test]
fn main_pass_skips_priority_when_disabled_but_main_actions_remain() {
    let db = make_db();
    let config = make_config(vec![CARD_BASIC; 50], vec![CARD_BASIC; 50]);
    let replay_config = ReplayConfig {
        enabled: false,
        sample_rate: 0.0,
        ..Default::default()
    };
    let curriculum = CurriculumConfig {
        enable_priority_windows: false,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 5, replay_config, None, 0);

    let card = env.state.players[0].deck.pop().expect("deck card");
    env.state.players[0].hand.push(card);
    set_main_decision(&mut env, 0);

    let actions = weiss_core::legal::legal_actions(
        &env.state,
        env.decision.as_ref().unwrap(),
        &env.db,
        &env.curriculum,
    );
    assert!(actions
        .iter()
        .any(|action| matches!(action, ActionDesc::MainPlayCharacter { .. })));

    env.apply_action(ActionDesc::Pass).unwrap();
    assert!(env.state.turn.priority.is_none());
    assert!(matches!(
        env.decision.as_ref().map(|d| d.kind),
        Some(DecisionKind::Climax)
    ));
}

#[test]
fn priority_window_closes_with_no_actions() {
    let db = make_db();
    let config = make_config(vec![CARD_BASIC; 50], vec![CARD_BASIC; 50]);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        99,
        replay_config,
        None,
        0,
    );

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::Pass).unwrap();

    assert!(env.state.turn.priority.is_none());
    assert!(env.state.terminal.is_none());
    assert!(matches!(
        env.decision.as_ref().map(|d| d.kind),
        Some(DecisionKind::Climax) | Some(DecisionKind::AttackDeclaration)
    ));
    assert!(env.state.turn.tick_count < env.config.max_ticks);
}

#[test]
fn priority_single_action_autopick_does_not_repeat() {
    let db = make_db();
    let config = make_config(vec![CARD_ACT; 50], vec![CARD_BASIC; 50]);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        priority_allow_pass: false,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, curriculum, 100, replay_config, None, 0);

    env.config.deck_lists = [vec![CARD_ACT, CARD_BASIC], vec![CARD_BASIC]];
    for player in 0..2 {
        env.state.players[player].deck.clear();
        env.state.players[player].hand.clear();
        env.state.players[player].waiting_room.clear();
        env.state.players[player].clock.clear();
        env.state.players[player].level.clear();
        env.state.players[player].stock.clear();
        env.state.players[player].memory.clear();
        env.state.players[player].climax.clear();
        env.state.players[player].resolution.clear();
        env.state.players[player].stage = [
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
        ];
    }
    env.state.players[0].deck = vec![make_instance(CARD_BASIC, 0, 8, 0)];
    env.state.players[1].deck = vec![make_instance(CARD_BASIC, 1, 8, 0)];

    let mut slot = StageSlot::empty();
    slot.card = Some(make_instance(CARD_ACT, 0, 4, 0));
    slot.status = StageStatus::Stand;
    env.state.players[0].stage[0] = slot;

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::Pass).unwrap();

    let pushes = env
        .replay_events
        .iter()
        .filter(|e| {
            matches!(e,
                weiss_core::replay::ReplayEvent::StackPushed { item } if item.source_id == CARD_ACT
            )
        })
        .count();
    assert_eq!(pushes, 1);
    assert!(env.state.turn.priority.is_none());
    assert!(env.state.terminal.is_none());
}
