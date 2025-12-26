use std::sync::Arc;

#[path = "deck_support.rs"]
mod deck_support;

use weiss_core::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use weiss_core::db::{CardColor, CardDb, CardStatic, CardType, TriggerIcon};
use weiss_core::effects::{EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec};
use weiss_core::env::{GameEnv, STACK_AUTO_RESOLVE_CAP};
use weiss_core::legal::ActionDesc;
use weiss_core::replay::{ReplayConfig, ReplayEvent};
use weiss_core::state::{StackItem, TerminalResult};

fn make_db() -> Arc<CardDb> {
    let mut cards = vec![CardStatic {
        id: 1,
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
    }];
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
        deck_ids: [10, 11],
        max_decisions: 10,
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

#[test]
fn auto_resolve_cap_exceeded_sets_engine_error() {
    let db = make_db();
    let mut env = GameEnv::new(
        db,
        make_config(),
        CurriculumConfig::default(),
        99,
        replay_config(),
        None,
        0,
    );

    let spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::System, 0, 0, 0),
        kind: EffectKind::TriggerIcon {
            icon: TriggerIcon::Soul,
        },
        target: None,
        optional: false,
    };
    let payload = EffectPayload {
        spec,
        targets: Vec::new(),
    };

    let count = (STACK_AUTO_RESOLVE_CAP + 1) as usize;
    let mut stack = Vec::with_capacity(count);
    for idx in 0..count {
        stack.push(StackItem {
            id: idx as u32 + 1,
            controller: 0,
            source_id: 0,
            effect_id: payload.spec.id,
            payload: payload.clone(),
        });
    }
    env.state.turn.stack = stack;

    env.apply_action(ActionDesc::MulliganConfirm).unwrap();

    assert!(env.last_engine_error);
    assert!(matches!(env.state.terminal, Some(TerminalResult::Timeout)));
    assert!(env.replay_events.iter().any(|e| matches!(
        e,
        ReplayEvent::AutoResolveCapExceeded { cap, .. } if *cap == STACK_AUTO_RESOLVE_CAP
    )));
}
