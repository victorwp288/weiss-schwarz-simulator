use super::prelude::*;

pub(super) fn move_trigger_card_to_hand(env: &mut GameEnv, controller: u8, source_id: CardId) {
    let _ = env.move_trigger_card_from_stock_to_hand(controller, source_id);
}

pub(super) fn move_trigger_card_to_stock(env: &mut GameEnv, controller: u8) {
    let p = controller as usize;
    let Some(instance_id) = env
        .state
        .turn
        .attack
        .as_ref()
        .and_then(|ctx| ctx.trigger_instance_id)
    else {
        return;
    };
    if let Some(pos) = env.state.players[p]
        .resolution
        .iter()
        .position(|card| card.instance_id == instance_id)
    {
        let card = env.state.players[p].resolution.remove(pos);
        env.move_card_between_zones(controller, card, Zone::Resolution, Zone::Stock, None, None);
    }
}

pub(super) fn standby(env: &mut GameEnv, controller: u8, payload: &EffectPayload, target_slot: u8) {
    let Some(target) = payload.targets.first() else {
        return;
    };
    if target.zone != TargetZone::WaitingRoom {
        return;
    }
    let option = ChoiceOptionRef {
        card_id: target.card_id,
        instance_id: target.instance_id,
        zone: ChoiceZone::WaitingRoom,
        index: Some(target.index as u16),
        target_slot: Some(target_slot),
    };
    env.move_waiting_room_to_stage_standby(controller, option);
}

pub(super) fn treasure_stock(env: &mut GameEnv, controller: u8, take_stock: bool) {
    if take_stock {
        if let Some(card) = env.draw_from_deck(controller) {
            env.move_card_between_zones(controller, card, Zone::Deck, Zone::Stock, None, None);
        }
    }
}

pub(super) fn set_trigger_check_count(env: &mut GameEnv, count: u8) {
    if let Some(ctx) = &mut env.state.turn.attack {
        let desired = count.max(1);
        let floor = ctx.trigger_checks_resolved.max(1);
        ctx.trigger_checks_total = ctx.trigger_checks_total.max(desired).max(floor);
    }
}

pub(super) fn trigger_icon(
    env: &mut GameEnv,
    controller: u8,
    source_id: CardId,
    payload: &EffectPayload,
    icon: TriggerIcon,
) {
    let trigger_id = env.state.turn.next_trigger_id;
    env.state.turn.next_trigger_id = env.state.turn.next_trigger_id.wrapping_add(1);
    let trigger = PendingTrigger {
        id: trigger_id,
        group_id: 0,
        player: controller,
        source_card: source_id,
        effect: match icon {
            TriggerIcon::Soul => TriggerEffect::Soul,
            TriggerIcon::Draw => TriggerEffect::Draw,
            TriggerIcon::Shot => TriggerEffect::Shot,
            TriggerIcon::Bounce => TriggerEffect::Bounce,
            TriggerIcon::Choice => TriggerEffect::Choice,
            TriggerIcon::Pool => TriggerEffect::Pool,
            TriggerIcon::Treasure => TriggerEffect::Treasure,
            TriggerIcon::Gate => TriggerEffect::Gate,
            TriggerIcon::Standby => TriggerEffect::Standby,
        },
        effect_id: Some(payload.spec.id),
    };
    let trigger_effect = trigger.effect;
    let resolved = match icon {
        TriggerIcon::Draw => env.resolve_trigger_draw(trigger),
        TriggerIcon::Choice => env.resolve_trigger_choice(trigger),
        TriggerIcon::Pool
        | TriggerIcon::Soul
        | TriggerIcon::Shot
        | TriggerIcon::Gate
        | TriggerIcon::Bounce => {
            let _ = env.resolve_trigger(trigger);
            false
        }
        TriggerIcon::Treasure => env.resolve_trigger_treasure(trigger),
        TriggerIcon::Standby => env.resolve_trigger_standby(trigger),
    };
    if !resolved && env.state.turn.choice.is_none() {
        env.log_event(Event::TriggerResolved {
            trigger_id,
            player: controller,
            effect: trigger_effect,
        });
    }
}
