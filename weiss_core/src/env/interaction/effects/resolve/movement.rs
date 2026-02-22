use super::prelude::*;

pub(super) fn move_to_hand(env: &mut GameEnv, payload: &EffectPayload) {
    let mut waiting_room_targets: Vec<TargetRef> = Vec::new();
    let mut deck_targets: Vec<TargetRef> = Vec::new();
    for target in &payload.targets {
        match target.zone {
            TargetZone::Stage => {
                let option = ChoiceOptionRef {
                    card_id: target.card_id,
                    instance_id: target.instance_id,
                    zone: ChoiceZone::Stage,
                    index: Some(target.index as u16),
                    target_slot: None,
                };
                env.move_stage_to_hand(target.player, option);
            }
            TargetZone::WaitingRoom => {
                waiting_room_targets.push(*target);
            }
            TargetZone::DeckTop => {
                deck_targets.push(*target);
            }
            _ => {}
        }
    }
    waiting_room_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in waiting_room_targets {
        let option = ChoiceOptionRef {
            card_id: target.card_id,
            instance_id: target.instance_id,
            zone: ChoiceZone::WaitingRoom,
            index: Some(target.index as u16),
            target_slot: None,
        };
        env.move_waiting_room_to_hand(target.player, option);
    }
    deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in deck_targets {
        let p = target.player as usize;
        let offset = target.index as usize;
        if offset >= env.state.players[p].deck.len() {
            continue;
        }
        let deck_idx = env.state.players[p].deck.len().saturating_sub(1 + offset);
        if deck_idx >= env.state.players[p].deck.len() {
            continue;
        }
        if env.state.players[p].deck[deck_idx].instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].deck.remove(deck_idx);
        env.move_card_between_zones(target.player, card, Zone::Deck, Zone::Hand, None, None);
    }
}

