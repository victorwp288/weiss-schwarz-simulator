use super::prelude::*;

#[inline(always)]
fn collect_targets_for_zone(payload: &EffectPayload, zone: TargetZone) -> Vec<TargetRef> {
    payload
        .targets
        .iter()
        .copied()
        .filter(|target| target.zone == zone)
        .collect()
}

#[inline(always)]
fn apply_targets_in_reverse_index<F>(targets: &mut Vec<TargetRef>, mut apply: F)
where
    F: FnMut(TargetRef),
{
    targets.sort_by_key(|target| std::cmp::Reverse(target.index));
    for target in targets.iter().copied() {
        apply(target);
    }
}

#[inline(always)]
fn remove_indexed_card_if_matches(
    cards: &mut Vec<CardInstance>,
    target: TargetRef,
) -> Option<CardInstance> {
    let idx = target.index as usize;
    if idx >= cards.len() {
        return None;
    }
    let Some(card) = cards.get(idx).copied() else {
        return None;
    };
    if card.instance_id != target.instance_id {
        return None;
    }
    Some(cards.remove(idx))
}

#[inline(always)]
fn remove_deck_card_by_top_offset_if_matches(
    deck: &mut Vec<CardInstance>,
    target: TargetRef,
) -> Option<CardInstance> {
    let offset = target.index as usize;
    if offset >= deck.len() {
        return None;
    }
    let deck_idx = deck.len().saturating_sub(1 + offset);
    if deck_idx >= deck.len() {
        return None;
    }
    if deck[deck_idx].instance_id != target.instance_id {
        return None;
    }
    Some(deck.remove(deck_idx))
}

#[inline(always)]
fn move_indexed_zone_targets<ZoneCards, AfterMove>(
    env: &mut GameEnv,
    targets: &mut Vec<TargetRef>,
    from_zone: Zone,
    to_zone: Zone,
    mut zone_cards: ZoneCards,
    mut after_move: AfterMove,
) where
    ZoneCards: for<'a> FnMut(&'a mut PlayerState) -> &'a mut Vec<CardInstance>,
    AfterMove: FnMut(&mut GameEnv, u8),
{
    apply_targets_in_reverse_index(targets, |target| {
        let p = target.player as usize;
        let card = {
            let cards = zone_cards(&mut env.state.players[p]);
            remove_indexed_card_if_matches(cards, target)
        };
        let Some(card) = card else {
            return;
        };
        env.move_card_between_zones(target.player, card, from_zone, to_zone, None, None);
        after_move(env, target.player);
    });
}

#[inline(always)]
fn move_deck_targets<AfterMove>(
    env: &mut GameEnv,
    targets: &mut Vec<TargetRef>,
    to_zone: Zone,
    mut after_move: AfterMove,
) where
    AfterMove: FnMut(&mut GameEnv, u8),
{
    apply_targets_in_reverse_index(targets, |target| {
        let p = target.player as usize;
        let card =
            remove_deck_card_by_top_offset_if_matches(&mut env.state.players[p].deck, target);
        let Some(card) = card else {
            return;
        };
        env.move_card_between_zones(target.player, card, Zone::Deck, to_zone, None, None);
        after_move(env, target.player);
    });
}

pub(super) fn move_to_hand(env: &mut GameEnv, payload: &EffectPayload) {
    // Invariants:
    // - Preserve deterministic reverse-index application for indexed zone moves.
    // - Reverse-order behavior is covered by `weiss_core/tests/zone_move_semantics_tests.rs`.
    let stage_targets = collect_targets_for_zone(payload, TargetZone::Stage);
    let mut waiting_room_targets = collect_targets_for_zone(payload, TargetZone::WaitingRoom);
    let mut deck_targets = collect_targets_for_zone(payload, TargetZone::DeckTop);
    for target in stage_targets {
        let option = ChoiceOptionRef {
            card_id: target.card_id,
            instance_id: target.instance_id,
            zone: ChoiceZone::Stage,
            index: Some(target.index as u16),
            target_slot: None,
        };
        env.move_stage_to_hand(target.player, option);
    }
    apply_targets_in_reverse_index(&mut waiting_room_targets, |target| {
        let option = ChoiceOptionRef {
            card_id: target.card_id,
            instance_id: target.instance_id,
            zone: ChoiceZone::WaitingRoom,
            index: Some(target.index as u16),
            target_slot: None,
        };
        env.move_waiting_room_to_hand(target.player, option);
    });
    move_deck_targets(env, &mut deck_targets, Zone::Hand, |_, _| {});
}

