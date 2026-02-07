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
//! - `events`: canonical event stream schema
//! - `fingerprint`: stable hashing helpers for config/state/events
//! - `rules`: rules metadata and policy helpers
//! - `state`: core game-state structures
//! - `util`: utility helpers (RNG and small infra)
//! - `visibility_policy`: visibility/sanitization policy helpers

pub mod config;
pub mod db;
pub mod effects;
pub mod encode;
pub mod env;
pub mod events;
pub mod fingerprint;
pub mod legal;
pub mod pool;
pub mod replay;
pub mod rules;
pub mod state;
pub mod util;
pub mod visibility_policy;

pub use config::{
    CurriculumConfig, EndConditionPolicy, EnvConfig, ErrorPolicy, ObservationVisibility,
    RewardConfig, SimultaneousLossPolicy,
};
pub use db::{CardDb, CardId};
pub use env::{DebugConfig, GameEnv, StepOutcome};
pub use legal::{ActionDesc, Decision, DecisionKind};
pub use pool::{
    BatchOutDebug, BatchOutDebugBuffers, BatchOutMinimal, BatchOutMinimalBuffers, EnvPool,
};
