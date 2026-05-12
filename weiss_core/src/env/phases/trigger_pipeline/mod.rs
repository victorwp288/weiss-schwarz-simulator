use super::super::{
    EngineErrorCode, FaultSource, GameEnv, TriggerCompileContext, TRIGGER_EFFECT_BOUNCE,
    TRIGGER_EFFECT_DRAW, TRIGGER_EFFECT_GATE, TRIGGER_EFFECT_POOL_MOVE, TRIGGER_EFFECT_POOL_STOCK,
    TRIGGER_EFFECT_SHOT, TRIGGER_EFFECT_SOUL, TRIGGER_EFFECT_STANDBY, TRIGGER_EFFECT_TREASURE_MOVE,
    TRIGGER_EFFECT_TREASURE_STOCK,
};
use crate::db::*;
use crate::effects::*;
use crate::encode::MAX_STAGE;
use crate::events::*;
use crate::legal::*;
use crate::state::*;
use anyhow::Result;

struct TriggerSeed {
    player: u8,
    source: CardId,
    effect: TriggerEffect,
}

fn trigger_effect_sort_key(effect: TriggerEffect) -> (u8, u64) {
    match effect {
        TriggerEffect::Soul => (0, 0),
        TriggerEffect::Draw => (1, 0),
        TriggerEffect::Shot => (2, 0),
        TriggerEffect::Bounce => (3, 0),
        TriggerEffect::Choice => (4, 0),
        TriggerEffect::Pool => (5, 0),
        TriggerEffect::Treasure => (6, 0),
        TriggerEffect::Gate => (7, 0),
        TriggerEffect::Standby => (8, 0),
        TriggerEffect::AutoAbility { ability_index } => (9, ability_index as u64),
        TriggerEffect::GrantedAutoAbility { grant_id } => (10, grant_id),
    }
}

fn trigger_seed_sort_key(seed: &TriggerSeed) -> (u8, u32, u8, u64) {
    let (kind, sub) = trigger_effect_sort_key(seed.effect);
    (seed.player, seed.source, kind, sub)
}

fn pending_trigger_sort_key(trigger: &PendingTrigger) -> (u32, u8, u32, u8, u64, u32) {
    let (kind, sub) = trigger_effect_sort_key(trigger.effect);
    (
        trigger.group_id,
        trigger.player,
        trigger.source_card,
        kind,
        sub,
        trigger.id,
    )
}

mod auto_cost;
mod compile;
mod queue;
mod resolve;
