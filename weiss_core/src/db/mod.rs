//! Card database types, loading, and validation.
//!
//! Related docs:
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/README.md>
//! - <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/rules_coverage.md>

mod ability;
mod card;
mod serialization;
mod store;
mod types;

pub(crate) use ability::compile_effects_from_def;
pub use ability::{
    AbilityCost, AbilityDef, AbilityDefClimaxAreaCondition, AbilityDefConditions, AbilityKind,
    AbilitySpec, AbilityTemplate, AbilityTemplateTag, AbilityTiming,
};
pub use card::CardStatic;
pub use serialization::WSDB_SCHEMA_VERSION;
pub use store::CardDb;
pub use types::{
    BattleOpponentMoveDestination, BattleOpponentMovePreludeAction, BrainstormMode, CardColor,
    CardId, CardType, ConditionTurn, CountCmp, CountZone, EffectTemplate, GrantDuration,
    TargetTemplate, TriggerIcon, ZoneCountCondition,
};
