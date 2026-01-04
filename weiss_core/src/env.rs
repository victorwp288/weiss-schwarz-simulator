use anyhow::{anyhow, Result};
use std::collections::BTreeSet;
use std::sync::Arc;

use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
};
use crate::db::{CardDb, CardId, CardStatic, CardType};
use crate::encode::{
    encode_observation_with_slot_power, fill_action_mask, ACTION_ENCODING_VERSION,
    OBS_ENCODING_VERSION, OBS_LEN,
};
use crate::events::{Event, Zone};
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::replay::{ReplayConfig, ReplayEvent, ReplayWriter, StepMeta};
use crate::state::{
    CardInstance, CardInstanceId, ChoiceOptionRef, DamageType, GameState, ModifierDuration,
    ModifierKind, Phase, TargetRef, TerminalResult, TimingWindow,
};
use crate::util::Rng64;

/// Metadata describing the current environment state for Python info payloads.
#[derive(Clone, Debug)]
pub struct EnvInfo {
    pub obs_version: u32,
    pub action_version: u32,
    pub decision_kind: i8,
    pub current_player: i8,
    pub actor: i8,
    pub decision_count: u32,
    pub tick_count: u32,
    pub terminal: Option<TerminalResult>,
    pub illegal_action: bool,
    pub engine_error: bool,
    pub engine_error_code: u8,
}

/// Outcome from applying a single decision action.
#[derive(Clone, Debug)]
pub struct StepOutcome {
    pub obs: Vec<i32>,
    pub reward: f32,
    pub terminated: bool,
    pub truncated: bool,
    pub info: EnvInfo,
}

#[derive(Clone, Copy, Debug)]
struct VisibilityContext {
    viewer: Option<u8>,
    mode: ObservationVisibility,
    policies_enabled: bool,
}

