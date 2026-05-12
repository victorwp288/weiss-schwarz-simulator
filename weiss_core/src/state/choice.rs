use serde::{Deserialize, Serialize};

use crate::db::CardId;

use super::{CardInstanceId, PendingTrigger};

/// Reasons for prompting a choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceReason {
    /// Choose a standby target from trigger effect.
    TriggerStandbySelect,
    /// Choose a treasure target from trigger effect.
    TriggerTreasureSelect,
    /// Choose whether to resolve a draw trigger (and/or its target).
    TriggerDrawSelect,
    /// Choose a target for a choice trigger.
    TriggerChoiceSelect,
    /// Choose targets for an auto ability cost step.
    TriggerAutoCostSelect,
    /// Choose whether to draw after a brainstorm effect.
    BrainstormDrawSelect,
    /// Choose an ordering for simultaneous stack items.
    StackOrderSelect,
    /// Choose an action during a priority window.
    PriorityActionSelect,
    /// Choose a payment option for a staged cost.
    CostPayment,
    /// Choose effect targets.
    TargetSelect,
    /// Choose cards to discard during end phase.
    EndPhaseDiscard,
}

/// Cost payment step kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostStepKind {
    /// Rest another character.
    RestOther,
    /// Sacrifice a character from stage.
    SacrificeFromStage,
    /// Discard a card from hand.
    DiscardFromHand,
    /// Clock a card from hand.
    ClockFromHand,
    /// Clock the top card(s) of the deck.
    ClockFromDeckTop,
    /// Reveal a card from hand.
    RevealFromHand,
}

/// Result to execute when staged cost payment finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CostPaymentOutcome {
    /// Continue by resolving the originating ability.
    #[default]
    ResolveAbility,
    /// Keep a character for encore by selecting the specified slot.
    EncoreKeep {
        /// Stage slot index being kept for encore.
        slot: u8,
    },
}

/// State for a multi-step cost payment.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct CostPaymentState {
    /// Controlling player paying the cost.
    pub controller: u8,
    /// Source card id of the ability being paid for.
    pub source_id: CardId,
    /// Source card instance id (stable per-game).
    pub source_instance_id: CardInstanceId,
    /// Optional source slot when the source is on stage.
    pub source_slot: Option<u8>,
    /// Ability index on the source card.
    pub ability_index: u8,
    /// Remaining cost to pay.
    pub remaining: crate::db::AbilityCost,
    /// Currently active staged step, if any.
    pub current_step: Option<CostStepKind>,
    /// Outcome to execute once the cost is fully paid.
    #[serde(default)]
    pub outcome: CostPaymentOutcome,
}

/// Zones that choices can draw from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChoiceZone {
    /// Waiting room.
    WaitingRoom,
    /// Stage.
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
    /// Climax zone.
    Climax,
    /// Resolution zone.
    Resolution,
    /// Effect stack.
    Stack,
    /// Priority window: counter action.
    PriorityCounter,
    /// Priority window: ACT action.
    PriorityAct,
    /// Priority window: pass action.
    PriorityPass,
    /// Skip / decline an optional choice.
    Skip,
}

/// Reference to a concrete choice option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChoiceOptionRef {
    /// Static card id for the referenced option.
    pub card_id: CardId,
    /// Stable per-game card instance id.
    pub instance_id: CardInstanceId,
    /// Zone the option is sourced from.
    pub zone: ChoiceZone,
    /// Optional index within the zone (for list-like zones).
    pub index: Option<u16>,
    /// Optional target slot index (for stage-based options).
    pub target_slot: Option<u8>,
}

/// Choice prompt state for a player.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ChoiceState {
    /// Unique choice id.
    pub id: u32,
    /// Reason this choice is being requested.
    pub reason: ChoiceReason,
    /// Player seat making the choice.
    pub player: u8,
    /// Candidate options in the current page.
    pub options: Vec<ChoiceOptionRef>,
    /// Total candidates available across all pages.
    pub total_candidates: u16,
    /// Start offset of the current page.
    pub page_start: u16,
    /// Optional trigger associated with this choice (when choosing trigger order/targets).
    pub pending_trigger: Option<PendingTrigger>,
}
