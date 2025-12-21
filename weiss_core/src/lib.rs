//! Weiss Schwarz simulator core.

pub mod config;
pub mod db;
pub mod encode;
pub mod env;
pub mod events;
pub mod effects;
pub mod legal;
pub mod replay;
pub mod rules;
pub mod state;
pub mod util;
pub mod pool;

pub use config::{CurriculumConfig, EnvConfig, RewardConfig, ErrorPolicy, ObservationVisibility};
pub use db::{CardDb, CardId};
pub use env::{GameEnv, StepOutcome};
pub use legal::{ActionDesc, Decision, DecisionKind};
pub use pool::{EnvPool, StepBatchResult};
