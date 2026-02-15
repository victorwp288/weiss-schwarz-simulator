//! Weiss Schwarz simulator core.
//!
//! ## Overview
//! The engine advances until a decision point, then exposes a fixed action space,
//! canonical legal actions, and deterministic replays for RL training and analysis.
//!
//! ## Docs
//! - Docs hub: <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/README.md>
//! - RL contract: <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/rl_contract.md>
//! - Encodings: <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/encodings.md>
//! - Replays & determinism: <https://github.com/victorwp288/weiss-schwarz-simulator/blob/main/docs/replays_determinism.md>
//!
//! ## Module map
//! Primary runtime modules:
//! - `env`: Game environment and advance loop
//! - `legal`: canonical legal action generation
//! - `encode`: observation/action encoding and specs
//! - `pool`: batched stepping and parallelism
//! - `replay`: replay types and serialization
//!
//! Supporting public modules:
//! - `config`: environment/curriculum/reward configuration types
//! - `db`: card database, ability templates/defs, and lookups
//! - `effects`: compiled effect identifiers and payload types
//! - `error`: common error types and diagnostics helpers
//! - `events`: canonical event stream schema
//! - `fingerprint`: stable hashing helpers for config/state/events
//! - `rules`: rules metadata and policy helpers
//! - `state`: core game-state structures
//! - `util`: utility helpers (RNG and small infra)
//! - `visibility_policy`: visibility/sanitization policy helpers
#![deny(missing_docs)]

/// Environment, reward, and curriculum configuration types.
pub mod config;
/// Card database, ability templates/defs, and lookup helpers.
pub mod db;
/// Compiled effect identifiers, payloads, and replay-safe effect schemas.
pub mod effects;
/// Observation/action encodings and schema constants.
pub mod encode;
/// Core game environment and phase/interaction execution.
pub mod env;
/// Shared error types used across core, pool, and Python bindings.
pub mod error;
/// Canonical event stream structures used by replays and debugging.
pub mod events;
/// Stable hashing helpers for state, config, and event fingerprints.
pub mod fingerprint;
/// Legal action generation and decision modeling.
pub mod legal;
/// Batched stepping/reset and parallel environment pooling.
pub mod pool;
/// Replay file format, IO, and supporting data structures.
pub mod replay;
/// Rules metadata helpers and policy wiring.
pub mod rules;
/// Core game state model and turn/phase state machines.
pub mod state;
/// Utility helpers shared across modules.
pub mod util;
/// Observation/replay visibility policy helpers.
pub mod visibility_policy;

pub use config::{
    CurriculumConfig, EndConditionPolicy, EnvConfig, ErrorPolicy, ObservationVisibility,
    RewardConfig, SimultaneousLossPolicy,
};
pub use db::{CardDb, CardId};
pub use env::{DebugConfig, GameEnv, StepOutcome};
pub use error::{ActionError, ConfigError, EnvError, InvariantError, StateError};
pub use legal::{ActionDesc, Decision, DecisionKind};
pub use pool::{
    BatchOutDebug, BatchOutDebugBuffers, BatchOutMinimal, BatchOutMinimalBuffers, EnvPool,
};
