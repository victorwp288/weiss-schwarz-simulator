use std::sync::Arc;

use weiss_core::config::{CurriculumConfig, EnvConfig, RewardConfig, ObservationVisibility, ErrorPolicy};
use weiss_core::db::{CardDb, CardStatic, CardType, CardColor, TriggerIcon, AbilityTemplate};
use weiss_core::env::GameEnv;
use weiss_core::legal::{DecisionKind, ActionDesc};
use weiss_core::state::{AttackType, StageStatus};
use weiss_core::replay::{ReplayConfig, ReplayWriter, read_replay_file, ReplayEvent};
use std::fs;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn make_db() -> Arc<CardDb> {
    let cards = vec![
        CardStatic {
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
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 2,
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
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 3,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 500,
            soul: 2,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 4,
            card_set: None,
            card_type: CardType::Climax,
            color: CardColor::Red,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 5,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Green,
            level: 0,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![TriggerIcon::Soul],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 6,
            card_set: None,
            card_type: CardType::Event,
            color: CardColor::Blue,
            level: 0,
            cost: 0,
            power: 0,
            soul: 0,
            triggers: vec![],
            traits: vec![],
            abilities: vec![AbilityTemplate::CounterBackup { power: 1000 }],
            counter_timing: true,
            raw_text: None,
        },
        CardStatic {
            id: 7,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 9000,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 8,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Yellow,
            level: 0,
            cost: 0,
            power: 1000,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
        CardStatic {
            id: 9,
            card_set: None,
            card_type: CardType::Character,
            color: CardColor::Red,
            level: 1,
            cost: 0,
            power: 500,
            soul: 1,
            triggers: vec![],
            traits: vec![],
            abilities: vec![],
            counter_timing: false,
            raw_text: None,
        },
    ];
    Arc::new(CardDb::new(cards).expect("db build"))
}

fn make_config(deck_a: Vec<u32>, deck_b: Vec<u32>) -> EnvConfig {
    EnvConfig {
        deck_lists: [deck_a, deck_b],
        deck_ids: [10, 11],
        max_decisions: 500,
        max_ticks: 100_000,
        reward: RewardConfig::default(),
        error_policy: ErrorPolicy::Strict,
        observation_visibility: ObservationVisibility::Public,
    }
}

fn default_curriculum() -> CurriculumConfig {
    CurriculumConfig::default()
}

fn temp_dir(label: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    dir.push(format!("ws_sim_{label}_{stamp}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_wsdb_header_roundtrip() {
    let cards = vec![
        CardStatic {
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
            counter_timing: false,
            raw_text: None,
        },
    ];
    let db = CardDb::new(cards).expect("db build");
    let bytes = db.to_bytes_with_header().expect("wsdb bytes");
    let loaded = CardDb::from_wsdb_bytes(&bytes).expect("wsdb load");
    assert!(loaded.get(1).is_some());
}

#[test]
fn test_wsdb_bad_magic() {
    let cards = vec![
        CardStatic {
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
            counter_timing: false,
            raw_text: None,
        },
    ];
    let db = CardDb::new(cards).expect("db build");
    let mut bytes = db.to_bytes_with_header().expect("wsdb bytes");
    bytes[0] = b'X';
    assert!(CardDb::from_wsdb_bytes(&bytes).is_err());
}

#[test]
fn test_wsdb_bad_schema_version() {
    let cards = vec![
        CardStatic {
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
            counter_timing: false,
            raw_text: None,
        },
    ];
    let db = CardDb::new(cards).expect("db build");
    let mut bytes = db.to_bytes_with_header().expect("wsdb bytes");
    let bad = (CardDb::schema_version() + 1).to_le_bytes();
    bytes[4..8].copy_from_slice(&bad);
    assert!(CardDb::from_wsdb_bytes(&bytes).is_err());
}

#[test]
fn test_illegal_mainplay_no_leak_lenient_noop() {
    let db = make_db();
    let deck_a = vec![9; 20];
    let deck_b = vec![9; 20];
    let mut config = make_config(deck_a, deck_b);
    config.error_policy = ErrorPolicy::LenientNoop;
    let mut env = GameEnv::new(db, config, default_curriculum(), 7, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    // clock decision
    env.apply_action(ActionDesc::ClockPass).unwrap();
    let hand_len = env.state.players[env.state.turn.active_player as usize].hand.len();
    let _ = env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    let hand_after = env.state.players[env.state.turn.active_player as usize].hand.len();
    assert_eq!(hand_len, hand_after);
}

#[test]
fn test_illegal_mainplay_no_leak_lenient_terminate() {
    let db = make_db();
    let deck_a = vec![9; 20];
    let deck_b = vec![9; 20];
    let mut config = make_config(deck_a, deck_b);
    config.error_policy = ErrorPolicy::LenientTerminate;
    let mut env = GameEnv::new(db, config, default_curriculum(), 9, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    let hand_len = env.state.players[env.state.turn.active_player as usize].hand.len();
    let outcome = env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    let hand_after = env.state.players[env.state.turn.active_player as usize].hand.len();
    assert!(outcome.terminated);
    assert_eq!(hand_len, hand_after);
}

#[test]
fn test_full_turn_cycle_golden() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![1; 20];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, default_curriculum(), 42, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Clock);
    env.apply_action(ActionDesc::ClockPass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Main);
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Climax);
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::AttackDeclaration);
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::AttackDeclaration);
    env.apply_action(ActionDesc::AttackPass).unwrap();
    // After attack pass, opponent should be active on their clock phase.
    assert_eq!(env.state.turn.active_player, 1);
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::Clock);
}

