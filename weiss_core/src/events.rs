use serde::{Deserialize, Serialize};
use crate::db::CardId;
use crate::state::{AttackType, ChoiceOptionRef, ChoiceReason, DamageModifierKind, DamageType, ModifierDuration, ModifierKind, TriggerEffect};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RevealReason {
    TriggerCheck,
    DamageCheck,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RevealAudience {
    Public,
    OwnerOnly,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TriggerCancelReason {
    InvalidSource,
    Suppressed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ChoiceSkipReason {
    NoCandidates,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Zone {
    Deck,
    Hand,
    WaitingRoom,
    Clock,
    Level,
    Stock,
    Memory,
    Climax,
    Stage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceOptionSummary {
    pub option_id: u64,
    pub label: String,
    pub reference: ChoiceOptionRef,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Draw { player: u8, card: CardId },
    Damage { player: u8, card: CardId },
    DamageCancel { player: u8 },
    DamageIntent { event_id: u32, source_player: u8, source_slot: Option<u8>, target: u8, amount: i32, damage_type: DamageType, cancelable: bool },
    DamageModifierApplied { event_id: u32, modifier: DamageModifierKind, before_amount: i32, after_amount: i32, before_cancelable: bool, after_cancelable: bool, before_canceled: bool, after_canceled: bool },
    DamageModified { event_id: u32, target: u8, original: i32, modified: i32, canceled: bool, damage_type: DamageType },
    DamageCommitted { event_id: u32, target: u8, card: CardId, damage_type: DamageType },
    ReversalCommitted { player: u8, slot: u8, cause_damage_event: Option<u32> },
    Reveal { player: u8, card: CardId, reason: RevealReason, audience: RevealAudience },
    TriggerQueued { trigger_id: u32, group_id: u32, player: u8, source: CardId, effect: TriggerEffect },
    TriggerResolved { trigger_id: u32, player: u8, effect: TriggerEffect },
    TriggerCanceled { trigger_id: u32, player: u8, reason: TriggerCancelReason },
    ChoicePresented { choice_id: u32, player: u8, reason: ChoiceReason, options: Vec<ChoiceOptionSummary>, total_candidates: u16 },
    ChoiceMade { choice_id: u32, player: u8, option: ChoiceOptionRef },
    ChoiceAutopicked { choice_id: u32, player: u8, option: ChoiceOptionRef },
    ChoiceSkipped { choice_id: u32, player: u8, reason: ChoiceReason, skip_reason: ChoiceSkipReason },
    ZoneMove { player: u8, card: CardId, from: Zone, to: Zone, from_slot: Option<u8>, to_slot: Option<u8> },
    ModifierAdded { id: u32, source: CardId, target_player: u8, target_slot: u8, target_card: CardId, kind: ModifierKind, magnitude: i32, duration: ModifierDuration },
    ModifierRemoved { id: u32, reason: ModifierRemoveReason },
    Play { player: u8, card: CardId, slot: u8 },
    PlayEvent { player: u8, card: CardId },
    PlayClimax { player: u8, card: CardId },
    Trigger { player: u8, icon: crate::db::TriggerIcon },
    Attack { player: u8, slot: u8 },
    AttackType { player: u8, attacker_slot: u8, attack_type: AttackType },
    Counter { player: u8, card: CardId, power: i32 },
    Clock { player: u8, card: Option<CardId> },
    Refresh { player: u8 },
    RefreshPenalty { player: u8, card: CardId },
    LevelUpChoice { player: u8, card: CardId },
    Encore { player: u8, slot: u8, kept: bool },
    Stand { player: u8 },
    EndTurn { player: u8 },
    Terminal { winner: Option<u8> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ModifierRemoveReason {
    EndOfTurn,
    TargetLeftStage,
}
