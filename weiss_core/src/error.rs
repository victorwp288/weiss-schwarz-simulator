use thiserror::Error;

use crate::db::CardId;

/// Stable error categories for environment configuration validation.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// A player's deck length did not match the expected size.
    #[error("deck length invalid for player {player}: got {got}, expected {expected}")]
    DeckLength {
        /// Player index.
        player: u8,
        /// Observed deck length.
        got: usize,
        /// Expected deck length.
        expected: usize,
    },
    /// A player's deck referenced a card id that is not present in the database.
    #[error("unknown card id {card_id} in player {player} deck")]
    UnknownCardId {
        /// Player index.
        player: u8,
        /// Unknown card id.
        card_id: CardId,
    },
    /// A player's deck contained too many climax cards.
    #[error("too many climax cards for player {player}: got {got}, max {max}")]
    ClimaxCount {
        /// Player index.
        player: u8,
        /// Observed climax count.
        got: usize,
        /// Maximum allowed climax count.
        max: usize,
    },
    /// A player's deck contained too many copies of a single card id.
    #[error("too many copies of card {card_id} for player {player}: got {got}, max {max}")]
    CardCopyCount {
        /// Player index.
        player: u8,
        /// Card id with excessive copies.
        card_id: CardId,
        /// Observed copy count.
        got: usize,
        /// Maximum allowed copies.
        max: usize,
    },
}

/// Stable error categories for game-state construction.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum StateError {
    /// Starting player must be either player 0 or player 1.
    #[error("starting player must be 0 or 1 (got {got})")]
    InvalidStartingPlayer {
        /// Provided starting player index.
        got: u8,
    },
    /// A player's deck length did not match the expected size for state construction.
    #[error("deck length invalid for owner {owner}: got {got}, expected {expected}")]
    DeckLength {
        /// Player index.
        owner: u8,
        /// Observed deck length.
        got: usize,
        /// Expected deck length.
        expected: usize,
    },
}

/// Stable error categories for action application.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ActionError {
    /// An action was attempted when no decision is pending.
    #[error("no pending decision")]
    NoPendingDecision,
    /// An action id could not be decoded or is outside the action space.
    #[error("invalid action id {action_id}")]
    InvalidActionId {
        /// Provided action id.
        action_id: usize,
    },
    /// An action id was decoded but is not legal for the current decision.
    #[error("action is not legal for current decision")]
    ActionNotLegal,
}

/// Stable error categories for runtime invariant violations.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum InvariantError {
    /// Internal choice paging index exceeded the available options.
    #[error("choice paging index out of range: {index}")]
    ChoicePagingIndexOutOfRange {
        /// Provided choice paging index.
        index: usize,
    },
    /// Trigger id counter overflowed its representable range.
    #[error("trigger id overflow")]
    TriggerIdOverflow,
    /// Action mask bit buffer size did not match the expected capacity.
    #[error("mask bits buffer size mismatch")]
    MaskBitsBufferSizeMismatch,
    /// A computed action id exceeded `u16::MAX`.
    #[error("action id out of u16 range: {id}")]
    ActionIdOutOfU16Range {
        /// Computed action id.
        id: usize,
    },
    /// A generic stage mover received an invalid stage zone target.
    #[error("invalid stage zone target for generic zone mover")]
    InvalidStageZoneTarget,
    /// Fingerprint serialization failed unexpectedly.
    #[error("fingerprint serialization failed")]
    FingerprintSerializationFailed,
}

/// Top-level environment construction errors.
#[derive(Debug, Error)]
pub enum EnvError {
    /// Environment configuration validation failed.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Initial game-state construction failed.
    #[error(transparent)]
    State(#[from] StateError),
    /// The initial reset trapped a fault and returned a status code.
    #[error("initial reset faulted with engine status code {code}")]
    InitialResetFault {
        /// Engine error code returned by reset.
        code: u8,
    },
}
