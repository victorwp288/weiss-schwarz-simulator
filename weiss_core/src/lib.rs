//! Weiss Schwarz simulator core.

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

pub use config::{
    CurriculumConfig, EndConditionPolicy, EnvConfig, ErrorPolicy, ObservationVisibility,
    RewardConfig, SimultaneousLossPolicy,
};
pub use db::{CardDb, CardId};
pub use env::{GameEnv, StepOutcome};
pub use legal::{ActionDesc, Decision, DecisionKind};
pub use pool::{EnvPool, StepBatchResult};
