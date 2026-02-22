use crate::db::CardId;
use crate::state::{
    AttackType, ChoiceOptionRef, ChoiceReason, DamageModifierKind, DamageType, ModifierDuration,
    ModifierKind, StackItem, TimingWindow, TriggerEffect,
};
use serde::{Deserialize, Serialize};

/// Reason for revealing a card.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RevealReason {
    /// Reveal was due to a trigger check.
    TriggerCheck,
    /// Reveal was due to a damage check.
    DamageCheck,
    /// Reveal was due to a refresh penalty.
    RefreshPenalty,
    /// Reveal was due to a card being played.
    Play,
    /// Reveal was due to an ability or effect.
    AbilityEffect,
}

/// Audience allowed to see a revealed card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RevealAudience {
    /// Reveal is visible to all observers.
    Public,
    /// Reveal is visible only to the card owner.
    OwnerOnly,
    /// Reveal is visible only to the controller of the card/effect.
    ControllerOnly,
    /// Reveal is visible to both players.
    BothPlayers,
    /// Reveal is visible only in full replay output.
    ReplayOnly,
}

/// Reason a trigger was canceled.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TriggerCancelReason {
    /// Trigger source was invalid or left play before resolution.
    InvalidSource,
    /// Trigger was suppressed by a rule/effect.
    Suppressed,
}

/// Reason a choice was skipped.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ChoiceSkipReason {
    /// No valid candidates were available.
    NoCandidates,
}

/// Logical zone in the game state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Zone {
    /// Deck zone.
    Deck,
    /// Hand zone.
    Hand,
    /// Waiting room zone.
    WaitingRoom,
    /// Clock zone.
    Clock,
    /// Level zone.
    Level,
    /// Stock zone.
    Stock,
    /// Memory zone.
    Memory,
    /// Climax zone.
    Climax,
    /// Resolution zone.
    Resolution,
    /// Stage zone.
    Stage,
}

/// Snapshot of a choice option for replay/debugging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChoiceOptionSnapshot {
    /// Stable id for the option.
    pub option_id: u64,
    /// Reference to the underlying option.
    pub reference: ChoiceOptionRef,
}

