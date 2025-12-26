use crate::db::CardId;
use crate::state::{
    AttackType, ChoiceOptionRef, ChoiceReason, DamageModifierKind, DamageType, ModifierDuration,
    ModifierKind, StackItem, TimingWindow, TriggerEffect,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RevealReason {
    TriggerCheck,
    DamageCheck,
    RefreshPenalty,
    Play,
    AbilityEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RevealAudience {
    Public,
    OwnerOnly,
    ControllerOnly,
    BothPlayers,
    ReplayOnly,
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
    Resolution,
    Stage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceOptionSnapshot {
    pub option_id: u64,
    pub reference: ChoiceOptionRef,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Draw {
        player: u8,
        card: CardId,
    },
    Damage {
        player: u8,
        card: CardId,
    },
    DamageCancel {
        player: u8,
    },
    DamageIntent {
        event_id: u32,
        source_player: u8,
        source_slot: Option<u8>,
        target: u8,
        amount: i32,
        damage_type: DamageType,
        cancelable: bool,
    },
    DamageModifierApplied {
        event_id: u32,
        modifier: DamageModifierKind,
        before_amount: i32,
        after_amount: i32,
        before_cancelable: bool,
        after_cancelable: bool,
        before_canceled: bool,
        after_canceled: bool,
    },
    DamageModified {
        event_id: u32,
        target: u8,
        original: i32,
        modified: i32,
        canceled: bool,
        damage_type: DamageType,
    },
    DamageCommitted {
        event_id: u32,
        target: u8,
        card: CardId,
        damage_type: DamageType,
    },
    ReversalCommitted {
        player: u8,
        slot: u8,
        cause_damage_event: Option<u32>,
    },
    Reveal {
        player: u8,
        card: CardId,
        reason: RevealReason,
        audience: RevealAudience,
    },
    TriggerQueued {
        trigger_id: u32,
        group_id: u32,
        player: u8,
        source: CardId,
        effect: TriggerEffect,
    },
    TriggerGrouped {
        group_id: u32,
        trigger_ids: Vec<u32>,
    },
    TriggerResolved {
        trigger_id: u32,
        player: u8,
        effect: TriggerEffect,
    },
    TriggerCanceled {
        trigger_id: u32,
        player: u8,
        reason: TriggerCancelReason,
    },
    TimingWindowEntered {
        window: TimingWindow,
        player: u8,
    },
    PriorityGranted {
        window: TimingWindow,
        player: u8,
    },
    PriorityPassed {
        player: u8,
        window: TimingWindow,
        pass_count: u8,
    },
    StackGroupPresented {
        group_id: u32,
        controller: u8,
        items: Vec<StackItem>,
    },
    StackOrderChosen {
        group_id: u32,
        controller: u8,
        stack_id: u32,
    },
    StackPushed {
        item: StackItem,
    },
    StackResolved {
        item: StackItem,
    },
    AutoResolveCapExceeded {
        cap: u32,
        stack_len: u32,
        window: Option<TimingWindow>,
    },
    WindowAdvanced {
        from: TimingWindow,
        to: Option<TimingWindow>,
    },
    ChoicePresented {
        choice_id: u32,
        player: u8,
        reason: ChoiceReason,
        options: Vec<ChoiceOptionSnapshot>,
        total_candidates: u16,
        page_start: u16,
    },
    ChoicePageChanged {
        choice_id: u32,
        player: u8,
        from_start: u16,
        to_start: u16,
    },
    ChoiceMade {
        choice_id: u32,
        player: u8,
        reason: ChoiceReason,
        option: ChoiceOptionRef,
    },
    ChoiceAutopicked {
        choice_id: u32,
        player: u8,
        reason: ChoiceReason,
        option: ChoiceOptionRef,
    },
    ChoiceSkipped {
        choice_id: u32,
        player: u8,
        reason: ChoiceReason,
        skip_reason: ChoiceSkipReason,
    },
    ZoneMove {
        player: u8,
        card: CardId,
        from: Zone,
        to: Zone,
        from_slot: Option<u8>,
        to_slot: Option<u8>,
    },
    ControlChanged {
        card: CardId,
        owner: u8,
        from_controller: u8,
        to_controller: u8,
        from_slot: u8,
        to_slot: u8,
    },
    ModifierAdded {
        id: u32,
        source: CardId,
        target_player: u8,
        target_slot: u8,
        target_card: CardId,
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    },
    ModifierRemoved {
        id: u32,
        reason: ModifierRemoveReason,
    },
    Concede {
        player: u8,
    },
    Play {
        player: u8,
        card: CardId,
        slot: u8,
    },
    PlayEvent {
        player: u8,
        card: CardId,
    },
    PlayClimax {
        player: u8,
        card: CardId,
    },
    Trigger {
        player: u8,
        icon: crate::db::TriggerIcon,
        card: Option<CardId>,
    },
    Attack {
        player: u8,
        slot: u8,
    },
    AttackType {
        player: u8,
        attacker_slot: u8,
        attack_type: AttackType,
    },
    Counter {
        player: u8,
        card: CardId,
        power: i32,
    },
    Clock {
        player: u8,
        card: Option<CardId>,
    },
    Shuffle {
        player: u8,
        zone: Zone,
    },
    Refresh {
        player: u8,
    },
    RefreshPenalty {
        player: u8,
        card: CardId,
    },
    LevelUpChoice {
        player: u8,
        card: CardId,
    },
    Encore {
        player: u8,
        slot: u8,
        kept: bool,
    },
    Stand {
        player: u8,
    },
    EndTurn {
        player: u8,
    },
    Terminal {
        winner: Option<u8>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ModifierRemoveReason {
    EndOfTurn,
    TargetLeftStage,
    ContinuousRefresh,
}
