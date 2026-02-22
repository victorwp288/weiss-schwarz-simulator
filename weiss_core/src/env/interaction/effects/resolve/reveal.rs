use super::prelude::*;

pub(super) fn reveal_deck_top(
    env: &mut GameEnv,
    controller: u8,
    count: u8,
    audience: RevealAudience,
) {
    let p = controller as usize;
    let deck_len = env.state.players[p].deck.len();
    let reveal_count = std::cmp::min(deck_len, count as usize);
    for offset in 0..reveal_count {
        let deck_idx = deck_len.saturating_sub(1 + offset);
        let Some(card) = env.state.players[p].deck.get(deck_idx).copied() else {
            continue;
        };
        env.reveal_card(controller, &card, RevealReason::AbilityEffect, audience);
    }
}

pub(super) fn reveal_zone_top(
    env: &mut GameEnv,
    controller: u8,
    target: TargetSide,
    zone: TargetZone,
    count: u8,
    audience: RevealAudience,
) {
    let target_player = match target {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    match zone {
        TargetZone::DeckTop => {
            let p = target_player as usize;
            let deck_len = env.state.players[p].deck.len();
            let reveal_count = std::cmp::min(deck_len, count as usize);
            for offset in 0..reveal_count {
                let deck_idx = deck_len.saturating_sub(1 + offset);
                let Some(card) = env.state.players[p].deck.get(deck_idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Hand => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].hand.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].hand.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::WaitingRoom => {
            let p = target_player as usize;
            let reveal_count =
                std::cmp::min(env.state.players[p].waiting_room.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].waiting_room.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Clock => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].clock.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].clock.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Level => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].level.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].level.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Stock => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].stock.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].stock.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Memory => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].memory.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].memory.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Climax => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].climax.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].climax.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Resolution => {
            let p = target_player as usize;
            let reveal_count = std::cmp::min(env.state.players[p].resolution.len(), count as usize);
            for idx in 0..reveal_count {
                let Some(card) = env.state.players[p].resolution.get(idx).copied() else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
            }
        }
        TargetZone::Stage => {
            let p = target_player as usize;
            let reveal_count = if env.curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            let mut revealed = 0usize;
            for slot in 0..reveal_count {
                if revealed >= count as usize {
                    break;
                }
                let Some(card) = env.state.players[p].stage[slot].card else {
                    continue;
                };
                env.reveal_card(target_player, &card, RevealReason::AbilityEffect, audience);
                revealed = revealed.saturating_add(1);
            }
        }
    }
}

pub(super) fn reveal_top_if_level_at_least_move_this_to_hand(
    env: &mut GameEnv,
    controller: u8,
    payload: &EffectPayload,
    min_level: u8,
) {
    if !env.reveal_top_deck_card_passes_level_gate(controller, min_level) {
        return;
    }
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let option = ChoiceOptionRef {
        card_id: source_ref.card_id,
        instance_id: source_ref.instance_id,
        zone: ChoiceZone::Stage,
        index: Some(source_ref.index as u16),
        target_slot: None,
    };
    env.move_stage_to_hand(source_ref.player, option);
}

pub(super) fn reveal_top_if_level_at_least_rest_this(
    env: &mut GameEnv,
    controller: u8,
    payload: &EffectPayload,
    min_level: u8,
) {
    if !env.reveal_top_deck_card_passes_level_gate(controller, min_level) {
        return;
    }
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let p = source_ref.player as usize;
    let slot = source_ref.index as usize;
    if slot >= env.state.players[p].stage.len() {
        return;
    }
    let Some(card_inst) = env.state.players[p].stage[slot].card else {
        return;
    };
    if card_inst.instance_id != source_ref.instance_id {
        return;
    }
    env.state.players[p].stage[slot].status = StageStatus::Rest;
    env.mark_slot_power_dirty(source_ref.player, source_ref.index);
    env.mark_continuous_modifiers_dirty();
    env.touch_player_obs(source_ref.player);
}

pub(super) fn reveal_top_if_level_at_least_move_top_to_stock(
    env: &mut GameEnv,
    controller: u8,
    min_level: u8,
) {
    if !env.reveal_top_deck_card_passes_level_gate(controller, min_level) {
        return;
    }
    let p = controller as usize;
    let Some(card) = env.state.players[p].deck.pop() else {
        return;
    };
    env.move_card_between_zones(controller, card, Zone::Deck, Zone::Stock, None, None);
}