#[test]
fn test_direct_attack_adds_soul() {
    let db = make_db();
    let deck_a = vec![3; 20];
    let deck_b = vec![3; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 11, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let defender = 1 - env.state.turn.active_player as usize;
    let clock_before = env.state.players[defender].clock.len();
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    let clock_after = env.state.players[defender].clock.len();
    assert_eq!(clock_after - clock_before, 3); // base soul 2 + direct +1
}

#[test]
fn test_side_attack_reduces_damage() {
    let db = make_db();
    let deck_a = vec![3; 20];
    let deck_b = vec![3; 20];
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 12, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    // Give defender a character so side attack is legal.
    let defender = 1 - env.state.turn.active_player as usize;
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].stage[0].card = Some(card);
    }
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].level.push(card); // defender level 1
    }
    let clock_before = env.state.players[defender].clock.len();
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Side }).unwrap();
    let clock_after = env.state.players[defender].clock.len();
    assert_eq!(clock_after - clock_before, 1); // soul 2 - level 1
}

#[test]
fn test_trigger_moves_card_to_stock_and_logs() {
    let db = make_db();
    let deck_a = vec![5; 20];
    let deck_b = vec![5; 20];
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
    };
    let writer = Some(ReplayWriter::new(&replay_config).unwrap());
    let mut env = GameEnv::new(db, config, curriculum, 13, replay_config.clone(), writer);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let attacker = env.state.turn.active_player as usize;
    let stock_before = env.state.players[attacker].stock.len();
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    let stock_after = env.state.players[attacker].stock.len();
    assert_eq!(stock_after - stock_before, 1);
    assert!(env.replay_events.iter().any(|e| matches!(e, ReplayEvent::Trigger { .. })));
}

#[test]
fn test_refresh_penalty_applied() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![1; 20];
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, default_curriculum(), 14, Default::default(), None);
    // Force refresh on draw after mulligan completes.
    let active = env.state.turn.starting_player as usize;
    let mut deck = Vec::new();
    std::mem::swap(&mut deck, &mut env.state.players[active].deck);
    env.state.players[active].waiting_room = deck;
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    // Draw phase should have refreshed and applied penalty.
    assert_eq!(env.state.players[active].clock.len(), 1);
    assert!(!env.state.players[active].deck.is_empty());
}

#[test]
fn test_damage_cancel_on_climax() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![4; 20]; // all climax
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 15, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let defender = 1 - env.state.turn.active_player as usize;
    let clock_before = env.state.players[defender].clock.len();
    let waiting_before = env.state.players[defender].waiting_room.len();
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
    let clock_after = env.state.players[defender].clock.len();
    let waiting_after = env.state.players[defender].waiting_room.len();
    assert_eq!(clock_after, clock_before);
    assert!(waiting_after > waiting_before);
}

