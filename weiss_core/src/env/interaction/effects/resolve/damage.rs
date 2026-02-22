use super::prelude::*;

pub(super) fn damage(
    env: &mut GameEnv,
    controller: u8,
    source_id: CardId,
    payload: &EffectPayload,
    amount: i32,
    cancelable: bool,
) {
    let target_player = if let Some(target) = payload.targets.first() {
        target.player
    } else if let Some(spec) = payload.spec.target.as_ref() {
        match spec.side {
            TargetSide::SelfSide => controller,
            TargetSide::Opponent => 1 - controller,
        }
    } else if payload.spec.id.source_kind == EffectSourceKind::System {
        controller
    } else {
        1 - controller
    };
    let (amount, target_player) =
        env.apply_replacements_to_damage(controller, target_player, amount);
    let refresh_penalty = payload.spec.id.source_kind == EffectSourceKind::System
        && payload.spec.id.source_card == 0
        && payload.spec.id.ability_index == 0
        && payload.spec.id.effect_index == 0
        && !cancelable;
    if amount > 0 {
        let _ = env.resolve_effect_damage(
            controller,
            target_player,
            amount,
            cancelable,
            refresh_penalty,
            Some(source_id),
            payload.source_ref,
        );
    }
}

pub(super) fn modify_pending_attack_damage(env: &mut GameEnv, delta: i32) {
    if let Some(ctx) = &mut env.state.turn.attack {
        ctx.damage = ctx.damage.saturating_add(delta);
    }
}

pub(super) fn enable_shot_damage(env: &mut GameEnv, amount: u8) {
    if let Some(ctx) = &mut env.state.turn.attack {
        ctx.pending_shot_damage = ctx.pending_shot_damage.saturating_add(amount);
    }
}

pub(super) fn counter_backup(env: &mut GameEnv, controller: u8, source_id: CardId, power: i32) {
    let mut dirty_slot = None;
    if let Some(ctx) = &mut env.state.turn.attack {
        if let Some(def_slot) = ctx.defender_slot {
            let slot_state = &mut env.state.players[controller as usize].stage[def_slot as usize];
            slot_state.power_mod_battle += power;
            ctx.counter_power += power;
            dirty_slot = Some(def_slot);
        }
    }
    if let Some(def_slot) = dirty_slot {
        env.mark_slot_power_dirty(controller, def_slot);
    }
    // Log the counter play attempt even if it doesn't apply power (e.g., no active defender slot),
    // so replays/debug streams still reflect the counter action.
    env.log_event(Event::Counter {
        player: controller,
        card: source_id,
        power,
    });
}

pub(super) fn counter_damage_reduce(env: &mut GameEnv, source_id: CardId, amount: u8) {
    if let Some(ctx) = &mut env.state.turn.attack {
        if amount > 0 {
            GameEnv::push_attack_damage_modifier(
                ctx,
                DamageModifierKind::AddAmount {
                    delta: -(amount as i32),
                },
                source_id,
            );
        }
    }
}

pub(super) fn counter_damage_cancel(env: &mut GameEnv, source_id: CardId) {
    if let Some(ctx) = &mut env.state.turn.attack {
        GameEnv::push_attack_damage_modifier(ctx, DamageModifierKind::CancelNext, source_id);
    }
}
