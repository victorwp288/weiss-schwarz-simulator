use std::sync::Arc;

#[path = "scenario_support.rs"]
mod scenario_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{
    AbilityTemplate, CardColor, CardDb, CardStatic, CardType, TargetTemplate, TriggerIcon,
};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{AttackType, Phase, StageStatus};

const CARD_BASIC: u32 = 1;
const CARD_CONTROL_CHANGE: u32 = 50;
const CARD_BOUNCE: u32 = 51;

fn enable_validate() {
    scenario_support::enable_validate();
}

fn replay_config() -> ReplayConfig {
    scenario_support::replay_config()
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
            id: CARD_CONTROL_CHANGE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::ActivatedChangeController {
                count: 1,
                target: TargetTemplate::OppFrontRow,
            }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_BOUNCE,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Bounce],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    scenario_support::make_db(cards)
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    let pool = [CARD_BASIC];
    EnvConfig {
        deck_lists: [
            scenario_support::legalize_deck(deck_a, &pool),
            scenario_support::legalize_deck(deck_b, &pool),
        ],
        deck_ids: [400, 401],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn choose_priority_activation(env: &mut GameEnv) {
    scenario_support::choose_priority_activation(env);
}

fn build_deck_list(size: usize, extras: &[u32]) -> Vec<u32> {
    scenario_support::build_deck_list(size, extras, CARD_BASIC)
}

#[allow(clippy::too_many_arguments)]
fn setup_player_state(
    env: &mut GameEnv,
    player: usize,
    hand: Vec<u32>,
    stock: Vec<u32>,
    stage_cards: Vec<(usize, u32)>,
    deck_top: Vec<u32>,
    clock: Vec<u32>,
    level: Vec<u32>,
    waiting_room: Vec<u32>,
    memory: Vec<u32>,
    climax: Vec<u32>,
) {
    scenario_support::setup_player_state_with_stage_zone_tag(
        env,
        player,
        hand,
        stock,
        stage_cards,
        deck_top,
        clock,
        level,
        waiting_room,
        memory,
        climax,
        9,
    );
}

fn force_main_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.turn_number = 1;
    env.state.turn.attack_subphase_count = 0;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.target_selection = None;
    env.state.turn.priority = None;
    env.state.turn.stack.clear();
    env.state.turn.pending_stack_groups.clear();
    env.state.turn.stack_order = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.state.turn.main_passed = false;
    env.decision = Some(Decision {
        player,
        kind: DecisionKind::Main,
        focus_slot: None,
    });
}

fn force_attack_decision(env: &mut GameEnv, player: u8) {
    env.state.turn.phase = Phase::Attack;
    env.state.turn.active_player = player;
    env.state.turn.starting_player = player;
    env.state.turn.turn_number = 1;
    env.state.turn.attack_subphase_count = 0;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.target_selection = None;
    env.state.turn.priority = None;
    env.state.turn.stack.clear();
    env.state.turn.pending_stack_groups.clear();
    env.state.turn.stack_order = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.state.turn.main_passed = false;
    env.decision = Some(Decision {
        player,
        kind: DecisionKind::AttackDeclaration,
        focus_slot: None,
    });
}

#[test]
fn control_change_allows_attack() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CONTROL_CHANGE]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        ..Default::default()
    };
    let mut env = GameEnv::new_or_panic(db, config, curriculum, 130, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(1, CARD_CONTROL_CHANGE)],
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
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let moved = env.state.players[0].stage[0].card.expect("controlled card");
    assert_eq!(moved.id, CARD_BASIC);
    assert_eq!(moved.owner, 1);
    assert_eq!(moved.controller, 0);
    assert!(env.state.players[1].stage[0].card.is_none());

    let control_logged = env.replay_events.iter().any(|e| matches!(e,
        ReplayEvent::ControlChanged { card, owner, from_controller, to_controller, from_slot: 0, to_slot: 0 }
        if *card == CARD_BASIC && *owner == 1 && *from_controller == 1 && *to_controller == 0
    ));
    assert!(control_logged);

    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
}

#[test]
fn control_persists_after_leaving_stage() {
    enable_validate();
    let db = make_db();
    let deck_a = build_deck_list(20, &[CARD_CONTROL_CHANGE, CARD_BOUNCE]);
    let deck_b = build_deck_list(20, &[]);
    let config = make_config(deck_a, deck_b);
    let curriculum = CurriculumConfig {
        enable_priority_windows: true,
        enable_encore: false,
        ..Default::default()
    };
    let mut env = GameEnv::new_or_panic(db, config, curriculum, 131, replay_config(), None, 0);

    setup_player_state(
        &mut env,
        0,
        vec![],
        vec![],
        vec![(1, CARD_CONTROL_CHANGE), (2, CARD_BASIC)],
        vec![CARD_BOUNCE],
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
        vec![(0, CARD_BASIC)],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    force_main_decision(&mut env, 0);
    env.validate_state().unwrap();

    env.apply_action(ActionDesc::Pass).unwrap();
    choose_priority_activation(&mut env);

    let controlled_on_stage = env.state.players[0].stage[0]
        .card
        .expect("controlled card on stage");
    assert_eq!(controlled_on_stage.owner, 1);
    assert_eq!(controlled_on_stage.controller, 0);
    env.state.players[0].stage[0].status = StageStatus::Reverse;
    force_attack_decision(&mut env, 0);
    env.apply_action(ActionDesc::Pass).unwrap();

    let controlled_in_waiting_room = env.state.players[0]
        .waiting_room
        .iter()
        .find(|c| c.id == CARD_BASIC && c.owner == 1)
        .map(|c| c.controller)
        .unwrap_or(1);
    assert_eq!(controlled_in_waiting_room, 0);
}
