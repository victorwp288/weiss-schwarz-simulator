/// Internal max choice options mirrored from encoded action constants.
pub(crate) const MAX_CHOICE_OPTIONS: usize = crate::encode::CHOICE_COUNT;

/// Maximum stack depth the engine will auto-resolve before faulting.
pub const STACK_AUTO_RESOLVE_CAP: u32 = 256;
/// Maximum check-timing iterations allowed before faulting (prevents infinite loops).
pub const CHECK_TIMING_QUIESCENCE_CAP: u32 = 256;
/// End-of-turn hand limit enforced by cleanup rules.
pub const HAND_LIMIT: usize = 7;

/// Encoded trigger effect id for Soul.
pub(crate) const TRIGGER_EFFECT_SOUL: u8 = 0;
/// Encoded trigger effect id for Draw.
pub(crate) const TRIGGER_EFFECT_DRAW: u8 = 1;
/// Encoded trigger effect id for Shot.
pub(crate) const TRIGGER_EFFECT_SHOT: u8 = 2;
/// Encoded trigger effect id for Gate.
pub(crate) const TRIGGER_EFFECT_GATE: u8 = 3;
/// Encoded trigger effect id for Bounce.
pub(crate) const TRIGGER_EFFECT_BOUNCE: u8 = 4;
/// Encoded trigger effect id for Standby.
pub(crate) const TRIGGER_EFFECT_STANDBY: u8 = 5;
/// Encoded trigger effect id for Treasure (stock branch).
pub(crate) const TRIGGER_EFFECT_TREASURE_STOCK: u8 = 6;
/// Encoded trigger effect id for Treasure (move branch).
pub(crate) const TRIGGER_EFFECT_TREASURE_MOVE: u8 = 7;
/// Encoded trigger effect id for Pool (stock branch).
pub(crate) const TRIGGER_EFFECT_POOL_STOCK: u8 = 8;
/// Encoded trigger effect id for Pool (move branch).
pub(crate) const TRIGGER_EFFECT_POOL_MOVE: u8 = 9;
