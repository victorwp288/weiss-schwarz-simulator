use serde::{Deserialize, Serialize};

use crate::db::CardId;
use crate::effects::{EffectId, EffectPayload};

use super::TimingWindow;

/// Item on the effect stack.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackItem {
    /// Unique stack item id.
    pub id: u32,
    /// Controlling player for resolution ordering.
    pub controller: u8,
    /// Source card id that created the stack item.
    pub source_id: CardId,
    /// Compiled effect id for this item.
    pub effect_id: EffectId,
    /// Payload to resolve.
    pub payload: EffectPayload,
}

/// Priority window state.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct PriorityState {
    /// Current priority holder seat.
    pub holder: u8,
    /// Consecutive pass count (for window termination).
    pub passes: u8,
    /// Active timing window.
    pub window: TimingWindow,
    /// Bitmask of already-used ACT abilities (by index).
    pub used_act_mask: u32,
}

/// Stack ordering state when multiple items are pending.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StackOrderState {
    /// Group identifier for the ordering prompt.
    pub group_id: u32,
    /// Controller making the ordering decision.
    pub controller: u8,
    /// Items to order.
    pub items: Vec<StackItem>,
}
