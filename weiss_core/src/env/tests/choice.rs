use super::*;
use crate::effects::{EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec};
use crate::events::{RevealAudience, RevealReason};
use crate::replay::{ReplayConfig, ReplayEvent};
use crate::state::{
    ChoiceReason, PendingTargetEffect, TargetSelectionState, TargetSide, TargetSlotFilter,
    TargetSpec, TargetZone,
};

#[test]
fn visibility_policy_masks_opponent_hidden_choices() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    let mut next_id = 1u32;
    env.state.players[1].hand = vec![
        make_instance(1, 1, &mut next_id),
        make_instance(2, 1, &mut next_id),
    ];

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::Opponent,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
        card_ids: Vec::new(),
        count: 1,
        limit: None,
        source_only: false,
        reveal_to_controller: false,
    };
    let effect_spec = EffectSpec {
        id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
        kind: EffectKind::MoveToHand,
        target: Some(spec.clone()),
        optional: false,
    };
    let candidates = enumerate_targets_for_test(&env, 0, &spec, &[]);
    env.state.turn.target_selection = Some(TargetSelectionState {
        controller: 0,
        source_id: 1,
        remaining: 1,
        spec,
        selected: Vec::new(),
        candidates,
        effect: PendingTargetEffect::EffectPending {
            instance_id: 1,
            payload: EffectPayload {
                spec: effect_spec,
                targets: Vec::new(),
                source_ref: None,
            },
        },
        allow_skip: false,
    });
    env.present_target_choice();

    let (choice_id, options) = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                choice_id,
                options,
                ..
            } = e
            {
                Some((*choice_id, options))
            } else {
                None
            }
        })
        .expect("choice presented");
    assert_eq!(options.len(), 2);
    assert!(options.iter().all(|opt| opt.reference.card_id == 0));
    assert!(options.iter().all(|opt| opt.reference.index.is_none()));
    assert!(options
        .iter()
        .all(|opt| opt.option_id >> 32 == choice_id as u64));
    let mut unique = std::collections::BTreeSet::new();
    for opt in options {
        assert!(unique.insert(opt.option_id));
    }

    env.replay_events.clear();
    env.state.turn.choice = None;
    let revealed = env.state.players[1].hand[1];
    env.reveal_card(
        1,
        &revealed,
        RevealReason::TriggerCheck,
        RevealAudience::Public,
    );
    env.present_target_choice();

    let options = env
        .replay_events
        .iter()
        .find_map(|e| {
            if let ReplayEvent::ChoicePresented {
                reason: ChoiceReason::TargetSelect,
                options,
                ..
            } = e
            {
                Some(options)
            } else {
                None
            }
        })
        .expect("choice presented");
    assert_eq!(options.len(), 2);
    assert!(options.iter().any(|opt| opt.reference.card_id == 2));
    assert!(options.iter().any(|opt| opt.reference.card_id == 0));
}
