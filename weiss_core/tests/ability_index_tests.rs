use std::sync::Arc;
use weiss_core::db::{
    AbilityDef, AbilityKind, AbilityTemplate, AbilityTiming, CardColor, CardDb, CardStatic,
    CardType, EffectTemplate,
};
use weiss_core::effects::{EffectKind, EffectSourceKind};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, Decision, DecisionKind};
use weiss_core::replay::ReplayConfig;
use weiss_core::state::{CardInstance, Phase, StageSlot, StageStatus};

#[test]
fn ability_index_ordering_matches_specs() {
    let ability_def = AbilityDef {
        kind: AbilityKind::Activated,
        timing: Some(AbilityTiming::MainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        targets: Vec::new(),
    };
    let card = CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 0,
        soul: 1,
        triggers: Vec::new(),
        traits: Vec::new(),
        abilities: vec![AbilityTemplate::ActivatedPlaceholder],
        ability_defs: vec![ability_def.clone()],
        counter_timing: false,
        raw_text: None,
    };
    let db = Arc::new(CardDb::new(vec![card]).expect("db"));
    let specs = db.iter_card_abilities_in_canonical_order(1);
    assert_eq!(specs.len(), 2);
    assert!(matches!(
        specs[0].template,
        AbilityTemplate::ActivatedPlaceholder
    ));
    assert!(matches!(specs[1].template, AbilityTemplate::AbilityDef(_)));

    let legacy_effects = db.compiled_effects_for_ability(1, 0);
    let def_effects = db.compiled_effects_for_ability(1, 1);
    assert!(legacy_effects
        .iter()
        .any(|effect| matches!(effect.kind, EffectKind::AddModifier { .. })));
    assert!(def_effects
        .iter()
        .any(|effect| matches!(effect.kind, EffectKind::Draw { count } if count == 1)));
}

#[test]
fn priority_actions_and_replays_use_canonical_ability_indices() {
    let ability_def = AbilityDef {
        kind: AbilityKind::Activated,
        timing: Some(AbilityTiming::MainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        targets: Vec::new(),
    };
    let card = CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 0,
        soul: 1,
        triggers: Vec::new(),
        traits: Vec::new(),
        abilities: vec![AbilityTemplate::ActivatedPlaceholder],
        ability_defs: vec![ability_def],
        counter_timing: false,
        raw_text: None,
    };
    let db = Arc::new(CardDb::new(vec![card]).expect("db"));

    let mut curriculum = weiss_core::config::CurriculumConfig::default();
    curriculum.enable_priority_windows = true;
    curriculum.priority_autopick_single_action = false;
    let config = weiss_core::config::EnvConfig {
        deck_lists: [vec![1; 10], vec![1; 10]],
        deck_ids: [1, 2],
        max_decisions: 50,
        max_ticks: 10_000,
        reward: weiss_core::config::RewardConfig::default(),
        error_policy: weiss_core::config::ErrorPolicy::Strict,
        observation_visibility: weiss_core::config::ObservationVisibility::Public,
        end_condition_policy: Default::default(),
    };
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = GameEnv::new(db.clone(), config, curriculum, 33, replay_config, None);
    env.config.deck_lists[0] = vec![1];
    env.config.deck_lists[1] = Vec::new();
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
    let mut slot = StageSlot::empty();
    slot.card = Some(CardInstance::new(1, 0, 1));
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

    let choice = env.state.turn.choice.as_ref().expect("priority choice");
    assert_eq!(choice.options.len(), 2);
    assert_eq!(choice.options[0].target_slot, Some(0));
    assert_eq!(choice.options[1].target_slot, Some(1));

    env.apply_action(ActionDesc::ChoiceSelect { index: 1 })
        .unwrap();
    assert!(env.replay_events.iter().any(|e| matches!(
        e,
        weiss_core::replay::ReplayEvent::StackPushed { item }
            if item.effect_id.source_kind == EffectSourceKind::Activated
                && item.effect_id.ability_index == 1
                && matches!(item.payload.spec.kind, EffectKind::Draw { .. })
    )));
}
