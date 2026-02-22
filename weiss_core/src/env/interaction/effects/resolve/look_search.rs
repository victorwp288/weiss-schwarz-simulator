use super::prelude::*;

pub(super) fn look_top_deck_reorder(env: &mut GameEnv, controller: u8, payload: &EffectPayload) {
    if payload.targets.is_empty() {
        return;
    }
    let p = controller as usize;
    if env.state.players[p].deck.is_empty() {
        return;
    }
    let mut reordered: Vec<CardInstance> = Vec::with_capacity(payload.targets.len());
    for instance_id in payload.targets.iter().map(|target| target.instance_id) {
        if let Some(pos) = env.state.players[p]
            .deck
            .iter()
            .position(|card| card.instance_id == instance_id)
        {
            reordered.push(env.state.players[p].deck.remove(pos));
        }
    }
    if reordered.is_empty() {
        return;
    }
    for card in reordered.iter().rev().copied() {
        env.state.players[p].deck.push(card);
    }
    env.touch_player_obs(controller);
    env.mark_rule_actions_dirty();
}

pub(super) fn look_top_card_top_or_waiting_room(env: &mut GameEnv, payload: &EffectPayload) {
    if let Some(target) = payload.targets.first().copied() {
        if target.zone != TargetZone::DeckTop {
            return;
        }
        let p = target.player as usize;
        let offset = target.index as usize;
        if offset >= env.state.players[p].deck.len() {
            return;
        }
        let deck_idx = env.state.players[p].deck.len().saturating_sub(1 + offset);
        if deck_idx >= env.state.players[p].deck.len() {
            return;
        }
        if env.state.players[p].deck[deck_idx].instance_id != target.instance_id {
            return;
        }
        let card = env.state.players[p].deck.remove(deck_idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Deck,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
}

pub(super) fn look_top_card_top_or_bottom(env: &mut GameEnv, payload: &EffectPayload) {
    if let Some(target) = payload.targets.first().copied() {
        env.move_target_to_deck_bottom(target);
    }
}

pub(super) fn search_top_deck_to_hand_level_at_least_mill_rest(
    env: &mut GameEnv,
    controller: u8,
    look_count: u8,
    choose_count: u8,
    min_level: u8,
) {
    let p = controller as usize;
    let reveal_count = std::cmp::min(look_count as usize, env.state.players[p].deck.len());
    let mut looked: Vec<CardInstance> = Vec::with_capacity(reveal_count);
    for _ in 0..reveal_count {
        let Some(card) = env.draw_from_deck(controller) else {
            break;
        };
        looked.push(card);
    }
    let mut moved_to_hand = 0u8;
    for card in looked {
        let eligible = env
            .db
            .get(card.id)
            .map(|static_card| {
                let effective_level = if static_card.card_type == CardType::Climax {
                    0
                } else {
                    static_card.level
                };
                effective_level >= min_level
            })
            .unwrap_or(false);
        if eligible && moved_to_hand < choose_count {
            env.reveal_card(
                controller,
                &card,
                RevealReason::AbilityEffect,
                RevealAudience::Public,
            );
            moved_to_hand = moved_to_hand.saturating_add(1);
            env.move_card_between_zones(controller, card, Zone::Deck, Zone::Hand, None, None);
        } else {
            env.move_card_between_zones(
                controller,
                card,
                Zone::Deck,
                Zone::WaitingRoom,
                None,
                None,
            );
        }
    }
}

pub(super) fn reveal_top_and_salvage_by_revealed_level(
    env: &mut GameEnv,
    controller: u8,
    count: u8,
    climax_level: u8,
) {
    let p = controller as usize;
    let Some(top_card) = env.state.players[p].deck.last().copied() else {
        return;
    };
    env.reveal_card(
        controller,
        &top_card,
        RevealReason::AbilityEffect,
        RevealAudience::Public,
    );
    let revealed_level = env
        .db
        .get(top_card.id)
        .map(|static_card| {
            if static_card.card_type == CardType::Climax {
                climax_level
            } else {
                static_card.level
            }
        })
        .unwrap_or(climax_level);
    let mut selected: Vec<usize> = Vec::new();
    for idx in (0..env.state.players[p].waiting_room.len()).rev() {
        if selected.len() >= count as usize {
            break;
        }
        let Some(card_inst) = env.state.players[p].waiting_room.get(idx).copied() else {
            continue;
        };
        let Some(static_card) = env.db.get(card_inst.id) else {
            continue;
        };
        if static_card.card_type != CardType::Character {
            continue;
        }
        if static_card.level <= revealed_level {
            selected.push(idx);
        }
    }
    for idx in selected {
        if idx >= env.state.players[p].waiting_room.len() {
            continue;
        }
        let card = env.state.players[p].waiting_room.remove(idx);
        env.move_card_between_zones(controller, card, Zone::WaitingRoom, Zone::Hand, None, None);
    }
}