pub(super) fn move_to_waiting_room(env: &mut GameEnv, payload: &EffectPayload) {
    // Invariants:
    // - Preserve deterministic reverse-index application for indexed zone moves.
    // - Reverse-order behavior is covered by `weiss_core/tests/zone_move_semantics_tests.rs`.
    let stage_targets = collect_targets_for_zone(payload, TargetZone::Stage);
    let mut hand_targets = collect_targets_for_zone(payload, TargetZone::Hand);
    let mut deck_targets = collect_targets_for_zone(payload, TargetZone::DeckTop);
    let mut clock_targets = collect_targets_for_zone(payload, TargetZone::Clock);
    let mut level_targets = collect_targets_for_zone(payload, TargetZone::Level);
    let mut stock_targets = collect_targets_for_zone(payload, TargetZone::Stock);
    let mut memory_targets = collect_targets_for_zone(payload, TargetZone::Memory);
    let mut climax_targets = collect_targets_for_zone(payload, TargetZone::Climax);
    let mut resolution_targets = collect_targets_for_zone(payload, TargetZone::Resolution);
    let mut waiting_targets = collect_targets_for_zone(payload, TargetZone::WaitingRoom);
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
    move_indexed_zone_targets(
        env,
        &mut hand_targets,
        Zone::Hand,
        Zone::WaitingRoom,
        |player| &mut player.hand,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut clock_targets,
        Zone::Clock,
        Zone::WaitingRoom,
        |player| &mut player.clock,
        |env, player| env.check_level_up(player),
    );
    move_indexed_zone_targets(
        env,
        &mut level_targets,
        Zone::Level,
        Zone::WaitingRoom,
        |player| &mut player.level,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut stock_targets,
        Zone::Stock,
        Zone::WaitingRoom,
        |player| &mut player.stock,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut memory_targets,
        Zone::Memory,
        Zone::WaitingRoom,
        |player| &mut player.memory,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut climax_targets,
        Zone::Climax,
        Zone::WaitingRoom,
        |player| &mut player.climax,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut resolution_targets,
        Zone::Resolution,
        Zone::WaitingRoom,
        |player| &mut player.resolution,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut waiting_targets,
        Zone::WaitingRoom,
        Zone::WaitingRoom,
        |player| &mut player.waiting_room,
        |_, _| {},
    );
    move_deck_targets(env, &mut deck_targets, Zone::WaitingRoom, |_, _| {});
}

pub(super) fn move_to_stock(env: &mut GameEnv, payload: &EffectPayload) {
    // Invariants:
    // - Preserve deterministic reverse-index application for indexed zone moves.
    // - Reverse-order behavior is covered by `weiss_core/tests/zone_move_semantics_tests.rs`.
    let stage_targets = collect_targets_for_zone(payload, TargetZone::Stage);
    let mut hand_targets = collect_targets_for_zone(payload, TargetZone::Hand);
    let mut deck_targets = collect_targets_for_zone(payload, TargetZone::DeckTop);
    let mut clock_targets = collect_targets_for_zone(payload, TargetZone::Clock);
    let mut level_targets = collect_targets_for_zone(payload, TargetZone::Level);
    let mut waiting_targets = collect_targets_for_zone(payload, TargetZone::WaitingRoom);
    let mut memory_targets = collect_targets_for_zone(payload, TargetZone::Memory);
    let mut climax_targets = collect_targets_for_zone(payload, TargetZone::Climax);
    let mut resolution_targets = collect_targets_for_zone(payload, TargetZone::Resolution);
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
    move_indexed_zone_targets(
        env,
        &mut hand_targets,
        Zone::Hand,
        Zone::Stock,
        |player| &mut player.hand,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut clock_targets,
        Zone::Clock,
        Zone::Stock,
        |player| &mut player.clock,
        |env, player| env.check_level_up(player),
    );
    move_indexed_zone_targets(
        env,
        &mut level_targets,
        Zone::Level,
        Zone::Stock,
        |player| &mut player.level,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut waiting_targets,
        Zone::WaitingRoom,
        Zone::Stock,
        |player| &mut player.waiting_room,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut memory_targets,
        Zone::Memory,
        Zone::Stock,
        |player| &mut player.memory,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut climax_targets,
        Zone::Climax,
        Zone::Stock,
        |player| &mut player.climax,
        |_, _| {},
    );
    move_indexed_zone_targets(
        env,
        &mut resolution_targets,
        Zone::Resolution,
        Zone::Stock,
        |player| &mut player.resolution,
        |_, _| {},
    );
    move_deck_targets(env, &mut deck_targets, Zone::Stock, |_, _| {});
}