impl VisibilityContext {
    fn is_public(self) -> bool {
        self.policies_enabled && self.mode == ObservationVisibility::Public
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineErrorCode {
    None = 0,
    StackAutoResolveCap = 1,
    TriggerQuiescenceCap = 2,
    Panic = 3,
    ActionError = 4,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DebugConfig {
    pub fingerprint_every_n: u32,
    pub event_ring_capacity: usize,
}

/// A single Weiss Schwarz environment instance with deterministic RNG state.
pub struct GameEnv {
    pub db: Arc<CardDb>,
    pub config: EnvConfig,
    pub curriculum: CurriculumConfig,
    pub state: GameState,
    pub env_id: u32,
    pub episode_index: u32,
    pub decision: Option<Decision>,
    action_cache: ActionCache,
    decision_id: u32,
    pub last_action_desc: Option<ActionDesc>,
    pub last_action_player: Option<u8>,
    pub last_illegal_action: bool,
    pub last_engine_error: bool,
    pub last_engine_error_code: EngineErrorCode,
    pub last_perspective: u8,
    pub pending_damage_delta: [i32; 2],
    pub obs_buf: Vec<i32>,
    slot_power_cache: [[i32; crate::encode::MAX_STAGE]; 2],
    slot_power_dirty: [[bool; crate::encode::MAX_STAGE]; 2],
    slot_power_cache_card: [[CardId; crate::encode::MAX_STAGE]; 2],
    slot_power_cache_mod_turn: [[i32; crate::encode::MAX_STAGE]; 2],
    slot_power_cache_mod_battle: [[i32; crate::encode::MAX_STAGE]; 2],
    rule_actions_dirty: bool,
    continuous_modifiers_dirty: bool,
    last_rule_action_phase: Phase,
    pub replay_config: ReplayConfig,
    pub replay_writer: Option<ReplayWriter>,
    pub replay_actions: Vec<ActionDesc>,
    pub replay_events: Vec<ReplayEvent>,
    canonical_events: Vec<Event>,
    pub replay_steps: Vec<StepMeta>,
    pub recording: bool,
    pub meta_rng: Rng64,
    pub episode_seed: u64,
    pub scratch_replacement_indices: Vec<usize>,
    scratch: EnvScratch,
    revealed_to_viewer: [BTreeSet<CardInstanceId>; 2],
    debug: DebugConfig,
    debug_event_ring: Option<[EventRing; 2]>,
}

#[derive(Clone, Copy, Debug)]
struct DamageIntentLocal {
    source_player: u8,
    source_slot: Option<u8>,
    target: u8,
    amount: i32,
    damage_type: DamageType,
    cancelable: bool,
    refresh_penalty: bool,
}

struct EnvScratch {
    targets: Vec<TargetRef>,
    choice_options: Vec<ChoiceOptionRef>,
    priority_actions: Vec<ActionDesc>,
}

impl EnvScratch {
    fn new() -> Self {
        Self {
            targets: Vec::with_capacity(32),
            choice_options: Vec::with_capacity(32),
            priority_actions: Vec::with_capacity(16),
        }
    }
}

struct ActionCache {
    mask: Vec<u8>,
    lookup: Vec<Option<ActionDesc>>,
    legal_actions: Vec<ActionDesc>,
    decision_id: u32,
    decision_kind: Option<DecisionKind>,
    decision_player: u8,
}

impl ActionCache {
    fn new() -> Self {
        Self {
            mask: vec![0u8; crate::encode::ACTION_SPACE_SIZE],
            lookup: vec![None; crate::encode::ACTION_SPACE_SIZE],
            legal_actions: Vec::new(),
            decision_id: 0,
            decision_kind: None,
            decision_player: 0,
        }
    }

    fn clear(&mut self) {
        if self.mask.len() != crate::encode::ACTION_SPACE_SIZE {
            self.mask.resize(crate::encode::ACTION_SPACE_SIZE, 0);
        }
        self.mask.fill(0);
        if self.lookup.len() != crate::encode::ACTION_SPACE_SIZE {
            self.lookup.resize(crate::encode::ACTION_SPACE_SIZE, None);
        }
        for slot in self.lookup.iter_mut() {
            *slot = None;
        }
        self.legal_actions.clear();
        self.decision_id = 0;
        self.decision_kind = None;
        self.decision_player = 0;
    }

    fn update(
        &mut self,
        state: &GameState,
        decision: &Decision,
        decision_id: u32,
        db: &CardDb,
        curriculum: &CurriculumConfig,
        allowed_card_sets: Option<&std::collections::HashSet<String>>,
    ) {
        if self.decision_id == decision_id
            && self.decision_kind == Some(decision.kind)
            && self.decision_player == decision.player
        {
            return;
        }
        let actions =
            crate::legal::legal_actions_cached(state, decision, db, curriculum, allowed_card_sets);
        fill_action_mask(&actions, &mut self.mask, &mut self.lookup);
        self.legal_actions = actions;
        self.decision_id = decision_id;
        self.decision_kind = Some(decision.kind);
        self.decision_player = decision.player;
    }
}

struct EventRing {
    capacity: usize,
    events: Vec<ReplayEvent>,
    next: usize,
    full: bool,
}

impl EventRing {
    fn new(capacity: usize) -> Self {
        let mut events = Vec::with_capacity(capacity);
        events.reserve(capacity);
        Self {
            capacity,
            events,
            next: 0,
            full: false,
        }
    }

    fn clear(&mut self) {
        self.events.clear();
        self.next = 0;
        self.full = false;
    }

    fn push(&mut self, event: ReplayEvent) {
        if self.capacity == 0 {
            return;
        }
        if self.events.len() < self.capacity {
            self.events.push(event);
            if self.events.len() == self.capacity {
                self.full = true;
                self.next = 0;
            }
        } else {
            self.events[self.next] = event;
            self.next = (self.next + 1) % self.capacity;
        }
    }

    fn len(&self) -> usize {
        if self.capacity == 0 {
            0
        } else if self.full {
            self.capacity
        } else {
            self.events.len()
        }
    }

    fn snapshot_codes<F: Fn(&ReplayEvent) -> u32>(&self, out: &mut [u32], code_fn: F) -> usize {
        let len = self.len();
        if len == 0 {
            for slot in out.iter_mut() {
                *slot = 0;
            }
            return 0;
        }
        let cap = self.capacity;
        for (i, slot) in out.iter_mut().enumerate() {
            if i >= len {
                *slot = 0;
                continue;
            }
            let idx = if self.full { (self.next + i) % cap } else { i };
            *slot = code_fn(&self.events[idx]);
        }
        len
    }

    fn snapshot_events(&self) -> Vec<ReplayEvent> {
        let len = self.len();
        if len == 0 {
            return Vec::new();
        }
        let cap = self.capacity;
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            let idx = if self.full { (self.next + i) % cap } else { i };
            out.push(self.events[idx].clone());
        }
        out
    }
}

fn event_code(event: &ReplayEvent) -> u32 {
    use crate::events::Event;
    match event {
        Event::Draw { .. } => 1,
        Event::Damage { .. } => 2,
        Event::DamageCancel { .. } => 3,
        Event::DamageIntent { .. } => 4,
        Event::DamageModifierApplied { .. } => 5,
        Event::DamageModified { .. } => 6,
        Event::DamageCommitted { .. } => 7,
        Event::ReversalCommitted { .. } => 8,
        Event::Reveal { .. } => 9,
        Event::TriggerQueued { .. } => 10,
        Event::TriggerGrouped { .. } => 11,
        Event::TriggerResolved { .. } => 12,
        Event::TriggerCanceled { .. } => 13,
        Event::TimingWindowEntered { .. } => 14,
        Event::PriorityGranted { .. } => 15,
        Event::PriorityPassed { .. } => 16,
        Event::StackGroupPresented { .. } => 17,
        Event::StackOrderChosen { .. } => 18,
        Event::StackPushed { .. } => 19,
        Event::StackResolved { .. } => 20,
        Event::AutoResolveCapExceeded { .. } => 21,
        Event::WindowAdvanced { .. } => 22,
        Event::ChoicePresented { .. } => 23,
        Event::ChoicePageChanged { .. } => 24,
        Event::ChoiceMade { .. } => 25,
        Event::ChoiceAutopicked { .. } => 26,
        Event::ChoiceSkipped { .. } => 27,
        Event::ZoneMove { .. } => 28,
        Event::ControlChanged { .. } => 29,
        Event::ModifierAdded { .. } => 30,
        Event::ModifierRemoved { .. } => 31,
        Event::Concede { .. } => 32,
        Event::Play { .. } => 33,
        Event::PlayEvent { .. } => 34,
        Event::PlayClimax { .. } => 35,
        Event::Trigger { .. } => 36,
        Event::Attack { .. } => 37,
        Event::AttackType { .. } => 38,
        Event::Counter { .. } => 39,
        Event::Clock { .. } => 40,
        Event::Shuffle { .. } => 41,
        Event::Refresh { .. } => 42,
        Event::RefreshPenalty { .. } => 43,
        Event::LevelUpChoice { .. } => 44,
        Event::Encore { .. } => 45,
        Event::Stand { .. } => 46,
        Event::EndTurn { .. } => 47,
        Event::Terminal { .. } => 48,
    }
}

const MAX_CHOICE_OPTIONS: usize = crate::encode::CHOICE_COUNT;
pub const STACK_AUTO_RESOLVE_CAP: u32 = 256;
pub const CHECK_TIMING_QUIESCENCE_CAP: u32 = 256;
pub const HAND_LIMIT: usize = 7;

const TRIGGER_EFFECT_SOUL: u8 = 0;
const TRIGGER_EFFECT_DRAW: u8 = 1;
const TRIGGER_EFFECT_SHOT: u8 = 2;
const TRIGGER_EFFECT_GATE: u8 = 3;
const TRIGGER_EFFECT_BOUNCE: u8 = 4;
const TRIGGER_EFFECT_STANDBY: u8 = 5;
const TRIGGER_EFFECT_TREASURE_STOCK: u8 = 6;
const TRIGGER_EFFECT_TREASURE_MOVE: u8 = 7;

#[derive(Clone, Copy, Debug)]
struct TriggerCompileContext {
    source_card: CardId,
    standby_slot: Option<u8>,
    treasure_take_stock: Option<bool>,
}

mod interaction;
mod modifiers;
mod movement;
mod phases;
mod visibility;

impl GameEnv {
    fn validate_deck_lists(db: &CardDb, deck_lists: &[Vec<CardId>; 2]) {
        for (player, deck) in deck_lists.iter().enumerate() {
            assert!(
                deck.len() == crate::encode::MAX_DECK,
                "Deck {player} has {} cards (must be {})",
                deck.len(),
                crate::encode::MAX_DECK
            );
            let mut climax_count = 0usize;
            let mut counts: std::collections::HashMap<CardId, usize> =
                std::collections::HashMap::new();
            for &card_id in deck {
                let card = db
                    .get(card_id)
                    .unwrap_or_else(|| panic!("Deck {player} contains unknown card id {card_id}"));
                if card.card_type == CardType::Climax {
                    climax_count += 1;
                }
                *counts.entry(card_id).or_insert(0) += 1;
            }
            assert!(
                climax_count <= 8,
                "Deck {player} has {climax_count} climax cards (max 8)"
            );
            for (card_id, count) in counts {
                assert!(
                    count <= 4,
                    "Deck {player} has {count} copies of card {card_id} (max 4)"
                );
            }
        }
    }

    pub fn add_modifier(
        &mut self,
        source: CardId,
        target_player: u8,
        target_slot: u8,
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    ) -> Option<u32> {
        self.add_modifier_instance(
            source,
            None,
            target_player,
            target_slot,
            kind,
            magnitude,
            duration,
            crate::state::ModifierLayer::Effect,
        )
    }

    pub(crate) fn mark_rule_actions_dirty(&mut self) {
        self.rule_actions_dirty = true;
    }

    pub(crate) fn mark_continuous_modifiers_dirty(&mut self) {
        self.continuous_modifiers_dirty = true;
    }

    pub fn new(
        db: Arc<CardDb>,
        config: EnvConfig,
        curriculum: CurriculumConfig,
        seed: u64,
        replay_config: ReplayConfig,
        replay_writer: Option<ReplayWriter>,
        env_id: u32,
    ) -> Self {
        Self::validate_deck_lists(&db, &config.deck_lists);
        let starting_player = (seed as u8) & 1;
        let state = GameState::new(
            config.deck_lists[0].clone(),
            config.deck_lists[1].clone(),
            seed,
            starting_player,
        );
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
            episode_index: 0,
            decision: None,
            action_cache: ActionCache::new(),
            decision_id: 0,
            last_action_desc: None,
            last_action_player: None,
            last_illegal_action: false,
            last_engine_error: false,
            last_engine_error_code: EngineErrorCode::None,
            last_perspective: 0,
            pending_damage_delta: [0, 0],
            obs_buf: vec![0; OBS_LEN],
            slot_power_cache: [[0; crate::encode::MAX_STAGE]; 2],
            slot_power_dirty: [[true; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_card: [[0; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_mod_turn: [[0; crate::encode::MAX_STAGE]; 2],
            slot_power_cache_mod_battle: [[0; crate::encode::MAX_STAGE]; 2],
            rule_actions_dirty: true,
            continuous_modifiers_dirty: true,
            last_rule_action_phase: Phase::Stand,
            replay_config,
            replay_writer,
            replay_actions: Vec::new(),
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
        };
        env.reset();
        env
    }

    pub fn reset(&mut self) -> StepOutcome {
        self.reset_with_obs(true)
    }

    pub fn reset_no_copy(&mut self) -> StepOutcome {
        self.reset_with_obs(false)
    }

    pub fn canonical_events(&self) -> &[Event] {
        &self.canonical_events
    }

    pub fn decision_id(&self) -> u32 {
        self.decision_id
    }

    pub fn action_mask(&self) -> &[u8] {
        &self.action_cache.mask
    }

    pub fn action_lookup(&self) -> &[Option<ActionDesc>] {
        &self.action_cache.lookup
    }

    pub fn legal_actions(&self) -> &[ActionDesc] {
        &self.action_cache.legal_actions
    }

    pub fn debug_event_ring_codes(&self, viewer: u8, out: &mut [u32]) -> u16 {
        let Some(rings) = self.debug_event_ring.as_ref() else {
            for slot in out.iter_mut() {
                *slot = 0;
            }
            return 0;
        };
        let ring = &rings[viewer as usize % 2];
        let count = ring.snapshot_codes(out, event_code);
        count as u16
    }

    pub fn debug_event_ring_snapshot(&self, viewer: u8) -> Vec<ReplayEvent> {
        let Some(rings) = self.debug_event_ring.as_ref() else {
            return Vec::new();
        };
        rings[viewer as usize % 2].snapshot_events()
    }

    fn reset_with_obs(&mut self, copy_obs: bool) -> StepOutcome {
        let episode_seed = self.meta_rng.next_u64();
        let starting_player = if (episode_seed & 1) == 1 { 1 } else { 0 };
        self.episode_seed = episode_seed;
        self.episode_index = self.episode_index.wrapping_add(1);
        Self::validate_deck_lists(&self.db, &self.config.deck_lists);
        self.state = GameState::new(
            self.config.deck_lists[0].clone(),
            self.config.deck_lists[1].clone(),
            episode_seed,
            starting_player,
        );
        self.slot_power_cache = [[0; crate::encode::MAX_STAGE]; 2];
        self.slot_power_dirty = [[true; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_card = [[0; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_mod_turn = [[0; crate::encode::MAX_STAGE]; 2];
        self.slot_power_cache_mod_battle = [[0; crate::encode::MAX_STAGE]; 2];
        self.rule_actions_dirty = true;
        self.continuous_modifiers_dirty = true;
        self.last_rule_action_phase = self.state.turn.phase;
        self.decision = None;
        self.action_cache.clear();
        self.decision_id = 0;
        self.last_action_desc = None;
        self.last_action_player = None;
        self.last_illegal_action = false;
        self.last_engine_error = false;
        self.last_engine_error_code = EngineErrorCode::None;
        self.last_perspective = self.state.turn.starting_player;
        self.pending_damage_delta = [0, 0];
        if self.obs_buf.len() != OBS_LEN {
            self.obs_buf.resize(OBS_LEN, 0);
        }
        self.replay_actions.clear();
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
        self.recording = self.replay_config.enabled
            && self.meta_rng.next_u32() <= self.replay_config.sample_threshold;
        self.scratch_replacement_indices.clear();

        for player in 0..2 {
            self.shuffle_deck(player as u8);
            self.draw_to_hand(player as u8, 5);
        }

        self.advance_until_decision();
        self.update_action_cache();
        self.maybe_validate_state("reset");
        self.build_outcome_with_obs(0.0, copy_obs)
    }

    pub(crate) fn clear_status_flags(&mut self) {
        self.last_illegal_action = false;
        self.last_engine_error = false;
        self.last_engine_error_code = EngineErrorCode::None;
    }

    fn run_rule_actions_if_needed(&mut self) {
        if self.state.turn.phase != self.last_rule_action_phase {
            self.rule_actions_dirty = true;
            self.last_rule_action_phase = self.state.turn.phase;
        }
        if !self.rule_actions_dirty {
            return;
        }
        self.rule_actions_dirty = false;
        self.resolve_rule_actions_until_stable();
        self.rule_actions_dirty = false;
    }

    pub(super) fn set_decision(&mut self, decision: Decision) {
        self.decision = Some(decision);
        self.decision_id = self.decision_id.wrapping_add(1);
    }

    pub(super) fn clear_decision(&mut self) {
        self.decision = None;
    }

    pub fn set_debug_config(&mut self, debug: DebugConfig) {
        self.debug = debug;
        if debug.event_ring_capacity == 0 {
            self.debug_event_ring = None;
        } else {
            self.debug_event_ring = Some(std::array::from_fn(|_| {
                EventRing::new(debug.event_ring_capacity)
            }));
        }
    }

    pub fn apply_action_id(&mut self, action_id: usize) -> Result<StepOutcome> {
        self.apply_action_id_internal(action_id, true)
    }

    pub fn apply_action_id_no_copy(&mut self, action_id: usize) -> Result<StepOutcome> {
        self.apply_action_id_internal(action_id, false)
    }

    fn apply_action_id_internal(
        &mut self,
        action_id: usize,
        copy_obs: bool,
    ) -> Result<StepOutcome> {
        self.last_illegal_action = false;
        self.last_engine_error = false;
        self.last_engine_error_code = EngineErrorCode::None;
        if self.decision.is_none() {
            return Err(anyhow!("No pending decision"));
        }
        self.last_perspective = self.decision.as_ref().unwrap().player;
        let action = match self
            .action_cache
            .lookup
            .get(action_id)
            .and_then(|a| a.clone())
        {
            Some(action) => action,
            None => {
                let player = self.decision.as_ref().unwrap().player;
                return self.handle_illegal_action(player, "Invalid action id", copy_obs);
            }
        };
        self.apply_action_internal(action, copy_obs)
    }

    pub fn apply_action(&mut self, action: ActionDesc) -> Result<StepOutcome> {
        self.apply_action_internal(action, true)
    }

    fn apply_action_internal(&mut self, action: ActionDesc, copy_obs: bool) -> Result<StepOutcome> {
        let acting_player = self
            .decision
            .as_ref()
            .map(|d| d.player)
            .unwrap_or(self.last_perspective);
        self.last_perspective = acting_player;
        self.pending_damage_delta = [0, 0];
        let decision_kind = self
            .decision
            .as_ref()
            .map(|d| d.kind)
            .unwrap_or(DecisionKind::Main);
        let action_clone = action.clone();
        if self.should_validate_state() {
            if let Some(decision) = &self.decision {
                let legal = crate::legal::legal_actions_cached(
                    &self.state,
                    decision,
                    &self.db,
                    &self.curriculum,
                    self.curriculum.allowed_card_sets_cache.as_ref(),
                );
                if !legal.contains(&action_clone) {
                    return self.handle_illegal_action(
                        decision.player,
                        "Action not in legal set",
                        copy_obs,
                    );
                }
            }
        }
        let outcome = match self.apply_action_impl(action, copy_obs) {
            Ok(outcome) => Ok(outcome),
            Err(err) => match self.config.error_policy {
                ErrorPolicy::Strict => Err(err),
                ErrorPolicy::LenientTerminate => {
                    self.last_engine_error = true;
                    self.last_engine_error_code = EngineErrorCode::ActionError;
                    self.last_perspective = acting_player;
                    self.state.terminal = Some(TerminalResult::Win {
                        winner: 1 - acting_player,
                    });
                    self.decision = None;
                    self.update_action_cache();
                    Ok(self
                        .build_outcome_with_obs(self.terminal_reward_for(acting_player), copy_obs))
                }
                ErrorPolicy::LenientNoop => {
                    self.last_engine_error = true;
                    self.last_engine_error_code = EngineErrorCode::ActionError;
                    self.last_perspective = acting_player;
                    self.update_action_cache();
                    Ok(self.build_outcome_with_obs(0.0, copy_obs))
                }
            },
        }?;
        if self.recording || self.should_validate_state() {
            self.log_action(acting_player, action_clone);
            self.replay_steps.push(StepMeta {
                actor: acting_player,
                decision_kind,
                illegal_action: self.last_illegal_action,
                engine_error: self.last_engine_error,
            });
        }
        Ok(outcome)
    }

    fn apply_action_impl(&mut self, action: ActionDesc, copy_obs: bool) -> Result<StepOutcome> {
        let decision = self
            .decision
            .clone()
            .ok_or_else(|| anyhow!("No decision to apply"))?;
        self.last_perspective = decision.player;
        self.last_action_desc = Some(action.clone());
        self.last_action_player = Some(decision.player);

        let mut reward = 0.0f32;

        if action == ActionDesc::Concede {
            self.log_event(Event::Concede {
                player: decision.player,
            });
            self.state.terminal = Some(TerminalResult::Win {
                winner: 1 - decision.player,
            });
            self.decision = None;
            self.state.turn.decision_count += 1;
            self.update_action_cache();
            self.maybe_validate_state("post_concede");
            reward += self.compute_reward(decision.player, &self.pending_damage_delta);
            return Ok(self.build_outcome_with_obs(reward, copy_obs));
        }

        match decision.kind {
            DecisionKind::Mulligan => match action {
                ActionDesc::MulliganSelect { hand_index } => {
                    let p = decision.player as usize;
                    let hi = hand_index as usize;
                    if hi >= self.state.players[p].hand.len() {
                        return self.handle_illegal_action(
                            decision.player,
                            "Mulligan hand index out of range",
                            copy_obs,
                        );
                    }
                    if hi >= crate::encode::MAX_HAND {
                        return self.handle_illegal_action(
                            decision.player,
                            "Mulligan hand index exceeds encoding",
                            copy_obs,
                        );
                    }
                    let bit = 1u64 << hi;
                    let current = &mut self.state.turn.mulligan_selected[p];
                    if *current & bit != 0 {
                        *current &= !bit;
                    } else {
                        *current |= bit;
                    }
                }
                ActionDesc::MulliganConfirm => {
                    let p = decision.player as usize;
                    let hand_len = self.state.players[p].hand.len();
                    let mut indices: Vec<usize> = Vec::new();
                    let mask = self.state.turn.mulligan_selected[p];
                    for idx in 0..hand_len.min(crate::encode::MAX_HAND) {
                        if mask & (1u64 << idx) != 0 {
                            indices.push(idx);
                        }
                    }
                    indices.sort_by(|a, b| b.cmp(a));
                    for idx in indices.iter().copied() {
                        if idx >= self.state.players[p].hand.len() {
                            continue;
                        }
                        let card = self.state.players[p].hand.remove(idx);
                        let from_slot = if idx <= u8::MAX as usize {
                            Some(idx as u8)
                        } else {
                            None
                        };
                        self.move_card_between_zones(
                            p as u8,
                            card,
                            Zone::Hand,
                            Zone::WaitingRoom,
                            from_slot,
                            None,
                        );
                    }
                    let draw_count = indices.len();
                    if draw_count > 0 {
                        self.draw_to_hand(p as u8, draw_count);
                    }
                    self.state.turn.mulligan_done[p] = true;
                    self.state.turn.mulligan_selected[p] = 0;
                }
                _ => {
                    return self.handle_illegal_action(
                        decision.player,
                        "Invalid mulligan action",
                        copy_obs,
                    )
                }
            },
            DecisionKind::Clock => {
                match action {
                    ActionDesc::Pass => {
                        self.log_event(Event::Clock {
                            player: decision.player,
                            card: None,
                        });
                    }
                    ActionDesc::Clock { hand_index } => {
                        let p = decision.player as usize;
                        let hi = hand_index as usize;
                        if hi >= self.state.players[p].hand.len() {
                            return self.handle_illegal_action(
                                decision.player,
                                "Clock hand index out of range",
                                copy_obs,
                            );
                        }
                        let card = self.state.players[p].hand.remove(hi);
                        let card_id = card.id;
                        self.move_card_between_zones(
                            decision.player,
                            card,
                            Zone::Hand,
                            Zone::Clock,
                            Some(hand_index),
                            None,
                        );
                        self.log_event(Event::Clock {
                            player: decision.player,
                            card: Some(card_id),
                        });
                        self.draw_to_hand(decision.player, 2);
                        self.check_level_up(decision.player);
                    }
                    _ => {
                        return self.handle_illegal_action(
                            decision.player,
                            "Invalid clock action",
                            copy_obs,
                        )
                    }
                }
                self.state.turn.phase_step = 2;
            }
            DecisionKind::Main => match action {
                ActionDesc::Pass => {
                    if self.curriculum.enable_priority_windows {
                        self.state.turn.main_passed = true;
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(TimingWindow::MainWindow, decision.player);
                        }
                    } else {
                        self.state.turn.main_passed = false;
                        self.state.turn.phase = Phase::Climax;
                        self.state.turn.phase_step = 0;
                    }
                }
                ActionDesc::MainPlayCharacter {
                    hand_index,
                    stage_slot,
                } => {
                    if let Err(err) = self.play_character(decision.player, hand_index, stage_slot) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                ActionDesc::MainPlayEvent { hand_index } => {
                    if let Err(err) = self.play_event(decision.player, hand_index) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                ActionDesc::MainMove { from_slot, to_slot } => {
                    let p = decision.player as usize;
                    let fs = from_slot as usize;
                    let ts = to_slot as usize;
                    if fs >= self.state.players[p].stage.len()
                        || ts >= self.state.players[p].stage.len()
                        || fs == ts
                    {
                        return self.handle_illegal_action(
                            decision.player,
                            "Invalid move slots",
                            copy_obs,
                        );
                    }
                    if self.state.players[p].stage[fs].card.is_none() {
                        return self.handle_illegal_action(
                            decision.player,
                            "Move requires a source slot with a card",
                            copy_obs,
                        );
                    }
                    self.state.players[p].stage.swap(fs, ts);
                    self.remove_modifiers_for_slot(decision.player, from_slot);
                    self.remove_modifiers_for_slot(decision.player, to_slot);
                    self.mark_slot_power_dirty(decision.player, from_slot);
                    self.mark_slot_power_dirty(decision.player, to_slot);
                    self.mark_rule_actions_dirty();
                    self.mark_continuous_modifiers_dirty();
                }
                ActionDesc::MainActivateAbility {
                    slot,
                    ability_index,
                } => {
                    let _ = (slot, ability_index);
                    return self.handle_illegal_action(
                        decision.player,
                        "Activated abilities only via priority window",
                        copy_obs,
                    );
                }
                _ => {
                    return self.handle_illegal_action(
                        decision.player,
                        "Invalid main action",
                        copy_obs,
                    )
                }
            },
            DecisionKind::Climax => match action {
                ActionDesc::Pass => {
                    self.state.turn.phase_step = 2;
                    if self.curriculum.enable_priority_windows {
                        self.enter_timing_window(TimingWindow::ClimaxWindow, decision.player);
                    }
                }
                ActionDesc::ClimaxPlay { hand_index } => {
                    if let Err(err) = self.play_climax(decision.player, hand_index) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                    self.state.turn.phase_step = 2;
                    if self.curriculum.enable_priority_windows {
                        self.enter_timing_window(TimingWindow::ClimaxWindow, decision.player);
                    }
                }
                _ => {
                    return self.handle_illegal_action(
                        decision.player,
                        "Invalid climax action",
                        copy_obs,
                    )
                }
            },
            DecisionKind::AttackDeclaration => match action {
                ActionDesc::Pass => {
                    if self.curriculum.enable_encore {
                        self.queue_encore_requests();
                    } else {
                        self.cleanup_reversed_to_waiting_room();
                    }
                    self.state.turn.phase = Phase::End;
                    self.state.turn.phase_step = 0;
                    self.state.turn.attack_phase_begin_done = false;
                    self.state.turn.attack_decl_check_done = false;
                }
                ActionDesc::Attack { slot, attack_type } => {
                    if let Err(err) = self.declare_attack(decision.player, slot, attack_type) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                _ => {
                    return self.handle_illegal_action(
                        decision.player,
                        "Invalid attack action",
                        copy_obs,
                    )
                }
            },
            DecisionKind::LevelUp => match action {
                ActionDesc::LevelUp { index } => {
                    if self.state.turn.pending_level_up != Some(decision.player) {
                        return self.handle_illegal_action(
                            decision.player,
                            "No pending level up",
                            copy_obs,
                        );
                    }
                    if let Err(err) = self.resolve_level_up(decision.player, index) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                _ => {
                    return self.handle_illegal_action(
                        decision.player,
                        "Invalid level up action",
                        copy_obs,
                    )
                }
            },
            DecisionKind::Encore => match action {
                ActionDesc::EncorePay { slot } => {
                    if let Err(err) = self.resolve_encore(decision.player, slot, true) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                ActionDesc::EncoreDecline { slot } => {
                    if let Err(err) = self.resolve_encore(decision.player, slot, false) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                _ => {
                    return self.handle_illegal_action(
                        decision.player,
                        "Invalid encore action",
                        copy_obs,
                    )
                }
            },
            DecisionKind::TriggerOrder => {
                let Some(order) = self.state.turn.trigger_order.clone() else {
                    return self.handle_illegal_action(
                        decision.player,
                        "No trigger order pending",
                        copy_obs,
                    );
                };
                if order.player != decision.player {
                    return self.handle_illegal_action(
                        decision.player,
                        "Trigger order player mismatch",
                        copy_obs,
                    );
                }
                match action {
                    ActionDesc::TriggerOrder { index } => {
                        let idx = index as usize;
                        if idx >= order.choices.len() {
                            return self.handle_illegal_action(
                                decision.player,
                                "Trigger order index out of range",
                                copy_obs,
                            );
                        }
                        let trigger_id = order.choices[idx];
                        let trigger_index = self
                            .state
                            .turn
                            .pending_triggers
                            .iter()
                            .position(|t| t.id == trigger_id);
                        let Some(trigger_index) = trigger_index else {
                            return self.handle_illegal_action(
                                decision.player,
                                "Trigger already resolved",
                                copy_obs,
                            );
                        };
                        let trigger = self.state.turn.pending_triggers.remove(trigger_index);
                        let _ = self.resolve_trigger(trigger);
                        self.state.turn.trigger_order = None;
                    }
                    _ => {
                        return self.handle_illegal_action(
                            decision.player,
                            "Invalid trigger order action",
                            copy_obs,
                        )
                    }
                }
            }
            DecisionKind::Choice => {
                let Some(choice_ref) = self.state.turn.choice.as_ref() else {
                    return self.handle_illegal_action(
                        decision.player,
                        "No choice pending",
                        copy_obs,
                    );
                };
                if choice_ref.player != decision.player {
                    return self.handle_illegal_action(
                        decision.player,
                        "Choice player mismatch",
                        copy_obs,
                    );
                }
                match action {
                    ActionDesc::ChoiceSelect { index } => {
                        let Some(choice) = self.state.turn.choice.take() else {
                            return self.handle_illegal_action(
                                decision.player,
                                "No choice pending",
                                copy_obs,
                            );
                        };
                        let idx = index as usize;
                        if idx >= MAX_CHOICE_OPTIONS {
                            return self.handle_illegal_action(
                                decision.player,
                                "Choice index out of range",
                                copy_obs,
                            );
                        }
                        let total = choice.total_candidates as usize;
                        let page_start = choice.page_start as usize;
                        let global_idx = page_start + idx;
                        if global_idx >= total {
                            return self.handle_illegal_action(
                                decision.player,
                                "Choice index out of range",
                                copy_obs,
                            );
                        }
                        let Some(option) = choice.options.get(global_idx).copied() else {
                            return self.handle_illegal_action(
                                decision.player,
                                "Choice option missing",
                                copy_obs,
                            );
                        };
                        if self.recording {
                            self.log_event(Event::ChoiceMade {
                                choice_id: choice.id,
                                player: decision.player,
                                reason: choice.reason,
                                option,
                            });
                        }
                        self.recycle_choice_options(choice.options);
                        self.apply_choice_effect(
                            choice.reason,
                            choice.player,
                            option,
                            choice.pending_trigger,
                        );
                    }
                    ActionDesc::ChoicePrevPage | ActionDesc::ChoiceNextPage => {
                        let nav = {
                            let Some(choice) = self.state.turn.choice.as_mut() else {
                                return self.handle_illegal_action(
                                    decision.player,
                                    "No choice pending",
                                    copy_obs,
                                );
                            };
                            let total = choice.total_candidates as usize;
                            let page_size = MAX_CHOICE_OPTIONS;
                            let current = choice.page_start as usize;
                            let new_start = match action {
                                ActionDesc::ChoicePrevPage => {
                                    if current < page_size {
                                        None
                                    } else {
                                        Some(current - page_size)
                                    }
                                }
                                ActionDesc::ChoiceNextPage => {
                                    if current + page_size >= total {
                                        None
                                    } else {
                                        Some(current + page_size)
                                    }
                                }
                                _ => None,
                            };
                            if let Some(new_start) = new_start {
                                let from_start = choice.page_start;
                                choice.page_start = new_start as u16;
                                Some((choice.id, choice.player, from_start, choice.page_start))
                            } else {
                                None
                            }
                        };
                        let Some((choice_id, player, from_start, to_start)) = nav else {
                            return self.handle_illegal_action(
                                decision.player,
                                "Choice page out of range",
                                copy_obs,
                            );
                        };
                        if self.recording {
                            self.log_event(Event::ChoicePageChanged {
                                choice_id,
                                player,
                                from_start,
                                to_start,
                            });
                        }
                    }
                    _ => {
                        return self.handle_illegal_action(
                            decision.player,
                            "Invalid choice action",
                            copy_obs,
                        )
                    }
                }
            }
        }

        self.decision = None;
        self.state.turn.decision_count += 1;
        if self.state.turn.decision_count >= self.config.max_decisions {
            self.state.terminal = Some(TerminalResult::Timeout);
        }

        self.advance_until_decision();
        self.update_action_cache();
        self.maybe_validate_state("post_action");

        reward += self.compute_reward(decision.player, &self.pending_damage_delta);
        Ok(self.build_outcome_with_obs(reward, copy_obs))
    }

    fn compute_reward(&self, perspective: u8, damage_delta: &[i32; 2]) -> f32 {
        let RewardConfig {
            terminal_win,
            terminal_loss,
            terminal_draw,
            enable_shaping,
            damage_reward,
        } = &self.config.reward;
        if let Some(term) = self.state.terminal {
            return match term {
                TerminalResult::Win { winner } => {
                    if winner == perspective {
                        *terminal_win
                    } else {
                        *terminal_loss
                    }
                }
                TerminalResult::Draw | TerminalResult::Timeout => *terminal_draw,
            };
        }
        if *enable_shaping {
            let mut reward = 0.0;
            let p = perspective as usize;
            let opp = 1 - p;
            reward += *damage_reward * damage_delta[opp] as f32;
            reward -= *damage_reward * damage_delta[p] as f32;
            return reward;
        }
        0.0
    }

    fn resolve_quiescence_until_decision(&mut self) {
        let mut auto_resolve_steps: u32 = 0;
        loop {
            if self.state.terminal.is_some() || self.decision.is_some() {
                return;
            }
            self.run_rule_actions_if_needed();
            self.refresh_continuous_modifiers_if_needed();
            if let Some(player) = self.state.turn.pending_level_up {
                self.set_decision(Decision {
                    player,
                    kind: DecisionKind::LevelUp,
                    focus_slot: None,
                });
                return;
            }
            if self.handle_trigger_pipeline() {
                if self.decision.is_some() {
                    return;
                }
                continue;
            }
            if self.handle_priority_window() {
                if self.decision.is_some() {
                    return;
                }
                continue;
            }
            if !self.curriculum.enable_priority_windows
                && self.state.turn.priority.is_none()
                && self.state.turn.choice.is_none()
                && self.state.turn.stack_order.is_none()
                && !self.state.turn.stack.is_empty()
            {
                auto_resolve_steps = auto_resolve_steps.saturating_add(1);
                if auto_resolve_steps > CHECK_TIMING_QUIESCENCE_CAP {
                    self.log_event(Event::AutoResolveCapExceeded {
                        cap: CHECK_TIMING_QUIESCENCE_CAP,
                        stack_len: self.state.turn.stack.len() as u32,
                        window: self.state.turn.active_window,
                    });
                    self.last_engine_error = true;
                    self.last_engine_error_code = EngineErrorCode::TriggerQuiescenceCap;
                    self.state.terminal = Some(TerminalResult::Timeout);
                    return;
                }
                if let Some(item) = self.state.turn.stack.pop() {
                    self.resolve_stack_item(&item);
                    self.log_event(Event::StackResolved { item });
                    continue;
                }
            }
            break;
        }
    }

    pub(crate) fn update_action_cache(&mut self) {
        if self.decision.is_some() {
            let decision_kind = self
                .decision
                .as_ref()
                .map(|d| d.kind)
                .expect("decision kind");
            if decision_kind == DecisionKind::AttackDeclaration
                && self.state.turn.derived_attack.is_none()
            {
                self.recompute_derived_attack();
            }
            let decision = self.decision.as_ref().expect("decision present");
            self.last_perspective = decision.player;
            self.action_cache.update(
                &self.state,
                decision,
                self.decision_id,
                &self.db,
                &self.curriculum,
                self.curriculum.allowed_card_sets_cache.as_ref(),
            );
        } else {
            self.action_cache.clear();
        }
    }

    fn should_validate_state(&self) -> bool {
        if cfg!(debug_assertions) {
            return true;
        }
        std::env::var("WEISS_VALIDATE_STATE").ok().as_deref() == Some("1")
    }

    fn maybe_validate_state(&self, context: &str) {
        if !self.should_validate_state() {
            return;
        }
        if let Err(err) = self.validate_state() {
            panic!("validate_state failed at {context}: {err}");
        }
    }

    pub fn validate_state(&self) -> Result<()> {
        use std::collections::{HashMap, HashSet};
        let mut errors = Vec::new();

        let mut counts: [HashMap<CardId, i32>; 2] = [HashMap::new(), HashMap::new()];
        for (owner, owner_counts) in counts.iter_mut().enumerate() {
            let deck_list = &self.config.deck_lists[owner];
            for card in deck_list.iter().copied() {
                *owner_counts.entry(card).or_insert(0) += 1;
            }
        }

        fn consume(
            counts: &mut [HashMap<CardId, i32>; 2],
            errors: &mut Vec<String>,
            owner: u8,
            card: CardId,
            zone: &str,
        ) {
            let owner_idx = owner as usize;
            let entry = counts[owner_idx].entry(card).or_insert(0);
            *entry -= 1;
            if *entry < 0 {
                errors.push(format!("Owner {owner} has extra card {card} in {zone}"));
            }
        }

        let mut instance_ids: HashSet<CardInstanceId> = HashSet::new();
        fn check_instance(
            instance_ids: &mut HashSet<CardInstanceId>,
            errors: &mut Vec<String>,
            card: &CardInstance,
            zone: &str,
        ) {
            if card.instance_id == 0 {
                errors.push(format!("Card instance id 0 in {zone}"));
                return;
            }
            if !instance_ids.insert(card.instance_id) {
                errors.push(format!(
                    "Duplicate instance id {} in {zone}",
                    card.instance_id
                ));
            }
        }

        for zone_player in 0..2 {
            let p = &self.state.players[zone_player];
            for card in &p.deck {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} deck"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} deck"),
                );
            }
            for card in &p.hand {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} hand"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} hand"),
                );
            }
            for card in &p.waiting_room {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} waiting_room"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} waiting_room"),
                );
            }
            for card in &p.clock {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} clock"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} clock"),
                );
            }
            for card in &p.level {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} level"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} level"),
                );
            }
            for card in &p.stock {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} stock"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} stock"),
                );
            }
            for card in &p.memory {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} memory"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} memory"),
                );
            }
            for card in &p.climax {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} climax"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} climax"),
                );
            }
            for card in &p.resolution {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} resolution"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} resolution"),
                );
            }
            for (slot_idx, slot) in p.stage.iter().enumerate() {
                if let Some(card) = slot.card {
                    consume(
                        &mut counts,
                        &mut errors,
                        card.owner,
                        card.id,
                        &format!("p{zone_player} stage[{slot_idx}]"),
                    );
                    check_instance(
                        &mut instance_ids,
                        &mut errors,
                        &card,
                        &format!("p{zone_player} stage[{slot_idx}]"),
                    );
                }
            }
        }

        for (owner, owner_counts) in counts.iter().enumerate() {
            for (card, remaining) in owner_counts.iter() {
                if *remaining != 0 {
                    errors.push(format!(
                        "Owner {owner} card {card} count mismatch ({remaining})"
                    ));
                }
            }
        }

        if let Some(decision) = &self.decision {
            if let Some(slot) = decision.focus_slot {
                if slot as usize >= self.state.players[decision.player as usize].stage.len() {
                    errors.push("Decision focus slot out of range".to_string());
                }
            }
            match decision.kind {
                DecisionKind::AttackDeclaration => {
                    if self.state.turn.attack.is_some() {
                        errors.push("Attack declaration while attack context active".to_string());
                    }
                }
                DecisionKind::LevelUp => {
                    if self.state.turn.pending_level_up.is_none() {
                        errors.push("Level up decision without pending level".to_string());
                    }
                }
                DecisionKind::Encore => {
                    let has = self
                        .state
                        .turn
                        .encore_queue
                        .iter()
                        .any(|r| r.player == decision.player);
                    if !has {
                        errors.push("Encore decision without reversed options".to_string());
                    }
                }
                DecisionKind::TriggerOrder => {
                    if self.state.turn.trigger_order.is_none() {
                        errors.push("Trigger order decision without pending order".to_string());
                    }
                }
                DecisionKind::Choice => {
                    if let Some(choice) = &self.state.turn.choice {
                        if choice.player != decision.player {
                            errors.push("Choice decision player mismatch".to_string());
                        }
                    } else {
                        errors.push("Choice decision without pending choice".to_string());
                    }
                }
                _ => {}
            }
        }

        if self.state.turn.attack.is_some() && self.state.turn.phase != Phase::Attack {
            errors.push("Attack context outside Attack phase".to_string());
        }

        if errors.is_empty() {
            return Ok(());
        }

        let state_hash = crate::fingerprint::state_fingerprint(&self.state);
        let phase = self.state.turn.phase;
        let attack_step = self.state.turn.attack.as_ref().map(|c| c.step);
        let tail_len = 8usize;
        let actions_tail: Vec<String> = self
            .replay_actions
            .iter()
            .rev()
            .take(tail_len)
            .rev()
            .map(|a| format!("{a:?}"))
            .collect();
        let decisions_tail: Vec<String> = self
            .replay_steps
            .iter()
            .rev()
            .take(tail_len)
            .rev()
            .map(|s| format!("{:?}/{:?}", s.decision_kind, s.actor))
            .collect();
        let fallback_action = self
            .last_action_desc
            .as_ref()
            .map(|a| format!("{a:?}"))
            .unwrap_or_else(|| "None".to_string());
        let payload = format!(
            "seed={}\nphase={:?}\nattack_step={:?}\nlast_action={}\nactions_tail={:?}\ndecisions_tail={:?}\nstate_hash={}",
            self.episode_seed,
            phase,
            attack_step,
            fallback_action,
            actions_tail,
            decisions_tail,
            state_hash,
        );
        Err(anyhow!("{}\n{}", payload, errors.join("; ")))
    }

    pub(crate) fn build_outcome_no_copy(&mut self, reward: f32) -> StepOutcome {
        self.build_outcome_with_obs(reward, false)
    }

    fn build_outcome_with_obs(&mut self, reward: f32, copy_obs: bool) -> StepOutcome {
        let perspective = self
            .decision
            .as_ref()
            .map(|d| d.player)
            .unwrap_or(self.last_perspective);
        self.refresh_slot_power_cache();
        encode_observation_with_slot_power(
            &self.state,
            &self.db,
            &self.curriculum,
            perspective,
            self.decision.as_ref(),
            self.last_action_desc.as_ref(),
            self.last_action_player,
            self.config.observation_visibility,
            &self.slot_power_cache,
            &mut self.obs_buf,
        );
        let obs = if copy_obs {
            self.obs_buf.clone()
        } else {
            Vec::new()
        };
        let info = EnvInfo {
            obs_version: OBS_ENCODING_VERSION,
            action_version: ACTION_ENCODING_VERSION,
            decision_kind: self
                .decision
                .as_ref()
                .map(|d| match d.kind {
                    DecisionKind::Mulligan => 0,
                    DecisionKind::Clock => 1,
                    DecisionKind::Main => 2,
                    DecisionKind::Climax => 3,
                    DecisionKind::AttackDeclaration => 4,
                    DecisionKind::LevelUp => 5,
                    DecisionKind::Encore => 6,
                    DecisionKind::TriggerOrder => 7,
                    DecisionKind::Choice => 8,
                })
                .unwrap_or(-1),
            current_player: self.decision.as_ref().map(|d| d.player as i8).unwrap_or(-1),
            actor: self.last_perspective as i8,
            decision_count: self.state.turn.decision_count,
            tick_count: self.state.turn.tick_count,
            terminal: self.state.terminal,
            illegal_action: self.last_illegal_action,
            engine_error: self.last_engine_error,
            engine_error_code: self.last_engine_error_code as u8,
        };
        let truncated = matches!(self.state.terminal, Some(TerminalResult::Timeout));
        let terminated = matches!(
            self.state.terminal,
            Some(TerminalResult::Win { .. } | TerminalResult::Draw)
        );
        StepOutcome {
            obs,
            reward,
            terminated,
            truncated,
            info,
        }
    }

    pub(crate) fn advance_until_decision(&mut self) {
        let mut auto_resolve_steps: u32 = 0;
        loop {
            if self.state.terminal.is_some() {
                break;
            }
            self.resolve_pending_losses();
            self.run_rule_actions_if_needed();
            self.refresh_continuous_modifiers_if_needed();
            if self.decision.is_some() {
                break;
            }
            if self.state.turn.tick_count >= self.config.max_ticks {
                self.state.terminal = Some(TerminalResult::Timeout);
                break;
            }
            self.state.turn.tick_count += 1;

            if let Some(player) = self.state.turn.pending_level_up {
                self.set_decision(Decision {
                    player,
                    kind: DecisionKind::LevelUp,
                    focus_slot: None,
                });
                break;
            }

            if self.handle_trigger_pipeline() {
                if self.decision.is_some() {
                    break;
                }
                continue;
            }

            if self.handle_priority_window() {
                if self.decision.is_some() {
                    break;
                }
                continue;
            }
            if !self.curriculum.enable_priority_windows
                && self.state.turn.priority.is_none()
                && self.state.turn.choice.is_none()
                && self.state.turn.stack_order.is_none()
                && !self.state.turn.stack.is_empty()
            {
                auto_resolve_steps = auto_resolve_steps.saturating_add(1);
                if auto_resolve_steps > STACK_AUTO_RESOLVE_CAP {
                    self.log_event(Event::AutoResolveCapExceeded {
                        cap: STACK_AUTO_RESOLVE_CAP,
                        stack_len: self.state.turn.stack.len() as u32,
                        window: self.state.turn.active_window,
                    });
                    self.last_engine_error = true;
                    self.last_engine_error_code = EngineErrorCode::StackAutoResolveCap;
                    self.state.terminal = Some(TerminalResult::Timeout);
                    break;
                }
                if let Some(item) = self.state.turn.stack.pop() {
                    self.resolve_stack_item(&item);
                    self.log_event(Event::StackResolved { item });
                    continue;
                }
            }

            if self.state.turn.stack.is_empty()
                && self.state.turn.pending_triggers.is_empty()
                && self.state.turn.choice.is_none()
                && self.state.turn.priority.is_none()
                && self.state.turn.stack_order.is_none()
            {
                self.cleanup_pending_resolution_cards();
            }

            if !self.state.turn.encore_queue.is_empty() {
                if !self.state.turn.encore_begin_done {
                    self.run_check_timing(crate::db::AbilityTiming::BeginEncoreStep);
                    self.state.turn.encore_begin_done = true;
                    continue;
                }
                if self.curriculum.enable_priority_windows && !self.state.turn.encore_window_done {
                    self.state.turn.encore_window_done = true;
                    if self.state.turn.priority.is_none() {
                        self.enter_timing_window(
                            TimingWindow::EncoreWindow,
                            self.state.turn.active_player,
                        );
                    }
                    break;
                }
                if self.state.turn.encore_step_player.is_none() {
                    self.state.turn.encore_step_player = Some(self.state.turn.active_player);
                }
                let current = self.state.turn.encore_step_player.unwrap();
                let has_current = self
                    .state
                    .turn
                    .encore_queue
                    .iter()
                    .any(|r| r.player == current);
                let next_player = if has_current {
                    Some(current)
                } else {
                    let other = 1 - current;
                    if self
                        .state
                        .turn
                        .encore_queue
                        .iter()
                        .any(|r| r.player == other)
                    {
                        self.state.turn.encore_step_player = Some(other);
                        Some(other)
                    } else {
                        self.state.turn.encore_step_player = None;
                        None
                    }
                };
                if let Some(player) = next_player {
                    self.set_decision(Decision {
                        player,
                        kind: DecisionKind::Encore,
                        focus_slot: None,
                    });
                    break;
                }
            }

            match self.state.turn.phase {
                Phase::Mulligan => {
                    if self.state.turn.mulligan_done[0] && self.state.turn.mulligan_done[1] {
                        self.state.turn.phase = Phase::Stand;
                        self.state.turn.phase_step = 0;
                        self.state.turn.active_player = self.state.turn.starting_player;
                        continue;
                    }
                    let sp = self.state.turn.starting_player as usize;
                    let next = if !self.state.turn.mulligan_done[sp] {
                        sp
                    } else {
                        1 - sp
                    };
                    self.set_decision(Decision {
                        player: next as u8,
                        kind: DecisionKind::Mulligan,
                        focus_slot: None,
                    });
                    break;
                }
                Phase::Stand => {
                    let p = self.state.turn.active_player;
                    match self.state.turn.phase_step {
                        0 => {
                            self.run_check_timing(crate::db::AbilityTiming::BeginTurn);
                            if self.state.turn.pending_level_up.is_some()
                                || !self.state.turn.pending_triggers.is_empty()
                            {
                                continue;
                            }
                            self.run_check_timing(crate::db::AbilityTiming::BeginStandPhase);
                            self.state.turn.phase_step = 1;
                            continue;
                        }
                        1 => {
                            self.resolve_stand_phase(p);
                            self.state.turn.phase_step = 2;
                            continue;
                        }
                        2 => {
                            self.run_check_timing(crate::db::AbilityTiming::AfterStandPhase);
                            self.state.turn.phase_step = 3;
                            continue;
                        }
                        _ => {
                            if self.state.turn.pending_level_up.is_some()
                                || !self.state.turn.pending_triggers.is_empty()
                            {
                                continue;
                            }
                            self.state.turn.phase = Phase::Draw;
                            self.state.turn.phase_step = 0;
                            continue;
                        }
                    }
                }
                Phase::Draw => {
                    let p = self.state.turn.active_player;
                    match self.state.turn.phase_step {
                        0 => {
                            self.run_check_timing(crate::db::AbilityTiming::BeginDrawPhase);
                            self.state.turn.phase_step = 1;
                            continue;
                        }
                        1 => {
                            self.draw_to_hand(p, 1);
                            self.state.turn.phase_step = 2;
                            continue;
                        }
                        2 => {
                            self.run_check_timing(crate::db::AbilityTiming::AfterDrawPhase);
                            self.state.turn.phase_step = 3;
                            continue;
                        }
                        _ => {
                            if self.state.turn.pending_level_up.is_some()
                                || !self.state.turn.pending_triggers.is_empty()
                            {
                                continue;
                            }
                            self.state.turn.phase = if self.curriculum.enable_clock_phase {
                                Phase::Clock
                            } else {
                                Phase::Main
                            };
                            self.state.turn.phase_step = 0;
                            continue;
                        }
                    }
                }
                Phase::Clock => {
                    if !self.curriculum.enable_clock_phase {
                        self.state.turn.phase = Phase::Main;
                        self.state.turn.phase_step = 0;
                        continue;
                    }
                    let p = self.state.turn.active_player;
                    match self.state.turn.phase_step {
                        0 => {
                            self.run_check_timing(crate::db::AbilityTiming::BeginClockPhase);
                            self.state.turn.phase_step = 1;
                            continue;
                        }
                        1 => {
                            self.set_decision(Decision {
                                player: p,
                                kind: DecisionKind::Clock,
                                focus_slot: None,
                            });
                            break;
                        }
                        2 => {
                            self.run_check_timing(crate::db::AbilityTiming::AfterClockPhase);
                            self.state.turn.phase_step = 3;
                            continue;
                        }
                        _ => {
                            if self.state.turn.pending_level_up.is_some()
                                || !self.state.turn.pending_triggers.is_empty()
                            {
                                continue;
                            }
                            self.state.turn.phase = Phase::Main;
                            self.state.turn.phase_step = 0;
                            continue;
                        }
                    }
                }
                Phase::Main => {
                    let p = self.state.turn.active_player;
                    if self.state.turn.phase_step == 0 {
                        self.run_check_timing(crate::db::AbilityTiming::BeginMainPhase);
                        self.state.turn.phase_step = 1;
                        continue;
                    }
                    self.set_decision(Decision {
                        player: p,
                        kind: DecisionKind::Main,
                        focus_slot: None,
                    });
                    break;
                }
                Phase::Climax => {
                    if !self.curriculum.enable_climax_phase {
                        self.state.turn.phase = Phase::Attack;
                        self.state.turn.phase_step = 0;
                        self.state.turn.attack_phase_begin_done = false;
                        self.state.turn.attack_decl_check_done = false;
                        continue;
                    }
                    let p = self.state.turn.active_player;
                    match self.state.turn.phase_step {
                        0 => {
                            self.run_check_timing(crate::db::AbilityTiming::BeginClimaxPhase);
                            self.state.turn.phase_step = 1;
                            continue;
                        }
                        1 => {
                            self.set_decision(Decision {
                                player: p,
                                kind: DecisionKind::Climax,
                                focus_slot: None,
                            });
                            break;
                        }
                        2 => {
                            self.run_check_timing(crate::db::AbilityTiming::AfterClimaxPhase);
                            self.state.turn.phase_step = 3;
                            continue;
                        }
                        _ => {
                            if self.state.turn.pending_level_up.is_some()
                                || !self.state.turn.pending_triggers.is_empty()
                            {
                                continue;
                            }
                            self.state.turn.phase = Phase::Attack;
                            self.state.turn.phase_step = 0;
                            self.state.turn.attack_phase_begin_done = false;
                            self.state.turn.attack_decl_check_done = false;
                            continue;
                        }
                    }
                }
                Phase::Attack => {
                    if !self.state.turn.attack_phase_begin_done {
                        self.run_check_timing(crate::db::AbilityTiming::BeginAttackPhase);
                        self.state.turn.attack_phase_begin_done = true;
                        continue;
                    }
                    if self.state.turn.attack.is_none() {
                        if !self.state.turn.attack_decl_check_done {
                            self.run_check_timing(
                                crate::db::AbilityTiming::BeginAttackDeclarationStep,
                            );
                            self.state.turn.attack_decl_check_done = true;
                            continue;
                        }
                        let p = self.state.turn.active_player;
                        self.recompute_derived_attack();
                        self.set_decision(Decision {
                            player: p,
                            kind: DecisionKind::AttackDeclaration,
                            focus_slot: None,
                        });
                        break;
                    }
                    self.resolve_attack_pipeline();
                }
                Phase::End => {
                    let p = self.state.turn.active_player;
                    if self.resolve_end_phase(p) {
                        self.state.turn.active_player = 1 - p;
                        self.state.turn.phase = Phase::Stand;
                        self.state.turn.phase_step = 0;
                    }
                }
            }
            self.maybe_validate_state("advance_loop");
        }
    }

    fn card_set_allowed(&self, card: &CardStatic) -> bool {
        match (&self.curriculum.allowed_card_sets_cache, &card.card_set) {
            (None, _) => true,
            (Some(set), Some(set_id)) => set.contains(set_id),
            (Some(_), None) => false,
        }
    }

    fn handle_illegal_action(
        &mut self,
        acting_player: u8,
        reason: &str,
        copy_obs: bool,
    ) -> Result<StepOutcome> {
        self.last_illegal_action = true;
        self.last_perspective = acting_player;
        match self.config.error_policy {
            ErrorPolicy::Strict => Err(anyhow!("Illegal action: {reason}")),
            ErrorPolicy::LenientTerminate => {
                let winner = 1 - acting_player;
                self.state.terminal = Some(TerminalResult::Win { winner });
                self.decision = None;
                self.update_action_cache();
                Ok(self.build_outcome_with_obs(self.terminal_reward_for(acting_player), copy_obs))
            }
            ErrorPolicy::LenientNoop => {
                self.update_action_cache();
                Ok(self.build_outcome_with_obs(0.0, copy_obs))
            }
        }
    }

    pub(crate) fn terminal_reward_for(&self, perspective: u8) -> f32 {
        let RewardConfig {
            terminal_win,
            terminal_loss,
            terminal_draw,
            ..
        } = &self.config.reward;
        match self.state.terminal {
            Some(TerminalResult::Win { winner }) => {
                if winner == perspective {
                    *terminal_win
                } else {
                    *terminal_loss
                }
            }
            Some(TerminalResult::Draw | TerminalResult::Timeout) => *terminal_draw,
            None => 0.0,
        }
    }
}

#[cfg(test)]
mod tests;