pub(super) fn move_to_waiting_room(env: &mut GameEnv, payload: &EffectPayload) {
    let mut stage_targets: Vec<TargetRef> = Vec::new();
    let mut hand_targets: Vec<TargetRef> = Vec::new();
    let mut deck_targets: Vec<TargetRef> = Vec::new();
    let mut clock_targets: Vec<TargetRef> = Vec::new();
    let mut level_targets: Vec<TargetRef> = Vec::new();
    let mut stock_targets: Vec<TargetRef> = Vec::new();
    let mut memory_targets: Vec<TargetRef> = Vec::new();
    let mut climax_targets: Vec<TargetRef> = Vec::new();
    let mut resolution_targets: Vec<TargetRef> = Vec::new();
    let mut waiting_targets: Vec<TargetRef> = Vec::new();
    for target in &payload.targets {
        match target.zone {
            TargetZone::Stage => stage_targets.push(*target),
            TargetZone::Hand => hand_targets.push(*target),
            TargetZone::DeckTop => deck_targets.push(*target),
            TargetZone::Clock => clock_targets.push(*target),
            TargetZone::Level => level_targets.push(*target),
            TargetZone::Stock => stock_targets.push(*target),
            TargetZone::Memory => memory_targets.push(*target),
            TargetZone::Climax => climax_targets.push(*target),
            TargetZone::Resolution => resolution_targets.push(*target),
            TargetZone::WaitingRoom => waiting_targets.push(*target),
        }
    }
    for target in stage_targets {
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= env.state.players[p].stage.len() {
            continue;
        }
        let Some(card_inst) = env.state.players[p].stage[slot].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.remove_modifiers_for_slot(target.player, target.index);
        env.drain_stage_markers_to_waiting_room(target.player, target.index);
        env.state.players[p].stage[slot] = StageSlot::empty();
        env.mark_slot_power_dirty(target.player, target.index);
        env.move_card_between_zones(
            target.player,
            card_inst,
            Zone::Stage,
            Zone::WaitingRoom,
            Some(target.index),
            None,
        );
    }
    hand_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in hand_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].hand.len() {
            continue;
        }
        let Some(card) = env.state.players[p].hand.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].hand.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Hand,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    clock_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in clock_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].clock.len() {
            continue;
        }
        let Some(card) = env.state.players[p].clock.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].clock.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Clock,
            Zone::WaitingRoom,
            None,
            None,
        );
        env.check_level_up(target.player);
    }
    level_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in level_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].level.len() {
            continue;
        }
        let Some(card) = env.state.players[p].level.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].level.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Level,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    stock_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in stock_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].stock.len() {
            continue;
        }
        let Some(card) = env.state.players[p].stock.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].stock.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Stock,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    memory_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in memory_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].memory.len() {
            continue;
        }
        let Some(card) = env.state.players[p].memory.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].memory.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Memory,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    climax_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in climax_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].climax.len() {
            continue;
        }
        let Some(card) = env.state.players[p].climax.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].climax.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Climax,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    resolution_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in resolution_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].resolution.len() {
            continue;
        }
        let Some(card) = env.state.players[p].resolution.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].resolution.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Resolution,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    waiting_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in waiting_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].waiting_room.len() {
            continue;
        }
        let Some(card) = env.state.players[p].waiting_room.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].waiting_room.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::WaitingRoom,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in deck_targets {
        let p = target.player as usize;
        let offset = target.index as usize;
        if offset >= env.state.players[p].deck.len() {
            continue;
        }
        let deck_idx = env.state.players[p].deck.len().saturating_sub(1 + offset);
        if deck_idx >= env.state.players[p].deck.len() {
            continue;
        }
        if env.state.players[p].deck[deck_idx].instance_id != target.instance_id {
            continue;
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

pub(super) fn move_to_stock(env: &mut GameEnv, payload: &EffectPayload) {
    let mut stage_targets: Vec<TargetRef> = Vec::new();
    let mut hand_targets: Vec<TargetRef> = Vec::new();
    let mut deck_targets: Vec<TargetRef> = Vec::new();
    let mut clock_targets: Vec<TargetRef> = Vec::new();
    let mut level_targets: Vec<TargetRef> = Vec::new();
    let mut waiting_targets: Vec<TargetRef> = Vec::new();
    let mut memory_targets: Vec<TargetRef> = Vec::new();
    let mut climax_targets: Vec<TargetRef> = Vec::new();
    let mut resolution_targets: Vec<TargetRef> = Vec::new();
    for target in &payload.targets {
        match target.zone {
            TargetZone::Stage => stage_targets.push(*target),
            TargetZone::Hand => hand_targets.push(*target),
            TargetZone::DeckTop => deck_targets.push(*target),
            TargetZone::Clock => clock_targets.push(*target),
            TargetZone::Level => level_targets.push(*target),
            TargetZone::WaitingRoom => waiting_targets.push(*target),
            TargetZone::Memory => memory_targets.push(*target),
            TargetZone::Climax => climax_targets.push(*target),
            TargetZone::Resolution => resolution_targets.push(*target),
            TargetZone::Stock => {}
        }
    }
    for target in stage_targets {
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= env.state.players[p].stage.len() {
            continue;
        }
        let Some(card_inst) = env.state.players[p].stage[slot].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.remove_modifiers_for_slot(target.player, target.index);
        env.drain_stage_markers_to_waiting_room(target.player, target.index);
        env.state.players[p].stage[slot] = StageSlot::empty();
        env.mark_slot_power_dirty(target.player, target.index);
        env.move_card_between_zones(
            target.player,
            card_inst,
            Zone::Stage,
            Zone::Stock,
            Some(target.index),
            None,
        );
    }
    hand_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in hand_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].hand.len() {
            continue;
        }
        let Some(card) = env.state.players[p].hand.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].hand.remove(idx);
        env.move_card_between_zones(target.player, card, Zone::Hand, Zone::Stock, None, None);
    }
    clock_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in clock_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].clock.len() {
            continue;
        }
        let Some(card) = env.state.players[p].clock.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].clock.remove(idx);
        env.move_card_between_zones(target.player, card, Zone::Clock, Zone::Stock, None, None);
        env.check_level_up(target.player);
    }
    level_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in level_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].level.len() {
            continue;
        }
        let Some(card) = env.state.players[p].level.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].level.remove(idx);
        env.move_card_between_zones(target.player, card, Zone::Level, Zone::Stock, None, None);
    }
    waiting_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in waiting_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].waiting_room.len() {
            continue;
        }
        let Some(card) = env.state.players[p].waiting_room.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].waiting_room.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::WaitingRoom,
            Zone::Stock,
            None,
            None,
        );
    }
    memory_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in memory_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].memory.len() {
            continue;
        }
        let Some(card) = env.state.players[p].memory.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].memory.remove(idx);
        env.move_card_between_zones(target.player, card, Zone::Memory, Zone::Stock, None, None);
    }
    climax_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in climax_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].climax.len() {
            continue;
        }
        let Some(card) = env.state.players[p].climax.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].climax.remove(idx);
        env.move_card_between_zones(target.player, card, Zone::Climax, Zone::Stock, None, None);
    }
    resolution_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in resolution_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].resolution.len() {
            continue;
        }
        let Some(card) = env.state.players[p].resolution.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].resolution.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Resolution,
            Zone::Stock,
            None,
            None,
        );
    }
    deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in deck_targets {
        let p = target.player as usize;
        let offset = target.index as usize;
        if offset >= env.state.players[p].deck.len() {
            continue;
        }
        let deck_idx = env.state.players[p].deck.len().saturating_sub(1 + offset);
        if deck_idx >= env.state.players[p].deck.len() {
            continue;
        }
        if env.state.players[p].deck[deck_idx].instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].deck.remove(deck_idx);
        env.move_card_between_zones(target.player, card, Zone::Deck, Zone::Stock, None, None);
    }
}

