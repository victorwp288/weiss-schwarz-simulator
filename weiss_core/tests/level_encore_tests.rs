mod engine_support;

use std::sync::Arc;

use engine_support::*;
use weiss_core::db::{AbilityCost, AbilityTemplate, CardColor, CardDb, CardStatic, CardType};
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, DecisionKind};
use weiss_core::state::{AttackType, CardInstance, PlayerState, StageStatus};

fn take_card_by_id(state: &mut PlayerState, id: u32) -> CardInstance {
    if let Some(pos) = state.deck.iter().position(|c| c.id == id) {
        return state.deck.remove(pos);
    }
    if let Some(pos) = state.hand.iter().position(|c| c.id == id) {
        return state.hand.remove(pos);
    }
    if let Some(pos) = state.waiting_room.iter().position(|c| c.id == id) {
        return state.waiting_room.remove(pos);
    }
    if let Some(pos) = state.clock.iter().position(|c| c.id == id) {
        return state.clock.remove(pos);
    }
    if let Some(pos) = state.stock.iter().position(|c| c.id == id) {
        return state.stock.remove(pos);
    }
    panic!("card missing");
}

fn make_encore_variant_db() -> Arc<CardDb> {
    let mut cards = Vec::new();
    for id in 100..=112u32 {
        cards.push(CardStatic {
            id,
            card_set: None,
            card_type: CardType::Character,
            color: if id % 2 == 0 {
                CardColor::Red
            } else {
                CardColor::Blue
            },
            level: 0,
            cost: 0,
            power: if id == 100 { 1000 } else { 500 },
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: if id == 100 {
                vec![AbilityTemplate::EncoreVariant {
                    cost: AbilityCost {
                        discard_from_hand: 1,
                        ..AbilityCost::default()
                    },
                }]
            } else {
                vec![]
            },
            ability_defs: vec![],
            counter_timing: false,
            raw_text: None,
        });
    }
    Arc::new(CardDb::new(cards).expect("db build"))
}

#[test]
fn level_up_decision_changes_level_card() {
    let db = make_db();
    let mut deck_a = vec![1; 19];
    deck_a.push(2);
    let deck_b = deck_a.clone();
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a.clone(), deck_b.clone());

    let build_env = |seed| -> GameEnv {
        let mut env = GameEnv::new_or_panic(
            db.clone(),
            config.clone(),
            curriculum.clone(),
            seed,
            Default::default(),
            None,
            0,
        );
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
        let defender = 1 - env.state.turn.active_player as usize;
        env.state.players[defender].clock.clear();
        let defender_state = &mut env.state.players[defender];
        let mut clock_cards = Vec::new();
        for _ in 0..6 {
            clock_cards.push(defender_state.deck.pop().expect("clock fill"));
        }
        defender_state.clock = clock_cards;
        let mut ids: Vec<u32> = defender_state.deck.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        let primary_id = *ids.first().expect("deck ids missing");
        let secondary_id = ids
            .iter()
            .copied()
            .find(|id| *id != primary_id)
            .unwrap_or(primary_id);
        let card2 = take_card_by_id(defender_state, secondary_id);
        let card1a = take_card_by_id(defender_state, primary_id);
        let card1b = take_card_by_id(defender_state, primary_id);
        defender_state.deck.push(card2);
        defender_state.deck.push(card1a);
        defender_state.deck.push(card1b);
        env.apply_action(ActionDesc::Attack {
            slot: 0,
            attack_type: AttackType::Direct,
        })
        .unwrap();
        env
    };

    let mut env_a = build_env(16);
    let mut env_b = build_env(16);
    let defender = 1 - env_a.state.turn.active_player as usize;
    assert_eq!(env_a.decision.as_ref().unwrap().kind, DecisionKind::LevelUp);
    env_a
        .apply_action(ActionDesc::LevelUp { index: 0 })
        .unwrap();
    env_b
        .apply_action(ActionDesc::LevelUp { index: 3 })
        .unwrap();
    let level_card_a = env_a.state.players[defender].level.last().cloned().unwrap();
    let level_card_b = env_b.state.players[defender].level.last().cloned().unwrap();
    assert_ne!(level_card_a, level_card_b);
    let total_a = total_cards(&env_a, defender);
    let total_b = total_cards(&env_b, defender);
    assert_eq!(total_a, total_b);
}

