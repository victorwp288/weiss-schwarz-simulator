use super::prelude::*;

pub(super) fn random_discard_from_hand(
    env: &mut GameEnv,
    controller: u8,
    target: TargetSide,
    count: u8,
) {
    let target_player = match target {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    let p = target_player as usize;
    for _ in 0..count {
        let hand_len = env.state.players[p].hand.len();
        if hand_len == 0 {
            break;
        }
        let idx = env.state.rng.gen_range(hand_len);
        if idx >= env.state.players[p].hand.len() {
            break;
        }
        let card = env.state.players[p].hand.remove(idx);
        let from_slot = if idx <= u8::MAX as usize {
            Some(idx as u8)
        } else {
            None
        };
        env.move_card_between_zones(
            target_player,
            card,
            Zone::Hand,
            Zone::WaitingRoom,
            from_slot,
            None,
        );
    }
}

pub(super) fn random_mill(env: &mut GameEnv, controller: u8, target: TargetSide, count: u8) {
    let target_player = match target {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    for _ in 0..count {
        let Some(card) = env.draw_from_deck(target_player) else {
            break;
        };
        env.move_card_between_zones(
            target_player,
            card,
            Zone::Deck,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
}
