use super::prelude::*;

use crate::db::{AbilityDef, ConditionTurn, GrantDuration, ZoneCountCondition};

pub(super) fn add_modifier(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    kind: ModifierKind,
    magnitude: i32,
    duration: ModifierDuration,
) {
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let s = target.index as usize;
        if s >= env.state.players[p].stage.len() {
            continue;
        }
        if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(target.instance_id) {
            continue;
        }
        let _ = env.add_modifier(
            source_id,
            target.player,
            target.index,
            kind,
            magnitude,
            duration,
        );
    }
}

pub(super) fn grant_ability_def(
    env: &mut GameEnv,
    controller: u8,
    payload: &EffectPayload,
    ability: &AbilityDef,
    duration: GrantDuration,
) {
    for target in &payload.targets {
        env.add_granted_ability_to_target(controller, target, ability, duration);
    }
}

pub(super) fn add_power_if_target_level_at_least(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
    min_level: u8,
    duration: ModifierDuration,
) {
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let s = target.index as usize;
        if s >= env.state.players[p].stage.len() {
            continue;
        }
        if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(target.instance_id) {
            continue;
        }
        let level = env.compute_slot_level(p, s);
        if level < i32::from(min_level) {
            continue;
        }
        let _ = env.add_modifier(
            source_id,
            target.player,
            target.index,
            ModifierKind::Power,
            amount,
            duration,
        );
    }
}

pub(super) fn add_power_by_target_level(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    multiplier: i32,
    duration: ModifierDuration,
) {
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let s = target.index as usize;
        if s >= env.state.players[p].stage.len() {
            continue;
        }
        if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(target.instance_id) {
            continue;
        }
        let level = env.compute_slot_level(p, s);
        let magnitude = multiplier.saturating_mul(level);
        if magnitude == 0 {
            continue;
        }
        let _ = env.add_modifier(
            source_id,
            target.player,
            target.index,
            ModifierKind::Power,
            magnitude,
            duration,
        );
    }
}

pub(super) fn add_power_if_battle_opponent_level_at_least(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
    min_level: u8,
    duration: ModifierDuration,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let Some(opponent_ref) = env.battle_opponent_target_from_source(&source_ref) else {
        return;
    };
    let opponent_level =
        env.compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
    if opponent_level < i32::from(min_level) {
        return;
    }
    let p = source_ref.player as usize;
    let s = source_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(source_ref.instance_id) {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        source_ref.player,
        source_ref.index,
        ModifierKind::Power,
        amount,
        duration,
    );
}

pub(super) fn add_soul_if_battle_opponent_level_at_least(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
    min_level: u8,
    duration: ModifierDuration,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let Some(opponent_ref) = env.battle_opponent_target_from_source(&source_ref) else {
        return;
    };
    let opponent_level =
        env.compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
    if opponent_level < i32::from(min_level) {
        return;
    }
    let p = source_ref.player as usize;
    let s = source_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(source_ref.instance_id) {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        source_ref.player,
        source_ref.index,
        ModifierKind::Soul,
        amount,
        duration,
    );
}

pub(super) fn add_power_if_battle_opponent_level_exact(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
    level: u8,
    duration: ModifierDuration,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let Some(opponent_ref) = env.battle_opponent_target_from_source(&source_ref) else {
        return;
    };
    let opponent_level =
        env.compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
    if opponent_level != i32::from(level) {
        return;
    }
    let p = source_ref.player as usize;
    let s = source_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(source_ref.instance_id) {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        source_ref.player,
        source_ref.index,
        ModifierKind::Power,
        amount,
        duration,
    );
}

pub(super) fn add_power_if_other_attacker_matches(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
    duration: ModifierDuration,
    attacker_card_ids: &[CardId],
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    if source_ref.zone != TargetZone::Stage {
        return;
    }
    let Some(attack) = env.state.turn.attack.as_ref() else {
        return;
    };
    let active_player = env.state.turn.active_player as usize;
    if source_ref.player as usize != active_player {
        return;
    }
    let attacker_slot = attack.attacker_slot as usize;
    if attacker_slot >= env.state.players[active_player].stage.len() {
        return;
    }
    if source_ref.index as usize == attacker_slot {
        return;
    }
    let Some(attacker_card) = env.state.players[active_player].stage[attacker_slot].card else {
        return;
    };
    if !attacker_card_ids.is_empty() && !attacker_card_ids.contains(&attacker_card.id) {
        return;
    }
    let source_slot = source_ref.index as usize;
    if source_slot >= env.state.players[active_player].stage.len() {
        return;
    }
    if env.state.players[active_player].stage[source_slot]
        .card
        .map(|c| c.instance_id)
        != Some(source_ref.instance_id)
    {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        source_ref.player,
        source_ref.index,
        ModifierKind::Power,
        amount,
        duration,
    );
}