#[test]
fn test_level_up_decision_changes_level_card() {
    let db = make_db();
    let deck_a = vec![3; 20];
    let mut deck_b = vec![1; 19];
    deck_b.push(2);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a.clone(), deck_b.clone());

    let build_env = |seed| -> GameEnv {
        let mut env = GameEnv::new(db.clone(), config.clone(), curriculum.clone(), seed, Default::default(), None);
        env.apply_action(ActionDesc::MulliganKeep).unwrap();
        env.apply_action(ActionDesc::MulliganKeep).unwrap();
        env.apply_action(ActionDesc::ClockPass).unwrap();
        env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
        env.apply_action(ActionDesc::MainPass).unwrap();
        env.apply_action(ActionDesc::ClimaxPass).unwrap();
        let defender = 1 - env.state.turn.active_player as usize;
        env.state.players[defender].clock.clear();
        let deck = &mut env.state.players[defender].deck;
        let mut take_card = |id| {
            let pos = deck.iter().position(|c| *c == id).expect("card missing");
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
        env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Direct }).unwrap();
        env
    };

    let mut env_a = build_env(16);
    let mut env_b = build_env(16);
    let defender = 1 - env_a.state.turn.active_player as usize;
    assert_eq!(env_a.decision.as_ref().unwrap().kind, DecisionKind::LevelUp);
    env_a.apply_action(ActionDesc::LevelUp { index: 0 }).unwrap();
    env_b.apply_action(ActionDesc::LevelUp { index: 3 }).unwrap();
    let level_card_a = env_a.state.players[defender].level.last().cloned().unwrap();
    let level_card_b = env_b.state.players[defender].level.last().cloned().unwrap();
    assert_ne!(level_card_a, level_card_b);
    let total_a = total_cards(&env_a, defender);
    let total_b = total_cards(&env_b, defender);
    assert_eq!(total_a, total_b);
}

#[test]
fn test_encore_with_and_without_stock() {
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
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    let defender = 1 - env.state.turn.active_player as usize;
    if let Some(card) = env.state.players[defender].deck.pop() {
        env.state.players[defender].stage[0].card = Some(card);
    }
    // No stock -> must send to waiting room on encore no.
    env.state.players[defender].stock.clear();
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::AttackDeclaration);
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

    // Reset a new env to test encore keep.
    let config = make_config(vec![7; 20], vec![8; 20]);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    curriculum.enable_counters = false;
    let mut env = GameEnv::new(make_db(), config, curriculum, 18, Default::default(), None);
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPlayCharacter { hand_index: 0, stage_slot: 0 }).unwrap();
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
    env.apply_action(ActionDesc::Attack { slot: 0, attack_type: AttackType::Frontal }).unwrap();
    assert_eq!(env.decision.as_ref().unwrap().kind, DecisionKind::AttackDeclaration);
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

#[test]
fn test_replay_roundtrip() {
    let db = make_db();
    let deck_a = vec![1; 20];
    let deck_b = vec![1; 20];
    let config = make_config(deck_a, deck_b);
    let replay_dir = temp_dir("roundtrip");
    let replay_config = ReplayConfig {
        enabled: true,
        sample_rate: 1.0,
        out_dir: replay_dir.clone(),
        compress: false,
        include_trigger_card_id: true,
    };
    let writer = ReplayWriter::new(&replay_config).unwrap();
    let mut env = GameEnv::new(db, config, default_curriculum(), 21, replay_config.clone(), Some(writer));
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::MulliganKeep).unwrap();
    env.apply_action(ActionDesc::ClockPass).unwrap();
    env.apply_action(ActionDesc::MainPass).unwrap();
    env.apply_action(ActionDesc::ClimaxPass).unwrap();
    env.apply_action(ActionDesc::AttackPass).unwrap();
    env.finish_episode_replay();
    sleep(Duration::from_millis(50));

    let mut files = Vec::new();
    for entry in fs::read_dir(replay_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|s| s == "wsr").unwrap_or(false) {
            files.push(path);
        }
    }
    assert!(!files.is_empty());
    let data = read_replay_file(&files[0]).unwrap();
    assert_eq!(data.header.obs_version, weiss_core::encode::OBS_ENCODING_VERSION);
    assert_eq!(data.header.action_version, weiss_core::encode::ACTION_ENCODING_VERSION);
}

fn total_cards(env: &GameEnv, player: usize) -> usize {
    let p = &env.state.players[player];
    let stage_count = p.stage.iter().filter(|c| c.card.is_some()).count();
    p.deck.len() + p.hand.len() + p.waiting_room.len() + p.clock.len() + p.level.len() + p.stock.len() + p.memory.len() + p.climax.len() + stage_count
}
