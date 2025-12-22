mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::legal::{ActionDesc, DecisionKind};
use weiss_core::state::{AttackType, StageStatus};

#[test]
fn level_up_decision_changes_level_card() {
    let db = make_db();
    let deck_a = vec![3; 20];
    let mut deck_b = vec![1; 19];
    deck_b.push(2);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a.clone(), deck_b.clone());

    let build_env = |seed| -> GameEnv {
        let mut env = GameEnv::new(
            db.clone(),
            config.clone(),
            curriculum.clone(),
            seed,
            Default::default(),
            None,
        );
        env.apply_action(ActionDesc::MulliganKeep).unwrap();
        env.apply_action(ActionDesc::MulliganKeep).unwrap();
        env.apply_action(ActionDesc::ClockPass).unwrap();
        env.apply_action(ActionDesc::MainPlayCharacter {
            hand_index: 0,
            stage_slot: 0,
        })
        .unwrap();
        env.apply_action(ActionDesc::MainPass).unwrap();
        env.apply_action(ActionDesc::ClimaxPass).unwrap();
        let defender = 1 - env.state.turn.active_player as usize;
        env.state.players[defender].clock.clear();
        let deck = &mut env.state.players[defender].deck;
        let mut take_card = |id| {
            let pos = deck.iter().position(|c| c.id == id).expect("card missing");
            deck.remove(pos)
        };
        let mut clock_cards = Vec::new();
        for _ in 0..6 {
            clock_cards.push(take_card(1));
        }
        env.state.players[defender].clock = clock_cards;
        let card2 = take_card(2);
        let card1a = take_card(1);
        let card1b = take_card(1);
        deck.push(card2);
        deck.push(card1a);
        deck.push(card1b);
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
    let deck_a = vec![7; 20];
    let deck_b = vec![8; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 17, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
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
    env.apply_action(ActionDesc::AttackPass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Encore);
    while let Some(decision) = env.decision.clone() {
        if decision.kind != DecisionKind::Encore {
            break;
        }
        env.apply_action(ActionDesc::EncoreNo).unwrap();
    }
    assert!(env.state.players[defender].stage[0].card.is_none());

    let config = make_config(vec![7; 20], vec![8; 20]);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let mut env = GameEnv::new(make_db(), config, curriculum, 18, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter {
        hand_index: 0,
        stage_slot: 0,
    })
    .unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
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
    env.apply_action(ActionDesc::AttackPass).unwrap();
    while let Some(decision) = env.decision.clone() {
        if decision.kind != DecisionKind::Encore {
            break;
        }
        if decision.player == defender as u8 {
            env.apply_action(ActionDesc::EncoreYes).unwrap();
        } else {
            env.apply_action(ActionDesc::EncoreNo).unwrap();
        }
    }
    assert!(env.state.players[defender].stage[0].card.is_some());
}
