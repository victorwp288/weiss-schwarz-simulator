use serde::{Deserialize, Serialize};

use crate::db::CardId;
use crate::effects::EffectPayload;

use super::CardInstanceId;

/// Zones that can be targeted by effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetZone {
    /// Stage (front/back row slots).
    Stage,
    /// Hand.
    Hand,
    /// Top of deck.
    DeckTop,
    /// Clock.
    Clock,
    /// Level zone.
    Level,
    /// Stock.
    Stock,
    /// Memory.
    Memory,
    /// Waiting room.
    WaitingRoom,
    /// Climax zone.
    Climax,
    /// Resolution zone (temporary).
    Resolution,
}

/// Side selection for targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSide {
    /// Current player / controller side.
    SelfSide,
    /// Opponent side.
    Opponent,
}

/// Slot filter for targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSlotFilter {
    /// Any slot (no restriction).
    Any,
    /// Front-row slots only.
    FrontRow,
    /// Back-row slots only.
    BackRow,
    /// A specific slot index.
    SpecificSlot(
        /// Slot index in `[0, 4]`.
        u8,
    ),
}

/// Targeting specification for effects.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Target zone to search/select from.
    pub zone: TargetZone,
    /// Which side to target.
    pub side: TargetSide,
    /// Optional slot filter (primarily for stage targeting).
    pub slot_filter: TargetSlotFilter,
    /// Optional card type restriction.
    pub card_type: Option<crate::db::CardType>,
    /// Optional trait restriction (packed trait id).
    #[serde(default)]
    pub card_trait: Option<u16>,
    /// Optional inclusive maximum level restriction.
    pub level_max: Option<u8>,
    /// Optional inclusive maximum cost restriction.
    #[serde(default)]
    pub cost_max: Option<u8>,
    /// Optional card id whitelist restriction.
    #[serde(default)]
    pub card_ids: Vec<CardId>,
    /// Number of cards/targets to select.
    pub count: u8,
    /// Optional hard limit for search-like effects.
    #[serde(default)]
    pub limit: Option<u8>,
    /// If true, only the source card is eligible.
    #[serde(default)]
    pub source_only: bool,
    /// If true, reveal selected cards to the controller.
    #[serde(default)]
    pub reveal_to_controller: bool,
}

/// Concrete target reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetRef {
    /// Owning/located player seat (0 or 1).
    pub player: u8,
    /// Zone containing the card.
    pub zone: TargetZone,
    /// Index within the zone (slot or list position).
    pub index: u8,
    /// Static card id.
    pub card_id: CardId,
    /// Stable per-game card instance id.
    pub instance_id: CardInstanceId,
}

/// Pending target effect awaiting resolution.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum PendingTargetEffect {
    /// Resolve an effect payload once targeting is complete.
    EffectPending {
        /// Effect instance id to associate with the payload.
        instance_id: u32,
        /// Effect payload to execute.
        payload: EffectPayload,
    },
}

/// State for a target-selection prompt.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TargetSelectionState {
    /// Controlling player making selections.
    pub controller: u8,
    /// Source card id producing the prompt.
    pub source_id: CardId,
    /// Targeting specification to satisfy.
    pub spec: TargetSpec,
    /// Remaining selections required.
    pub remaining: u8,
    /// Selected targets so far.
    pub selected: Vec<TargetRef>,
    /// Optional precomputed candidate list (for pagination/debugging).
    #[serde(default)]
    pub candidates: Vec<TargetRef>,
    /// Effect to apply once selection completes.
    pub effect: PendingTargetEffect,
    /// Whether the controller may skip instead of selecting.
    pub allow_skip: bool,
}
