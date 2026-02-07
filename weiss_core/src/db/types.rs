use serde::{Deserialize, Serialize};

use crate::events::RevealAudience;
use crate::state::{TargetSide, TargetZone};

/// Stable numeric card identifier (non-zero).
pub type CardId = u32;

/// Card type classification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardType {
    /// Character card.
    Character,
    /// Event card.
    Event,
    /// Climax card.
    Climax,
}

/// Card color classification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardColor {
    /// Yellow color.
    Yellow,
    /// Green color.
    Green,
    /// Red color.
    Red,
    /// Blue color.
    Blue,
    /// Colorless/neutral.
    Colorless,
}

/// Trigger icon types.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TriggerIcon {
    /// +1 soul trigger.
    Soul,
    /// Shot trigger.
    Shot,
    /// Bounce trigger.
    Bounce,
    /// Draw trigger.
    Draw,
    /// Treasure trigger.
    Treasure,
    /// Gate trigger.
    Gate,
    /// Standby trigger.
    Standby,
}

/// Target selection template for effects and abilities.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetTemplate {
    /// Opponent front row.
    OppFrontRow,
    /// Opponent back row.
    OppBackRow,
    /// Any opponent stage slot.
    OppStage,
    /// Specific opponent stage slot.
    OppStageSlot { slot: u8 },
    /// Self front row.
    SelfFrontRow,
    /// Self back row.
    SelfBackRow,
    /// Any self stage slot.
    SelfStage,
    /// Specific self stage slot.
    SelfStageSlot { slot: u8 },
    /// The source card itself.
    This,
    /// Self waiting room.
    SelfWaitingRoom,
    /// Self hand.
    SelfHand,
    /// Top of self deck.
    SelfDeckTop,
    /// Self clock.
    SelfClock,
    /// Self level.
    SelfLevel,
    /// Self stock.
    SelfStock,
    /// Self memory.
    SelfMemory,
}

/// Effect template used by ability definitions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EffectTemplate {
    /// Draw cards.
    Draw { count: u8 },
    /// Deal damage (optionally cancelable).
    DealDamage { amount: u8, cancelable: bool },
    /// Add power for a duration.
    AddPower { amount: i32, duration_turn: bool },
    /// Move target to hand.
    MoveToHand,
    /// Move target to waiting room.
    MoveToWaitingRoom,
    /// Move target to stock.
    MoveToStock,
    /// Move target to clock.
    MoveToClock,
    /// Heal (move top clock to waiting room).
    Heal,
    /// Rest the target.
    RestTarget,
    /// Stand the target.
    StandTarget,
    /// Stock charge by count.
    StockCharge { count: u8 },
    /// Mill top cards from deck.
    MillTop { target: TargetSide, count: u8 },
    /// Move target to a specific stage slot.
    MoveStageSlot { slot: u8 },
    /// Swap two stage slots.
    SwapStageSlots,
    /// Random discard from hand.
    RandomDiscardFromHand { target: TargetSide, count: u8 },
    /// Random mill from deck.
    RandomMill { target: TargetSide, count: u8 },
    /// Reveal the top of a zone.
    RevealZoneTop {
        target: TargetSide,
        zone: TargetZone,
        count: u8,
        audience: RevealAudience,
    },
    /// Change controller of a card.
    ChangeController,
    /// Counter backup (power).
    CounterBackup { power: i32 },
    /// Counter damage reduction.
    CounterDamageReduce { amount: u8 },
    /// Counter damage cancel.
    CounterDamageCancel,
}