pub(super) fn move_to_clock(env: &mut GameEnv, payload: &EffectPayload) {
    let mut stage_targets: Vec<TargetRef> = Vec::new();
    let mut hand_targets: Vec<TargetRef> = Vec::new();
    let mut deck_targets: Vec<TargetRef> = Vec::new();
    let mut waiting_targets: Vec<TargetRef> = Vec::new();
    let mut resolution_targets: Vec<TargetRef> = Vec::new();
    for target in &payload.targets {
        match target.zone {
            TargetZone::Stage => stage_targets.push(*target),
            TargetZone::Hand => hand_targets.push(*target),
            TargetZone::DeckTop => deck_targets.push(*target),
            TargetZone::WaitingRoom => waiting_targets.push(*target),
            TargetZone::Resolution => resolution_targets.push(*target),
            TargetZone::Clock => {}
            TargetZone::Level => {}
            TargetZone::Stock => {}
            TargetZone::Memory => {}
            TargetZone::Climax => {}
        }
    }
    for target in stage_targets {
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= env.state.players[p].stage.len() {
            continue;
        }
        let Some(card_inst) = env.state.players[p].stage[slot].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.remove_modifiers_for_slot(target.player, target.index);
        env.drain_stage_markers_to_waiting_room(target.player, target.index);
        env.state.players[p].stage[slot] = StageSlot::empty();
        env.mark_slot_power_dirty(target.player, target.index);
        env.move_card_between_zones(
            target.player,
            card_inst,
            Zone::Stage,
            Zone::Clock,
            Some(target.index),
            None,
        );
        env.check_level_up(target.player);
    }
    hand_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in hand_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].hand.len() {
            continue;
        }
        let Some(card) = env.state.players[p].hand.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].hand.remove(idx);
        env.move_card_between_zones(target.player, card, Zone::Hand, Zone::Clock, None, None);
        env.check_level_up(target.player);
    }
    waiting_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in waiting_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].waiting_room.len() {
            continue;
        }
        let Some(card) = env.state.players[p].waiting_room.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].waiting_room.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::WaitingRoom,
            Zone::Clock,
            None,
            None,
        );
        env.check_level_up(target.player);
    }
    resolution_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in resolution_targets {
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].resolution.len() {
            continue;
        }
        let Some(card) = env.state.players[p].resolution.get(idx).copied() else {
            continue;
        };
        if card.instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].resolution.remove(idx);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Resolution,
            Zone::Clock,
            None,
            None,
        );
        env.check_level_up(target.player);
    }
    deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in deck_targets {
        let p = target.player as usize;
        let offset = target.index as usize;
        if offset >= env.state.players[p].deck.len() {
            continue;
        }
        let deck_idx = env.state.players[p].deck.len().saturating_sub(1 + offset);
        if deck_idx >= env.state.players[p].deck.len() {
            continue;
        }
        if env.state.players[p].deck[deck_idx].instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].deck.remove(deck_idx);
        env.move_card_between_zones(target.player, card, Zone::Deck, Zone::Clock, None, None);
        env.check_level_up(target.player);
    }
}

