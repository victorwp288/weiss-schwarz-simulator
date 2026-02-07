use crate::config::ObservationVisibility;
use crate::db::CardId;
use crate::state::{DamageType, TerminalResult};

/// Metadata describing the current environment state for info payloads.
#[derive(Clone, Debug)]
pub struct EnvInfo {
    /// Observation encoding version.
    pub obs_version: u32,
    /// Action encoding version.
    pub action_version: u32,
    /// Decision kind encoded as a small integer (or none).
    pub decision_kind: i8,
    /// Current player index for the decision, or -1 when terminal.
    pub current_player: i8,
    /// Player perspective used for the observation.
    pub actor: i8,
    /// Decision count in the current episode.
    pub decision_count: u32,
    /// Tick count in the current episode.
    pub tick_count: u32,
    /// Terminal result if the episode has ended.
    pub terminal: Option<TerminalResult>,
    /// Whether the last action was illegal.
    pub illegal_action: bool,
    /// Whether the last step hit an engine error.
    pub engine_error: bool,
    /// Error code for the last engine error.
    pub engine_error_code: u8,
}

/// Outcome from applying a single decision action.
#[derive(Clone, Debug)]
pub struct StepOutcome {
    /// Observation buffer (empty when using no-copy methods).
    pub obs: Vec<i32>,
    /// Reward for this step.
    pub reward: f32,
    /// Episode terminated due to win/loss/draw.
    pub terminated: bool,
    /// Episode truncated due to limits (max decisions/ticks).
    pub truncated: bool,
    /// Auxiliary info and metadata.
    pub info: EnvInfo,
}

#[derive(Clone, Copy, Debug)]
pub struct VisibilityContext {
    pub(crate) viewer: Option<u8>,
    pub(crate) mode: ObservationVisibility,
    pub(crate) policies_enabled: bool,
}

impl VisibilityContext {
    pub(crate) fn is_public(self) -> bool {
        self.policies_enabled && self.mode == ObservationVisibility::Public
    }
}

/// Engine error codes surfaced in outputs.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineErrorCode {
    /// No engine error occurred.
    None = 0,
    /// Stack auto-resolve cap was exceeded.
    StackAutoResolveCap = 1,
    /// Trigger quiescence cap was exceeded.
    TriggerQuiescenceCap = 2,
    /// Engine panic was caught and recorded.
    Panic = 3,
    /// Action application returned an error.
    ActionError = 4,
}

/// Debug instrumentation settings.
#[derive(Clone, Copy, Debug, Default)]
pub struct DebugConfig {
    /// Emit state fingerprints every N decisions (0 = disabled).
    pub fingerprint_every_n: u32,
    /// Size of the per-viewer event ring buffer (0 = disabled).
    pub event_ring_capacity: usize,
}

/// Internal structure for queued damage intent.
#[derive(Clone, Copy, Debug)]
pub struct DamageIntentLocal {
    pub(crate) source_player: u8,
    pub(crate) source_slot: Option<u8>,
    pub(crate) target: u8,
    pub(crate) amount: i32,
    pub(crate) damage_type: DamageType,
    pub(crate) cancelable: bool,
    pub(crate) refresh_penalty: bool,
}

/// Context used when compiling trigger effects.
#[derive(Clone, Copy, Debug)]
pub struct TriggerCompileContext {
    pub(crate) source_card: CardId,
    pub(crate) standby_slot: Option<u8>,
    pub(crate) treasure_take_stock: Option<bool>,
}
