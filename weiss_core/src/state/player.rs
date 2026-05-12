use super::{CardInstance, StageSlot};

/// Full per-player state.
#[derive(Clone, Debug, Hash)]
pub struct PlayerState {
    /// Deck (top at end of vector).
    pub deck: Vec<CardInstance>,
    /// Hand.
    pub hand: Vec<CardInstance>,
    /// Waiting room.
    pub waiting_room: Vec<CardInstance>,
    /// Clock.
    pub clock: Vec<CardInstance>,
    /// Level zone.
    pub level: Vec<CardInstance>,
    /// Stock (top at end of vector).
    pub stock: Vec<CardInstance>,
    /// Memory.
    pub memory: Vec<CardInstance>,
    /// Climax zone.
    pub climax: Vec<CardInstance>,
    /// Resolution zone (temporary).
    pub resolution: Vec<CardInstance>,
    /// Stage slots (5 total).
    pub stage: [StageSlot; 5],
}

impl PlayerState {
    /// Create a new player state with an initial deck.
    pub fn new(deck: Vec<CardInstance>) -> Self {
        Self {
            deck,
            hand: Vec::new(),
            waiting_room: Vec::new(),
            clock: Vec::new(),
            level: Vec::new(),
            stock: Vec::new(),
            memory: Vec::new(),
            climax: Vec::new(),
            resolution: Vec::new(),
            stage: [
                StageSlot::empty(),
                StageSlot::empty(),
                StageSlot::empty(),
                StageSlot::empty(),
                StageSlot::empty(),
            ],
        }
    }
}
