use super::*;
use crate::effects::{EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec};
use crate::encode::{encode_observation, OBS_LEN, OBS_REVEAL_BASE, OBS_REVEAL_LEN};
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::fingerprint::{events_fingerprint, state_fingerprint};
use crate::legal::ActionDesc;
use crate::replay::{ReplayConfig, ReplayEvent};
use crate::state::{
    ChoiceReason, ChoiceZone, PendingTargetEffect, TargetSelectionState, TargetSide,
    TargetSlotFilter, TargetSpec, TargetZone, REVEAL_HISTORY_LEN,
};

#[test]
fn public_replay_masks_hidden_action_params() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.replay_actions.clear();

    env.log_action(
        1,
        ActionDesc::MainPlayCharacter {
            hand_index: 3,
            stage_slot: 2,
        },
    );

    let last = env.replay_actions.last().expect("action logged");
    match last {
        ActionDesc::MainPlayCharacter {
            hand_index,
            stage_slot,
        } => {
            assert_eq!(*hand_index, u8::MAX);
            assert_eq!(*stage_slot, 2);
        }
        _ => panic!("unexpected action: {last:?}"),
    }
}

#[test]
fn public_replay_masks_hidden_draws() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.recording = true;
    env.replay_events.clear();

    env.log_event(Event::Draw {
        player: 1,
        card: 99,
    });

    let last = env.replay_events.last().expect("draw event");
    match last {
        ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
        _ => panic!("unexpected event: {last:?}"),
    }
}

#[test]
fn public_replay_no_hidden_zone_leaks() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.recording = true;
    env.replay_events.clear();

    env.draw_to_hand(1, 1);

    let mut next_id = 1u32;
    env.state.players[1].hand.clear();
    env.state.players[1]
        .hand
        .push(make_instance(2, 1, &mut next_id));

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::Opponent,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
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
            },
        },
        allow_skip: false,
    });
    env.present_target_choice();

    for event in &env.replay_events {
        match event {
            ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
            ReplayEvent::ZoneMove {
                card,
                from,
                to,
                from_slot,
                to_slot,
                ..
            } => {
                let hidden_from = matches!(from, Zone::Deck | Zone::Hand | Zone::Stock);
                let hidden_to = matches!(to, Zone::Deck | Zone::Hand | Zone::Stock);
                if hidden_from && hidden_to {
                    assert_eq!(*card, 0);
                    assert_eq!(*from_slot, None);
                    assert_eq!(*to_slot, None);
                }
            }
            ReplayEvent::ChoicePresented { options, .. } => {
                for opt in options {
                    if matches!(
                        opt.reference.zone,
                        ChoiceZone::Hand | ChoiceZone::DeckTop | ChoiceZone::Stock
                    ) {
                        assert_eq!(opt.reference.card_id, 0);
                        assert_eq!(opt.reference.instance_id, 0);
                        assert!(opt.reference.index.is_none());
                    }
                }
            }
            _ => {}
        }
    }
}

#[test]
fn reveal_history_updates_for_controller_only() {
    let mut env = make_env();
    let card = env.state.players[0]
        .deck
        .last()
        .cloned()
        .expect("deck card");
    env.reveal_card(
        0,
        &card,
        RevealReason::AbilityEffect,
        RevealAudience::ControllerOnly,
    );

    let mut obs_p0 = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs_p0,
    );
    let reveal_p0 = &obs_p0[OBS_REVEAL_BASE..OBS_REVEAL_BASE + OBS_REVEAL_LEN];
    assert_eq!(reveal_p0[0], card.id as i32);

    let mut obs_p1 = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        1,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs_p1,
    );
    let reveal_p1 = &obs_p1[OBS_REVEAL_BASE..OBS_REVEAL_BASE + OBS_REVEAL_LEN];
    assert!(reveal_p1.iter().all(|v| *v == 0));
}

#[test]
fn reveal_history_ring_buffer_keeps_latest_reveals() {
    let mut env = make_env();
    let mut revealed = Vec::new();
    let total = REVEAL_HISTORY_LEN + 2;
    for idx in 0..total {
        let card = env.state.players[0]
            .deck
            .get(idx)
            .cloned()
            .expect("deck card");
        env.reveal_card(
            0,
            &card,
            RevealReason::AbilityEffect,
            RevealAudience::ControllerOnly,
        );
        revealed.push(card.id);
    }
    let start = revealed.len() - REVEAL_HISTORY_LEN;
    let expected = &revealed[start..];

    let mut obs_p0 = vec![0; OBS_LEN];
    encode_observation(
        &env.state,
        &env.db,
        &env.curriculum,
        0,
        env.decision.as_ref(),
        env.last_action_desc.as_ref(),
        env.last_action_player,
        env.config.observation_visibility,
        &mut obs_p0,
    );
    let reveal_p0 = &obs_p0[OBS_REVEAL_BASE..OBS_REVEAL_BASE + OBS_REVEAL_LEN];
    let expected_vals: Vec<i32> = expected.iter().map(|id| *id as i32).collect();
    assert_eq!(&reveal_p0[..expected_vals.len()], expected_vals.as_slice());
}

#[test]
fn reveal_one_copy_does_not_unmask_duplicates() {
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = make_env_with_replay(replay_config);
    env.curriculum.enable_visibility_policies = true;
    env.recording = true;
    env.replay_events.clear();

    let mut next_id = 1u32;
    let first = make_instance(1, 1, &mut next_id);
    let second = make_instance(1, 1, &mut next_id);
    env.state.players[1].hand = vec![first, second];

    env.reveal_card(
        1,
        &first,
        RevealReason::TriggerCheck,
        RevealAudience::Public,
    );

    let spec = TargetSpec {
        zone: TargetZone::Hand,
        side: TargetSide::Opponent,
        slot_filter: TargetSlotFilter::Any,
        card_type: None,
        card_trait: None,
        level_max: None,
        cost_max: None,
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
            },
        },
        allow_skip: false,
    });
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
    let revealed = options
        .iter()
        .filter(|opt| opt.reference.card_id == 1)
        .count();
    let hidden = options
        .iter()
        .filter(|opt| opt.reference.card_id == 0)
        .count();
    assert_eq!(revealed, 1);
    assert_eq!(hidden, 1);
    assert!(options.iter().all(|opt| opt.reference.instance_id == 0));
}

#[test]
fn deterministic_replay_from_seed_and_actions() {
    let mut env_a = make_env();
    let _ = env_a.reset_no_copy();
    env_a.recording = true;
    env_a.replay_events.clear();
    env_a.canonical_events.clear();

    let mut actions: Vec<ActionDesc> = Vec::new();
    for _ in 0..40 {
        if env_a.state.terminal.is_some() {
            break;
        }
        let action = env_a.legal_actions().first().expect("legal action").clone();
        actions.push(action.clone());
        env_a.apply_action(action).expect("apply action");
    }
    let state_hash = state_fingerprint(&env_a.state);
    let events_hash = events_fingerprint(env_a.canonical_events());

    let mut env_b = make_env();
    let _ = env_b.reset_no_copy();
    env_b.recording = true;
    env_b.replay_events.clear();
    env_b.canonical_events.clear();

    for action in actions {
        if env_b.state.terminal.is_some() {
            break;
        }
        env_b.apply_action(action).expect("apply action");
    }
    assert_eq!(state_fingerprint(&env_b.state), state_hash);
    assert_eq!(events_fingerprint(env_b.canonical_events()), events_hash);
}
