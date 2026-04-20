use std::collections::BTreeSet;
use std::sync::Arc;

use crate::config::{CurriculumConfig, EnvConfig};
use crate::db::{CardDb, CardId};
use crate::events::Event;
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::replay::{ReplayConfig, ReplayEvent, ReplayWriter, StepMeta};
use crate::state::{CardInstanceId, GameState, Phase};
use crate::util::Rng64;

use super::cache::{ActionCache, EnvScratch};
use super::types::{DebugConfig, EngineErrorCode, FaultRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProgressSignature {
    pub active_player: u8,
    pub turn_number: u32,
    pub phase: Phase,
    pub choice_id: Option<u32>,
    pub choice_page_start: u16,
    pub choice_total_candidates: u16,
    pub deck_counts: [u16; 2],
    pub hand_counts: [u16; 2],
    pub waiting_room_counts: [u16; 2],
    pub clock_counts: [u16; 2],
    pub level_counts: [u16; 2],
    pub stock_counts: [u16; 2],
    pub memory_counts: [u16; 2],
    pub climax_counts: [u16; 2],
    pub resolution_counts: [u16; 2],
    pub occupied_stage_counts: [u16; 2],
    pub reversed_stage_counts: [u16; 2],
    pub live_stage_counts: [u16; 2],
}

impl Default for ProgressSignature {
    fn default() -> Self {
        Self {
            active_player: 0,
            turn_number: 0,
            phase: Phase::Mulligan,
            choice_id: None,
            choice_page_start: 0,
            choice_total_candidates: 0,
            deck_counts: [0; 2],
            hand_counts: [0; 2],
            waiting_room_counts: [0; 2],
            clock_counts: [0; 2],
            level_counts: [0; 2],
            stock_counts: [0; 2],
            memory_counts: [0; 2],
            climax_counts: [0; 2],
            resolution_counts: [0; 2],
            occupied_stage_counts: [0; 2],
            reversed_stage_counts: [0; 2],
            live_stage_counts: [0; 2],
        }
    }
}

/// A single Weiss Schwarz environment instance with deterministic RNG state.
pub struct GameEnv {
    /// Shared card database backing this environment.
    pub db: Arc<CardDb>,
    /// Environment configuration (reward, limits, visibility).
    pub config: EnvConfig,
    /// Curriculum feature toggles for rules and mechanics.
    pub curriculum: CurriculumConfig,
    /// Current game state.
    pub state: GameState,
    /// Stable environment identifier within a pool.
    pub env_id: u32,
    /// Base seed used to derive per-episode randomness.
    pub base_seed: u64,
    /// Episode counter for this environment.
    pub episode_index: u32,
    /// Current decision, if the engine is waiting for an action.
    pub decision: Option<Decision>,
    pub(crate) action_cache: ActionCache,
    pub(crate) output_mask_enabled: bool,
    pub(crate) output_mask_bits_enabled: bool,
    pub(crate) decision_id: u32,
    /// Last action applied in canonical form.
    pub last_action_desc: Option<ActionDesc>,
    /// Player index that performed the last action.
    pub last_action_player: Option<u8>,
    /// Decision kind associated with the last applied action.
    pub(crate) last_action_decision_kind: Option<DecisionKind>,
    /// Whether the last action was illegal.
    pub last_illegal_action: bool,
    /// Whether the last step hit an engine error.
    pub last_engine_error: bool,
    /// Error code recorded for the last engine error.
    pub last_engine_error_code: EngineErrorCode,
    /// Perspective used for the last emitted observation.
    pub last_perspective: u8,
    /// Pending damage deltas awaiting resolution per player.
    pub pending_damage_delta: [i32; 2],
    /// Consecutive decisions without a meaningful progress signature change.
    pub no_progress_decisions: u32,
    /// Scratch buffer holding the most recent observation.
    pub obs_buf: Vec<i32>,
    pub(crate) obs_dirty: bool,
    pub(crate) obs_perspective: u8,
    pub(crate) player_obs_version: [u32; 2],
    pub(crate) player_block_cache_version: [u32; 2],
    pub(crate) player_block_cache_self: [Vec<i32>; 2],
    pub(crate) player_block_cache_opp: [Vec<i32>; 2],
    pub(crate) slot_power_cache: [[i32; crate::encode::MAX_STAGE]; 2],
    pub(crate) slot_power_dirty: [[bool; crate::encode::MAX_STAGE]; 2],
    pub(crate) slot_power_cache_card: [[Option<CardId>; crate::encode::MAX_STAGE]; 2],
    pub(crate) slot_power_cache_mod_turn: [[i32; crate::encode::MAX_STAGE]; 2],
    pub(crate) slot_power_cache_mod_battle: [[i32; crate::encode::MAX_STAGE]; 2],
    pub(crate) slot_power_cache_modifiers_version: [[u64; crate::encode::MAX_STAGE]; 2],
    pub(crate) modifiers_version: u64,
    pub(crate) rule_actions_dirty: bool,
    pub(crate) continuous_modifiers_dirty: bool,
    pub(crate) last_rule_action_phase: Phase,
    /// Replay configuration controlling sampling and storage.
    pub replay_config: ReplayConfig,
    /// Optional writer for replay output.
    pub replay_writer: Option<ReplayWriter>,
    /// Canonical action list for replay output.
    pub replay_actions: Vec<ActionDesc>,
    /// Raw action list prior to sanitization.
    pub replay_actions_raw: Vec<ActionDesc>,
    /// Canonical action ids for replay output.
    pub replay_action_ids: Vec<u16>,
    /// Raw action ids prior to sanitization.
    pub replay_action_ids_raw: Vec<u16>,
    /// Collected replay events.
    pub replay_events: Vec<ReplayEvent>,
    pub(crate) canonical_events: Vec<Event>,
    /// Per-decision metadata for replay output.
    pub replay_steps: Vec<StepMeta>,
    /// Whether replay recording is active.
    pub recording: bool,
    /// RNG for non-gameplay metadata sampling.
    pub meta_rng: Rng64,
    /// Current episode seed.
    pub episode_seed: u64,
    /// Scratch buffer for slot replacement during play/move.
    pub scratch_replacement_indices: Vec<usize>,
    pub(crate) scratch: EnvScratch,
    pub(crate) revealed_to_viewer: [BTreeSet<CardInstanceId>; 2],
    pub(crate) debug: DebugConfig,
    pub(crate) debug_event_ring: Option<[super::debug_events::EventRing; 2]>,
    pub(crate) validate_state_enabled: bool,
    pub(crate) fault_latched: Option<FaultRecord>,
}
