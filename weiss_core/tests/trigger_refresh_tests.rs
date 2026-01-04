mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::events::{RevealReason, Zone};
use weiss_core::legal::ActionDesc;
use weiss_core::replay::{ReplayConfig, ReplayEvent, ReplayWriter};
use weiss_core::state::AttackType;

#[test]
fn trigger_moves_card_to_stock_and_logs() {
    let db = make_db();
    let deck_a = vec![5; 50];
    let deck_b = vec![5; 50];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = true;
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("trigger");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        include_trigger_card_id: true,
        ..Default::default()
    };
    let writer = Some(ReplayWriter::new(&replay_config).unwrap());
    let mut env = GameEnv::new(db, config, curriculum, 13, replay_config.clone(), writer, 0);
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.state.turn.turn_number = 1;
    let attacker = env.state.turn.active_player as usize;
    let stock_before = env.state.players[attacker].stock.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let stock_after = env.state.players[attacker].stock.len();
    assert_eq!(stock_after - stock_before, 1);
    assert!(env
        .replay_events
        .iter()
        .any(|e| matches!(e, ReplayEvent::Trigger { .. })));
}

#[test]
fn refresh_penalty_applied() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        14,
        Default::default(),
        None,
        0,
    );
    let active = env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[active].deck);
    env.state.players[active].waiting_room = deck;
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    for _ in 0..12 {
        if env.state.players[active].clock.len() == 1 {
            break;
        }
        let action = env.legal_actions().first().cloned().expect("legal action");
        env.apply_action(action).unwrap();
    }
    assert_eq!(env.state.players[active].clock.len(), 1);
    assert!(!env.state.players[active].deck.is_empty());
}

#[test]
fn refresh_penalty_event_ordering() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut env = GameEnv::new(db, config, default_curriculum(), 22, replay_config, None, 0);
    let active = env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[active].deck);
    env.state.players[active].waiting_room = deck;
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();

    let events = &env.replay_events;
    let refresh_idx = events
        .iter()
        .position(|e| matches!(e, ReplayEvent::Refresh { player } if *player == active as u8))
        .expect("refresh event");
    let shuffle_idx = events[..refresh_idx]
        .iter()
        .rposition(|e| matches!(e, ReplayEvent::Shuffle { player, zone: Zone::Deck } if *player == active as u8))
        .expect("shuffle event");
    let (reveal_idx, penalty_card) = events
        .iter()
        .enumerate()
        .find_map(|(idx, event)| match event {
            ReplayEvent::Reveal {
                player,
                card,
                reason: RevealReason::RefreshPenalty,
                ..
            } if *player == active as u8 => Some((idx, *card)),
            _ => None,
        })
        .expect("refresh penalty reveal");
    let zone_idx = events
        .iter()
        .enumerate()
        .find_map(|(idx, event)| match event {
            ReplayEvent::ZoneMove {
                player,
                card,
                from: Zone::Deck,
                to: Zone::Clock,
                ..
            } if *player == active as u8 && *card == penalty_card => Some(idx),
            _ => None,
        })
        .expect("refresh penalty zone move");
    let penalty_idx = events
        .iter()
        .enumerate()
        .find_map(|(idx, event)| match event {
            ReplayEvent::RefreshPenalty { player, card }
                if *player == active as u8 && *card == penalty_card =>
            {
                Some(idx)
            }
            _ => None,
        })
        .expect("refresh penalty event");

    assert!(shuffle_idx < refresh_idx);
    assert!(refresh_idx < reveal_idx);
    assert!(reveal_idx < zone_idx);
    assert!(zone_idx < penalty_idx);
}

#[test]
fn refresh_empty_waiting_room_outside_damage_causes_loss() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        31,
        Default::default(),
        None,
        0,
    );
    let active = env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[active].deck);
    env.state.players[active].hand.extend(deck);
    env.state.players[active].waiting_room.clear();

    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();

    assert!(env.state.players[active].deck.is_empty());
    assert!(env.state.players[active].waiting_room.is_empty());
    assert!(matches!(
        env.state.terminal,
        Some(weiss_core::state::TerminalResult::Win { winner }) if winner == (1 - active as u8)
    ));
}

#[test]
fn refresh_empty_waiting_room_during_damage_causes_loss() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(
        db,
        config,
        default_curriculum(),
        32,
        Default::default(),
        None,
        0,
    );
    let defender = 1 - env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[defender].deck);
    env.state.players[defender].hand.extend(deck);
    env.state.players[defender].waiting_room.clear();
    let card = env.state.players[defender].hand.pop().expect("deck card");
    env.state.players[defender].deck.push(card);

    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.apply_action(ActionDesc::Pass).unwrap();
    env.state.turn.turn_number = 1;
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();

    assert!(matches!(
        env.state.terminal,
        Some(weiss_core::state::TerminalResult::Win { winner }) if winner == (1 - defender as u8)
    ));
}

#[test]
fn refresh_penalty_public_reveal_visible() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let deck_b = vec![1; 50];
    let config = make_config(deck_a, deck_b);
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        ..Default::default()
    };
    let mut curriculum = default_curriculum();
    curriculum.enable_visibility_policies = true;
    let mut env = GameEnv::new(db, config, curriculum, 23, replay_config, None, 0);
    let active = env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[active].deck);
    env.state.players[active].waiting_room = deck;
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();
    env.apply_action(ActionDesc::MulliganConfirm).unwrap();

    let reveal = env.replay_events.iter().find(|event| {
        matches!(
            event,
            ReplayEvent::Reveal {
                reason: RevealReason::RefreshPenalty,
                ..
            }
        )
    });
    let penalty = env
        .replay_events
        .iter()
        .find(|event| matches!(event, ReplayEvent::RefreshPenalty { .. }));

    match reveal {
        Some(ReplayEvent::Reveal { card, .. }) => assert_ne!(*card, 0),
        _ => panic!("refresh penalty reveal missing"),
    }
    match penalty {
        Some(ReplayEvent::RefreshPenalty { card, .. }) => assert_ne!(*card, 0),
        _ => panic!("refresh penalty event missing"),
    }
}
