use std::sync::Arc;

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
    let cards = vec![
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
            abilities: vec![AbilityTemplate::ActivatedPlaceholder],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
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
fn priority_window_closes_with_no_actions() {
    let db = make_db();
    let config = make_config(vec![CARD_BASIC; 20], vec![CARD_BASIC; 20]);
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
    );

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPass).unwrap();

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
    let config = make_config(vec![CARD_ACT; 20], vec![CARD_BASIC; 20]);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = GameEnv::new(
        db,
        config,
        CurriculumConfig::default(),
        100,
        replay_config,
        None,
    );

    env.config.deck_lists = [vec![CARD_ACT], vec![CARD_BASIC]];
    for player in 0..2 {
        env.state.players[player].deck.clear();
        env.state.players[player].hand.clear();
        env.state.players[player].waiting_room.clear();
        env.state.players[player].clock.clear();
        env.state.players[player].level.clear();
        env.state.players[player].stock.clear();
        env.state.players[player].memory.clear();
        env.state.players[player].climax.clear();
        env.state.players[player].stage = [
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
        ];
    }
    env.state.players[1].deck = vec![make_instance(CARD_BASIC, 1, 8, 0)];

    let mut slot = StageSlot::empty();
    slot.card = Some(make_instance(CARD_ACT, 0, 4, 0));
    slot.status = StageStatus::Stand;
    env.state.players[0].stage[0] = slot;

    set_main_decision(&mut env, 0);
    env.apply_action(ActionDesc::MainPass).unwrap();

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