pub(super) fn move_to_memory(env: &mut GameEnv, payload: &EffectPayload) {
    let mut stage_targets: Vec<TargetRef> = Vec::new();
    let mut deck_targets: Vec<TargetRef> = Vec::new();
    for target in &payload.targets {
        match target.zone {
            TargetZone::Stage => stage_targets.push(*target),
            TargetZone::DeckTop => deck_targets.push(*target),
            _ => {}
        }
    }
    for target in stage_targets {
        let _ = env.move_stage_target_to_memory(target);
    }
    deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in deck_targets {
        let p = target.player as usize;
        let offset = target.index as usize;
        if offset >= env.state.players[p].deck.len() {
            continue;
        }
        let deck_idx = env.state.players[p].deck.len().saturating_sub(1 + offset);
        if deck_idx >= env.state.players[p].deck.len() {
            continue;
        }
        if env.state.players[p].deck[deck_idx].instance_id != target.instance_id {
            continue;
        }
        let card = env.state.players[p].deck.remove(deck_idx);
        env.move_card_between_zones(target.player, card, Zone::Deck, Zone::Memory, None, None);
    }
}

pub(super) fn move_to_deck_bottom(env: &mut GameEnv, payload: &EffectPayload) {
    let mut stage_targets: Vec<TargetRef> = Vec::new();
    let mut deck_targets: Vec<TargetRef> = Vec::new();
    for target in &payload.targets {
        match target.zone {
            TargetZone::Stage => stage_targets.push(*target),
            TargetZone::DeckTop => deck_targets.push(*target),
            _ => {}
        }
    }
    for target in stage_targets {
        env.move_target_to_deck_bottom(target);
    }
    deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
    for target in deck_targets {
        env.move_target_to_deck_bottom(target);
    }
}

pub(super) fn move_waiting_room_card_to_source_slot(env: &mut GameEnv, payload: &EffectPayload) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let p = source_ref.player as usize;
    let s = source_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    let mut waiting_targets: Vec<TargetRef> = payload
        .targets
        .iter()
        .copied()
        .filter(|target| {
            target.zone == TargetZone::WaitingRoom && target.player == source_ref.player
        })
        .collect();
    waiting_targets.sort_by_key(|target| std::cmp::Reverse(target.index));
    let Some(target) = waiting_targets.into_iter().next() else {
        return;
    };
    let tp = target.player as usize;
    let idx = target.index as usize;
    if idx >= env.state.players[tp].waiting_room.len() {
        return;
    }
    let Some(card_inst) = env.state.players[tp].waiting_room.get(idx).copied() else {
        return;
    };
    if card_inst.instance_id != target.instance_id {
        return;
    }
    let card = env.state.players[tp].waiting_room.remove(idx);
    env.place_card_on_stage(
        target.player,
        card,
        source_ref.index,
        StageStatus::Stand,
        Zone::WaitingRoom,
        Some(target.index),
    );
}

pub(super) fn recycle_waiting_room_to_deck_shuffle(env: &mut GameEnv, controller: u8) {
    let p = controller as usize;
    while let Some(card) = env.state.players[p].waiting_room.pop() {
        env.move_card_between_zones(controller, card, Zone::WaitingRoom, Zone::Deck, None, None);
    }
    env.shuffle_deck(controller);
}

pub(super) fn reset_stock_from_deck_top(env: &mut GameEnv, controller: u8, target: TargetSide) {
    let target_player = match target {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    let p = target_player as usize;
    let stock_count = env.state.players[p].stock.len();
    while let Some(card) = env.state.players[p].stock.pop() {
        env.move_card_between_zones(
            target_player,
            card,
            Zone::Stock,
            Zone::WaitingRoom,
            None,
            None,
        );
    }
    for _ in 0..stock_count {
        if let Some(card) = env.draw_from_deck(target_player) {
            env.move_card_between_zones(target_player, card, Zone::Deck, Zone::Stock, None, None);
        }
    }
}

pub(super) fn move_to_marker(env: &mut GameEnv, payload: &EffectPayload) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let source_player = source_ref.player as usize;
    let source_slot = source_ref.index as usize;
    if source_slot >= env.state.players[source_player].stage.len() {
        return;
    }
    if env.state.players[source_player].stage[source_slot]
        .card
        .map(|card| card.instance_id)
        != Some(source_ref.instance_id)
    {
        return;
    }
    let mut waiting_targets: Vec<TargetRef> = payload
        .targets
        .iter()
        .copied()
        .filter(|target| target.zone == TargetZone::WaitingRoom)
        .collect();
    waiting_targets.sort_by_key(|target| std::cmp::Reverse(target.index));
    for target in waiting_targets {
        let _ = env.move_waiting_room_to_marker(
            target.player,
            target.index,
            target.instance_id,
            source_ref.player,
            source_ref.index,
        );
    }
}

