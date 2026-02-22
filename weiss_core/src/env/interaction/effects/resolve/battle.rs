use super::prelude::*;

use crate::db::{BattleOpponentMoveDestination, BattleOpponentMovePreludeAction};

pub(super) fn battle_opponent_reverse_if(
    env: &mut GameEnv,
    payload: &EffectPayload,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    let p = target.player as usize;
    let s = target.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].status == StageStatus::Reverse {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(target.instance_id) {
        return;
    }
    env.state.players[p].stage[s].status = StageStatus::Reverse;
    env.mark_slot_power_dirty(target.player, target.index);
    let cause_damage_event = env
        .state
        .turn
        .attack
        .as_ref()
        .and_then(|ctx| ctx.last_damage_event_id);
    env.log_event(Event::ReversalCommitted {
        player: target.player,
        slot: target.index,
        cause_damage_event,
    });
    env.queue_on_reverse_triggers(&[(target.player, target.card_id)]);
    env.queue_battle_opponent_reverse_triggers(&[(source_ref.player, source_ref.card_id)]);
}

pub(super) fn battle_opponent_move_to_deck_bottom_if(
    env: &mut GameEnv,
    payload: &EffectPayload,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    env.move_target_to_deck_bottom(target);
}

pub(super) fn battle_opponent_move_to_stock_then_bottom_stock_to_waiting_room_if(
    env: &mut GameEnv,
    payload: &EffectPayload,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    if !env.move_stage_target_to_stock(target) {
        return;
    }
    let _ = env.move_bottom_stock_to_waiting_room(target.player);
}

pub(super) fn battle_opponent_move_to_clock_after_clock_top_to_waiting_room_if(
    env: &mut GameEnv,
    payload: &EffectPayload,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    if !env.move_top_clock_to_waiting_room(target.player) {
        return;
    }
    let _ = env.move_stage_target_to_clock(target);
}

pub(super) fn battle_opponent_move_to_memory_if(
    env: &mut GameEnv,
    payload: &EffectPayload,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    let _ = env.move_stage_target_to_memory(target);
}

pub(super) fn battle_opponent_move_to_clock_if(
    env: &mut GameEnv,
    payload: &EffectPayload,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    let _ = env.move_stage_target_to_clock(target);
}

pub(super) fn battle_opponent_move(
    env: &mut GameEnv,
    payload: &EffectPayload,
    destination: BattleOpponentMoveDestination,
    prelude: Option<BattleOpponentMovePreludeAction>,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_opponent_level: bool,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if !env.battle_opponent_condition_met(&target, max_level, max_cost, level_gt_opponent_level) {
        return;
    }
    if matches!(
        prelude,
        Some(BattleOpponentMovePreludeAction::OpponentClockTopToWaitingRoom)
    ) && !env.move_top_clock_to_waiting_room(target.player)
    {
        return;
    }
    match destination {
        BattleOpponentMoveDestination::DeckBottom => {
            env.move_target_to_deck_bottom(target);
        }
        BattleOpponentMoveDestination::StockThenBottomStockToWaitingRoom => {
            if env.move_stage_target_to_stock(target) {
                let _ = env.move_bottom_stock_to_waiting_room(target.player);
            }
        }
        BattleOpponentMoveDestination::Clock => {
            let _ = env.move_stage_target_to_clock(target);
        }
        BattleOpponentMoveDestination::Memory => {
            let _ = env.move_stage_target_to_memory(target);
        }
    }
}

pub(super) fn battle_opponent_top_deck_to_stock_if(
    env: &mut GameEnv,
    controller: u8,
    payload: &EffectPayload,
    min_level: u8,
) {
    let Some(source_ref) = payload.source_ref.as_ref() else {
        return;
    };
    let Some(target) = env.battle_opponent_target_from_source(source_ref) else {
        return;
    };
    if target.zone != TargetZone::Stage {
        return;
    }
    let target_level = env.compute_slot_level(target.player as usize, target.index as usize);
    if target_level < i32::from(min_level) {
        return;
    }
    if let Some(card) = env.draw_from_deck(controller) {
        env.move_card_between_zones(controller, card, Zone::Deck, Zone::Stock, None, None);
    }
}
