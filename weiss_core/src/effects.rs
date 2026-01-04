use serde::{Deserialize, Serialize};

use crate::db::{CardId, TriggerIcon};
use crate::events::RevealAudience;
use crate::state::{
    DamageType, ModifierDuration, ModifierKind, TargetSide, TargetSpec, TargetZone,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectSourceKind {
    Trigger,
    Auto,
    Activated,
    Continuous,
    EventPlay,
    Counter,
    Replacement,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EffectId {
    pub source_kind: EffectSourceKind,
    pub source_card: CardId,
    pub ability_index: u8,
    pub effect_index: u8,
}

impl EffectId {
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

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct EffectSpec {
    pub id: EffectId,
    pub kind: EffectKind,
    pub target: Option<TargetSpec>,
    pub optional: bool,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum EffectKind {
    Draw {
        count: u8,
    },
    Damage {
        amount: i32,
        cancelable: bool,
        damage_type: DamageType,
    },
    AddModifier {
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    },
    MoveToHand,
    MoveToWaitingRoom,
    MoveToStock,
    MoveToClock,
    Heal,
    RestTarget,
    StandTarget,
    StockCharge {
        count: u8,
    },
    MillTop {
        target: TargetSide,
        count: u8,
    },
    MoveStageSlot {
        slot: u8,
    },
    SwapStageSlots,
    RandomDiscardFromHand {
        target: TargetSide,
        count: u8,
    },
    RandomMill {
        target: TargetSide,
        count: u8,
    },
    RevealZoneTop {
        target: TargetSide,
        zone: TargetZone,
        count: u8,
        audience: RevealAudience,
    },
    MoveTriggerCardToHand,
    ChangeController {
        new_controller: TargetSide,
    },
    Standby {
        target_slot: u8,
    },
    TreasureStock {
        take_stock: bool,
    },
    ModifyPendingAttackDamage {
        delta: i32,
    },
    TriggerIcon {
        icon: TriggerIcon,
    },
    RevealDeckTop {
        count: u8,
        audience: RevealAudience,
    },
    CounterBackup {
        power: i32,
    },
    CounterDamageReduce {
        amount: u8,
    },
    CounterDamageCancel,
}

impl EffectKind {
    pub fn expects_target(&self) -> bool {
        matches!(
            self,
            EffectKind::AddModifier { .. }
                | EffectKind::MoveToHand
                | EffectKind::MoveToWaitingRoom
                | EffectKind::MoveToStock
                | EffectKind::MoveToClock
                | EffectKind::Heal
                | EffectKind::RestTarget
                | EffectKind::StandTarget
                | EffectKind::MoveStageSlot { .. }
                | EffectKind::SwapStageSlots
                | EffectKind::ChangeController { .. }
                | EffectKind::Standby { .. }
        )
    }

    pub fn requires_target_zone(&self, zone: TargetZone) -> bool {
        match self {
            EffectKind::MoveToHand => {
                matches!(
                    zone,
                    TargetZone::Stage | TargetZone::WaitingRoom | TargetZone::DeckTop
                )
            }
            EffectKind::MoveToWaitingRoom => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::Clock
                    | TargetZone::Level
                    | TargetZone::Stock
                    | TargetZone::Memory
                    | TargetZone::Climax
                    | TargetZone::Resolution
                    | TargetZone::WaitingRoom
            ),
            EffectKind::MoveToStock => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::Clock
                    | TargetZone::Level
                    | TargetZone::WaitingRoom
                    | TargetZone::Memory
                    | TargetZone::Climax
                    | TargetZone::Resolution
                    | TargetZone::Stock
            ),
            EffectKind::MoveToClock => matches!(
                zone,
                TargetZone::Stage
                    | TargetZone::Hand
                    | TargetZone::DeckTop
                    | TargetZone::WaitingRoom
                    | TargetZone::Resolution
                    | TargetZone::Clock
            ),
            EffectKind::Heal => matches!(zone, TargetZone::Clock),
            EffectKind::ChangeController { .. } => matches!(zone, TargetZone::Stage),
            EffectKind::AddModifier { .. } => matches!(zone, TargetZone::Stage),
            EffectKind::RestTarget
            | EffectKind::StandTarget
            | EffectKind::MoveStageSlot { .. }
            | EffectKind::SwapStageSlots => matches!(zone, TargetZone::Stage),
            EffectKind::Standby { .. } => matches!(zone, TargetZone::WaitingRoom),
            EffectKind::RandomDiscardFromHand { .. } => matches!(zone, TargetZone::Hand),
            EffectKind::RandomMill { .. } => matches!(zone, TargetZone::DeckTop),
            EffectKind::RevealZoneTop {
                zone: reveal_zone, ..
            } => zone == *reveal_zone,
            _ => true,
        }
    }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct EffectPayload {
    pub spec: EffectSpec,
    pub targets: Vec<crate::state::TargetRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplacementHook {
    Damage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplacementKind {
    CancelDamage,
    RedirectDamage { new_target: TargetSide },
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ReplacementSpec {
    pub id: EffectId,
    pub source: CardId,
    pub hook: ReplacementHook,
    pub kind: ReplacementKind,
    pub priority: i16,
    pub insertion: u32,
}
