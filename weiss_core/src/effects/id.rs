use serde::{Deserialize, Serialize};

use crate::db::CardId;
use crate::state::TargetSpec;

/// Source category for an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectSourceKind {
    /// Trigger resolution.
    Trigger,
    /// Auto ability.
    Auto,
    /// Activated ability.
    Activated,
    /// Continuous modifier.
    Continuous,
    /// Event card play.
    EventPlay,
    /// Counter timing.
    Counter,
    /// Replacement effect.
    Replacement,
    /// System-generated effect.
    System,
}

/// Stable identifier for an effect instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EffectId {
    /// Effect source category.
    pub source_kind: EffectSourceKind,
    /// Source card id (0 means none; see EffectSourceKind).
    pub source_card: CardId,
    /// Ability index on the source card.
    pub ability_index: u8,
    /// Effect index within the ability.
    pub effect_index: u8,
}

impl EffectId {
    /// Build an effect id from its components.
    pub fn new(
        source_kind: EffectSourceKind,
        source_card: CardId,
        ability_index: u8,
        effect_index: u8,
    ) -> Self {
        Self {
            source_kind,
            source_card,
            ability_index,
            effect_index,
        }
    }
}

/// Fully specified effect with targeting metadata.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct EffectSpec {
    /// Stable effect id.
    pub id: EffectId,
    /// Effect kind.
    pub kind: super::EffectKind,
    /// Optional target specification.
    pub target: Option<TargetSpec>,
    /// Whether this effect is optional.
    pub optional: bool,
}