pub(super) fn move_to_clock(env: &mut GameEnv, payload: &EffectPayload) {
    // Invariants:
    // - Preserve deterministic reverse-index application for indexed zone moves.
    // - Reverse-order behavior is covered by `weiss_core/tests/zone_move_semantics_tests.rs`.
    let stage_targets = collect_targets_for_zone(payload, TargetZone::Stage);
    let mut hand_targets = collect_targets_for_zone(payload, TargetZone::Hand);
    let mut deck_targets = collect_targets_for_zone(payload, TargetZone::DeckTop);
    let mut waiting_targets = collect_targets_for_zone(payload, TargetZone::WaitingRoom);
    let mut resolution_targets = collect_targets_for_zone(payload, TargetZone::Resolution);
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
    move_indexed_zone_targets(
        env,
        &mut hand_targets,
        Zone::Hand,
        Zone::Clock,
        |player| &mut player.hand,
        |env, player| env.check_level_up(player),
    );
    move_indexed_zone_targets(
        env,
        &mut waiting_targets,
        Zone::WaitingRoom,
        Zone::Clock,
        |player| &mut player.waiting_room,
        |env, player| env.check_level_up(player),
    );
    move_indexed_zone_targets(
        env,
        &mut resolution_targets,
        Zone::Resolution,
        Zone::Clock,
        |player| &mut player.resolution,
        |env, player| env.check_level_up(player),
    );
    move_deck_targets(env, &mut deck_targets, Zone::Clock, |env, player| {
        env.check_level_up(player);
    });
}

pub(super) fn move_to_memory(env: &mut GameEnv, payload: &EffectPayload) {
    // Invariants:
    // - Preserve deterministic reverse-index application for indexed zone moves.
    // - Reverse-order behavior is covered by `weiss_core/tests/zone_move_semantics_tests.rs`.
    let stage_targets = collect_targets_for_zone(payload, TargetZone::Stage);
    let mut deck_targets = collect_targets_for_zone(payload, TargetZone::DeckTop);
    for target in stage_targets {
        let _ = env.move_stage_target_to_memory(target);
    }
    move_deck_targets(env, &mut deck_targets, Zone::Memory, |_, _| {});
}

pub(super) fn move_to_deck_bottom(env: &mut GameEnv, payload: &EffectPayload) {
    // Invariants:
    // - Preserve deterministic reverse-index application for indexed zone moves.
    // - Reverse-order behavior is covered by `weiss_core/tests/zone_move_semantics_tests.rs`.
    let stage_targets = collect_targets_for_zone(payload, TargetZone::Stage);
    let mut deck_targets = collect_targets_for_zone(payload, TargetZone::DeckTop);
    for target in stage_targets {
        env.move_target_to_deck_bottom(target);
    }
    apply_targets_in_reverse_index(&mut deck_targets, |target| {
        env.move_target_to_deck_bottom(target);
    });
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
        env.touch_player_obs(target.player);
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
        env.touch_player_obs(target.player);
    }
}

pub(super) fn stock_charge(env: &mut GameEnv, controller: u8, count: u8) {
    for _ in 0..count {
        if let Some(card) = env.draw_from_deck(controller) {
            env.move_card_between_zones(controller, card, Zone::Deck, Zone::Stock, None, None);
        }
    }
}
