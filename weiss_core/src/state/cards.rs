use serde::{Deserialize, Serialize};

use crate::db::CardId;

/// Unique identifier for a card instance within a game.
pub type CardInstanceId = u32;

/// Concrete card instance with ownership and controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardInstance {
    /// Static card identifier in the card database.
    pub id: CardId,
    /// Stable per-game identifier for this physical card instance.
    pub instance_id: CardInstanceId,
    /// Owning seat (0 or 1).
    pub owner: u8,
    /// Current controlling seat (0 or 1).
    pub controller: u8,
}

impl CardInstance {
    /// Create a new card instance owned by `owner`.
    pub fn new(id: CardId, owner: u8, instance_id: CardInstanceId) -> Self {
        Self {
            id,
            instance_id,
            owner,
            controller: owner,
        }
    }
}