#[test]
fn encore_with_and_without_stock() {
    let db = make_db();
    let deck_a = vec![7; 50];
    let deck_b = vec![8; 50];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(db, config, curriculum, 17, Default::default(), None, 0);
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
    let defender = 1 - env.state.turn.active_player as usize;
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].stage[0].card = Some(card);
    }
    env.state.players[defender].stock.clear();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::AttackDeclaration
    );
    if env.state.players[defender].stage[0].status != StageStatus::Reverse {
        env.state.players[defender].stage[0].status = StageStatus::Reverse;
    }
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Encore);
    while let Some(decision) = env.decision.clone() {
        if decision.kind != DecisionKind::Encore {
            break;
        }
        if let Some(req) = env
            .state
            .turn
            .encore_queue
            .iter()
            .find(|req| req.player == decision.player)
        {
            env.apply_action(ActionDesc::EncoreDecline { slot: req.slot })
                .unwrap();
        } else {
            break;
        }
    }
    assert!(env.state.players[defender].stage[0].card.is_none());

    let config = make_config(vec![7; 50], vec![8; 50]);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let mut env = GameEnv::new_or_panic(
        make_db(),
        config,
        curriculum,
        18,
        Default::default(),
        None,
        0,
    );
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
    let defender = 1 - env.state.turn.active_player as usize;
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].stage[0].card = Some(card);
    }
    env.state.players[defender].stock.clear();
    for _ in 0..3 {
        if let Some(card) = env.state.players[defender].deck.pop() {
            env.state.players[defender].stock.push(card);
        }
    }
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::AttackDeclaration
    );
    if env.state.players[defender].stage[0].status != StageStatus::Reverse {
        env.state.players[defender].stage[0].status = StageStatus::Reverse;
    }
    env.apply_action(ActionDesc::Pass).unwrap();
    while let Some(decision) = env.decision.clone() {
        if decision.kind != DecisionKind::Encore {
            break;
        }
        if let Some(req) = env
            .state
            .turn
            .encore_queue
            .iter()
            .find(|req| req.player == decision.player)
        {
            env.apply_action(ActionDesc::EncorePay { slot: req.slot })
                .unwrap();
        } else {
            break;
        }
    }
    assert!(env.state.players[defender].stage[0].card.is_some());
}

#[test]
fn encore_variant_discard_cost_is_payable_without_stock() {
    let db = make_encore_variant_db();
    let mut deck_a = Vec::new();
    for id in 100..=112u32 {
        for _ in 0..4 {
            deck_a.push(id);
        }
    }
    deck_a.truncate(50);
    let deck_b = deck_a.clone();
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new_or_panic(db, config, curriculum, 19, Default::default(), None, 0);
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

    let defender = 1 - env.state.turn.active_player as usize;
    if let Some(pos) = env.state.players[defender]
        .hand
        .iter()
        .position(|c| c.id == 100)
    {
        let card = env.state.players[defender].hand.remove(pos);
        env.state.players[defender].stage[0].card = Some(card);
    } else if let Some(pos) = env.state.players[defender]
        .deck
        .iter()
        .position(|c| c.id == 100)
    {
        let card = env.state.players[defender].deck.remove(pos);
        env.state.players[defender].stage[0].card = Some(card);
    } else {
        panic!("expected card id=100 for defender not found in hand or deck");
    }
    env.state.players[defender].stock.clear();

    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Frontal,
    })
    .unwrap();
    assert_eq!(
        env.decision.as_ref().unwrap().kind,
        DecisionKind::AttackDeclaration
    );
    if env.state.players[defender].stage[0].status != StageStatus::Reverse {
        env.state.players[defender].stage[0].status = StageStatus::Reverse;
    }
    env.apply_action(ActionDesc::Pass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Encore);

    while env
        .decision
        .as_ref()
        .is_some_and(|d| d.kind == DecisionKind::Encore && d.player as usize != defender)
    {
        let decision = env.decision.clone().expect("encore decision");
        let req = env
            .state
            .turn
            .encore_queue
            .iter()
            .find(|req| req.player == decision.player)
            .cloned()
            .expect("encore request for active encore player");
        env.apply_action(ActionDesc::EncoreDecline { slot: req.slot })
            .unwrap();
    }
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Encore);
    assert_eq!(env.decision.as_ref().unwrap().player as usize, defender);

    let legal = env.legal_actions();
    assert!(
        legal.contains(&ActionDesc::EncorePay { slot: 0 }),
        "EncorePay should be legal via EncoreVariant discard cost"
    );
    env.apply_action(ActionDesc::EncorePay { slot: 0 }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Choice);
    env.apply_action(ActionDesc::ChoiceSelect { index: 0 })
        .unwrap();

    assert_eq!(env.state.players[defender].stock.len(), 0);
    assert!(env.state.players[defender].stage[0].card.is_some());
    assert_ne!(
        env.state.players[defender].stage[0].status,
        StageStatus::Reverse
    );
}
