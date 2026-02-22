use super::prelude::*;

pub(super) fn heal(env: &mut GameEnv, payload: &EffectPayload) {
    for target in &payload.targets {
        if target.zone != TargetZone::Clock {
            continue;
        }
        let p = target.player as usize;
        let Some(pos) = env.state.players[p]
            .clock
            .iter()
            .position(|card| card.instance_id == target.instance_id)
        else {
            continue;
        };
        let card = env.state.players[p].clock.remove(pos);
        env.move_card_between_zones(
            target.player,
            card,
            Zone::Clock,
            Zone::WaitingRoom,
            Some(target.index),
            None,
        );
    }
}

pub(super) fn heal_if_source_played_from_hand_this_turn(
    env: &mut GameEnv,
    payload: &EffectPayload,
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
    let slot_state = &env.state.players[p].stage[s];
    if slot_state.card.map(|card| card.instance_id) != Some(source_ref.instance_id) {
        return;
    }
    if !slot_state.played_from_hand_this_turn {
        return;
    }
    let clock_len = env.state.players[p].clock.len();
    if clock_len == 0 {
        return;
    }
    let idx = clock_len - 1;
    let Ok(idx_u8) = u8::try_from(idx) else {
        return;
    };
    let card = env.state.players[p].clock.remove(idx);
    env.move_card_between_zones(
        source_ref.player,
        card,
        Zone::Clock,
        Zone::WaitingRoom,
        Some(idx_u8),
        None,
    );
}