pub(super) fn add_soul_if_middle_center(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
) {
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
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(source_ref.instance_id) {
        return;
    }
    let middle_slot = if env.curriculum.reduced_stage_mode {
        0
    } else {
        1
    };
    if s != middle_slot {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        source_ref.player,
        source_ref.index,
        ModifierKind::Soul,
        amount,
        ModifierDuration::WhileOnStage,
    );
}

pub(super) fn facing_opponent_add_soul(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let Some(opponent_ref) = env.battle_opponent_target_from_source(&source_ref) else {
        return;
    };
    let p = opponent_ref.player as usize;
    let s = opponent_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(opponent_ref.instance_id) {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        opponent_ref.player,
        opponent_ref.index,
        ModifierKind::Soul,
        amount,
        ModifierDuration::WhileOnStage,
    );
}

pub(super) fn facing_opponent_add_modifier(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    kind: ModifierKind,
    magnitude: i32,
    duration: ModifierDuration,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let Some(opponent_ref) = env.battle_opponent_target_from_source(&source_ref) else {
        return;
    };
    let p = opponent_ref.player as usize;
    let s = opponent_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(opponent_ref.instance_id) {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        opponent_ref.player,
        opponent_ref.index,
        kind,
        magnitude,
        duration,
    );
}

pub(super) fn self_add_modifier_if_facing_opponent(
    env: &mut GameEnv,
    source_id: CardId,
    payload: &EffectPayload,
    kind: ModifierKind,
    magnitude: i32,
    duration: ModifierDuration,
    max_level: Option<u8>,
    max_cost: Option<u8>,
    level_gt_source_level: bool,
) {
    let Some(source_ref) = payload.source_ref else {
        return;
    };
    let Some(opponent_ref) = env.battle_opponent_target_from_source(&source_ref) else {
        return;
    };
    let Some(opponent_card) = env.db.get(opponent_ref.card_id) else {
        return;
    };
    let opponent_level =
        env.compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
    if let Some(level_cap) = max_level {
        if opponent_level > i32::from(level_cap) {
            return;
        }
    }
    if let Some(cost_cap) = max_cost {
        if opponent_card.cost > cost_cap {
            return;
        }
    }
    if level_gt_source_level {
        let source_level =
            env.compute_slot_level(source_ref.player as usize, source_ref.index as usize);
        if opponent_level <= source_level {
            return;
        }
    }
    let p = source_ref.player as usize;
    let s = source_ref.index as usize;
    if s >= env.state.players[p].stage.len() {
        return;
    }
    if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(source_ref.instance_id) {
        return;
    }
    let _ = env.add_modifier(
        source_id,
        source_ref.player,
        source_ref.index,
        kind,
        magnitude,
        duration,
    );
}

pub(super) fn conditional_add_modifier(
    env: &mut GameEnv,
    controller: u8,
    source_id: CardId,
    payload: &EffectPayload,
    kind: ModifierKind,
    magnitude: i32,
    duration: ModifierDuration,
    turn: Option<ConditionTurn>,
    zone_count: Option<&ZoneCountCondition>,
    require_source_marker: bool,
    per_source_marker: bool,
    per_zone_count: bool,
    exclude_source: bool,
) {
    if !env.conditional_modifier_context_satisfied(
        controller,
        payload.source_ref.as_ref(),
        turn,
        zone_count,
        require_source_marker,
    ) {
        return;
    }
    let mut resolved_magnitude = magnitude;
    if per_source_marker {
        let marker_count = env.source_marker_count(payload.source_ref.as_ref()) as i32;
        resolved_magnitude = resolved_magnitude.saturating_mul(marker_count);
    }
    if per_zone_count {
        let Some(zone_condition) = zone_count else {
            return;
        };
        let count = env.zone_count_value(controller, zone_condition) as i32;
        resolved_magnitude = resolved_magnitude.saturating_mul(count);
    }
    if resolved_magnitude == 0 {
        return;
    }
    for target in &payload.targets {
        if target.zone != TargetZone::Stage {
            continue;
        }
        let p = target.player as usize;
        let s = target.index as usize;
        if s >= env.state.players[p].stage.len() {
            continue;
        }
        if env.state.players[p].stage[s].card.map(|c| c.instance_id) != Some(target.instance_id) {
            continue;
        }
        if exclude_source
            && payload
                .source_ref
                .as_ref()
                .map(|source| {
                    source.player == target.player
                        && source.zone == target.zone
                        && source.index == target.index
                        && source.instance_id == target.instance_id
                })
                .unwrap_or(false)
        {
            continue;
        }
        let _ = env.add_modifier(
            source_id,
            target.player,
            target.index,
            kind,
            resolved_magnitude,
            duration,
        );
    }
}

pub(super) fn cannot_use_auto_encore_for_player(
    env: &mut GameEnv,
    controller: u8,
    target: TargetSide,
) {
    let target_player = match target {
        TargetSide::SelfSide => controller,
        TargetSide::Opponent => 1 - controller,
    };
    env.state.turn.cannot_use_auto_encore[target_player as usize] = true;
}
