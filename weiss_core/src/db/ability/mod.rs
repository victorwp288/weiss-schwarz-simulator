use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::events::RevealAudience;
use crate::state::{TargetSide, TargetSpec, TargetZone};

use super::types::{
    BrainstormMode, CardId, CardType, ConditionTurn, CountCmp, CountZone, EffectTemplate,
    RuleOverrideKind, TargetTemplate, TerminalOutcomeSpec, TriggerIcon, ZoneCountCondition,
};

mod compile;
mod keys;
mod models;

pub(crate) use compile::{compile_effects_from_def, compile_effects_from_template};
pub(crate) use keys::ability_sort_key;
pub use models::{
    AbilityCost, AbilityCostStep, AbilityDef, AbilityDefClimaxAreaCondition, AbilityDefConditions,
    AbilityKind, AbilitySpec, AbilityTemplate, AbilityTemplateTag, AbilityTiming,
};
