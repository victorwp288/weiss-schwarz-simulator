use std::sync::Arc;

#[path = "deck_support.rs"]
mod deck_support;
use weiss_core::db::{
    AbilityCost, AbilityDef, AbilityKind, AbilityTemplate, AbilityTiming, CardColor, CardDb,
    CardStatic, CardType, EffectTemplate,
};
use weiss_core::effects::{EffectKind, EffectSourceKind};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, DecisionKind};
use weiss_core::replay::ReplayConfig;
use weiss_core::state::ChoiceZone;

#[test]
fn ability_index_ordering_matches_specs() {
    let ability_def = AbilityDef {
        kind: AbilityKind::Activated,
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        targets: Vec::new(),
        cost: AbilityCost::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_limit: None,
    };
    let card = CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: Vec::new(),
        traits: Vec::new(),
        abilities: vec![AbilityTemplate::ActivatedTargetedPower {
            amount: 1000,
            count: 1,
            target: weiss_core::db::TargetTemplate::SelfStage,
        }],
        ability_defs: vec![ability_def.clone()],
        counter_timing: false,
        raw_text: None,
    };
    let mut cards = vec![card];
    deck_support::add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db"));
    let specs = db.iter_card_abilities_in_canonical_order(1);
    assert_eq!(specs.len(), 2);
    assert!(matches!(
        specs[0].template,
        AbilityTemplate::ActivatedTargetedPower { .. }
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
        timing: Some(AbilityTiming::BeginMainPhase),
        effects: vec![EffectTemplate::Draw { count: 1 }],
        targets: Vec::new(),
        cost: AbilityCost::default(),
        target_card_type: None,
        target_trait: None,
        target_level_max: None,
        target_cost_max: None,
        target_limit: None,
    };
    let card = CardStatic {
        id: 1,
        card_set: None,
        card_type: CardType::Character,
        color: CardColor::Red,
        level: 0,
        cost: 0,
        power: 500,
        soul: 1,
        triggers: Vec::new(),
        traits: Vec::new(),
        abilities: vec![AbilityTemplate::ActivatedTargetedPower {
            amount: 1000,
            count: 1,
            target: weiss_core::db::TargetTemplate::SelfStage,
        }],
        ability_defs: vec![ability_def],
        counter_timing: false,
        raw_text: None,
    };
    let mut cards = vec![card];
    deck_support::add_clone_cards(&mut cards);
    let db = Arc::new(CardDb::new(cards).expect("db"));

    let curriculum = weiss_core::config::CurriculumConfig {
        enable_priority_windows: true,
        priority_autopick_single_action: false,
        ..Default::default()
    };
    let pool = [1];
    let config = weiss_core::config::EnvConfig {
        deck_lists: [
            deck_support::legalize_deck(vec![1; 50], &pool),
            deck_support::legalize_deck(vec![1; 50], &pool),
        ],
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
    let mut env = GameEnv::new(db.clone(), config, curriculum, 0, replay_config, None, 0);
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    assert!(matches!(
        env.decision.as_ref().map(|d| d.kind),
        Some(DecisionKind::Main)
    ));
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();

    let choice = env.state.turn.choice.as_ref().expect("priority choice");
    let mut ability_options = choice
        .options
        .iter()
        .filter(|opt| opt.zone == ChoiceZone::PriorityAct)
        .collect::<Vec<_>>();
    assert_eq!(ability_options.len(), 2);
    ability_options.sort_by_key(|opt| opt.target_slot);
    assert_eq!(ability_options[0].target_slot, Some(0));
    assert_eq!(ability_options[1].target_slot, Some(1));

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
