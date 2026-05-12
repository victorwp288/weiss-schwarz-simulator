use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::state::AttackType;

/// Player decision kinds exposed to callers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionKind {
    /// Mulligan decision (select cards to discard).
    Mulligan,
    /// Clock phase decision.
    Clock,
    /// Main phase decision.
    Main,
    /// Climax phase decision.
    Climax,
    /// Attack declaration decision.
    AttackDeclaration,
    /// Level-up choice decision.
    LevelUp,
    /// Encore step decision.
    Encore,
    /// Trigger order decision.
    TriggerOrder,
    /// Choice selection decision.
    Choice,
}

/// A pending decision describing which player must act next.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    /// Player index who must act.
    pub player: u8,
    /// Decision kind.
    pub kind: DecisionKind,
    /// Optional focus slot for contextual decisions.
    pub focus_slot: Option<u8>,
}

/// Canonical action descriptor used as the truth representation of legal actions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionDesc {
    /// Confirm mulligan without selecting additional cards.
    MulliganConfirm,
    /// Select a hand card for mulligan.
    MulliganSelect {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Pass the current decision.
    Pass,
    /// Clock a hand card.
    Clock {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Play a character from hand to a stage slot.
    MainPlayCharacter {
        /// Zero-based hand index parameter.
        hand_index: u8,
        /// Zero-based stage slot parameter.
        stage_slot: u8,
    },
    /// Play an event from hand.
    MainPlayEvent {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Move a character between stage slots.
    MainMove {
        /// Zero-based source stage slot parameter.
        from_slot: u8,
        /// Zero-based destination stage slot parameter.
        to_slot: u8,
    },
    /// Activate a character ability from a stage slot.
    MainActivateAbility {
        /// Zero-based stage slot parameter.
        slot: u8,
        /// Zero-based ability index on the source card.
        ability_index: u8,
    },
    /// Play a climax from hand.
    ClimaxPlay {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Declare an attack from a stage slot.
    Attack {
        /// Zero-based stage slot parameter.
        slot: u8,
        /// Attack type parameter.
        attack_type: AttackType,
    },
    /// Play a counter from hand.
    CounterPlay {
        /// Zero-based hand index parameter.
        hand_index: u8,
    },
    /// Select a card for level up.
    LevelUp {
        /// Zero-based selection index parameter.
        index: u8,
    },
    /// Pay encore for a character.
    EncorePay {
        /// Zero-based stage slot parameter.
        slot: u8,
    },
    /// Decline encore for a character.
    EncoreDecline {
        /// Zero-based stage slot parameter.
        slot: u8,
    },
    /// Select trigger order index.
    TriggerOrder {
        /// Zero-based selection index parameter.
        index: u8,
    },
    /// Select a choice option by index.
    ChoiceSelect {
        /// Zero-based selection index parameter.
        index: u8,
    },
    /// Page to previous choice options.
    ChoicePrevPage,
    /// Page to next choice options.
    ChoiceNextPage,
    /// Concede the game.
    Concede,
}

/// Compact list of canonical legal actions.
pub type LegalActions = SmallVec<[ActionDesc; 64]>;
/// Compact list of legal action ids.
pub type LegalActionIds = SmallVec<[u16; 64]>;
