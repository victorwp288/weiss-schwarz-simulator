mod engine_support;

use engine_support::*;
use weiss_core::env::GameEnv;
use weiss_core::legal::ActionDesc;
use weiss_core::state::AttackType;

#[test]
fn direct_attack_adds_soul() {
    let db = make_db();
    let mut deck_a = vec![3; 49];
    deck_a.push(9);
    let mut deck_b = vec![3; 49];
    deck_b.push(9);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 11, Default::default(), None, 0);
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
    let clock_before = env.state.players[defender].clock.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let clock_after = env.state.players[defender].clock.len();
    assert_eq!(clock_after - clock_before, 3);
}

#[test]
fn side_attack_reduces_damage() {
    let db = make_db();
    let mut deck_a = vec![3; 49];
    deck_a.push(9);
    let mut deck_b = vec![3; 49];
    deck_b.push(9);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 12, Default::default(), None, 0);
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
    let defender_state = &mut env.state.players[defender];
    let is_level_one = |card_id| {
        env.db
            .get(card_id)
            .map(|card| card.level == 1)
            .unwrap_or(false)
    };
    let stage_has_level_one = defender_state
        .stage
        .first()
        .and_then(|slot| slot.card)
        .map(|card| is_level_one(card.id))
        .unwrap_or(false);
    if !stage_has_level_one {
        let replacement =
            if let Some(pos) = defender_state.deck.iter().position(|c| is_level_one(c.id)) {
                Some(defender_state.deck.remove(pos))
            } else if let Some(pos) = defender_state.hand.iter().position(|c| is_level_one(c.id)) {
                Some(defender_state.hand.remove(pos))
            } else if let Some(pos) = defender_state
                .waiting_room
                .iter()
                .position(|c| is_level_one(c.id))
            {
                Some(defender_state.waiting_room.remove(pos))
            } else if let Some(pos) = defender_state.clock.iter().position(|c| is_level_one(c.id)) {
                Some(defender_state.clock.remove(pos))
            } else if let Some(pos) = defender_state.stock.iter().position(|c| is_level_one(c.id)) {
                Some(defender_state.stock.remove(pos))
            } else {
                None
            }
            .expect("missing level 1 defender for side attack test");
        let previous = defender_state.stage[0].card.replace(replacement);
        if let Some(card) = previous {
            defender_state.deck.push(card);
        }
    }
    let clock_before = env.state.players[defender].clock.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Side,
    })
    .unwrap();
    let clock_after = env.state.players[defender].clock.len();
    assert_eq!(clock_after - clock_before, 1);
}

#[test]
fn damage_cancel_on_climax() {
    let db = make_db();
    let deck_a = vec![1; 50];
    let mut deck_b = vec![4; 8];
    deck_b.resize(50, 1);
    let mut curriculum = default_curriculum();
    curriculum.enable_triggers = false;
    let config = make_config(deck_a, deck_b);
    let mut env = GameEnv::new(db, config, curriculum, 15, Default::default(), None, 0);
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
    let clock_before = env.state.players[defender].clock.len();
    let waiting_before = env.state.players[defender].waiting_room.len();
    env.apply_action(ActionDesc::Attack {
        slot: 0,
        attack_type: AttackType::Direct,
    })
    .unwrap();
    let clock_after = env.state.players[defender].clock.len();
    let waiting_after = env.state.players[defender].waiting_room.len();
    assert_eq!(clock_after, clock_before);
    assert!(waiting_after > waiting_before);
}
