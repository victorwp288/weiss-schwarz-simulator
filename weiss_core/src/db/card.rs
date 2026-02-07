use serde::{Deserialize, Serialize};

use super::ability::{AbilityDef, AbilityTemplate};
use super::types::{CardColor, CardId, CardType, TriggerIcon};

/// Static card definition loaded from the card database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardStatic {
    /// Unique card id (non-zero).
    pub id: CardId,
    #[serde(default)]
    /// Optional card set identifier.
    pub card_set: Option<String>,
    /// Card type (character/event/climax).
    pub card_type: CardType,
    /// Card color.
    pub color: CardColor,
    /// Level requirement.
    pub level: u8,
    /// Stock cost.
    pub cost: u8,
    /// Power value.
    pub power: i32,
    /// Soul value.
    pub soul: u8,
    /// Trigger icons on the card.
    pub triggers: Vec<TriggerIcon>,
    /// Trait identifiers (app-specific).
    pub traits: Vec<u16>,
    /// Template-based abilities.
    pub abilities: Vec<AbilityTemplate>,
    #[serde(default)]
    /// Explicit ability definitions (advanced/override).
    pub ability_defs: Vec<AbilityDef>,
    #[serde(default)]
    /// Whether this card can be played as a counter.
    pub counter_timing: bool,
    #[serde(default)]
    /// Optional raw text for reference/debugging.
    pub raw_text: Option<String>,
}
