use std::collections::BTreeSet;
use std::sync::Arc;

use crate::config::{CurriculumConfig, EnvConfig};
use crate::db::CardDb;
use crate::encode::{OBS_LEN, PER_PLAYER_BLOCK_LEN};
use crate::error::EnvError;
use crate::events::Event;
use crate::replay::{ReplayConfig, ReplayWriter};
use crate::state::{GameState, Phase};
use crate::util::Rng64;

use super::{ActionCache, DebugConfig, EngineErrorCode, EnvScratch, GameEnv, StepOutcome};

impl GameEnv {
    fn validate_deck_lists(db: &CardDb, config: &EnvConfig) -> Result<(), EnvError> {
        config.validate_with_db(db)?;
        Ok(())
    }

    /// Construct a new environment and immediately reset it to a valid decision.
    ///
    /// Validates deck lists and initializes replay/config caches.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use weiss_core::{CardDb, CurriculumConfig, EnvConfig, GameEnv};
    /// use weiss_core::replay::ReplayConfig;
    ///
    /// # let db: CardDb = todo!("load card db");
    /// # let config: EnvConfig = todo!("build env config");
    /// let mut env = GameEnv::new(
    ///     Arc::new(db),
    ///     config,
    ///     CurriculumConfig::default(),
    ///     0,
    ///     ReplayConfig::default(),
    ///     None,
    ///     0,
    /// )?;
    ///
    /// let outcome = env.apply_action_id(weiss_core::encode::PASS_ACTION_ID)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new(
        db: Arc<CardDb>,
        config: EnvConfig,
        curriculum: CurriculumConfig,
        seed: u64,
        replay_config: ReplayConfig,
        replay_writer: Option<ReplayWriter>,
        env_id: u32,
    ) -> Result<Self, EnvError> {
        Self::validate_deck_lists(&db, &config)?;
        let starting_player = (seed as u8) & 1;
        let state = GameState::new(
            config.deck_lists[0].clone(),
            config.deck_lists[1].clone(),
            seed,
            starting_player,
        )?;
        let mut curriculum = curriculum;
        curriculum.rebuild_cache();
        let mut replay_config = replay_config;
        replay_config.rebuild_cache();
        let mut env = Self {
            db,
            config,
            curriculum,
            state,
            env_id,
            base_seed: seed,
            episode_index: 0,
            decision: None,
            action_cache: ActionCache::new(),
            output_mask_enabled: true,
            output_mask_bits_enabled: true,
            decision_id: 0,
            last_action_desc: None,
            last_action_player: None,
            last_action_decision_kind: None,
            last_illegal_action: false,
            last_engine_error: false,
            last_engine_error_code: EngineErrorCode::None,
            last_perspective: 0,
            pending_damage_delta: [0, 0],
            no_progress_decisions: 0,
            obs_buf: vec![0; OBS_LEN],
            obs_dirty: true,
            obs_perspective: starting_player,
            player_obs_version: [0; 2],
            player_block_cache_version: [u32::MAX; 2],
            player_block_cache_self: std::array::from_fn(|_| vec![0; PER_PLAYER_BLOCK_LEN]),
            player_block_cache_opp: std::array::from_fn(|_| vec![0; PER_PLAYER_BLOCK_LEN]),
            slot_power_cache: [[0; crate::encode::MAX_STAGE]; 2],
            slot_power_dirty: [[true; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_card: [[None; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_mod_turn: [[0; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_mod_battle: [[0; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_modifiers_version: [[0; crate::encode::MAX_STAGE]; 2],
            modifiers_version: 0,
            rule_actions_dirty: true,
            continuous_modifiers_dirty: true,
            last_rule_action_phase: Phase::Stand,
            replay_config,
            replay_writer,
            replay_actions: Vec::new(),
            replay_actions_raw: Vec::new(),
            replay_action_ids: Vec::new(),
            replay_action_ids_raw: Vec::new(),
            replay_events: Vec::new(),
            canonical_events: Vec::new(),
            replay_steps: Vec::new(),
            recording: false,
            meta_rng: Rng64::new(seed ^ 0xABCDEF1234567890),
            episode_seed: seed,
            scratch_replacement_indices: Vec::new(),
            scratch: EnvScratch::new(),
            revealed_to_viewer: std::array::from_fn(|_| BTreeSet::new()),
            debug: DebugConfig::default(),
            debug_event_ring: None,
            validate_state_enabled: std::env::var("WEISS_VALIDATE_STATE").ok().as_deref()
                == Some("1"),
            fault_latched: None,
        };
        let reset_outcome = env.reset();
        Self::finalize_after_initial_reset(env, reset_outcome)
    }

    /// Compatibility helper for tests/benches.
    pub fn new_or_panic(
        db: Arc<CardDb>,
        config: EnvConfig,
        curriculum: CurriculumConfig,
        seed: u64,
        replay_config: ReplayConfig,
        replay_writer: Option<ReplayWriter>,
        env_id: u32,
    ) -> Self {
        Self::new(
            db,
            config,
            curriculum,
            seed,
            replay_config,
            replay_writer,
            env_id,
        )
        .expect("GameEnv::new_or_panic failed")
    }

    fn finalize_after_initial_reset(
        env: Self,
        reset_outcome: StepOutcome,
    ) -> Result<Self, EnvError> {
        if env.is_fault_latched() || reset_outcome.info.engine_error {
            return Err(EnvError::InitialResetFault {
                code: reset_outcome.info.engine_error_code,
            });
        }
        Ok(env)
    }

    /// Reset the environment and return a full observation.
    pub fn reset(&mut self) -> StepOutcome {
        self.reset_with_obs(true)
    }

    /// Reset the environment without copying the observation buffer.
    ///
    /// The returned `StepOutcome.obs` will be empty; use this when you
    /// manage observation buffers externally (e.g. EnvPool outputs).
    pub fn reset_no_copy(&mut self) -> StepOutcome {
        self.reset_with_obs(false)
    }

    /// Reset the environment with an explicit episode seed.
    ///
    /// Passing the same `episode_seed` with identical config/db yields the same
    /// initial game state and first decision.
    pub fn reset_with_episode_seed(&mut self, episode_seed: u64) -> StepOutcome {
        self.reset_with_episode_seed_internal(episode_seed, true)
    }

    /// Reset the environment with an explicit seed, without copying obs.
    pub fn reset_with_episode_seed_no_copy(&mut self, episode_seed: u64) -> StepOutcome {
        self.reset_with_episode_seed_internal(episode_seed, false)
    }

    /// Canonical event stream for the current episode.
    pub fn canonical_events(&self) -> &[Event] {
        &self.canonical_events
    }

    /// Monotonic decision id for the current episode.
    pub fn decision_id(&self) -> u32 {
        self.decision_id
    }

    /// Reset using the env-local meta RNG to derive the next episode seed.
    ///
    /// The meta RNG is seeded from `base_seed`, so seed progression is stable
    /// for a fixed call order.
    fn reset_with_obs(&mut self, copy_obs: bool) -> StepOutcome {
        let episode_seed = self.meta_rng.next_u64();
        self.reset_with_episode_seed_internal(episode_seed, copy_obs)
    }

    /// Reset all per-episode state from `episode_seed`.
    ///
    /// Determinism invariants:
    /// - `episode_seed` fully controls starting player and initial deck shuffle.
    /// - caches/dirty flags/replay buffers are reset to canonical defaults.
    /// - `decision_id` starts at `u32::MAX` so the first `set_decision` wraps to `0`.
    fn reset_with_episode_seed_internal(
        &mut self,
        episode_seed: u64,
        copy_obs: bool,
    ) -> StepOutcome {
        // Keep the same starting-player rule in construction and reset paths.
        let starting_player = if (episode_seed & 1) == 1 { 1 } else { 0 };
        self.episode_seed = episode_seed;
        self.episode_index = self.episode_index.wrapping_add(1);
        if Self::validate_deck_lists(&self.db, &self.config).is_err() {
            return self.latch_fault(
                EngineErrorCode::ResetError,
                None,
                super::FaultSource::Reset,
                copy_obs,
            );
        }
        self.state = match GameState::new(
            self.config.deck_lists[0].clone(),
            self.config.deck_lists[1].clone(),
            episode_seed,
            starting_player,
        ) {
            Ok(state) => state,
            Err(err) => {
                eprintln!("reset GameState::new failed: {err}");
                return self.latch_fault(
                    EngineErrorCode::ResetError,
                    None,
                    super::FaultSource::Reset,
                    copy_obs,
                );
            }
        };
        self.slot_power_cache = [[0; crate::encode::MAX_STAGE]; 2];
        self.slot_power_dirty = [[true; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_card = [[None; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_mod_turn = [[0; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_mod_battle = [[0; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_modifiers_version = [[0; crate::encode::MAX_STAGE]; 2];
        self.modifiers_version = 0;
        self.rule_actions_dirty = true;
        self.continuous_modifiers_dirty = true;
        self.last_rule_action_phase = self.state.turn.phase;
        self.decision = None;
        self.action_cache.clear();
        self.decision_id = u32::MAX;
        self.last_action_desc = None;
        self.last_action_player = None;
        self.last_action_decision_kind = None;
        self.last_illegal_action = false;
        self.last_engine_error = false;
        self.last_engine_error_code = EngineErrorCode::None;
        self.fault_latched = None;
        self.last_perspective = self.state.turn.starting_player;
        self.pending_damage_delta = [0, 0];
        self.no_progress_decisions = 0;
        self.obs_dirty = true;
        self.player_obs_version = [0; 2];
        self.player_block_cache_version = [u32::MAX; 2];
        if self.obs_buf.len() != OBS_LEN {
            self.obs_buf.resize(OBS_LEN, 0);
        }
        self.replay_actions.clear();
        self.replay_actions_raw.clear();
        self.replay_action_ids.clear();
        self.replay_action_ids_raw.clear();
        self.replay_events.clear();
        self.canonical_events.clear();
        self.replay_steps.clear();
        for set in &mut self.revealed_to_viewer {
            set.clear();
        }
        if let Some(rings) = self.debug_event_ring.as_mut() {
            for ring in rings.iter_mut() {
                ring.clear();
            }
        }
        // Replay sampling also consumes the deterministic meta RNG stream.
        let threshold = self.replay_config.sample_threshold;
        self.recording = self.replay_config.enabled
            && (threshold == u32::MAX || (threshold > 0 && self.meta_rng.next_u32() <= threshold));
        self.scratch_replacement_indices.clear();

        for player in 0..2 {
            self.shuffle_deck(player as u8);
            self.draw_to_hand(player as u8, 5);
        }

        self.advance_until_decision();
        self.update_action_cache();
        if self.maybe_validate_state("reset") || self.is_fault_latched() {
            return self.build_fault_step_outcome(copy_obs);
        }
        self.build_outcome_with_obs(0.0, copy_obs)
    }

    /// Clear per-step status flags while preserving latched fault visibility.
    pub(crate) fn clear_status_flags(&mut self) {
        self.last_illegal_action = false;
        if let Some(record) = self.fault_latched {
            self.last_engine_error = true;
            self.last_engine_error_code = record.code;
        } else {
            self.last_engine_error = false;
            self.last_engine_error_code = EngineErrorCode::None;
        }
    }

    /// Update debug settings for this environment instance.
    pub fn set_debug_config(&mut self, debug: DebugConfig) {
        self.debug = debug;
        if debug.event_ring_capacity == 0 {
            self.debug_event_ring = None;
        } else {
            self.debug_event_ring = Some(std::array::from_fn(|_| {
                super::debug_events::EventRing::new(debug.event_ring_capacity)
            }));
        }
    }

    /// Enable or disable output action masks.
    pub fn set_output_mask_enabled(&mut self, enabled: bool) {
        if self.output_mask_enabled == enabled {
            return;
        }
        self.output_mask_enabled = enabled;
        self.action_cache.decision_id = u32::MAX;
        if !enabled {
            self.action_cache.mask.fill(0);
        }
    }

    /// Enable or disable output action mask bits.
    pub fn set_output_mask_bits_enabled(&mut self, enabled: bool) {
        if self.output_mask_bits_enabled == enabled {
            return;
        }
        self.output_mask_bits_enabled = enabled;
        self.action_cache.decision_id = u32::MAX;
        if !enabled {
            self.action_cache.mask_bits.fill(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
    };
    use crate::db::{CardColor, CardDb, CardStatic, CardType};
    use crate::env::{EngineErrorCode, FaultRecord, FaultSource};
    use std::sync::Arc;

    fn make_db() -> Arc<CardDb> {
        let mut cards = Vec::new();
        for id in 1..=13 {
            cards.push(CardStatic {
                id,
                card_set: None,
                card_type: CardType::Character,
                color: CardColor::Red,
                level: 0,
                cost: 0,
                power: 500,
                soul: 1,
                triggers: vec![],
                traits: vec![],
                abilities: vec![],
                ability_defs: vec![],
                counter_timing: false,
                raw_text: None,
            });
        }
        Arc::new(CardDb::new(cards).expect("db build"))
    }

    fn make_deck() -> Vec<u32> {
        let mut deck = Vec::new();
        for id in 1..=13u32 {
            for _ in 0..4 {
                deck.push(id);
            }
        }
        deck.truncate(crate::encode::MAX_DECK);
        deck
    }

    fn make_env() -> GameEnv {
        let db = make_db();
        let deck = make_deck();
        let config = EnvConfig {
            deck_lists: [deck.clone(), deck],
            deck_ids: [1, 2],
            max_decisions: 100,
            max_ticks: 1000,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        };
        GameEnv::new_or_panic(
            db,
            config,
            CurriculumConfig::default(),
            77,
            ReplayConfig::default(),
            None,
            0,
        )
    }

    #[test]
    fn finalize_after_initial_reset_returns_error_when_engine_error_is_set() {
        let mut env = make_env();
        let mut outcome = env.reset();
        outcome.info.engine_error = true;
        outcome.info.engine_error_code = EngineErrorCode::ResetError as u8;

        let err = match GameEnv::finalize_after_initial_reset(env, outcome) {
            Err(err) => err,
            Ok(_) => panic!("engine error should fail initial reset"),
        };
        match err {
            EnvError::InitialResetFault { code } => {
                assert_eq!(code, EngineErrorCode::ResetError as u8)
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn finalize_after_initial_reset_returns_error_when_fault_is_latched() {
        let mut env = make_env();
        let mut outcome = env.reset();
        env.fault_latched = Some(FaultRecord {
            code: EngineErrorCode::ResetError,
            actor: None,
            fingerprint: 1,
            source: FaultSource::Reset,
            reward_emitted: false,
        });
        outcome.info.engine_error = false;
        outcome.info.engine_error_code = EngineErrorCode::ResetError as u8;

        let err = match GameEnv::finalize_after_initial_reset(env, outcome) {
            Err(err) => err,
            Ok(_) => panic!("latched fault should fail initial reset"),
        };
        match err {
            EnvError::InitialResetFault { code } => {
                assert_eq!(code, EngineErrorCode::ResetError as u8)
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn finalize_after_initial_reset_returns_env_when_no_faults() {
        let mut env = make_env();
        let outcome = env.reset();
        let ok_env =
            GameEnv::finalize_after_initial_reset(env, outcome).expect("expected healthy env");
        assert!(!ok_env.is_fault_latched());
    }
}