/// Canonical event stream emitted by the engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// Player drew a card.
    Draw {
        /// Player index.
        player: u8,
        /// Card id drawn.
        card: CardId,
    },
    /// Card was revealed during a damage check (damage resolution).
    Damage {
        /// Player receiving damage.
        player: u8,
        /// Card id revealed.
        card: CardId,
    },
    /// Damage was canceled.
    DamageCancel {
        /// Player receiving damage.
        player: u8,
    },
    /// Damage intent before modifiers/cancel resolution.
    DamageIntent {
        /// Stable event id for correlating downstream damage events.
        event_id: u32,
        /// Source player (if known).
        source_player: u8,
        /// Source stage slot (if known).
        source_slot: Option<u8>,
        /// Target player.
        target: u8,
        /// Intended damage amount.
        amount: i32,
        /// Damage classification (battle vs effect damage).
        damage_type: DamageType,
        /// Whether damage can be canceled by revealing a climax.
        cancelable: bool,
    },
    /// A damage modifier was applied.
    DamageModifierApplied {
        /// Correlated damage event id.
        event_id: u32,
        /// Modifier kind applied.
        modifier: DamageModifierKind,
        /// Amount before modifier.
        before_amount: i32,
        /// Amount after modifier.
        after_amount: i32,
        /// Cancelable flag before modifier.
        before_cancelable: bool,
        /// Cancelable flag after modifier.
        after_cancelable: bool,
        /// Canceled flag before modifier.
        before_canceled: bool,
        /// Canceled flag after modifier.
        after_canceled: bool,
    },
    /// Damage amount/cancel state was modified and finalized.
    DamageModified {
        /// Correlated damage event id.
        event_id: u32,
        /// Target player.
        target: u8,
        /// Original damage amount.
        original: i32,
        /// Modified damage amount.
        modified: i32,
        /// Whether damage was canceled.
        canceled: bool,
        /// Damage classification (battle vs effect damage).
        damage_type: DamageType,
    },
    /// Damage was committed (a damage card moved to clock).
    DamageCommitted {
        /// Correlated damage event id.
        event_id: u32,
        /// Target player.
        target: u8,
        /// Damage card id committed.
        card: CardId,
        /// Damage classification (battle vs effect damage).
        damage_type: DamageType,
    },
    /// A battle reversal was committed for a stage slot.
    ReversalCommitted {
        /// Player index.
        player: u8,
        /// Stage slot index.
        slot: u8,
        /// Damage event id which caused the reversal (if applicable).
        cause_damage_event: Option<u32>,
    },
    /// A card was revealed (with reason/audience metadata).
    Reveal {
        /// Player index.
        player: u8,
        /// Card id revealed.
        card: CardId,
        /// Reason for the reveal.
        reason: RevealReason,
        /// Audience allowed to see the revealed card.
        audience: RevealAudience,
    },
    /// A trigger was queued for resolution.
    TriggerQueued {
        /// Stable trigger id.
        trigger_id: u32,
        /// Trigger group id for simultaneous triggers.
        group_id: u32,
        /// Player index.
        player: u8,
        /// Source card id.
        source: CardId,
        /// Trigger effect kind.
        effect: TriggerEffect,
    },
    /// Multiple triggers were grouped for resolution ordering.
    TriggerGrouped {
        /// Trigger group id.
        group_id: u32,
        /// Trigger ids in the group.
        trigger_ids: Vec<u32>,
    },
    /// Trigger was resolved.
    TriggerResolved {
        /// Stable trigger id.
        trigger_id: u32,
        /// Player index.
        player: u8,
        /// Trigger effect kind.
        effect: TriggerEffect,
    },
    /// Trigger was canceled.
    TriggerCanceled {
        /// Stable trigger id.
        trigger_id: u32,
        /// Player index.
        player: u8,
        /// Cancellation reason.
        reason: TriggerCancelReason,
    },
    /// A timing window was entered.
    TimingWindowEntered {
        /// Timing window.
        window: TimingWindow,
        /// Active player index.
        player: u8,
    },
    /// Priority was granted to a player within a timing window.
    PriorityGranted {
        /// Timing window.
        window: TimingWindow,
        /// Player index.
        player: u8,
    },
    /// Priority was passed by a player.
    PriorityPassed {
        /// Player index.
        player: u8,
        /// Timing window.
        window: TimingWindow,
        /// Pass count within the window.
        pass_count: u8,
    },
    /// Stack group was presented for ordering/selection.
    StackGroupPresented {
        /// Stable group id.
        group_id: u32,
        /// Player index controlling the stack group.
        controller: u8,
        /// Items in the stack group.
        items: Vec<StackItem>,
    },
    /// Stack order was chosen for a presented stack group.
    StackOrderChosen {
        /// Stable group id.
        group_id: u32,
        /// Player index controlling the stack group.
        controller: u8,
        /// Stack id chosen.
        stack_id: u32,
    },
    /// A stack item was pushed.
    StackPushed {
        /// Stack item pushed.
        item: StackItem,
    },
    /// A stack item was resolved.
    StackResolved {
        /// Stack item resolved.
        item: StackItem,
    },
    /// Automatic resolution cap was exceeded.
    AutoResolveCapExceeded {
        /// Cap value.
        cap: u32,
        /// Stack length at the time of cap exceed.
        stack_len: u32,
        /// Timing window (if known).
        window: Option<TimingWindow>,
    },
    /// Timing window advanced.
    WindowAdvanced {
        /// Previous timing window.
        from: TimingWindow,
        /// Next timing window (or None if leaving timing windows).
        to: Option<TimingWindow>,
    },
    /// A choice was presented to a player.
    ChoicePresented {
        /// Stable choice id.
        choice_id: u32,
        /// Player index.
        player: u8,
        /// Reason for the choice.
        reason: ChoiceReason,
        /// Current page options.
        options: Vec<ChoiceOptionSnapshot>,
        /// Total candidate count (before paging).
        total_candidates: u16,
        /// Page start index.
        page_start: u16,
    },
    /// A choice page was changed.
    ChoicePageChanged {
        /// Stable choice id.
        choice_id: u32,
        /// Player index.
        player: u8,
        /// Previous page start index.
        from_start: u16,
        /// New page start index.
        to_start: u16,
    },
    /// A player made a choice selection.
    ChoiceMade {
        /// Stable choice id.
        choice_id: u32,
        /// Player index.
        player: u8,
        /// Reason for the choice.
        reason: ChoiceReason,
        /// Selected option reference.
        option: ChoiceOptionRef,
    },
    /// A choice was autopicked by the engine.
    ChoiceAutopicked {
        /// Stable choice id.
        choice_id: u32,
        /// Player index.
        player: u8,
        /// Reason for the choice.
        reason: ChoiceReason,
        /// Selected option reference.
        option: ChoiceOptionRef,
    },
    /// A choice was skipped.
    ChoiceSkipped {
        /// Stable choice id.
        choice_id: u32,
        /// Player index.
        player: u8,
        /// Reason for the choice.
        reason: ChoiceReason,
        /// Skip reason.
        skip_reason: ChoiceSkipReason,
    },
    /// A card moved between zones.
    ZoneMove {
        /// Player index.
        player: u8,
        /// Card id moved.
        card: CardId,
        /// Source zone.
        from: Zone,
        /// Destination zone.
        to: Zone,
        /// Source slot (if applicable).
        from_slot: Option<u8>,
        /// Destination slot (if applicable).
        to_slot: Option<u8>,
    },
    /// Control of a card changed.
    ControlChanged {
        /// Card id whose controller changed.
        card: CardId,
        /// Owner player index.
        owner: u8,
        /// Previous controller index.
        from_controller: u8,
        /// New controller index.
        to_controller: u8,
        /// Previous stage slot index.
        from_slot: u8,
        /// New stage slot index.
        to_slot: u8,
    },
    /// A modifier was added.
    ModifierAdded {
        /// Modifier id.
        id: u32,
        /// Source card id.
        source: CardId,
        /// Target player index.
        target_player: u8,
        /// Target stage slot index.
        target_slot: u8,
        /// Target card id.
        target_card: CardId,
        /// Modifier kind.
        kind: ModifierKind,
        /// Signed modifier magnitude.
        magnitude: i32,
        /// Modifier duration.
        duration: ModifierDuration,
    },
    /// A modifier was removed.
    ModifierRemoved {
        /// Modifier id.
        id: u32,
        /// Removal reason.
        reason: ModifierRemoveReason,
    },
    /// Player conceded the game.
    Concede {
        /// Player index.
        player: u8,
    },
    /// Player played a character to a stage slot.
    Play {
        /// Player index.
        player: u8,
        /// Card id played.
        card: CardId,
        /// Stage slot index.
        slot: u8,
    },
    /// Player played an event card.
    PlayEvent {
        /// Player index.
        player: u8,
        /// Event card id played.
        card: CardId,
    },
    /// Player played a climax card.
    PlayClimax {
        /// Player index.
        player: u8,
        /// Climax card id played.
        card: CardId,
    },
    /// Trigger step processed a trigger icon.
    Trigger {
        /// Player index.
        player: u8,
        /// Trigger icon.
        icon: crate::db::TriggerIcon,
        /// Triggered card id (if present).
        card: Option<CardId>,
    },
    /// Player declared an attack with a slot.
    Attack {
        /// Player index.
        player: u8,
        /// Attacker stage slot index.
        slot: u8,
    },
    /// Attack type was chosen (frontal/side/direct).
    AttackType {
        /// Player index.
        player: u8,
        /// Attacker stage slot index.
        attacker_slot: u8,
        /// Declared attack type.
        attack_type: AttackType,
    },
    /// Player played a counter (backup) during counter step.
    Counter {
        /// Player index.
        player: u8,
        /// Counter card id.
        card: CardId,
        /// Power granted by the counter.
        power: i32,
    },
    /// A card was placed into clock (e.g., from hand or damage).
    Clock {
        /// Player index.
        player: u8,
        /// Card id placed into clock (if known).
        card: Option<CardId>,
    },
    /// A zone was shuffled.
    Shuffle {
        /// Player index.
        player: u8,
        /// Zone shuffled.
        zone: Zone,
    },
    /// Refresh occurred (deck refilled from waiting room).
    Refresh {
        /// Player index.
        player: u8,
    },
    /// Refresh penalty card was applied to clock.
    RefreshPenalty {
        /// Player index.
        player: u8,
        /// Card id moved to clock as penalty.
        card: CardId,
    },
    /// Level-up choice was made.
    LevelUpChoice {
        /// Player index.
        player: u8,
        /// Card id selected for level.
        card: CardId,
    },
    /// Encore resolution occurred for a stage slot.
    Encore {
        /// Player index.
        player: u8,
        /// Stage slot index.
        slot: u8,
        /// Whether the character was kept (encored) on stage.
        kept: bool,
    },
    /// Stand phase occurred for a player.
    Stand {
        /// Player index.
        player: u8,
    },
    /// Turn ended for a player.
    EndTurn {
        /// Player index.
        player: u8,
    },
    /// Terminal game event.
    Terminal {
        /// Winner player index, or None for draw/timeout.
        winner: Option<u8>,
    },
}

/// Reason a modifier was removed.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ModifierRemoveReason {
    /// Removed during end-phase cleanup.
    EndOfTurn,
    /// Removed because the target left stage.
    TargetLeftStage,
    /// Removed due to continuous refresh/recompute of continuous effects.
    ContinuousRefresh,
}