pub(super) fn move_top_deck_to_marker(env: &mut GameEnv, payload: &EffectPayload) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let p = source_ref.player as usize;
    let s = source_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s]
        .card
        .map(|card| card.instance_id)
        != Some(source_ref.instance_id)
    {
        return;
    }
    let Some(card) = env.draw_from_deck(source_ref.player) else {
        return;
    };
    env.state.players[p].stage[s].markers.push(card);
    env.touch_player_obs(source_ref.player);
    env.mark_continuous_modifiers_dirty();
    env.mark_slot_power_dirty(source_ref.player, source_ref.index);
}

pub(super) fn mill_top(env: &mut GameEnv, controller: u8, target: TargetSide, count: u8) {
    let target_player = match target {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    for _ in 0..count {
        if let Some(card) = env.draw_from_deck(target_player) {
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
}

pub(super) fn move_stage_slot(env: &mut GameEnv, payload: &EffectPayload, slot: u8) {
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let idx = target.index as usize;
        if idx >= env.state.players[p].stage.len() {
            continue;
        }
        let Some(card_inst) = env.state.players[p].stage[idx].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.swap_stage_slots(target.player, target.index, slot);
    }
}

pub(super) fn move_this_to_open_center(
    env: &mut GameEnv,
    payload: &EffectPayload,
    require_facing: bool,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let p = source_ref.player as usize;
    let source_slot = source_ref.index as usize;
    if source_slot >= env.state.players[p].stage.len() {
        return;
    }
    let Some(card_inst) = env.state.players[p].stage[source_slot].card else {
        return;
    };
    if card_inst.instance_id != source_ref.instance_id {
        return;
    }
    let center_slots: &[u8] = if env.curriculum.reduced_stage_mode {
        &[0]
    } else {
        &[0, 1, 2]
    };
    let dest = center_slots.iter().copied().find(|&slot| {
        let idx = slot as usize;
        if idx >= env.state.players[p].stage.len() {
            return false;
        }
        if env.state.players[p].stage[idx].card.is_some() {
            return false;
        }
        if !require_facing {
            return true;
        }
        let opp = 1 - p;
        idx < env.state.players[opp].stage.len() && env.state.players[opp].stage[idx].card.is_some()
    });
    if let Some(dest_slot) = dest {
        env.swap_stage_slots(source_ref.player, source_ref.index, dest_slot);
    }
}

pub(super) fn move_this_to_open_back(env: &mut GameEnv, payload: &EffectPayload) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let p = source_ref.player as usize;
    let source_slot = source_ref.index as usize;
    if source_slot >= env.state.players[p].stage.len() {
        return;
    }
    let Some(card_inst) = env.state.players[p].stage[source_slot].card else {
        return;
    };
    if card_inst.instance_id != source_ref.instance_id {
        return;
    }
    let back_slots: &[u8] = if env.curriculum.reduced_stage_mode {
        &[]
    } else {
        &[3, 4]
    };
    let dest = back_slots.iter().copied().find(|&slot| {
        let idx = slot as usize;
        idx < env.state.players[p].stage.len() && env.state.players[p].stage[idx].card.is_none()
    });
    if let Some(dest_slot) = dest {
        env.swap_stage_slots(source_ref.player, source_ref.index, dest_slot);
    }
}

pub(super) fn swap_stage_slots(env: &mut GameEnv, payload: &EffectPayload) {
    let mut stage_targets: Vec<TargetRef> = payload
        .targets
        .iter()
        .copied()
        .filter(|t| t.zone == TargetZone::Stage)
        .collect();
    if stage_targets.len() < 2 {
        return;
    }
    stage_targets.sort_by_key(|t| (t.player, t.index, t.instance_id));
    let first = stage_targets[0];
    let second = stage_targets[1];
    if first.player != second.player {
        return;
    }
    let p = first.player as usize;
    let f_idx = first.index as usize;
    let s_idx = second.index as usize;
    if f_idx >= env.state.players[p].stage.len() || s_idx >= env.state.players[p].stage.len() {
        return;
    }
    let Some(f_card) = env.state.players[p].stage[f_idx].card else {
        return;
    };
    let Some(s_card) = env.state.players[p].stage[s_idx].card else {
        return;
    };
    if f_card.instance_id != first.instance_id || s_card.instance_id != second.instance_id {
        return;
    }
    env.swap_stage_slots(first.player, first.index, second.index);
}

pub(super) fn change_controller(
    env: &mut GameEnv,
    controller: u8,
    payload: &EffectPayload,
    new_controller: TargetSide,
) {
    let to_player = match new_controller {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let from_player = target.player;
        if from_player == to_player {
            continue;
        }
        let from_slot = target.index as usize;
        let to_slot = target.index as usize;
        if from_slot >= env.state.players[from_player as usize].stage.len()
            || to_slot >= env.state.players[to_player as usize].stage.len()
        {
            continue;
        }
        if env.state.players[to_player as usize].stage[to_slot]
            .card
            .is_some()
        {
            continue;
        }
        let Some(card_inst) = env.state.players[from_player as usize].stage[from_slot].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.remove_modifiers_for_slot(from_player, target.index);
        let mut moved_slot = std::mem::replace(
            &mut env.state.players[from_player as usize].stage[from_slot],
            StageSlot::empty(),
        );
        let Some(mut moved_card) = moved_slot.card.take() else {
            continue;
        };
        moved_card.controller = to_player;
        moved_slot.card = Some(moved_card);
        env.state.players[to_player as usize].stage[to_slot] = moved_slot;
        env.mark_slot_power_dirty(from_player, target.index);
        env.mark_slot_power_dirty(to_player, target.index);
        env.mark_rule_actions_dirty();
        env.mark_continuous_modifiers_dirty();
        env.log_event(Event::ControlChanged {
            card: moved_card.id,
            owner: moved_card.owner,
            from_controller: from_player,
            to_controller: to_player,
            from_slot: target.index,
            to_slot: target.index,
        });
    }
}

pub(super) fn rest_this_if_no_other_rest_center(env: &mut GameEnv, payload: &EffectPayload) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let p = source_ref.player as usize;
    let source_slot = source_ref.index as usize;
    if source_slot >= env.state.players[p].stage.len() {
        return;
    }
    let Some(card_inst) = env.state.players[p].stage[source_slot].card else {
        return;
    };
    if card_inst.instance_id != source_ref.instance_id {
        return;
    }
    let center_slots: &[u8] = if env.curriculum.reduced_stage_mode {
        &[0]
    } else {
        &[0, 1, 2]
    };
    let has_other_rest = center_slots.iter().copied().any(|slot| {
        let idx = slot as usize;
        if idx == source_slot || idx >= env.state.players[p].stage.len() {
            return false;
        }
        let slot_state = &env.state.players[p].stage[idx];
        slot_state.card.is_some() && slot_state.status == StageStatus::Rest
    });
    if has_other_rest {
        return;
    }
    let slot_state = &mut env.state.players[p].stage[source_slot];
    if slot_state.card.is_none() {
        return;
    }
    slot_state.status = StageStatus::Rest;
    env.mark_slot_power_dirty(source_ref.player, source_ref.index);
    env.mark_continuous_modifiers_dirty();
    env.touch_player_obs(source_ref.player);
}

pub(super) fn rest_target(env: &mut GameEnv, payload: &EffectPayload) {
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= env.state.players[p].stage.len() {
            continue;
        }
        let Some(card_inst) = env.state.players[p].stage[slot].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.state.players[p].stage[slot].status = StageStatus::Rest;
        env.mark_slot_power_dirty(target.player, target.index);
        env.mark_continuous_modifiers_dirty();
    }
}

pub(super) fn stand_target(env: &mut GameEnv, payload: &EffectPayload) {
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let slot = target.index as usize;
        if slot >= env.state.players[p].stage.len() {
            continue;
        }
        let Some(card_inst) = env.state.players[p].stage[slot].card else {
            continue;
        };
        if card_inst.instance_id != target.instance_id {
            continue;
        }
        env.state.players[p].stage[slot].status = StageStatus::Stand;
        env.mark_slot_power_dirty(target.player, target.index);
        env.mark_continuous_modifiers_dirty();
    }
}

pub(super) fn stock_charge(env: &mut GameEnv, controller: u8, count: u8) {
    for _ in 0..count {
        if let Some(card) = env.draw_from_deck(controller) {
            env.move_card_between_zones(controller, card, Zone::Deck, Zone::Stock, None, None);
        }
    }
}
