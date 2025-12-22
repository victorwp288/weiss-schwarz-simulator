use std::sync::Arc;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{
    AbilityDef, AbilityKind, AbilityTemplate, AbilityTiming, CardColor, CardDb, CardStatic,
    CardType, EffectTemplate, TriggerIcon,
};
use weiss_core::effects::{EffectKind, EffectSourceKind};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{AttackType, CardInstance, Phase, StageSlot, StageStatus};

const CARD_TRIGGER_DRAW: u32 = 100;
const CARD_ATTACKER: u32 = 101;
const CARD_ACTIVATED: u32 = 102;
const CARD_AUTO: u32 = 103;
const CARD_FILLER: u32 = 104;

fn make_db() -> Arc<CardDb> {
    let activated = AbilityDef {
        kind: AbilityKind::Activated,
        timing: Some(AbilityTiming::MainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        targets: Vec::new(),
    };
    let cards = vec![
        CardStatic {
            id: CARD_TRIGGER_DRAW,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Draw],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_ATTACKER,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Blue,
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
            id: CARD_ACTIVATED,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            ability_defs: vec![activated],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_AUTO,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::AutoOnPlayDraw { count: 1 }],
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: CARD_FILLER,
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
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [1, 2],
        max_decisions: 200,
        max_ticks: 10_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    }
}

fn replay_config() -> ReplayConfig {
    ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    }
}

fn stack_pushed_with_source(events: &[ReplayEvent], source: EffectSourceKind) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::StackPushed { item }
                if item.effect_id.source_kind == source
        )
    })
}

fn clear_player_state(env: &mut GameEnv, player: usize) {
    env.state.players[player].deck.clear();
    env.state.players[player].hand.clear();
    env.state.players[player].stock.clear();
    env.state.players[player].waiting_room.clear();
    env.state.players[player].clock.clear();
    env.state.players[player].level.clear();
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

#[test]
fn unified_effects_pipeline_coverage() {
    let db = make_db();

    // Trigger icon -> stack push.
    let mut env = GameEnv::new(
        db.clone(),
        make_config(vec![CARD_ATTACKER; 10], vec![CARD_ATTACKER; 10]),
        CurriculumConfig::default(),
        10,
        replay_config(),
        None,
    );
    clear_player_state(&mut env, 0);
    clear_player_state(&mut env, 1);
    env.config.deck_lists[0] =
        vec![CARD_ATTACKER, CARD_ATTACKER, CARD_TRIGGER_DRAW];
    env.config.deck_lists[1] = vec![CARD_ATTACKER];
    let mut slot = StageSlot::empty();
    slot.card = Some(CardInstance::new(CARD_ATTACKER, 0));
    slot.status = StageStatus::Stand;
    env.state.players[0].stage[0] = slot;
    env.state.players[0].deck = vec![
        CardInstance::new(CARD_ATTACKER, 0),
        CardInstance::new(CARD_TRIGGER_DRAW, 0),
    ];
    env.state.players[1].deck = vec![CardInstance::new(CARD_ATTACKER, 1)];
    env.state.turn.phase = Phase::Attack;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.mulligan_done = [true, true];
    env.state.turn.attack = None;
    env.state.turn.pending_level_up = None;
    env.state.turn.encore_queue.clear();
    env.state.turn.pending_triggers.clear();
    env.state.turn.trigger_order = None;
    env.state.turn.choice = None;
    env.state.turn.priority = None;
    env.state.turn.stack.clear();
    env.state.turn.pending_stack_groups.clear();
    env.state.turn.stack_order = None;
    env.state.turn.derived_attack = None;
    env.state.turn.end_phase_pending = false;
    env.state.turn.main_passed = false;
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::AttackDeclaration,
        focus_slot: None,
    });
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    assert!(env.replay_events.iter().any(|e| {
        matches!(
            e,
            ReplayEvent::StackPushed { item }
                if matches!(item.payload.spec.kind, EffectKind::Draw { .. })
                    && item.effect_id.source_kind == EffectSourceKind::Trigger
        )
    }));

    // Auto ability -> stack push.
    let mut env = GameEnv::new(
        db.clone(),
        make_config(vec![CARD_FILLER; 10], vec![CARD_FILLER; 10]),
        CurriculumConfig::default(),
        11,
        replay_config(),
        None,
    );
    clear_player_state(&mut env, 0);
    clear_player_state(&mut env, 1);
    env.config.deck_lists[0] = vec![CARD_AUTO, CARD_FILLER];
    env.config.deck_lists[1] = vec![CARD_FILLER];
    env.state.players[0].hand = vec![CardInstance::new(CARD_AUTO, 0)];
    env.state.players[0].deck = vec![CardInstance::new(CARD_FILLER, 0)];
    env.state.players[1].deck = vec![CardInstance::new(CARD_FILLER, 1)];
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    assert!(stack_pushed_with_source(
        &env.replay_events,
        EffectSourceKind::Auto
    ));

    // Activated ability -> stack push (priority window).
    let mut curriculum = CurriculumConfig::default();
    curriculum.enable_priority_windows = true;
    let mut env = GameEnv::new(
        db.clone(),
        make_config(vec![CARD_FILLER; 10], vec![CARD_FILLER; 10]),
        curriculum,
        12,
        replay_config(),
        None,
    );
    clear_player_state(&mut env, 0);
    clear_player_state(&mut env, 1);
    env.config.deck_lists[0] = vec![CARD_ACTIVATED];
    env.config.deck_lists[1] = vec![CARD_FILLER];
    env.state.players[1].deck = vec![CardInstance::new(CARD_FILLER, 1)];
    let mut slot = StageSlot::empty();
    slot.card = Some(CardInstance::new(CARD_ACTIVATED, 0));
    slot.status = StageStatus::Stand;
    env.state.players[0].stage[0] = slot;
    env.state.turn.phase = Phase::Main;
    env.state.turn.active_player = 0;
    env.state.turn.starting_player = 0;
    env.state.turn.mulligan_done = [true, true];
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Main,
        focus_slot: None,
    });
    env.apply_action(ActionDesc::MainPass).unwrap();
    assert!(stack_pushed_with_source(
        &env.replay_events,
        EffectSourceKind::Activated
    ));

    // Refresh penalty (system effect) -> stack push.
    let mut env = GameEnv::new(
        db.clone(),
        make_config(vec![CARD_FILLER; 10], vec![CARD_FILLER; 10]),
        CurriculumConfig::default(),
        13,
        replay_config(),
        None,
    );
    clear_player_state(&mut env, 0);
    clear_player_state(&mut env, 1);
    let active = env.state.turn.starting_player as usize;
    env.config.deck_lists[active] = vec![CARD_FILLER, CARD_FILLER];
    env.config.deck_lists[1 - active] = vec![CARD_FILLER];
    env.state.players[active].waiting_room = vec![
        CardInstance::new(CARD_FILLER, active as u8),
        CardInstance::new(CARD_FILLER, active as u8),
    ];
    env.state.players[1 - active].deck =
        vec![CardInstance::new(CARD_FILLER, (1 - active) as u8)];
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    assert!(stack_pushed_with_source(
        &env.replay_events,
        EffectSourceKind::System
    ));
}
