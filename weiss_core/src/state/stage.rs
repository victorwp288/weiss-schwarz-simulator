use serde::{Deserialize, Serialize};

use super::CardInstance;

/// Stage slot status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageStatus {
    /// Standing (upright) character.
    Stand,
    /// Rested (tapped) character.
    Rest,
    /// Reversed (defeated) character.
    Reverse,
}

/// Stage slot containing a character or empty.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct StageSlot {
    /// Occupying card instance, if any.
    pub card: Option<CardInstance>,
    /// Marker cards attached to the occupying character.
    #[serde(default)]
    pub markers: Vec<CardInstance>,
    /// Current stand/rest/reverse status.
    pub status: StageStatus,
    /// Whether the current card was played from hand this turn.
    #[serde(default)]
    pub played_from_hand_this_turn: bool,
    /// Battle-only power modifier.
    pub power_mod_battle: i32,
    /// Turn-long power modifier.
    pub power_mod_turn: i32,
    /// Whether this slot has attacked this turn.
    pub has_attacked: bool,
    /// Whether this slot is prevented from attacking.
    pub cannot_attack: bool,
    /// Additional stock cost required to declare an attack.
    pub attack_cost: u8,
}

impl StageSlot {
    /// Create an empty stage slot.
    pub fn empty() -> Self {
        Self {
            card: None,
            markers: Vec::new(),
            status: StageStatus::Stand,
            played_from_hand_this_turn: false,
            power_mod_battle: 0,
            power_mod_turn: 0,
            has_attacked: false,
            cannot_attack: false,
            attack_cost: 0,
        }
    }

    /// Whether the slot is empty.
    pub fn is_empty(&self) -> bool {
        self.card.is_none()
    }
}
