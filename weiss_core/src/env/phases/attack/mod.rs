use super::super::{DamageIntentLocal, DamageResolveResult, GameEnv};
use crate::db::*;
use crate::effects::EffectKind;
use crate::encode::MAX_STAGE;
use crate::events::*;
use crate::modifier_queries::collect_attack_slot_state;
use crate::state::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttackAutoResolvePhase {
    TriggerStep,
    DamageStep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttackPipelineFlow {
    Continue,
    Break,
    Return,
}

mod autos;
mod battle;
mod cleanup;
mod damage;
mod derived;
mod pipeline;
mod trigger_step;
