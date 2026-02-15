#![allow(missing_docs)]

use thiserror::Error;

use crate::db::CardId;

/// Stable error categories for environment configuration validation.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("deck length invalid for player {player}: got {got}, expected {expected}")]
    DeckLength {
        player: u8,
        got: usize,
        expected: usize,
    },
    #[error("unknown card id {card_id} in player {player} deck")]
    UnknownCardId { player: u8, card_id: CardId },
    #[error("too many climax cards for player {player}: got {got}, max {max}")]
    ClimaxCount { player: u8, got: usize, max: usize },
    #[error("too many copies of card {card_id} for player {player}: got {got}, max {max}")]
    CardCopyCount {
        player: u8,
        card_id: CardId,
        got: usize,
        max: usize,
    },
}

/// Stable error categories for game-state construction.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum StateError {
    #[error("starting player must be 0 or 1 (got {got})")]
    InvalidStartingPlayer { got: u8 },
    #[error("deck length invalid for owner {owner}: got {got}, expected {expected}")]
    DeckLength {
        owner: u8,
        got: usize,
        expected: usize,
    },
}

/// Stable error categories for action application.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ActionError {
    #[error("no pending decision")]
    NoPendingDecision,
    #[error("invalid action id {action_id}")]
    InvalidActionId { action_id: usize },
    #[error("action is not legal for current decision")]
    ActionNotLegal,
}

/// Stable error categories for runtime invariant violations.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum InvariantError {
    #[error("choice paging index out of range: {index}")]
    ChoicePagingIndexOutOfRange { index: usize },
    #[error("trigger id overflow")]
    TriggerIdOverflow,
    #[error("mask bits buffer size mismatch")]
    MaskBitsBufferSizeMismatch,
    #[error("action id out of u16 range: {id}")]
    ActionIdOutOfU16Range { id: usize },
    #[error("invalid stage zone target for generic zone mover")]
    InvalidStageZoneTarget,
    #[error("fingerprint serialization failed")]
    FingerprintSerializationFailed,
}

/// Top-level environment construction errors.
#[derive(Debug, Error)]
pub enum EnvError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error("initial reset faulted with engine status code {code}")]
    InitialResetFault { code: u8 },
}
