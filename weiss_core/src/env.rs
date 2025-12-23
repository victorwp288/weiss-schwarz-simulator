use std::collections::BTreeSet;
use std::sync::Arc;
use anyhow::{anyhow, Result};

use crate::config::{
    CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
    SimultaneousLossPolicy,
};
use crate::db::{
    AbilityKind, AbilityTemplate, CardColor, CardDb, CardId, CardStatic, CardType, TriggerIcon,
};
use crate::effects::{
    EffectId, EffectKind, EffectPayload, EffectSourceKind, EffectSpec, ReplacementHook,
    ReplacementKind,
};
use crate::encode::{
    encode_observation, fill_action_mask, ACTION_ENCODING_VERSION, MAX_ABILITIES_PER_CARD,
    MAX_STAGE, OBS_ENCODING_VERSION, OBS_LEN,
};
use crate::events::{
    ChoiceOptionSnapshot, ChoiceSkipReason, Event, ModifierRemoveReason, RevealAudience,
    RevealReason, TriggerCancelReason, Zone,
};
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::replay::{
    EpisodeBody, EpisodeHeader, ReplayConfig, ReplayData, ReplayEvent, ReplayFinal, ReplayWriter,
    StepMeta, REPLAY_SCHEMA_VERSION,
};
use crate::state::{
    AttackContext, AttackStep, AttackType, CardInstance, CardInstanceId, ChoiceOptionRef,
    ChoiceReason,
    ChoiceState, ChoiceZone, DamageModifier, DamageModifierKind, DamageType, EncoreRequest,
    GameState, ModifierDuration, ModifierKind, PendingTargetEffect, PendingTrigger, Phase,
    PriorityState, StackItem, StackOrderState, StageSlot, StageStatus, TargetRef,
    TargetSelectionState, TargetSide, TargetSlotFilter, TargetSpec, TargetZone, TerminalResult,
    TimingWindow, TriggerEffect, TriggerOrderState,
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

/// A single Weiss Schwarz environment instance with deterministic RNG state.
pub struct GameEnv {
    pub db: Arc<CardDb>,
    pub config: EnvConfig,
    pub curriculum: CurriculumConfig,
    pub state: GameState,
    pub decision: Option<Decision>,
    pub last_action_lookup: Vec<Option<ActionDesc>>,
    pub last_action_mask: Vec<u8>,
    pub last_legal_actions: Vec<ActionDesc>,
    pub last_action_desc: Option<ActionDesc>,
    pub last_action_player: Option<u8>,
    pub last_illegal_action: bool,
    pub last_engine_error: bool,
    pub last_perspective: u8,
    pub pending_damage_delta: [i32; 2],
    pub obs_buf: Vec<i32>,
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

const MAX_CHOICE_OPTIONS: usize = crate::encode::CHOICE_COUNT;
pub const STACK_AUTO_RESOLVE_CAP: u32 = 256;

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

impl GameEnv {
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
            target_player,
            target_slot,
            kind,
            magnitude,
            duration,
        )
    }

    pub fn new(
        db: Arc<CardDb>,
        config: EnvConfig,
        curriculum: CurriculumConfig,
        seed: u64,
        replay_config: ReplayConfig,
        replay_writer: Option<ReplayWriter>,
    ) -> Self {
        let starting_player = (seed as u8) & 1;
        let state = GameState::new(
            config.deck_lists[0].clone(),
            config.deck_lists[1].clone(),
            seed,
            starting_player,
        );
        let mut curriculum = curriculum;
        curriculum.rebuild_cache();
        let mut env = Self {
            db,
            config,
            curriculum,
            state,
            decision: None,
            last_action_lookup: vec![None; crate::encode::ACTION_SPACE_SIZE],
            last_action_mask: vec![0u8; crate::encode::ACTION_SPACE_SIZE],
            last_legal_actions: Vec::new(),
            last_action_desc: None,
            last_action_player: None,
            last_illegal_action: false,
            last_engine_error: false,
            last_perspective: 0,
            pending_damage_delta: [0, 0],
            obs_buf: vec![0; OBS_LEN],
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

    fn reset_with_obs(&mut self, copy_obs: bool) -> StepOutcome {
        let episode_seed = self.meta_rng.next_u64();
        let starting_player = if (episode_seed & 1) == 1 { 1 } else { 0 };
        self.episode_seed = episode_seed;
        self.state = GameState::new(
            self.config.deck_lists[0].clone(),
            self.config.deck_lists[1].clone(),
            episode_seed,
            starting_player,
        );
        self.decision = None;
        if self.last_action_lookup.len() != crate::encode::ACTION_SPACE_SIZE {
            self.last_action_lookup
                .resize(crate::encode::ACTION_SPACE_SIZE, None);
        }
        for slot in self.last_action_lookup.iter_mut() {
            *slot = None;
        }
        if self.last_action_mask.len() != crate::encode::ACTION_SPACE_SIZE {
            self.last_action_mask
                .resize(crate::encode::ACTION_SPACE_SIZE, 0);
        }
        self.last_action_mask.fill(0);
        self.last_legal_actions.clear();
        self.last_action_desc = None;
        self.last_action_player = None;
        self.last_illegal_action = false;
        self.last_engine_error = false;
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
        self.recording = self.replay_config.enabled
            && self.meta_rng.next_u32() as f32 / u32::MAX as f32 <= self.replay_config.sample_rate;
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
        if self.decision.is_none() {
            return Err(anyhow!("No pending decision"));
        }
        self.last_perspective = self.decision.as_ref().unwrap().player;
        let action = match self
            .last_action_lookup
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

        match decision.kind {
            DecisionKind::Mulligan => match action {
                ActionDesc::MulliganKeep => {
                    self.state.turn.mulligan_done[decision.player as usize] = true;
                }
                ActionDesc::MulliganAll => {
                    let p = decision.player as usize;
                    let hand_len = self.state.players[p].hand.len();
                    let mut discarded: Vec<CardInstance> = Vec::new();
                    std::mem::swap(&mut discarded, &mut self.state.players[p].hand);
                    for (idx, card) in discarded.iter().enumerate() {
                        let from_slot = if idx <= u8::MAX as usize {
                            Some(idx as u8)
                        } else {
                            None
                        };
                        self.move_card_between_zones(
                            p as u8,
                            *card,
                            Zone::Hand,
                            Zone::WaitingRoom,
                            from_slot,
                            None,
                        );
                    }
                    self.draw_to_hand(p as u8, hand_len);
                    self.shuffle_deck(p as u8);
                    self.state.turn.mulligan_done[p] = true;
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
                    ActionDesc::ClockPass => {
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
                self.state.turn.phase = Phase::Main;
            }
            DecisionKind::Main => match action {
                ActionDesc::MainPass => {
                    self.state.turn.main_passed = true;
                    if self.state.turn.priority.is_none() {
                        self.enter_timing_window(TimingWindow::MainWindow, decision.player);
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
                    if self.state.players[p].stage[fs].card.is_none()
                        || self.state.players[p].stage[ts].card.is_none()
                    {
                        return self.handle_illegal_action(
                            decision.player,
                            "Move requires two occupied slots",
                            copy_obs,
                        );
                    }
                    self.state.players[p].stage.swap(fs, ts);
                    self.remove_modifiers_for_slot(decision.player, from_slot);
                    self.remove_modifiers_for_slot(decision.player, to_slot);
                    if let Some(card) = self.state.players[p].stage[fs].card {
                        self.apply_continuous_modifiers_for_slot(
                            decision.player,
                            from_slot,
                            card.id,
                        );
                    }
                    if let Some(card) = self.state.players[p].stage[ts].card {
                        self.apply_continuous_modifiers_for_slot(decision.player, to_slot, card.id);
                    }
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
                ActionDesc::ClimaxPass => {
                    if self.curriculum.enable_priority_windows {
                        self.enter_timing_window(TimingWindow::ClimaxWindow, decision.player);
                    } else {
                        self.state.turn.phase = Phase::Attack;
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
                    if self.curriculum.enable_priority_windows {
                        self.enter_timing_window(TimingWindow::ClimaxWindow, decision.player);
                    } else {
                        self.state.turn.phase = Phase::Attack;
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
                ActionDesc::AttackPass => {
                    if self.has_attackers(decision.player) {
                        return self.handle_illegal_action(
                            decision.player,
                            "Attack pass not allowed",
                            copy_obs,
                        );
                    }
                    if self.curriculum.enable_encore {
                        self.queue_encore_requests();
                    } else {
                        self.cleanup_reversed_to_waiting_room();
                    }
                    self.state.turn.phase = Phase::End;
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
                ActionDesc::EncoreYes => {
                    if let Err(err) = self.resolve_encore(decision.player, true) {
                        return self.handle_illegal_action(
                            decision.player,
                            &err.to_string(),
                            copy_obs,
                        );
                    }
                }
                ActionDesc::EncoreNo => {
                    if let Err(err) = self.resolve_encore(decision.player, false) {
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

    pub(crate) fn update_action_cache(&mut self) {
        if self.decision.is_some() {
            let decision_kind = self.decision.as_ref().map(|d| d.kind);
            if decision_kind == Some(DecisionKind::AttackDeclaration)
                && self.state.turn.derived_attack.is_none()
            {
                self.recompute_derived_attack();
            }
            let decision = self.decision.as_ref().expect("decision present");
            self.last_perspective = decision.player;
            let actions = crate::legal::legal_actions_cached(
                &self.state,
                decision,
                &self.db,
                &self.curriculum,
                self.curriculum.allowed_card_sets_cache.as_ref(),
            );
            fill_action_mask(
                &actions,
                &mut self.last_action_mask,
                &mut self.last_action_lookup,
            );
            self.last_legal_actions = actions;
        } else {
            self.last_action_mask.fill(0);
            for slot in self.last_action_lookup.iter_mut() {
                *slot = None;
            }
            self.last_legal_actions.clear();
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
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} deck"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} deck"));
            }
            for card in &p.hand {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} hand"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} hand"));
            }
            for card in &p.waiting_room {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} waiting_room"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} waiting_room"));
            }
            for card in &p.clock {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} clock"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} clock"));
            }
            for card in &p.level {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} level"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} level"));
            }
            for card in &p.stock {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} stock"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} stock"));
            }
            for card in &p.memory {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} memory"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} memory"));
            }
            for card in &p.climax {
                consume(&mut counts, &mut errors, card.owner, card.id, &format!("p{zone_player} climax"));
                check_instance(&mut instance_ids, &mut errors, card, &format!("p{zone_player} climax"));
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
                    if self.state.turn.encore_queue.is_empty() {
                        errors.push("Encore decision without encore request".to_string());
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
        encode_observation(
            &self.state,
            &self.db,
            &self.curriculum,
            perspective,
            self.decision.as_ref(),
            self.last_action_desc.as_ref(),
            self.last_action_player,
            self.config.observation_visibility,
            self.curriculum.enable_visibility_policies,
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
        };
        StepOutcome {
            obs,
            reward,
            terminated: self.state.terminal.is_some(),
            truncated: matches!(self.state.terminal, Some(TerminalResult::Timeout)),
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
            if self.decision.is_some() {
                break;
            }
            if self.state.turn.tick_count >= self.config.max_ticks {
                self.state.terminal = Some(TerminalResult::Timeout);
                break;
            }
            self.state.turn.tick_count += 1;

            if let Some(player) = self.state.turn.pending_level_up {
                self.decision = Some(Decision {
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
                    self.state.terminal = Some(TerminalResult::Timeout);
                    break;
                }
                if let Some(item) = self.state.turn.stack.pop() {
                    self.resolve_stack_item(&item);
                    self.log_event(Event::StackResolved { item });
                    continue;
                }
            }

            if let Some(req) = self.state.turn.encore_queue.first().copied() {
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
                self.decision = Some(Decision {
                    player: req.player,
                    kind: DecisionKind::Encore,
                    focus_slot: Some(req.slot),
                });
                break;
            }

            match self.state.turn.phase {
                Phase::Mulligan => {
                    if self.state.turn.mulligan_done[0] && self.state.turn.mulligan_done[1] {
                        self.state.turn.phase = Phase::Stand;
                        self.state.turn.active_player = self.state.turn.starting_player;
                        continue;
                    }
                    let sp = self.state.turn.starting_player as usize;
                    let next = if !self.state.turn.mulligan_done[sp] {
                        sp
                    } else {
                        1 - sp
                    };
                    self.decision = Some(Decision {
                        player: next as u8,
                        kind: DecisionKind::Mulligan,
                        focus_slot: None,
                    });
                    break;
                }
                Phase::Stand => {
                    let p = self.state.turn.active_player;
                    self.resolve_stand_phase(p);
                    self.state.turn.phase = Phase::Draw;
                }
                Phase::Draw => {
                    let p = self.state.turn.active_player;
                    self.draw_to_hand(p, 1);
                    self.state.turn.phase = if self.curriculum.enable_clock_phase {
                        Phase::Clock
                    } else {
                        Phase::Main
                    };
                }
                Phase::Clock => {
                    if !self.curriculum.enable_clock_phase {
                        self.state.turn.phase = Phase::Main;
                        continue;
                    }
                    let p = self.state.turn.active_player;
                    self.decision = Some(Decision {
                        player: p,
                        kind: DecisionKind::Clock,
                        focus_slot: None,
                    });
                    break;
                }
                Phase::Main => {
                    let p = self.state.turn.active_player;
                    self.decision = Some(Decision {
                        player: p,
                        kind: DecisionKind::Main,
                        focus_slot: None,
                    });
                    break;
                }
                Phase::Climax => {
                    if !self.curriculum.enable_climax_phase {
                        self.state.turn.phase = Phase::Attack;
                        continue;
                    }
                    let p = self.state.turn.active_player;
                    self.decision = Some(Decision {
                        player: p,
                        kind: DecisionKind::Climax,
                        focus_slot: None,
                    });
                    break;
                }
                Phase::Attack => {
                    if self.state.turn.attack.is_none() {
                        let p = self.state.turn.active_player;
                        self.recompute_derived_attack();
                        self.decision = Some(Decision {
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
                    }
                }
            }
            self.maybe_validate_state("advance_loop");
        }
    }

    fn handle_trigger_pipeline(&mut self) -> bool {
        if let Some(choice) = &self.state.turn.choice {
            self.decision = Some(Decision {
                player: choice.player,
                kind: DecisionKind::Choice,
                focus_slot: None,
            });
            self.maybe_validate_state("choice_decision");
            return true;
        }
        if self.state.turn.pending_triggers.is_empty() {
            self.state.turn.trigger_order = None;
            return false;
        }

        if let Some(order) = &self.state.turn.trigger_order {
            self.decision = Some(Decision {
                player: order.player,
                kind: DecisionKind::TriggerOrder,
                focus_slot: None,
            });
            self.maybe_validate_state("trigger_order_decision");
            return true;
        }

        let group_id = match self
            .state
            .turn
            .pending_triggers
            .iter()
            .map(|t| t.group_id)
            .min()
        {
            Some(id) => id,
            None => return false,
        };
        let active = self.state.turn.active_player;
        for player in [active, 1 - active] {
            let mut choices: Vec<u32> = self
                .state
                .turn
                .pending_triggers
                .iter()
                .filter(|t| t.group_id == group_id && t.player == player)
                .map(|t| t.id)
                .collect();
            if choices.len() > 1 {
                choices.sort_by_key(|id| *id);
                self.state.turn.trigger_order = Some(TriggerOrderState {
                    group_id,
                    player,
                    choices,
                });
                self.decision = Some(Decision {
                    player,
                    kind: DecisionKind::TriggerOrder,
                    focus_slot: None,
                });
                self.maybe_validate_state("trigger_order_decision");
                return true;
            }
            if choices.len() == 1 {
                let trigger_id = choices[0];
                if let Some(index) = self
                    .state
                    .turn
                    .pending_triggers
                    .iter()
                    .position(|t| t.id == trigger_id)
                {
                    let trigger = self.state.turn.pending_triggers.remove(index);
                    if self.resolve_trigger(trigger) {
                        self.maybe_validate_state("trigger_choice_pause");
                        return true;
                    }
                }
                self.maybe_validate_state("trigger_pipeline");
                return true;
            }
        }
        self.maybe_validate_state("trigger_pipeline");
        true
    }

    fn handle_priority_window(&mut self) -> bool {
        let Some(priority) = self.state.turn.priority.clone() else {
            return false;
        };
        if self.decision.is_some() {
            return true;
        }
        self.collect_priority_actions(priority.holder);
        if self.scratch.priority_actions.is_empty() {
            self.priority_pass(priority.holder);
            return true;
        }
        if self.scratch.priority_actions.len() == 1
            && self.curriculum.priority_autopick_single_action
        {
            let action = self.scratch.priority_actions[0].clone();
            let _ = self.apply_priority_action(priority.holder, action);
            return true;
        }
        self.start_priority_choice(priority.holder);
        true
    }

    fn allocate_trigger_group(&mut self) -> u32 {
        let group_id = self.state.turn.next_trigger_group_id;
        self.state.turn.next_trigger_group_id =
            self.state.turn.next_trigger_group_id.wrapping_add(1);
        group_id
    }

    fn allocate_choice_id(&mut self) -> u32 {
        let choice_id = self.state.turn.next_choice_id;
        self.state.turn.next_choice_id = self.state.turn.next_choice_id.wrapping_add(1);
        choice_id
    }

    fn allocate_stack_group_id(&mut self) -> u32 {
        let group_id = self.state.turn.next_stack_group_id;
        self.state.turn.next_stack_group_id = self.state.turn.next_stack_group_id.wrapping_add(1);
        group_id
    }

    fn choice_option_id(&self, option: &ChoiceOptionRef, choice_id: u32, global_index: usize) -> u64 {
        let zone_id = match option.zone {
            ChoiceZone::WaitingRoom => 1u64,
            ChoiceZone::Stage => 2u64,
            ChoiceZone::DeckTop => 3u64,
            ChoiceZone::Hand => 4u64,
            ChoiceZone::Clock => 5u64,
            ChoiceZone::Level => 6u64,
            ChoiceZone::Stock => 7u64,
            ChoiceZone::Memory => 8u64,
            ChoiceZone::Climax => 9u64,
            ChoiceZone::Stack => 10u64,
            ChoiceZone::PriorityCounter => 11u64,
            ChoiceZone::PriorityAct => 12u64,
        };
        let index = option.index.unwrap_or(0) as u64;
        let target = option.target_slot.unwrap_or(0) as u64;
        let hidden_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::Memory
                | ChoiceZone::PriorityCounter
        );
        if option.instance_id != 0 {
            (option.instance_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
        } else if option.card_id != 0 && !hidden_zone {
            (option.card_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
        } else {
            let choice_tag = (choice_id as u64) << 32;
            let global_tag = (global_index as u64 & 0xFFFF) << 8;
            choice_tag | (zone_id << 24) | global_tag | target
        }
    }

    fn summarize_choice_options_for_event(
        &self,
        reason: ChoiceReason,
        player: u8,
        options: &[ChoiceOptionSnapshot],
        page_start: u16,
        choice_id: u32,
        ctx: VisibilityContext,
    ) -> Vec<ChoiceOptionSnapshot> {
        options
            .iter()
            .enumerate()
            .map(|(idx, opt)| {
                let global_index = page_start as usize + idx;
                let sanitized =
                    self.sanitize_choice_option_for_event(reason, player, ctx, &opt.reference);
                ChoiceOptionSnapshot {
                    option_id: self.choice_option_id(&sanitized, choice_id, global_index),
                    reference: sanitized,
                }
            })
            .collect()
    }

    fn sanitize_choice_option_for_event(
        &self,
        reason: ChoiceReason,
        player: u8,
        ctx: VisibilityContext,
        option: &ChoiceOptionRef,
    ) -> ChoiceOptionRef {
        if !ctx.is_public() {
            return *option;
        }
        let option_player = if reason == ChoiceReason::TargetSelect {
            self.state
                .turn
                .target_selection
                .as_ref()
                .map(|selection| match selection.spec.side {
                    TargetSide::SelfSide => selection.controller,
                    TargetSide::Opponent => 1 - selection.controller,
                })
                .unwrap_or(player)
        } else {
            player
        };
        let hide_for_viewer = match ctx.viewer {
            Some(viewer) => viewer != option_player,
            None => true,
        };
        if !hide_for_viewer {
            return *option;
        }
        let hide_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::Memory
                | ChoiceZone::PriorityCounter
        );
        if !hide_zone {
            return *option;
        }
        let revealed = self.instance_revealed_to_viewer(ctx, option.instance_id);
        ChoiceOptionRef {
            card_id: if revealed { option.card_id } else { 0 },
            instance_id: 0,
            zone: option.zone,
            index: None,
            target_slot: option.target_slot,
        }
    }

    fn choice_page_bounds(&self, total: usize, page_start: usize) -> (usize, usize) {
        let start = page_start.min(total);
        let end = total.min(start + MAX_CHOICE_OPTIONS);
        (start, end)
    }

    fn recycle_choice_options(&mut self, options: Vec<ChoiceOptionRef>) {
        self.scratch.choice_options = options;
    }

    fn start_choice(
        &mut self,
        reason: ChoiceReason,
        player: u8,
        candidates: Vec<ChoiceOptionRef>,
        pending_trigger: Option<PendingTrigger>,
    ) -> bool {
        let total = candidates.len();
        let choice_id = self.allocate_choice_id();
        if total == 0 {
            if self.recording {
                self.log_event(Event::ChoiceSkipped {
                    choice_id,
                    player,
                    reason,
                    skip_reason: ChoiceSkipReason::NoCandidates,
                });
            }
            if let Some(trigger) = pending_trigger {
                self.log_event(Event::TriggerResolved {
                    trigger_id: trigger.id,
                    player: trigger.player,
                    effect: trigger.effect,
                });
            }
            self.recycle_choice_options(candidates);
            return false;
        }
        if total == 1 {
            let option = candidates[0];
            if self.recording {
                self.log_event(Event::ChoiceAutopicked {
                    choice_id,
                    player,
                    reason,
                    option,
                });
            }
            self.recycle_choice_options(candidates);
            self.apply_choice_effect(reason, player, option, pending_trigger);
            return false;
        }
        let page_start = 0u16;
        let (page_start_idx, page_end_idx) = self.choice_page_bounds(total, 0);
        let page_slice = &candidates[page_start_idx..page_end_idx];
        let total_candidates = total.min(u16::MAX as usize) as u16;
        if self.recording {
            let mut options = Vec::with_capacity(page_slice.len());
            for (idx, opt) in page_slice.iter().enumerate() {
                options.push(ChoiceOptionSnapshot {
                    option_id: self.choice_option_id(
                        opt,
                        choice_id,
                        page_start as usize + idx,
                    ),
                    reference: *opt,
                });
            }
            self.log_event(Event::ChoicePresented {
                choice_id,
                player,
                reason,
                options,
                total_candidates,
                page_start,
            });
        }
        self.state.turn.choice = Some(ChoiceState {
            id: choice_id,
            reason,
            player,
            options: candidates,
            total_candidates,
            page_start,
            pending_trigger,
        });
        true
    }

    fn apply_choice_effect(
        &mut self,
        reason: ChoiceReason,
        player: u8,
        option: ChoiceOptionRef,
        pending_trigger: Option<PendingTrigger>,
    ) {
        match reason {
            ChoiceReason::TriggerStandbySelect => {
                let Some(target_slot) = option.target_slot else {
                    return;
                };
                let ctx = TriggerCompileContext {
                    source_card: pending_trigger
                        .as_ref()
                        .map(|t| t.source_card)
                        .unwrap_or(option.card_id),
                    standby_slot: Some(target_slot),
                    treasure_take_stock: None,
                };
                let effects = self.compile_trigger_icon_effects(TriggerIcon::Standby, ctx);
                if effects.is_empty() {
                    return;
                }
                let Some(index) = option.index else {
                    return;
                };
                let targets = vec![TargetRef {
                    player,
                    zone: TargetZone::WaitingRoom,
                    index,
                    card_id: option.card_id,
                    instance_id: option.instance_id,
                }];
                for effect in effects {
                    self.enqueue_effect_with_targets(
                        player,
                        ctx.source_card,
                        effect,
                        targets.clone(),
                    );
                }
            }
            ChoiceReason::TriggerTreasureSelect => {
                let take_stock = option.index.unwrap_or(1) == 0;
                let ctx = TriggerCompileContext {
                    source_card: pending_trigger.as_ref().map(|t| t.source_card).unwrap_or(0),
                    standby_slot: None,
                    treasure_take_stock: Some(take_stock),
                };
                let effects = self.compile_trigger_icon_effects(TriggerIcon::Treasure, ctx);
                for effect in effects {
                    self.enqueue_effect_spec(player, ctx.source_card, effect);
                }
            }
            ChoiceReason::StackOrderSelect => {
                self.apply_stack_order_choice(player, option);
            }
            ChoiceReason::PriorityActionSelect => {
                self.apply_priority_action_choice(player, option);
            }
            ChoiceReason::TargetSelect => {
                self.apply_target_choice(player, option);
            }
        }
        if let Some(trigger) = pending_trigger {
            self.log_event(Event::TriggerResolved {
                trigger_id: trigger.id,
                player: trigger.player,
                effect: trigger.effect,
            });
        }
    }

    fn start_target_selection(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: TargetSpec,
        effect: PendingTargetEffect,
    ) {
        self.state.turn.target_selection = Some(TargetSelectionState {
            controller,
            source_id,
            remaining: spec.count,
            spec,
            selected: Vec::new(),
            effect,
        });
        self.present_target_choice();
    }

    fn allocate_effect_instance_id(&mut self) -> u32 {
        let id = self.state.turn.next_effect_instance_id;
        self.state.turn.next_effect_instance_id =
            self.state.turn.next_effect_instance_id.wrapping_add(1);
        id
    }

    fn enqueue_effect_spec(&mut self, controller: u8, source_id: CardId, spec: EffectSpec) {
        let instance_id = self.allocate_effect_instance_id();
        if spec.kind.expects_target() {
            if let Some(target_spec) = spec.target.clone() {
                self.start_target_selection(
                    controller,
                    source_id,
                    target_spec,
                    PendingTargetEffect::EffectPending {
                        instance_id,
                        payload: EffectPayload {
                            spec,
                            targets: Vec::new(),
                        },
                    },
                );
                return;
            }
        }
        let item = StackItem {
            id: instance_id,
            controller,
            source_id,
            effect_id: spec.id,
            payload: EffectPayload {
                spec,
                targets: Vec::new(),
            },
        };
        self.enqueue_stack_items(vec![item]);
    }

    fn enqueue_effect_with_targets(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
        targets: Vec<TargetRef>,
    ) {
        let instance_id = self.allocate_effect_instance_id();
        let item = StackItem {
            id: instance_id,
            controller,
            source_id,
            effect_id: spec.id,
            payload: EffectPayload { spec, targets },
        };
        self.enqueue_stack_items(vec![item]);
    }

    fn enumerate_target_candidates_into(
        state: &GameState,
        db: &CardDb,
        curriculum: &CurriculumConfig,
        controller: u8,
        spec: &TargetSpec,
        selected: &[TargetRef],
        out: &mut Vec<TargetRef>,
    ) {
        let target_player = match spec.side {
            TargetSide::SelfSide => controller,
            TargetSide::Opponent => 1 - controller,
        };
        out.clear();
        match spec.zone {
            TargetZone::Stage => {
                let max_slot = if curriculum.reduced_stage_mode {
                    1
                } else {
                    MAX_STAGE
                };
                // Deterministic target ordering: stage slot ascending (front row is slots 0..2, then back row).
                for slot in 0..max_slot {
                    if spec.slot_filter == TargetSlotFilter::FrontRow && slot >= 3 {
                        continue;
                    }
                    let slot_state = &state.players[target_player as usize].stage[slot];
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Stage
                            && t.index as usize == slot
                    }) {
                        continue;
                    }
                    let index = slot as u8;
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Stage,
                        index,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::WaitingRoom => {
                // Deterministic target ordering: waiting room index ascending.
                for (idx, card_inst) in state.players[target_player as usize]
                    .waiting_room
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::WaitingRoom
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::WaitingRoom,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Hand => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .hand
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Hand
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Hand,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::DeckTop => {
                let deck = &state.players[target_player as usize].deck;
                for offset in 0..deck.len() {
                    if offset > u8::MAX as usize {
                        break;
                    }
                    let deck_idx = deck.len().saturating_sub(1 + offset);
                    let card_inst = deck.get(deck_idx).copied();
                    let Some(card_inst) = card_inst else {
                        continue;
                    };
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::DeckTop
                            && t.index as usize == offset
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::DeckTop,
                        index: offset as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Clock => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .clock
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Clock
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Clock,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Level => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .level
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Level
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Level,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Stock => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .stock
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Stock
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Stock,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Memory => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .memory
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Memory
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Memory,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Climax => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .climax
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Climax
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Climax,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
        }
    }

    fn present_target_choice(&mut self) {
        let controller = {
            let Some(selection) = self.state.turn.target_selection.as_ref() else {
                return;
            };
            Self::enumerate_target_candidates_into(
                &self.state,
                &self.db,
                &self.curriculum,
                selection.controller,
                &selection.spec,
                &selection.selected,
                &mut self.scratch.targets,
            );
            selection.controller
        };
        let candidates = self.scratch.targets.as_slice();
        if candidates.is_empty() {
            let _ = self.start_choice(
                ChoiceReason::TargetSelect,
                controller,
                Vec::new(),
                None,
            );
            self.state.turn.target_selection = None;
            return;
        }
        self.scratch.choice_options.clear();
        for target in candidates {
            let zone = match target.zone {
                TargetZone::Stage => ChoiceZone::Stage,
                TargetZone::WaitingRoom => ChoiceZone::WaitingRoom,
                TargetZone::Hand => ChoiceZone::Hand,
                TargetZone::DeckTop => ChoiceZone::DeckTop,
                TargetZone::Clock => ChoiceZone::Clock,
                TargetZone::Level => ChoiceZone::Level,
                TargetZone::Stock => ChoiceZone::Stock,
                TargetZone::Memory => ChoiceZone::Memory,
                TargetZone::Climax => ChoiceZone::Climax,
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: target.card_id,
                instance_id: target.instance_id,
                zone,
                index: Some(target.index),
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        let _ = self.start_choice(
            ChoiceReason::TargetSelect,
            controller,
            options,
            None,
        );
    }

    fn apply_target_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let Some(mut selection) = self.state.turn.target_selection.take() else {
            return;
        };
        if selection.controller != player {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        let Some(index) = option.index else {
            self.state.turn.target_selection = Some(selection);
            return;
        };
        let zone = match option.zone {
            ChoiceZone::Stage => TargetZone::Stage,
            ChoiceZone::WaitingRoom => TargetZone::WaitingRoom,
            ChoiceZone::Hand => TargetZone::Hand,
            ChoiceZone::DeckTop => TargetZone::DeckTop,
            ChoiceZone::Clock => TargetZone::Clock,
            ChoiceZone::Level => TargetZone::Level,
            ChoiceZone::Stock => TargetZone::Stock,
            ChoiceZone::Memory => TargetZone::Memory,
            ChoiceZone::Climax => TargetZone::Climax,
            _ => {
                self.state.turn.target_selection = Some(selection);
                return;
            }
        };
        if zone != selection.spec.zone {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        let target_player = match selection.spec.side {
            TargetSide::SelfSide => selection.controller,
            TargetSide::Opponent => 1 - selection.controller,
        };
        let valid = match zone {
            TargetZone::Stage => {
                let slot = index as usize;
                if slot >= self.state.players[target_player as usize].stage.len() {
                    false
                } else {
                    self.state.players[target_player as usize].stage[slot]
                        .card
                        .map(|c| c.instance_id)
                        == Some(option.instance_id)
                }
            }
            TargetZone::WaitingRoom => {
                let idx = index as usize;
                if idx
                    >= self.state.players[target_player as usize]
                        .waiting_room
                        .len()
                {
                    false
                } else {
                    self.state.players[target_player as usize].waiting_room[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Hand => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].hand.len() {
                    false
                } else {
                    self.state.players[target_player as usize].hand[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::DeckTop => {
                let offset = index as usize;
                let deck = &self.state.players[target_player as usize].deck;
                let deck_idx = deck.len().saturating_sub(1 + offset);
                if deck_idx >= deck.len() {
                    false
                } else {
                    deck[deck_idx].instance_id == option.instance_id
                }
            }
            TargetZone::Clock => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].clock.len() {
                    false
                } else {
                    self.state.players[target_player as usize].clock[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Level => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].level.len() {
                    false
                } else {
                    self.state.players[target_player as usize].level[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Stock => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].stock.len() {
                    false
                } else {
                    self.state.players[target_player as usize].stock[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Memory => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].memory.len() {
                    false
                } else {
                    self.state.players[target_player as usize].memory[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Climax => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].climax.len() {
                    false
                } else {
                    self.state.players[target_player as usize].climax[idx].instance_id
                        == option.instance_id
                }
            }
        };
        if !valid {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        let target = TargetRef {
            player: target_player,
            zone,
            index,
            card_id: option.card_id,
            instance_id: option.instance_id,
        };
        if selection
            .selected
            .iter()
            .any(|t| t.player == target.player && t.zone == target.zone && t.index == target.index)
        {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        selection.selected.push(target);
        if selection.remaining > 0 {
            selection.remaining -= 1;
        }
        if selection.remaining == 0 {
            let targets = selection.selected.clone();
            match selection.effect {
                PendingTargetEffect::EffectPending {
                    instance_id,
                    mut payload,
                } => {
                    payload.targets = targets;
                    let item = StackItem {
                        id: instance_id,
                        controller: selection.controller,
                        source_id: selection.source_id,
                        effect_id: payload.spec.id,
                        payload,
                    };
                    self.enqueue_stack_items(vec![item]);
                }
            }
            self.state.turn.target_selection = None;
            return;
        }
        self.state.turn.target_selection = Some(selection);
        self.present_target_choice();
    }

    fn enter_timing_window(&mut self, window: TimingWindow, holder: u8) {
        self.state.turn.priority = Some(PriorityState {
            holder,
            passes: 0,
            window,
            used_act_mask: 0,
        });
        self.state.turn.active_window = Some(window);
        self.log_event(Event::TimingWindowEntered {
            window,
            player: holder,
        });
        self.log_event(Event::PriorityGranted {
            window,
            player: holder,
        });
    }

    fn collect_priority_actions(&mut self, player: u8) {
        self.scratch.priority_actions.clear();
        let Some(priority) = self.state.turn.priority.as_ref() else {
            return;
        };
        if priority.holder != player {
            return;
        }
        match priority.window {
            TimingWindow::MainWindow => {
                if !self.curriculum.enable_activated_abilities {
                    return;
                }
                let p = &self.state.players[player as usize];
                let max_slot = if self.curriculum.reduced_stage_mode {
                    1
                } else {
                    MAX_STAGE
                };
                // Deterministic priority ordering: stage slot ascending, then ability index ascending.
                for slot in 0..max_slot {
                    let slot_state = &p.stage[slot];
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_none() {
                        continue;
                    }
                    let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                    for (idx, spec) in specs.iter().enumerate() {
                        if idx >= MAX_ABILITIES_PER_CARD || idx > u8::MAX as usize {
                            break;
                        }
                        if spec.kind != AbilityKind::Activated {
                            continue;
                        }
                        if self
                            .db
                            .compiled_effects_for_ability(card_id, idx)
                            .is_empty()
                        {
                            continue;
                        }
                        let bit = (slot * MAX_ABILITIES_PER_CARD + idx) as u32;
                        if priority.used_act_mask & (1u32 << bit) != 0 {
                            continue;
                        }
                        self.scratch.priority_actions.push(ActionDesc::MainActivateAbility {
                            slot: slot as u8,
                            ability_index: idx as u8,
                        });
                    }
                }
            }
            TimingWindow::CounterWindow => {
                let Some(ctx) = &self.state.turn.attack else {
                    return;
                };
                if ctx.attack_type != AttackType::Frontal
                    || ctx.defender_slot.is_none()
                    || ctx.counter_played
                {
                    return;
                }
                if self.curriculum.enable_counters {
                    let p = &self.state.players[player as usize];
                    // Deterministic priority ordering: hand index ascending.
                    for (hand_index, card_inst) in p.hand.iter().enumerate() {
                        if hand_index >= crate::encode::MAX_HAND || hand_index > u8::MAX as usize {
                            break;
                        }
                        let Some(card) = self.db.get(card_inst.id) else {
                            continue;
                        };
                        if !self.card_set_allowed(card) {
                            continue;
                        }
                        if self.is_counter_card(card)
                            && self.meets_level_requirement(player, card)
                            && self.meets_color_requirement(player, card)
                            && self.meets_cost_requirement(player, card)
                        {
                            self.scratch.priority_actions.push(ActionDesc::CounterPlay {
                                hand_index: hand_index as u8,
                            });
                        }
                    }
                }
            }
            TimingWindow::ClimaxWindow
            | TimingWindow::AttackDeclarationWindow
            | TimingWindow::TriggerResolutionWindow
            | TimingWindow::DamageResolutionWindow
            | TimingWindow::EncoreWindow
            | TimingWindow::EndPhaseWindow => {}
        }
    }

    fn start_priority_choice(&mut self, player: u8) {
        self.scratch.choice_options.clear();
        for action in self.scratch.priority_actions.iter() {
            match *action {
                ActionDesc::CounterPlay { hand_index } => {
                    let (card_id, instance_id) = self.state.players[player as usize]
                        .hand
                        .get(hand_index as usize)
                        .map(|c| (c.id, c.instance_id))
                        .unwrap_or((0, 0));
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id,
                        instance_id,
                        zone: ChoiceZone::PriorityCounter,
                        index: Some(hand_index),
                        target_slot: None,
                    });
                }
                ActionDesc::MainActivateAbility {
                    slot,
                    ability_index,
                } => {
                    let (card_id, instance_id) = self.state.players[player as usize]
                        .stage
                        .get(slot as usize)
                        .and_then(|s| s.card)
                        .map(|c| (c.id, c.instance_id))
                        .unwrap_or((0, 0));
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id,
                        instance_id,
                        zone: ChoiceZone::PriorityAct,
                        index: Some(slot),
                        target_slot: Some(ability_index),
                    });
                }
                _ => {}
            }
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(ChoiceReason::PriorityActionSelect, player, options, None);
    }

    fn apply_priority_action_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let action = match option.zone {
            ChoiceZone::PriorityCounter => option
                .index
                .map(|idx| ActionDesc::CounterPlay { hand_index: idx }),
            ChoiceZone::PriorityAct => {
                if let (Some(slot), Some(ability)) = (option.index, option.target_slot) {
                    Some(ActionDesc::MainActivateAbility {
                        slot,
                        ability_index: ability,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(action) = action {
            let _ = self.apply_priority_action(player, action);
        }
    }

    fn apply_priority_action(&mut self, player: u8, action: ActionDesc) -> Result<()> {
        let Some(priority) = self.state.turn.priority.as_ref() else {
            return Err(anyhow!("Priority window not active"));
        };
        if priority.holder != player {
            return Err(anyhow!("Priority holder mismatch"));
        }
        let window = priority.window;
        match action {
            ActionDesc::MainActivateAbility {
                slot,
                ability_index,
            } => {
                if window != TimingWindow::MainWindow {
                    return Err(anyhow!("Activated abilities not allowed in this window"));
                }
                self.queue_activated_ability_stack_item(player, slot, ability_index)?;
                let bit = slot as u32 * MAX_ABILITIES_PER_CARD as u32 + ability_index as u32;
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.used_act_mask |= 1u32 << bit;
                    priority.holder = 1 - player;
                    priority.passes = 0;
                    new_holder = Some(priority.holder);
                }
                if let Some(holder) = new_holder {
                    self.log_event(Event::PriorityGranted {
                        window,
                        player: holder,
                    });
                }
            }
            ActionDesc::CounterPlay { hand_index } => {
                if window != TimingWindow::CounterWindow {
                    return Err(anyhow!("Counter play not allowed in this window"));
                }
                self.queue_counter_stack_item(player, hand_index)?;
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.holder = 1 - player;
                    priority.passes = 0;
                    new_holder = Some(priority.holder);
                }
                if let Some(holder) = new_holder {
                    self.log_event(Event::PriorityGranted {
                        window,
                        player: holder,
                    });
                }
            }
            ActionDesc::MainPass | ActionDesc::CounterPass => {
                self.collect_priority_actions(player);
                if !self.scratch.priority_actions.is_empty() {
                    return Err(anyhow!(
                        "Explicit pass not allowed when priority actions exist"
                    ));
                }
                self.priority_pass(player);
            }
            _ => return Err(anyhow!("Invalid priority action")),
        }
        Ok(())
    }

    fn priority_pass(&mut self, player: u8) {
        let (window, pass_count, should_check_stack, new_holder) = {
            let Some(priority) = &mut self.state.turn.priority else {
                return;
            };
            if priority.holder != player {
                return;
            }
            priority.passes = priority.passes.saturating_add(1);
            let window = priority.window;
            let pass_count = priority.passes;
            let mut new_holder = None;
            if pass_count < 2 {
                priority.holder = 1 - player;
                new_holder = Some(priority.holder);
            }
            (window, pass_count, pass_count >= 2, new_holder)
        };
        self.log_event(Event::PriorityPassed {
            player,
            window,
            pass_count,
        });
        if let Some(holder) = new_holder {
            self.log_event(Event::PriorityGranted {
                window,
                player: holder,
            });
        }
        if should_check_stack {
            if let Some(item) = self.state.turn.stack.pop() {
                self.resolve_stack_item(&item);
                self.log_event(Event::StackResolved { item });
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.passes = 0;
                    priority.holder = self.state.turn.active_player;
                    new_holder = Some(priority.holder);
                }
                if let Some(holder) = new_holder {
                    self.log_event(Event::PriorityGranted {
                        window,
                        player: holder,
                    });
                }
            } else {
                self.close_priority_window(window);
            }
        }
    }

    fn close_priority_window(&mut self, window: TimingWindow) {
        self.state.turn.priority = None;
        self.state.turn.active_window = None;
        match window {
            TimingWindow::MainWindow => {
                if self.state.turn.main_passed {
                    self.state.turn.main_passed = false;
                    self.state.turn.phase = Phase::Climax;
                }
            }
            TimingWindow::CounterWindow => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.step = AttackStep::Damage;
                }
            }
            TimingWindow::ClimaxWindow => {
                self.state.turn.phase = Phase::Attack;
            }
            TimingWindow::AttackDeclarationWindow => {}
            TimingWindow::TriggerResolutionWindow => {}
            TimingWindow::DamageResolutionWindow => {}
            TimingWindow::EncoreWindow => {}
            TimingWindow::EndPhaseWindow => {}
        }
        self.log_event(Event::WindowAdvanced {
            from: window,
            to: self.state.turn.active_window,
        });
    }

    fn stack_effect_key(effect: &EffectKind) -> u8 {
        match effect {
            EffectKind::CounterBackup { .. } => 0,
            EffectKind::CounterDamageReduce { .. } => 1,
            EffectKind::CounterDamageCancel => 2,
            EffectKind::AddModifier { .. } => 3,
            EffectKind::MoveToHand => 4,
            EffectKind::MoveTriggerCardToHand => 5,
            EffectKind::ChangeController { .. } => 6,
            EffectKind::Standby { .. } => 7,
            EffectKind::TreasureStock { .. } => 8,
            EffectKind::ModifyPendingAttackDamage { .. } => 9,
            EffectKind::Damage { .. } => 10,
            EffectKind::Draw { .. } => 11,
            EffectKind::TriggerIcon { .. } => 12,
        }
    }

    fn enqueue_stack_items(&mut self, items: Vec<StackItem>) {
        if items.is_empty() {
            return;
        }
        let active = self.state.turn.active_player;
        let mut per_player: [Vec<StackItem>; 2] = [Vec::new(), Vec::new()];
        for item in items {
            per_player[item.controller as usize].push(item);
        }
        for controller in [active, 1 - active] {
            let list = &mut per_player[controller as usize];
            if list.is_empty() {
                continue;
            }
            // Deterministic ordering for simultaneous stack items: source id, effect kind, then stack id.
            list.sort_by_key(|item| {
                (
                    item.source_id,
                    Self::stack_effect_key(&item.payload.spec.kind),
                    item.id,
                )
            });
            let group_id = self.allocate_stack_group_id();
            let items = std::mem::take(list);
            let group = StackOrderState {
                group_id,
                controller,
                items,
            };
            self.state.turn.pending_stack_groups.push(group);
        }
        self.process_next_stack_group();
    }

    fn process_next_stack_group(&mut self) {
        if self.state.turn.stack_order.is_some() {
            return;
        }
        if self.state.turn.pending_stack_groups.is_empty() {
            return;
        }
        let group = self.state.turn.pending_stack_groups.remove(0);
        if group.items.len() == 1 {
            let item = group.items.into_iter().next().expect("group item");
            self.push_stack_item(item);
            self.process_next_stack_group();
            return;
        }
        self.log_event(Event::StackGroupPresented {
            group_id: group.group_id,
            controller: group.controller,
            items: group.items.clone(),
        });
        self.state.turn.stack_order = Some(group);
        self.present_stack_order_choice();
    }

    fn present_stack_order_choice(&mut self) {
        let Some(order) = &self.state.turn.stack_order else {
            return;
        };
        self.scratch.choice_options.clear();
        for (idx, item) in order.items.iter().enumerate() {
            let index = if idx <= u8::MAX as usize {
                Some(idx as u8)
            } else {
                None
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: item.source_id,
                instance_id: 0,
                zone: ChoiceZone::Stack,
                index,
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::StackOrderSelect,
            order.controller,
            options,
            None,
        );
    }

    fn apply_stack_order_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stack {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let Some(mut order) = self.state.turn.stack_order.take() else {
            return;
        };
        if order.controller != player {
            self.state.turn.stack_order = Some(order);
            return;
        }
        let index = idx as usize;
        if index >= order.items.len() {
            self.state.turn.stack_order = Some(order);
            return;
        }
        let item = order.items.remove(index);
        self.log_event(Event::StackOrderChosen {
            group_id: order.group_id,
            controller: order.controller,
            stack_id: item.id,
        });
        self.push_stack_item(item);
        if !order.items.is_empty() {
            self.state.turn.stack_order = Some(order);
            self.present_stack_order_choice();
        } else {
            self.state.turn.stack_order = None;
            self.process_next_stack_group();
        }
    }

    fn push_stack_item(&mut self, item: StackItem) {
        self.state.turn.stack.push(item.clone());
        self.log_event(Event::StackPushed { item });
    }

    fn resolve_stack_item(&mut self, item: &StackItem) {
        self.resolve_effect_payload(item.controller, item.source_id, &item.payload);
    }

    fn resolve_effect_payload(
        &mut self,
        controller: u8,
        source_id: CardId,
        payload: &EffectPayload,
    ) {
        match &payload.spec.kind {
            EffectKind::Draw { count } => {
                self.draw_to_hand(controller, *count as usize);
            }
            EffectKind::Damage {
                amount,
                cancelable,
                damage_type: _,
            } => {
                let target_player = if let Some(target) = payload.targets.first() {
                    target.player
                } else if let Some(spec) = payload.spec.target.as_ref() {
                    match spec.side {
                        TargetSide::SelfSide => controller,
                        TargetSide::Opponent => 1 - controller,
                    }
                } else if payload.spec.id.source_kind == EffectSourceKind::System {
                    controller
                } else {
                    1 - controller
                };
                let (amount, target_player) =
                    self.apply_replacements_to_damage(controller, target_player, *amount);
                let refresh_penalty = payload.spec.id.source_kind == EffectSourceKind::System
                    && payload.spec.id.source_card == 0
                    && payload.spec.id.ability_index == 0
                    && payload.spec.id.effect_index == 0
                    && !*cancelable;
                if amount > 0 {
                    let _ = self.resolve_effect_damage(
                        controller,
                        target_player,
                        amount,
                        *cancelable,
                        refresh_penalty,
                        Some(source_id),
                    );
                }
            }
            EffectKind::AddModifier {
                kind,
                magnitude,
                duration,
            } => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let s = target.index as usize;
                    if s >= self.state.players[p].stage.len() {
                        continue;
                    }
                    if self.state.players[p].stage[s]
                        .card
                        .map(|c| c.instance_id)
                        != Some(target.instance_id)
                    {
                        continue;
                    }
                    let _ = self.add_modifier(
                        source_id,
                        target.player,
                        target.index,
                        *kind,
                        *magnitude,
                        *duration,
                    );
                }
            }
            EffectKind::MoveToHand => {
                let mut waiting_room_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => {
                            let option = ChoiceOptionRef {
                                card_id: target.card_id,
                                instance_id: target.instance_id,
                                zone: ChoiceZone::Stage,
                                index: Some(target.index),
                                target_slot: None,
                            };
                            self.move_stage_to_hand(target.player, option);
                        }
                        TargetZone::WaitingRoom => {
                            waiting_room_targets.push(*target);
                        }
                        _ => {}
                    }
                }
                waiting_room_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in waiting_room_targets {
                    let option = ChoiceOptionRef {
                        card_id: target.card_id,
                        instance_id: target.instance_id,
                        zone: ChoiceZone::WaitingRoom,
                        index: Some(target.index),
                        target_slot: None,
                    };
                    self.move_waiting_room_to_hand(target.player, option);
                }
            }
            EffectKind::MoveTriggerCardToHand => {
                let _ = self.move_trigger_card_from_stock_to_hand(controller, source_id);
            }
            EffectKind::ChangeController { new_controller } => {
                let to_player = match new_controller {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let from_player = target.player;
                    if from_player == to_player {
                        continue;
                    }
                    let from_slot = target.index as usize;
                    let to_slot = target.index as usize;
                    if from_slot >= self.state.players[from_player as usize].stage.len()
                        || to_slot >= self.state.players[to_player as usize].stage.len()
                    {
                        continue;
                    }
                    if self.state.players[to_player as usize].stage[to_slot]
                        .card
                        .is_some()
                    {
                        continue;
                    }
                    let Some(card_inst) =
                        self.state.players[from_player as usize].stage[from_slot].card
                    else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.remove_modifiers_for_slot(from_player, target.index);
                    let mut moved_slot = std::mem::replace(
                        &mut self.state.players[from_player as usize].stage[from_slot],
                        StageSlot::empty(),
                    );
                    let mut moved_card = moved_slot.card.take().expect("card present");
                    moved_card.controller = to_player;
                    moved_slot.card = Some(moved_card);
                    self.state.players[to_player as usize].stage[to_slot] = moved_slot;
                    self.apply_continuous_modifiers_for_slot(
                        to_player,
                        target.index,
                        moved_card.id,
                    );
                    self.log_event(Event::ControlChanged {
                        card: moved_card.id,
                        owner: moved_card.owner,
                        from_controller: from_player,
                        to_controller: to_player,
                        from_slot: target.index,
                        to_slot: target.index,
                    });
                }
            }
            EffectKind::Standby { target_slot } => {
                let Some(target) = payload.targets.first() else {
                    return;
                };
                if target.zone != TargetZone::WaitingRoom {
                    return;
                }
                let option = ChoiceOptionRef {
                    card_id: target.card_id,
                    instance_id: target.instance_id,
                    zone: ChoiceZone::WaitingRoom,
                    index: Some(target.index),
                    target_slot: Some(*target_slot),
                };
                self.move_waiting_room_to_stage_standby(controller, option);
            }
            EffectKind::TreasureStock { take_stock } => {
                if *take_stock {
                    if let Some(card) = self.draw_from_deck(controller) {
                        self.move_card_between_zones(
                            controller,
                            card,
                            Zone::Deck,
                            Zone::Stock,
                            None,
                            None,
                        );
                    }
                }
            }
            EffectKind::ModifyPendingAttackDamage { delta } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.damage = ctx.damage.saturating_add(*delta);
                }
            }
            EffectKind::TriggerIcon { .. } => {}
            EffectKind::CounterBackup { power } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    if let Some(def_slot) = ctx.defender_slot {
                        let slot_state =
                            &mut self.state.players[controller as usize].stage[def_slot as usize];
                        slot_state.power_mod_battle += *power;
                        ctx.counter_power += *power;
                    }
                }
                self.log_event(Event::Counter {
                    player: controller,
                    card: source_id,
                    power: *power,
                });
            }
            EffectKind::CounterDamageReduce { amount } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    if *amount > 0 {
                        Self::push_attack_damage_modifier(
                            ctx,
                            DamageModifierKind::AddAmount {
                                delta: -(*amount as i32),
                            },
                            source_id,
                        );
                    }
                }
            }
            EffectKind::CounterDamageCancel => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    Self::push_attack_damage_modifier(
                        ctx,
                        DamageModifierKind::CancelNext,
                        source_id,
                    );
                }
            }
        }
    }

    fn apply_replacements_to_damage(
        &mut self,
        source_player: u8,
        target_player: u8,
        amount: i32,
    ) -> (i32, u8) {
        let mut amount = amount;
        let mut target = target_player;
        if amount <= 0 {
            return (0, target);
        }
        self.scratch_replacement_indices.clear();
        for (idx, replacement) in self.state.replacements.iter().enumerate() {
            if matches!(replacement.hook, ReplacementHook::Damage) {
                self.scratch_replacement_indices.push(idx);
            }
        }
        self.scratch_replacement_indices.sort_by_key(|idx| {
            let replacement = &self.state.replacements[*idx];
            (
                replacement.priority,
                replacement.insertion,
                replacement.source,
            )
        });
        for idx in self.scratch_replacement_indices.iter().copied() {
            let replacement = &self.state.replacements[idx];
            match replacement.kind {
                ReplacementKind::CancelDamage => {
                    amount = 0;
                    break;
                }
                ReplacementKind::RedirectDamage { new_target } => {
                    target = match new_target {
                        TargetSide::SelfSide => source_player,
                        TargetSide::Opponent => 1 - source_player,
                    };
                }
            }
        }
        (amount, target)
    }

    fn apply_continuous_modifiers_for_slot(&mut self, player: u8, slot: u8, card_id: CardId) {
        if !self.curriculum.enable_continuous_modifiers {
            return;
        }
        let db = self.db.clone();
        let specs = db.iter_card_abilities_in_canonical_order(card_id);
        for (idx, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Continuous {
                continue;
            }
            let effects = db.compiled_effects_for_ability(card_id, idx);
            if effects.is_empty() {
                continue;
            }
            for effect in effects {
                let instance_id = self.state.players[player as usize]
                    .stage
                    .get(slot as usize)
                    .and_then(|s| s.card)
                    .map(|c| c.instance_id)
                    .unwrap_or(0);
                let targets = vec![TargetRef {
                    player,
                    zone: TargetZone::Stage,
                    index: slot,
                    card_id,
                    instance_id,
                }];
                let payload = EffectPayload {
                    spec: effect.clone(),
                    targets,
                };
                self.resolve_effect_payload(player, card_id, &payload);
            }
        }
    }

    fn queue_activated_ability_stack_item(
        &mut self,
        player: u8,
        slot: u8,
        ability_index: u8,
    ) -> Result<()> {
        if !self.curriculum.enable_activated_abilities {
            return Err(anyhow!("Activated abilities disabled"));
        }
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return Err(anyhow!("Ability slot out of range"));
        }
        let card_inst = self.state.players[p].stage[s]
            .card
            .ok_or_else(|| anyhow!("No card in ability slot"))?;
        let card_id = card_inst.id;
        let db = self.db.clone();
        if db.get(card_id).is_none() {
            return Err(anyhow!("Card missing in db"));
        }
        let idx = ability_index as usize;
        let spec_kind = db
            .iter_card_abilities_in_canonical_order(card_id)
            .get(idx)
            .map(|spec| spec.kind);
        if idx >= MAX_ABILITIES_PER_CARD {
            return Err(anyhow!("Ability index out of range"));
        }
        let Some(spec_kind) = spec_kind else {
            return Err(anyhow!("Ability index out of range"));
        };
        if spec_kind != AbilityKind::Activated {
            return Err(anyhow!("Ability is not activated"));
        }
        let effects = db.compiled_effects_for_ability(card_id, idx);
        if effects.is_empty() {
            return Err(anyhow!("Activated ability has no effects"));
        }
        for effect in effects {
            self.enqueue_effect_spec(player, card_id, effect.clone());
        }
        Ok(())
    }

    fn queue_counter_stack_item(&mut self, player: u8, hand_index: u8) -> Result<()> {
        if !self.curriculum.enable_counters {
            return Err(anyhow!("Counters disabled"));
        }
        let Some(ctx) = &self.state.turn.attack else {
            return Err(anyhow!("No attack context for counter"));
        };
        if ctx.attack_type != AttackType::Frontal
            || ctx.defender_slot.is_none()
            || ctx.counter_played
        {
            return Err(anyhow!("Counter not allowed for this attack"));
        }
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Counter hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let card = self
            .db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.is_counter_card(card) {
            return Err(anyhow!("Card is not a counter"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Counter requirements not met"));
        }
        let power = self.counter_power(card);
        let damage_reductions = self.counter_damage_reductions(card);
        let damage_cancel = self.counter_damage_cancel(card);
        self.pay_cost(player, card.cost as usize)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        let card_id = card_inst.id;
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::WaitingRoom,
            Some(hand_index),
            None,
        );
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.counter_played = true;
        }
        if power != 0 {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, 0),
                kind: EffectKind::CounterBackup { power },
                target: None,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        for (idx, reduce) in damage_reductions.into_iter().enumerate() {
            if reduce > 0 {
                let spec = EffectSpec {
                    id: EffectId::new(EffectSourceKind::Counter, card_id, 0, idx as u8),
                    kind: EffectKind::CounterDamageReduce {
                        amount: reduce as u8,
                    },
                    target: None,
                };
                self.enqueue_effect_spec(player, card_id, spec);
            }
        }
        if damage_cancel {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, 10),
                kind: EffectKind::CounterDamageCancel,
                target: None,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        Ok(())
    }

    fn enumerate_open_stage_slots(&self, player: u8) -> Vec<u8> {
        let p = player as usize;
        let max_slot = if self.curriculum.reduced_stage_mode {
            1
        } else {
            MAX_STAGE
        };
        let mut slots = Vec::new();
        for slot in 0..max_slot {
            if self.state.players[p].stage[slot].card.is_none() {
                slots.push(slot as u8);
            }
        }
        slots
    }

    fn queue_trigger_group(&mut self, player: u8, source: CardId, effects: Vec<TriggerEffect>) {
        if effects.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_with_group(group_id, player, source, effects);
    }

    fn queue_trigger_group_with_group(
        &mut self,
        group_id: u32,
        player: u8,
        source: CardId,
        effects: Vec<TriggerEffect>,
    ) {
        for effect in effects {
            let id = self.state.turn.next_trigger_id;
            self.state.turn.next_trigger_id = self.state.turn.next_trigger_id.wrapping_add(1);
            let pending = PendingTrigger {
                id,
                group_id,
                player,
                source_card: source,
                effect,
                effect_id: None,
            };
            self.state.turn.pending_triggers.push(pending);
            self.log_event(Event::TriggerQueued {
                trigger_id: id,
                group_id,
                player,
                source,
                effect,
            });
        }
    }

    fn trigger_effect_id(&self, source_card: CardId, effect_index: u8) -> EffectId {
        EffectId::new(EffectSourceKind::Trigger, source_card, 0, effect_index)
    }

    fn compile_trigger_icon_effects(
        &self,
        icon: TriggerIcon,
        ctx: TriggerCompileContext,
    ) -> Vec<EffectSpec> {
        match icon {
            TriggerIcon::Soul => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SOUL),
                kind: EffectKind::ModifyPendingAttackDamage { delta: 1 },
                target: None,
            }],
            TriggerIcon::Draw => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_DRAW),
                kind: EffectKind::Draw { count: 1 },
                target: None,
            }],
            TriggerIcon::Shot => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SHOT),
                kind: EffectKind::Damage {
                    amount: 1,
                    cancelable: true,
                    damage_type: DamageType::Effect,
                },
                target: None,
            }],
            TriggerIcon::Gate => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_GATE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::WaitingRoom,
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Character),
                    count: 1,
                }),
            }],
            TriggerIcon::Bounce => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_BOUNCE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::Stage,
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Character),
                    count: 1,
                }),
            }],
            TriggerIcon::Standby => {
                let Some(slot) = ctx.standby_slot else {
                    return Vec::new();
                };
                vec![EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_STANDBY),
                    kind: EffectKind::Standby { target_slot: slot },
                    target: Some(TargetSpec {
                        zone: TargetZone::WaitingRoom,
                        side: TargetSide::SelfSide,
                        slot_filter: TargetSlotFilter::Any,
                        card_type: Some(CardType::Character),
                        count: 1,
                    }),
                }]
            }
            TriggerIcon::Treasure => {
                let Some(take_stock) = ctx.treasure_take_stock else {
                    return Vec::new();
                };
                let mut effects = Vec::new();
                if take_stock {
                    effects.push(EffectSpec {
                        id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_STOCK),
                        kind: EffectKind::TreasureStock { take_stock },
                        target: None,
                    });
                }
                effects.push(EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_MOVE),
                    kind: EffectKind::MoveTriggerCardToHand,
                    target: None,
                });
                effects
            }
        }
    }

    fn resolve_trigger(&mut self, trigger: PendingTrigger) -> bool {
        if self.db.get(trigger.source_card).is_none() {
            self.log_event(Event::TriggerCanceled {
                trigger_id: trigger.id,
                player: trigger.player,
                reason: TriggerCancelReason::InvalidSource,
            });
            return false;
        }
        match trigger.effect {
            TriggerEffect::Soul => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Soul, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Draw => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Draw, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Shot => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Shot, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Gate => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Gate, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Bounce => {
                let ctx = TriggerCompileContext {
                    source_card: trigger.source_card,
                    standby_slot: None,
                    treasure_take_stock: None,
                };
                for spec in self.compile_trigger_icon_effects(TriggerIcon::Bounce, ctx) {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, spec);
                }
            }
            TriggerEffect::Treasure => {
                return self.resolve_trigger_treasure(trigger);
            }
            TriggerEffect::Standby => {
                return self.resolve_trigger_standby(trigger);
            }
            TriggerEffect::EndPhaseDraw { ability_index, .. } => {
                let db = self.db.clone();
                let effects =
                    db.compiled_effects_for_ability(trigger.source_card, ability_index as usize);
                for effect in effects {
                    self.enqueue_effect_spec(trigger.player, trigger.source_card, effect.clone());
                }
            }
        }
        self.log_event(Event::TriggerResolved {
            trigger_id: trigger.id,
            player: trigger.player,
            effect: trigger.effect,
        });
        self.maybe_validate_state("trigger_resolve");
        false
    }

    fn resolve_trigger_standby(&mut self, trigger: PendingTrigger) -> bool {
        let open_slots = self.enumerate_open_stage_slots(trigger.player);
        let target_slots = if open_slots.is_empty() {
            let max_slot = if self.curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            (0..max_slot).map(|slot| slot as u8).collect::<Vec<_>>()
        } else {
            open_slots
        };
        let level_limit = self.state.players[trigger.player as usize]
            .level
            .len()
            .saturating_add(1);
        self.scratch.choice_options.clear();
        // Deterministic ordering: waiting room order, then slot order (ascending).
        for (idx, card_inst) in self.state.players[trigger.player as usize]
            .waiting_room
            .iter()
            .copied()
            .enumerate()
        {
            let Some(card) = self.db.get(card_inst.id) else {
                continue;
            };
            if card.card_type != CardType::Character {
                continue;
            }
            if card.level as usize > level_limit {
                continue;
            }
            let index = if idx <= u8::MAX as usize {
                Some(idx as u8)
            } else {
                None
            };
            for slot in &target_slots {
                self.scratch.choice_options.push(ChoiceOptionRef {
                    card_id: card_inst.id,
                    instance_id: card_inst.instance_id,
                    zone: ChoiceZone::WaitingRoom,
                    index,
                    target_slot: Some(*slot),
                });
            }
        }
        let candidates = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerStandbySelect,
            trigger.player,
            candidates,
            Some(trigger),
        )
    }

    fn resolve_trigger_treasure(&mut self, trigger: PendingTrigger) -> bool {
        self.scratch.choice_options.clear();
        if self.treasure_stock_available(trigger.player) {
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: 0,
                instance_id: 0,
                zone: ChoiceZone::DeckTop,
                index: Some(0),
                target_slot: None,
            });
        }
        self.scratch.choice_options.push(ChoiceOptionRef {
            card_id: 0,
            instance_id: 0,
            zone: ChoiceZone::DeckTop,
            index: Some(1),
            target_slot: None,
        });
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::TriggerTreasureSelect,
            trigger.player,
            options,
            Some(trigger),
        )
    }

    fn resolve_stand_phase(&mut self, player: u8) {
        let p = player as usize;
        for slot in &mut self.state.players[p].stage {
            if slot.card.is_some() {
                slot.status = StageStatus::Stand;
                slot.has_attacked = false;
            }
            slot.power_mod_battle = 0;
        }
        self.log_event(Event::Stand { player });
    }

    fn resolve_end_phase(&mut self, player: u8) -> bool {
        if !self.state.turn.end_phase_pending {
            self.expire_end_of_turn_effects();
            self.queue_end_phase_triggers();
            self.state.turn.end_phase_pending = true;
            self.state.turn.end_phase_window_done = false;
        }
        if !self.state.turn.pending_triggers.is_empty() {
            return false;
        }
        if self.curriculum.enable_priority_windows && !self.state.turn.end_phase_window_done {
            self.state.turn.end_phase_window_done = true;
            if self.state.turn.priority.is_none() {
                self.enter_timing_window(TimingWindow::EndPhaseWindow, player);
            }
            return false;
        }
        self.finish_end_phase(player);
        self.state.turn.end_phase_pending = false;
        true
    }

    fn expire_end_of_turn_effects(&mut self) {
        for pid in 0..2 {
            for slot in &mut self.state.players[pid].stage {
                slot.power_mod_battle = 0;
                slot.power_mod_turn = 0;
                slot.cannot_attack = false;
                slot.attack_cost = 0;
            }
        }
        let mut removed: Vec<u32> = Vec::new();
        self.state.modifiers.retain(|m| {
            if m.duration == ModifierDuration::UntilEndOfTurn {
                removed.push(m.id);
                false
            } else {
                true
            }
        });
        for id in removed {
            self.log_event(Event::ModifierRemoved {
                id,
                reason: ModifierRemoveReason::EndOfTurn,
            });
        }
        self.state.turn.derived_attack = None;
        self.maybe_validate_state("end_phase_expire");
    }

    fn recompute_derived_attack(&mut self) {
        let mut derived = crate::state::DerivedAttackState::new();
        for player in 0..2usize {
            let max_slot = if self.curriculum.reduced_stage_mode {
                1
            } else {
                MAX_STAGE
            };
            for slot in 0..max_slot {
                let slot_state = &self.state.players[player].stage[slot];
                let mut entry = crate::state::DerivedAttackSlot::empty();
                entry.cannot_attack = slot_state.cannot_attack;
                entry.attack_cost = slot_state.attack_cost;
                if let Some(card_inst) = slot_state.card {
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_none() {
                        derived.per_player[player][slot] = entry;
                        continue;
                    }
                    for modifier in &self.state.modifiers {
                        if modifier.target_player as usize != player
                            || modifier.target_slot as usize != slot
                        {
                            continue;
                        }
                        if modifier.target_card != card_id {
                            continue;
                        }
                        match modifier.kind {
                            ModifierKind::AttackCost => {
                                if modifier.magnitude > 0 {
                                    entry.attack_cost =
                                        entry.attack_cost.saturating_add(modifier.magnitude as u8);
                                }
                            }
                            ModifierKind::CannotAttack => {
                                if modifier.magnitude != 0 {
                                    entry.cannot_attack = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                derived.per_player[player][slot] = entry;
            }
        }
        self.state.turn.derived_attack = Some(derived);
        self.maybe_validate_state("derived_attack_recompute");
    }

    fn queue_end_phase_triggers(&mut self) {
        if !self.curriculum.enable_triggers {
            return;
        }
        let mut pending: Vec<(u8, CardId, TriggerEffect)> = Vec::new();
        for player in 0..2 {
            for slot in &self.state.players[player].stage {
                let Some(card_inst) = slot.card else {
                    continue;
                };
                let card_id = card_inst.id;
                if self.db.get(card_id).is_none() {
                    continue;
                }
                let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                for (ability_index, spec) in specs.iter().enumerate() {
                    match &spec.template {
                        AbilityTemplate::AutoEndPhaseDraw { count } => {
                            pending.push((
                                player as u8,
                                card_id,
                                TriggerEffect::EndPhaseDraw {
                                    count: *count,
                                    ability_index: ability_index as u8,
                                },
                            ));
                        }
                        AbilityTemplate::AbilityDef(def) => {
                            if def.kind != AbilityKind::Auto {
                                continue;
                            }
                            if def.timing != Some(crate::db::AbilityTiming::EndPhase) {
                                continue;
                            }
                            for effect in &def.effects {
                                if let crate::db::EffectTemplate::Draw { count } = effect {
                                    pending.push((
                                        player as u8,
                                        card_id,
                                        TriggerEffect::EndPhaseDraw {
                                            count: *count,
                                            ability_index: ability_index as u8,
                                        },
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if pending.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        for (player, source, effect) in pending {
            self.queue_trigger_group_with_group(group_id, player, source, vec![effect]);
        }
        self.maybe_validate_state("end_phase_triggers");
    }

    fn finish_end_phase(&mut self, player: u8) {
        let p = player as usize;
        if let Some(card) = self.state.players[p].climax.pop() {
            self.move_card_between_zones(player, card, Zone::Climax, Zone::WaitingRoom, None, None);
        }
        self.state.turn.pending_triggers.clear();
        self.state.turn.trigger_order = None;
        self.state.turn.choice = None;
        self.state.turn.priority = None;
        self.state.turn.stack.clear();
        self.state.turn.pending_stack_groups.clear();
        self.state.turn.stack_order = None;
        self.state.turn.derived_attack = None;
        self.state.turn.attack = None;
        self.state.turn.encore_queue.clear();
        self.state.turn.pending_level_up = None;
        self.state.turn.main_passed = false;
        self.state.turn.active_window = None;
        self.state.turn.end_phase_window_done = false;
        self.state.turn.encore_window_done = false;
        self.state.turn.pending_losses = [false; 2];
        self.log_event(Event::EndTurn { player });
        self.maybe_validate_state("end_phase_finish");
    }

    fn has_attackers(&self, player: u8) -> bool {
        !crate::legal::legal_attack_actions(&self.state, player, &self.curriculum).is_empty()
    }

    fn resolve_attack_pipeline(&mut self) {
        loop {
            let Some(mut ctx) = self.state.turn.attack.take() else {
                return;
            };
            match ctx.step {
                AttackStep::Trigger => {
                    if self.curriculum.enable_priority_windows && !ctx.decl_window_done {
                        ctx.decl_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::AttackDeclarationWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.resolve_trigger_step(&mut ctx);
                    if ctx.counter_allowed && self.curriculum.enable_counters {
                        ctx.step = AttackStep::Counter;
                    } else {
                        ctx.step = AttackStep::Damage;
                    }
                    if self.state.turn.pending_level_up.is_some()
                        || !self.state.turn.pending_triggers.is_empty()
                    {
                        self.state.turn.attack = Some(ctx);
                        self.maybe_validate_state("attack_trigger_pause");
                        break;
                    }
                    if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
                        ctx.trigger_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::TriggerResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.state.turn.attack = Some(ctx);
                }
                AttackStep::Counter => {
                    if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
                        ctx.trigger_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::TriggerResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    let defender = 1 - self.state.turn.active_player;
                    self.state.turn.attack = Some(ctx);
                    if self.state.turn.priority.is_none() {
                        self.enter_timing_window(TimingWindow::CounterWindow, defender);
                    }
                    self.maybe_validate_state("attack_counter_window");
                    break;
                }
                AttackStep::Damage => {
                    if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
                        ctx.trigger_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::TriggerResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    let pause = self.resolve_damage_step(&mut ctx);
                    if pause {
                        self.state.turn.attack = Some(ctx);
                        self.maybe_validate_state("attack_damage_pause");
                        break;
                    }
                    if ctx.attack_type == AttackType::Direct {
                        self.clear_battle_mods();
                        self.state.turn.attack = None;
                        self.maybe_validate_state("attack_direct_done");
                        break;
                    }
                    ctx.step = AttackStep::Battle;
                    if self.curriculum.enable_priority_windows && !ctx.damage_window_done {
                        ctx.damage_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::DamageResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.state.turn.attack = Some(ctx);
                }
                AttackStep::Battle => {
                    if self.curriculum.enable_priority_windows && !ctx.damage_window_done {
                        ctx.damage_window_done = true;
                        self.state.turn.attack = Some(ctx);
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(
                                TimingWindow::DamageResolutionWindow,
                                self.state.turn.active_player,
                            );
                        }
                        break;
                    }
                    self.resolve_battle_step(&ctx);
                    self.clear_battle_mods();
                    self.state.turn.attack = None;
                    self.maybe_validate_state("attack_battle_done");
                    break;
                }
                AttackStep::Encore => {
                    self.state.turn.attack = Some(ctx);
                    self.maybe_validate_state("attack_encore_hold");
                    break;
                }
            }
            self.maybe_validate_state("attack_pipeline");
        }
    }

    fn resolve_trigger_step(&mut self, ctx: &mut AttackContext) {
        let active = self.state.turn.active_player as usize;
        let card = self.draw_from_deck(active as u8);
        if let Some(card_inst) = card {
            let card_id = card_inst.id;
            ctx.trigger_card = Some(card_id);
            let _ = self.reveal_cards(
                active as u8,
                &[card_inst],
                RevealReason::TriggerCheck,
                RevealAudience::Public,
            );
            if self.curriculum.enable_triggers {
                if let Some(static_card) = self.db.get(card_id) {
                    let triggers = static_card.triggers.clone();
                    let mut effects = Vec::new();
                    for icon in triggers {
                        self.log_event(Event::Trigger {
                            player: active as u8,
                            icon,
                            card: Some(card_id),
                        });
                        match icon {
                            TriggerIcon::Soul if self.curriculum.enable_trigger_soul => {
                                effects.push(TriggerEffect::Soul)
                            }
                            TriggerIcon::Draw if self.curriculum.enable_trigger_draw => {
                                effects.push(TriggerEffect::Draw)
                            }
                            TriggerIcon::Shot if self.curriculum.enable_trigger_shot => {
                                effects.push(TriggerEffect::Shot)
                            }
                            TriggerIcon::Bounce if self.curriculum.enable_trigger_bounce => {
                                effects.push(TriggerEffect::Bounce)
                            }
                            TriggerIcon::Treasure if self.curriculum.enable_trigger_treasure => {
                                effects.push(TriggerEffect::Treasure)
                            }
                            TriggerIcon::Gate if self.curriculum.enable_trigger_gate => {
                                effects.push(TriggerEffect::Gate)
                            }
                            TriggerIcon::Standby if self.curriculum.enable_trigger_standby => {
                                effects.push(TriggerEffect::Standby)
                            }
                            _ => {}
                        }
                    }
                    self.queue_trigger_group(active as u8, card_id, effects);
                }
            }
            self.move_card_between_zones(
                active as u8,
                card_inst,
                Zone::Deck,
                Zone::Stock,
                None,
                None,
            );
        }
    }

    fn resolve_damage_step(&mut self, ctx: &mut AttackContext) -> bool {
        let attacker = self.state.turn.active_player;
        let defender = 1 - attacker;
        if !ctx.auto_damage_enqueued {
            self.enqueue_attack_auto_effects(ctx, attacker);
            ctx.auto_damage_enqueued = true;
            if !self.state.turn.stack.is_empty() {
                return true;
            }
        }
        if !ctx.battle_damage_applied {
            let intent = DamageIntentLocal {
                source_player: attacker,
                source_slot: Some(ctx.attacker_slot),
                target: defender,
                amount: ctx.damage,
                damage_type: DamageType::Battle,
                cancelable: true,
                refresh_penalty: false,
            };
            let event_id = self.resolve_damage_intent(intent, &mut ctx.damage_modifiers);
            ctx.last_damage_event_id = Some(event_id);
            ctx.battle_damage_applied = true;
        }
        self.state.turn.pending_level_up.is_some()
    }

    fn enqueue_attack_auto_effects(&mut self, ctx: &AttackContext, attacker: u8) {
        let attacker_slot = ctx.attacker_slot as usize;
        if let Some(card_inst) = self.state.players[attacker as usize].stage[attacker_slot].card {
            let card_id = card_inst.id;
            let db = self.db.clone();
            if db.get(card_id).is_none() {
                return;
            }
            let specs = db.iter_card_abilities_in_canonical_order(card_id);
            for (ability_index, spec) in specs.iter().enumerate() {
                if spec.kind != AbilityKind::Auto {
                    continue;
                }
                let timing = match &spec.template {
                    AbilityTemplate::AutoOnAttackDealDamage { .. } => {
                        Some(crate::db::AbilityTiming::AttackDeclaration)
                    }
                    AbilityTemplate::AbilityDef(def) => def.timing,
                    _ => None,
                };
                if timing == Some(crate::db::AbilityTiming::AttackDeclaration) {
                    let effects = db.compiled_effects_for_ability(card_id, ability_index);
                    for effect in effects {
                        self.enqueue_effect_spec(attacker, card_id, effect.clone());
                    }
                }
            }
        }
    }

    fn resolve_effect_damage(
        &mut self,
        source_player: u8,
        target: u8,
        amount: i32,
        cancelable: bool,
        refresh_penalty: bool,
        _source_card: Option<CardId>,
    ) -> bool {
        let intent = DamageIntentLocal {
            source_player,
            source_slot: None,
            target,
            amount,
            damage_type: DamageType::Effect,
            cancelable,
            refresh_penalty,
        };
        let mut modifiers = if let Some(ctx) = &mut self.state.turn.attack {
            std::mem::take(&mut ctx.damage_modifiers)
        } else {
            Vec::new()
        };
        let _ = self.resolve_damage_intent(intent, &mut modifiers);
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.damage_modifiers = modifiers;
        }
        self.state.turn.pending_level_up.is_some()
    }

    fn resolve_damage_intent(
        &mut self,
        intent: DamageIntentLocal,
        modifiers: &mut [DamageModifier],
    ) -> u32 {
        let event_id = self.state.turn.next_damage_event_id;
        self.state.turn.next_damage_event_id = self.state.turn.next_damage_event_id.wrapping_add(1);
        self.log_event(Event::DamageIntent {
            event_id,
            source_player: intent.source_player,
            source_slot: intent.source_slot,
            target: intent.target,
            amount: intent.amount,
            damage_type: intent.damage_type,
            cancelable: intent.cancelable,
        });

        let mut amount = intent.amount.max(0);
        let mut cancelable = intent.cancelable;
        let mut canceled = false;

        let mut order: Vec<usize> = (0..modifiers.len()).collect();
        order.sort_by_key(|idx| {
            let m = &modifiers[*idx];
            (m.priority, m.insertion, m.source_id)
        });
        for idx in order {
            let modifier = &mut modifiers[idx];
            let before_amount = amount;
            let before_cancelable = cancelable;
            let before_canceled = canceled;
            match modifier.kind {
                DamageModifierKind::AddAmount { delta } => {
                    if delta >= 0 {
                        amount = amount.saturating_add(delta);
                    } else if modifier.remaining > 0 {
                        let reduce = amount.min(modifier.remaining);
                        amount -= reduce;
                        modifier.remaining -= reduce;
                    }
                }
                DamageModifierKind::SetCancelable { cancelable: set } => {
                    cancelable = set;
                }
                DamageModifierKind::CancelNext => {
                    if !modifier.used && cancelable {
                        canceled = true;
                        modifier.used = true;
                    }
                }
                DamageModifierKind::SetAmount { amount: set_amount } => {
                    amount = set_amount;
                }
            }
            self.log_event(Event::DamageModifierApplied {
                event_id,
                modifier: modifier.kind,
                before_amount,
                after_amount: amount,
                before_cancelable,
                after_cancelable: cancelable,
                before_canceled,
                after_canceled: canceled,
            });
        }

        let mut revealed: Vec<CardInstance> = Vec::new();
        if cancelable && !canceled && amount > 0 {
            for _ in 0..amount {
                if let Some(card) = self.draw_from_deck(intent.target) {
                    self.reveal_card(
                        intent.target,
                        &card,
                        RevealReason::DamageCheck,
                        RevealAudience::Public,
                    );
                    revealed.push(card);
                    if let Some(static_card) = self.db.get(card.id) {
                        if static_card.card_type == CardType::Climax {
                            canceled = true;
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        }

        let committed = if canceled {
            0
        } else if cancelable {
            revealed.len() as i32
        } else {
            amount
        };
        self.log_event(Event::DamageModified {
            event_id,
            target: intent.target,
            original: intent.amount,
            modified: committed,
            canceled,
            damage_type: intent.damage_type,
        });

        let target = intent.target as usize;
        if canceled {
            self.log_event(Event::DamageCancel {
                player: intent.target,
            });
            for card in revealed {
                self.move_card_between_zones(
                    intent.target,
                    card,
                    Zone::Deck,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
            return event_id;
        }

        if cancelable {
            for card in revealed {
                let card_id = card.id;
                self.move_card_between_zones(
                    intent.target,
                    card,
                    Zone::Deck,
                    Zone::Clock,
                    None,
                    None,
                );
                self.log_event(Event::DamageCommitted {
                    event_id,
                    target: intent.target,
                    card: card_id,
                    damage_type: intent.damage_type,
                });
                self.log_event(Event::Damage {
                    player: intent.target,
                    card: card_id,
                });
                self.pending_damage_delta[target] += 1;
            }
        } else {
            let count = amount as usize;
            for _ in 0..count {
                if let Some(card) = self.draw_from_deck(intent.target) {
                    let card_id = card.id;
                    if intent.refresh_penalty {
                        self.reveal_card(
                            intent.target,
                            &card,
                            RevealReason::RefreshPenalty,
                            RevealAudience::Public,
                        );
                    }
                    self.move_card_between_zones(
                        intent.target,
                        card,
                        Zone::Deck,
                        Zone::Clock,
                        None,
                        None,
                    );
                    self.log_event(Event::DamageCommitted {
                        event_id,
                        target: intent.target,
                        card: card_id,
                        damage_type: intent.damage_type,
                    });
                    self.log_event(Event::Damage {
                        player: intent.target,
                        card: card_id,
                    });
                    if intent.refresh_penalty {
                        self.log_event(Event::RefreshPenalty {
                            player: intent.target,
                            card: card_id,
                        });
                    }
                    self.pending_damage_delta[target] += 1;
                }
            }
        }
        self.check_level_up(intent.target);
        event_id
    }

    fn resolve_battle_step(&mut self, ctx: &AttackContext) {
        let attacker = self.state.turn.active_player as usize;
        let defender = 1 - attacker;
        let atk_slot = ctx.attacker_slot as usize;
        let def_slot = match ctx.defender_slot {
            Some(s) => s as usize,
            None => return,
        };
        let atk_power = self.compute_slot_power(attacker, atk_slot);
        let def_power = self.compute_slot_power(defender, def_slot);
        if atk_power > def_power {
            self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted {
                player: defender as u8,
                slot: def_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
        } else if atk_power < def_power {
            self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted {
                player: attacker as u8,
                slot: atk_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
        } else {
            self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
            self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted {
                player: defender as u8,
                slot: def_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
            self.log_event(Event::ReversalCommitted {
                player: attacker as u8,
                slot: atk_slot as u8,
                cause_damage_event: ctx.last_damage_event_id,
            });
        }
    }

    fn queue_encore_requests(&mut self) {
        let mut queue = Vec::new();
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                let slot_state = &self.state.players[player].stage[slot];
                if slot_state.card.is_some() && slot_state.status == StageStatus::Reverse {
                    queue.push(EncoreRequest {
                        player: player as u8,
                        slot: slot as u8,
                    });
                }
            }
        }
        self.state.turn.encore_queue = queue;
        self.state.turn.encore_window_done = false;
    }

    fn cleanup_reversed_to_waiting_room(&mut self) {
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                if self.state.players[player].stage[slot].status == StageStatus::Reverse {
                    self.send_stage_to_waiting_room(player as u8, slot as u8);
                }
            }
        }
    }

    fn clear_battle_mods(&mut self) {
        for player in 0..2 {
            for slot in &mut self.state.players[player].stage {
                slot.power_mod_battle = 0;
            }
        }
    }

    fn play_character(&mut self, player: u8, hand_index: u8, stage_slot: u8) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        let ss = stage_slot as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Hand index out of range"));
        }
        if ss >= MAX_STAGE
            || (self.curriculum.reduced_stage_mode && ss > 0)
            || self.state.players[p].stage[ss].card.is_some()
        {
            return Err(anyhow!("Stage slot invalid"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let card = self
            .db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if card.card_type != CardType::Character {
            return Err(anyhow!("Card is not a character"));
        }
        if !self.curriculum.allow_character {
            return Err(anyhow!("Character play disabled"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Play requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        let card_id = card_inst.id;
        self.place_card_on_stage(
            player,
            card_inst,
            stage_slot,
            StageStatus::Stand,
            Zone::Hand,
            Some(hand_index),
        );
        self.log_event(Event::Play {
            player,
            card: card_id,
            slot: stage_slot,
        });
        self.apply_continuous_modifiers_for_slot(player, stage_slot, card_id);
        self.resolve_on_play_abilities(player, card_id);
        Ok(())
    }

    fn play_event(&mut self, player: u8, hand_index: u8) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Event hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let db = self.db.clone();
        let card = db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.looks_like_event(card) {
            return Err(anyhow!("Card is not an event"));
        }
        if !self.curriculum.allow_event {
            return Err(anyhow!("Event play disabled"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Event requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.log_event(Event::PlayEvent {
            player,
            card: card_inst.id,
        });
        self.resolve_on_play_abilities(player, card_id);
        let specs = db.iter_card_abilities_in_canonical_order(card_id);
        for (ability_index, spec) in specs.iter().enumerate() {
            if matches!(spec.template, AbilityTemplate::EventDealDamage { .. }) {
                let effects = db.compiled_effects_for_ability(card_id, ability_index);
                for effect in effects {
                    self.enqueue_effect_spec(player, card_inst.id, effect.clone());
                }
            }
        }
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::WaitingRoom,
            Some(hand_index),
            None,
        );
        Ok(())
    }

    fn play_climax(&mut self, player: u8, hand_index: u8) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Climax hand index out of range"));
        }
        if !self.curriculum.allow_climax {
            return Err(anyhow!("Climax play disabled"));
        }
        if !self.state.players[p].climax.is_empty() {
            return Err(anyhow!("Climax zone occupied"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card = self
            .db
            .get(card_inst.id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if card.card_type != CardType::Climax {
            return Err(anyhow!("Card is not a climax"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Climax requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        let card_id = card_inst.id;
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::Climax,
            Some(hand_index),
            None,
        );
        self.log_event(Event::PlayClimax {
            player,
            card: card_id,
        });
        Ok(())
    }

    fn declare_attack(&mut self, player: u8, slot: u8, attack_type: AttackType) -> Result<()> {
        if let Err(reason) = crate::legal::can_declare_attack(
            &self.state,
            player,
            slot,
            attack_type,
            &self.curriculum,
        ) {
            return Err(anyhow!(reason));
        }
        let p = player as usize;
        let s = slot as usize;
        let defender_player = 1 - p;
        let defender_slot = self.state.players[defender_player].stage[s].card.is_some();
        let attack_cost = self
            .state
            .turn
            .derived_attack
            .as_ref()
            .map(|d| d.per_player[p][s].attack_cost as usize)
            .unwrap_or(self.state.players[p].stage[s].attack_cost as usize);
        if attack_cost > 0 {
            self.pay_cost(player, attack_cost)?;
        }
        let attacker_slot = &mut self.state.players[p].stage[s];
        attacker_slot.status = StageStatus::Rest;
        attacker_slot.has_attacked = true;
        let card_inst = attacker_slot
            .card
            .ok_or_else(|| anyhow!("Missing attacker card"))?;
        let card = self
            .db
            .get(card_inst.id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        let mut damage = card.soul as i32;
        if attack_type == AttackType::Direct {
            damage += 1;
        } else if attack_type == AttackType::Side {
            let defender_level = self.state.players[defender_player].level.len() as i32;
            damage = (damage - defender_level).max(0);
        }
        self.log_event(Event::Attack { player, slot });
        self.log_event(Event::AttackType {
            player,
            attacker_slot: slot,
            attack_type,
        });
        let ctx = AttackContext {
            attacker_slot: slot,
            defender_slot: if defender_slot { Some(slot) } else { None },
            attack_type,
            trigger_card: None,
            damage,
            counter_allowed: attack_type == AttackType::Frontal,
            counter_played: false,
            counter_power: 0,
            damage_modifiers: Vec::new(),
            next_modifier_id: 1,
            last_damage_event_id: None,
            auto_damage_enqueued: false,
            battle_damage_applied: false,
            step: AttackStep::Trigger,
            decl_window_done: false,
            trigger_window_done: false,
            damage_window_done: false,
        };
        self.state.turn.attack = Some(ctx);
        Ok(())
    }

    fn resolve_level_up(&mut self, player: u8, index: u8) -> Result<()> {
        let p = player as usize;
        if self.state.players[p].clock.len() < 7 {
            return Err(anyhow!("Clock has fewer than 7 cards"));
        }
        let idx = index as usize;
        if idx >= 7 {
            return Err(anyhow!("Level up index out of range"));
        }
        let mut top = Vec::with_capacity(7);
        for _ in 0..7 {
            if let Some(card) = self.state.players[p].clock.pop() {
                top.push(card);
            }
        }
        if top.len() != 7 {
            return Err(anyhow!("Clock underflow on level up"));
        }
        let chosen_id = top[idx].id;
        for (i, card) in top.into_iter().enumerate() {
            if i == idx {
                self.move_card_between_zones(player, card, Zone::Clock, Zone::Level, None, None);
            } else {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Clock,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
        }
        self.log_event(Event::LevelUpChoice {
            player,
            card: chosen_id,
        });
        self.state.turn.pending_level_up = None;
        if self.state.players[p].level.len() >= 4 {
            self.register_loss(player);
        }
        self.check_level_up(player);
        Ok(())
    }

    fn resolve_encore(&mut self, player: u8, keep: bool) -> Result<()> {
        let req = self
            .state
            .turn
            .encore_queue
            .first()
            .copied()
            .ok_or_else(|| anyhow!("No encore request"))?;
        if req.player != player {
            return Err(anyhow!("Encore player mismatch"));
        }
        let p = player as usize;
        let s = req.slot as usize;
        if s >= self.state.players[p].stage.len() || self.state.players[p].stage[s].card.is_none() {
            return Err(anyhow!("Encore slot invalid"));
        }
        let mut kept = false;
        if keep {
            if self.state.players[p].stock.len() < 3 {
                return Err(anyhow!("Insufficient stock for encore"));
            }
            self.pay_cost(player, 3)?;
            let slot = &mut self.state.players[p].stage[s];
            slot.status = StageStatus::Rest;
            kept = true;
        } else {
            self.send_stage_to_waiting_room(player, req.slot);
        }
        self.log_event(Event::Encore {
            player,
            slot: req.slot,
            kept,
        });
        self.state.turn.encore_queue.remove(0);
        Ok(())
    }

    fn move_card_between_zones(
        &mut self,
        player: u8,
        card: CardInstance,
        from: Zone,
        to: Zone,
        from_slot: Option<u8>,
        to_slot: Option<u8>,
    ) {
        let p = player as usize;
        match to {
            Zone::Deck => self.state.players[p].deck.push(card),
            Zone::Hand => self.state.players[p].hand.push(card),
            Zone::WaitingRoom => self.state.players[p].waiting_room.push(card),
            Zone::Clock => self.state.players[p].clock.push(card),
            Zone::Level => self.state.players[p].level.push(card),
            Zone::Stock => self.state.players[p].stock.push(card),
            Zone::Memory => self.state.players[p].memory.push(card),
            Zone::Climax => self.state.players[p].climax.push(card),
            Zone::Stage => panic!("use place_card_on_stage for stage moves"),
        }
        self.on_card_enter_zone(&card, to);
        self.log_event(Event::ZoneMove {
            player,
            card: card.id,
            from,
            to,
            from_slot,
            to_slot,
        });
    }

    fn place_card_on_stage(
        &mut self,
        player: u8,
        card: CardInstance,
        slot: u8,
        status: StageStatus,
        from: Zone,
        from_slot: Option<u8>,
    ) {
        let p = player as usize;
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(card);
        slot_state.status = status;
        self.state.players[p].stage[slot as usize] = slot_state;
        self.log_event(Event::ZoneMove {
            player,
            card: card.id,
            from,
            to: Zone::Stage,
            from_slot,
            to_slot: Some(slot),
        });
    }

    fn send_stage_to_waiting_room(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        self.remove_modifiers_for_slot(player, slot);
        if let Some(card) = self.state.players[p].stage[s].card.take() {
            self.move_card_between_zones(
                player,
                card,
                Zone::Stage,
                Zone::WaitingRoom,
                Some(slot),
                None,
            );
        }
        self.state.players[p].stage[s] = StageSlot::empty();
    }

    fn move_waiting_room_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::WaitingRoom {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let p = player as usize;
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        if card.instance_id != option.instance_id {
            return;
        }
        self.move_card_between_zones(
            player,
            card,
            Zone::WaitingRoom,
            Zone::Hand,
            None,
            None,
        );
    }

    fn move_stage_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stage {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let p = player as usize;
        let slot = idx as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        self.remove_modifiers_for_slot(player, idx);
        let card = self.state.players[p].stage[slot].card.take();
        let Some(card) = card else {
            return;
        };
        if card.instance_id != option.instance_id {
            return;
        }
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.move_card_between_zones(player, card, Zone::Stage, Zone::Hand, Some(idx), None);
    }

    fn move_waiting_room_to_stage_standby(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::WaitingRoom {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let Some(target_slot) = option.target_slot else {
            return;
        };
        let p = player as usize;
        let slot = target_slot as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        if let Some(existing) = self.state.players[p].stage[slot].card {
            self.remove_modifiers_for_slot(player, target_slot);
            self.state.players[p].stage[slot] = StageSlot::empty();
            self.move_card_between_zones(
                player,
                existing,
                Zone::Stage,
                Zone::WaitingRoom,
                Some(target_slot),
                None,
            );
        }
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        if card.instance_id != option.instance_id {
            return;
        }
        let card_id = card.id;
        self.place_card_on_stage(
            player,
            card,
            target_slot,
            StageStatus::Rest,
            Zone::WaitingRoom,
            None,
        );
        self.apply_continuous_modifiers_for_slot(player, target_slot, card_id);
    }

    fn move_trigger_card_from_stock_to_hand(&mut self, player: u8, card_id: CardId) -> bool {
        let p = player as usize;
        // Deterministic removal: take the most recent matching card from stock.
        if let Some(pos) = self.state.players[p]
            .stock
            .iter()
            .rposition(|c| c.id == card_id)
        {
            let card = self.state.players[p].stock.remove(pos);
            self.move_card_between_zones(player, card, Zone::Stock, Zone::Hand, None, None);
            return true;
        }
        false
    }

    fn treasure_stock_available(&self, player: u8) -> bool {
        let p = player as usize;
        !self.state.players[p].deck.is_empty() || !self.state.players[p].waiting_room.is_empty()
    }

    fn push_attack_damage_modifier(
        ctx: &mut AttackContext,
        kind: DamageModifierKind,
        source_id: u32,
    ) {
        let insertion = ctx.next_modifier_id;
        ctx.next_modifier_id = ctx.next_modifier_id.wrapping_add(1);
        let priority = match kind {
            DamageModifierKind::CancelNext => 0,
            DamageModifierKind::SetCancelable { .. } => 1,
            DamageModifierKind::SetAmount { .. } => 2,
            DamageModifierKind::AddAmount { .. } => 3,
        };
        let remaining = match kind {
            DamageModifierKind::AddAmount { delta } if delta < 0 => -delta,
            _ => 0,
        };
        ctx.damage_modifiers.push(DamageModifier {
            kind,
            priority,
            insertion,
            source_id,
            remaining,
            used: false,
        });
    }

    fn add_modifier_instance(
        &mut self,
        source: CardId,
        target_player: u8,
        target_slot: u8,
        kind: ModifierKind,
        magnitude: i32,
        duration: ModifierDuration,
    ) -> Option<u32> {
        let p = target_player as usize;
        let s = target_slot as usize;
        if s >= self.state.players[p].stage.len() {
            return None;
        }
        let target_card = self.state.players[p].stage[s].card?.id;
        let id = self.state.next_modifier_id;
        self.state.next_modifier_id = self.state.next_modifier_id.wrapping_add(1);
        self.state.modifiers.push(crate::state::ModifierInstance {
            id,
            source,
            target_player,
            target_slot,
            target_card,
            kind,
            magnitude,
            duration,
            insertion: id,
        });
        self.log_event(Event::ModifierAdded {
            id,
            source,
            target_player,
            target_slot,
            target_card,
            kind,
            magnitude,
            duration,
        });
        Some(id)
    }

    fn remove_modifiers_for_slot(&mut self, player: u8, slot: u8) {
        let p = player;
        let s = slot;
        let mut removed: Vec<u32> = Vec::new();
        self.state.modifiers.retain(|m| {
            if m.target_player != p || m.target_slot != s {
                return true;
            }
            removed.push(m.id);
            false
        });
        for id in removed {
            self.log_event(Event::ModifierRemoved {
                id,
                reason: ModifierRemoveReason::TargetLeftStage,
            });
        }
    }

    fn resolve_on_play_abilities(&mut self, player: u8, source_id: CardId) {
        let db = self.db.clone();
        let specs = db.iter_card_abilities_in_canonical_order(source_id);
        for (ability_index, spec) in specs.iter().enumerate() {
            if spec.kind != AbilityKind::Auto {
                continue;
            }
            let timing = match &spec.template {
                AbilityTemplate::AutoOnPlayDraw { .. } => Some(crate::db::AbilityTiming::OnPlay),
                AbilityTemplate::AbilityDef(def) => def.timing,
                _ => None,
            };
            if timing == Some(crate::db::AbilityTiming::OnPlay) {
                let effects = db.compiled_effects_for_ability(source_id, ability_index);
                for effect in effects {
                    self.enqueue_effect_spec(player, source_id, effect.clone());
                }
            }
        }
    }

    fn compute_slot_power(&self, player: usize, slot: usize) -> i32 {
        let slot_state = &self.state.players[player].stage[slot];
        let Some(card_inst) = slot_state.card else {
            return 0;
        };
        let card_id = card_inst.id;
        let Some(card) = self.db.get(card_id) else {
            return 0;
        };
        let mut power = card.power + slot_state.power_mod_turn + slot_state.power_mod_battle;
        for modifier in &self.state.modifiers {
            if modifier.kind != ModifierKind::Power {
                continue;
            }
            if modifier.target_player as usize != player || modifier.target_slot as usize != slot {
                continue;
            }
            if modifier.target_card != card_id {
                continue;
            }
            power += modifier.magnitude;
        }
        power
    }

    fn meets_level_requirement(&self, player: u8, card: &CardStatic) -> bool {
        card.level as usize <= self.state.players[player as usize].level.len()
    }

    fn meets_cost_requirement(&self, player: u8, card: &CardStatic) -> bool {
        if !self.curriculum.enforce_cost_requirement {
            return true;
        }
        self.state.players[player as usize].stock.len() >= card.cost as usize
    }

    fn meets_color_requirement(&self, player: u8, card: &CardStatic) -> bool {
        if !self.curriculum.enforce_color_requirement {
            return true;
        }
        if card.level == 0 || card.color == CardColor::Colorless {
            return true;
        }
        let p = &self.state.players[player as usize];
        for card_id in p.level.iter().chain(p.clock.iter()) {
            if let Some(c) = self.db.get(card_id.id) {
                if c.color == card.color {
                    return true;
                }
            }
        }
        false
    }

    fn pay_cost(&mut self, player: u8, cost: usize) -> Result<()> {
        if cost == 0 {
            return Ok(());
        }
        let p = player as usize;
        if self.state.players[p].stock.len() < cost {
            return Err(anyhow!("Insufficient stock"));
        }
        for _ in 0..cost {
            if let Some(card) = self.state.players[p].stock.pop() {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Stock,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
        }
        Ok(())
    }

    fn looks_like_event(&self, card: &CardStatic) -> bool {
        matches!(card.card_type, CardType::Event)
    }

    fn is_counter_card(&self, card: &CardStatic) -> bool {
        if !card.counter_timing {
            return false;
        }
        self.db
            .iter_card_abilities_in_canonical_order(card.id)
            .iter()
            .any(|spec| {
                matches!(
                    spec.template,
                    AbilityTemplate::CounterBackup { .. }
                        | AbilityTemplate::CounterDamageReduce { .. }
                        | AbilityTemplate::CounterDamageCancel
                )
            })
    }

    fn counter_power(&self, card: &CardStatic) -> i32 {
        for spec in self.db.iter_card_abilities_in_canonical_order(card.id) {
            if let AbilityTemplate::CounterBackup { power } = spec.template {
                return power;
            }
        }
        0
    }

    fn counter_damage_reductions(&self, card: &CardStatic) -> Vec<i32> {
        let mut out = Vec::new();
        for spec in self.db.iter_card_abilities_in_canonical_order(card.id) {
            if let AbilityTemplate::CounterDamageReduce { amount } = spec.template {
                out.push(amount as i32);
            }
        }
        out
    }

    fn counter_damage_cancel(&self, card: &CardStatic) -> bool {
        self.db
            .iter_card_abilities_in_canonical_order(card.id)
            .iter()
            .any(|spec| matches!(spec.template, AbilityTemplate::CounterDamageCancel))
    }

    fn shuffle_deck(&mut self, player: u8) {
        let p = player as usize;
        self.state.rng.shuffle(&mut self.state.players[p].deck);
        self.log_event(Event::Shuffle {
            player,
            zone: Zone::Deck,
        });
        if self.curriculum.enable_visibility_policies {
            let instance_ids: Vec<CardInstanceId> = self.state.players[p]
                .deck
                .iter()
                .map(|card| card.instance_id)
                .collect();
            for instance_id in instance_ids {
                self.forget_instance_revealed(instance_id);
            }
        }
    }

    fn draw_to_hand(&mut self, player: u8, count: usize) {
        for _ in 0..count {
            if let Some(card) = self.draw_from_deck(player) {
                let card_id = card.id;
                self.move_card_between_zones(player, card, Zone::Deck, Zone::Hand, None, None);
                self.log_event(Event::Draw {
                    player,
                    card: card_id,
                });
            }
        }
    }

    fn reveal_card(
        &mut self,
        player: u8,
        card: &CardInstance,
        reason: RevealReason,
        audience: RevealAudience,
    ) {
        if self.curriculum.enable_visibility_policies {
            let viewers: Vec<u8> = match audience {
                RevealAudience::Public | RevealAudience::BothPlayers => vec![0, 1],
                RevealAudience::OwnerOnly => vec![card.owner],
                RevealAudience::ControllerOnly => vec![card.controller],
                RevealAudience::ReplayOnly => Vec::new(),
            };
            self.mark_instance_revealed(&viewers, card.instance_id);
        }
        self.log_event(Event::Reveal {
            player,
            card: card.id,
            reason,
            audience,
        });
    }

    fn reveal_cards(
        &mut self,
        player: u8,
        cards: &[CardInstance],
        reason: RevealReason,
        audience: RevealAudience,
    ) -> Vec<CardInstance> {
        for card in cards {
            self.reveal_card(player, card, reason, audience);
        }
        cards.to_vec()
    }

    fn draw_from_deck(&mut self, player: u8) -> Option<CardInstance> {
        let p = player as usize;
        if self.state.players[p].deck.is_empty() && !self.refresh_deck(player) {
            return None;
        }
        let card = self.state.players[p].deck.pop()?;
        Some(card)
    }

    fn check_level_up(&mut self, player: u8) {
        let p = player as usize;
        if self.state.players[p].clock.len() < 7 {
            return;
        }
        if self.curriculum.enable_level_up_choice {
            if self.state.turn.pending_level_up.is_none() {
                self.state.turn.pending_level_up = Some(player);
            }
        } else {
            let _ = self.resolve_level_up(player, 0);
        }
    }

    fn refresh_deck(&mut self, player: u8) -> bool {
        let p = player as usize;
        if self.state.players[p].waiting_room.is_empty() {
            self.register_loss(player);
            return false;
        }
        let mut reshuffle = Vec::new();
        std::mem::swap(&mut reshuffle, &mut self.state.players[p].waiting_room);
        self.state.players[p].deck = reshuffle;
        self.shuffle_deck(player);
        self.log_event(Event::Refresh { player });
        if self.curriculum.enable_refresh_penalty {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::System, 0, 0, 0),
                kind: EffectKind::Damage {
                    amount: 1,
                    cancelable: false,
                    damage_type: DamageType::Effect,
                },
                target: None,
            };
            self.enqueue_effect_spec(player, 0, spec);
        }
        true
    }

    fn log_event(&mut self, event: Event) {
        if self.recording {
            let ctx = self.replay_visibility_context();
            self.canonical_events.push(event.clone());
            let replay_event = self.sanitize_event_for_viewer(&event, ctx);
            self.replay_events.push(replay_event);
        }
    }

    fn log_action(&mut self, actor: u8, action: ActionDesc) {
        let ctx = self.replay_visibility_context();
        let logged = self.sanitize_action_for_viewer(&action, actor, ctx);
        self.replay_actions.push(logged);
    }

    fn sanitize_action_for_viewer(
        &self,
        action: &ActionDesc,
        actor: u8,
        ctx: VisibilityContext,
    ) -> ActionDesc {
        const UNKNOWN_INDEX: u8 = u8::MAX;
        if !ctx.is_public() {
            return action.clone();
        }
        let hide_for_viewer = match ctx.viewer {
            Some(viewer) => viewer != actor,
            None => true,
        };
        if !hide_for_viewer {
            return action.clone();
        }
        match action {
            ActionDesc::Clock { .. } => ActionDesc::Clock {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::MainPlayCharacter { stage_slot, .. } => ActionDesc::MainPlayCharacter {
                hand_index: UNKNOWN_INDEX,
                stage_slot: *stage_slot,
            },
            ActionDesc::MainPlayEvent { .. } => ActionDesc::MainPlayEvent {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::ClimaxPlay { .. } => ActionDesc::ClimaxPlay {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::CounterPlay { .. } => ActionDesc::CounterPlay {
                hand_index: UNKNOWN_INDEX,
            },
            ActionDesc::ChoiceSelect { .. } => ActionDesc::ChoiceSelect {
                index: UNKNOWN_INDEX,
            },
            _ => action.clone(),
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

    fn register_loss(&mut self, player: u8) {
        if !self.curriculum.use_alternate_end_conditions {
            self.state.terminal = Some(TerminalResult::Win { winner: 1 - player });
            return;
        }
        self.state.turn.pending_losses[player as usize] = true;
    }

    fn resolve_pending_losses(&mut self) {
        if !self.curriculum.use_alternate_end_conditions {
            return;
        }
        if self.state.terminal.is_some() {
            return;
        }
        let p0 = self.state.turn.pending_losses[0];
        let p1 = self.state.turn.pending_losses[1];
        if !(p0 || p1) {
            return;
        }
        let result = if p0 && p1 {
            match self.config.end_condition_policy.simultaneous_loss {
                SimultaneousLossPolicy::Draw => {
                    if self
                        .config
                        .end_condition_policy
                        .allow_draw_on_simultaneous_loss
                    {
                        TerminalResult::Draw
                    } else {
                        TerminalResult::Win {
                            winner: self.state.turn.active_player,
                        }
                    }
                }
                SimultaneousLossPolicy::ActivePlayerWins => TerminalResult::Win {
                    winner: self.state.turn.active_player,
                },
                SimultaneousLossPolicy::NonActivePlayerWins => TerminalResult::Win {
                    winner: 1 - self.state.turn.active_player,
                },
            }
        } else if p0 {
            TerminalResult::Win { winner: 1 }
        } else {
            TerminalResult::Win { winner: 0 }
        };
        self.state.terminal = Some(result);
    }

    fn replay_visibility_context(&self) -> VisibilityContext {
        let policies_enabled = self.curriculum.enable_visibility_policies;
        let mode = self.config.observation_visibility;
        let viewer = None;
        VisibilityContext {
            viewer,
            mode,
            policies_enabled,
        }
    }

    fn hidden_event_zone(zone: Zone) -> bool {
        matches!(zone, Zone::Deck | Zone::Hand | Zone::Stock | Zone::Memory)
    }

    fn hidden_target_zone(zone: TargetZone) -> bool {
        matches!(
            zone,
            TargetZone::Hand | TargetZone::DeckTop | TargetZone::Stock | TargetZone::Memory
        )
    }

    fn zone_hidden_for_viewer(&self, ctx: VisibilityContext, owner: u8, zone: Zone) -> bool {
        if !ctx.is_public() {
            return false;
        }
        match ctx.viewer {
            Some(viewer) => viewer != owner && Self::hidden_event_zone(zone),
            None => Self::hidden_event_zone(zone),
        }
    }

    fn instance_revealed_to_viewer(
        &self,
        ctx: VisibilityContext,
        instance_id: CardInstanceId,
    ) -> bool {
        if instance_id == 0 {
            return false;
        }
        match ctx.viewer {
            Some(viewer) => self.revealed_to_viewer[viewer as usize].contains(&instance_id),
            None => self.revealed_to_viewer[0].contains(&instance_id)
                && self.revealed_to_viewer[1].contains(&instance_id),
        }
    }

    fn mark_instance_revealed(&mut self, viewers: &[u8], instance_id: CardInstanceId) {
        if instance_id == 0 {
            return;
        }
        for &viewer in viewers {
            if let Some(set) = self.revealed_to_viewer.get_mut(viewer as usize) {
                set.insert(instance_id);
            }
        }
    }

    fn forget_instance_revealed(&mut self, instance_id: CardInstanceId) {
        if instance_id == 0 {
            return;
        }
        for set in &mut self.revealed_to_viewer {
            set.remove(&instance_id);
        }
    }

    fn on_card_enter_zone(&mut self, card: &CardInstance, zone: Zone) {
        if !self.curriculum.enable_visibility_policies {
            return;
        }
        if Self::hidden_event_zone(zone) {
            self.forget_instance_revealed(card.instance_id);
        }
    }

    fn target_hidden_for_viewer(
        &self,
        ctx: VisibilityContext,
        owner: u8,
        zone: TargetZone,
    ) -> bool {
        if !ctx.is_public() {
            return false;
        }
        match ctx.viewer {
            Some(viewer) => viewer != owner && Self::hidden_target_zone(zone),
            None => Self::hidden_target_zone(zone),
        }
    }

    fn reveal_visible_to_viewer(
        &self,
        ctx: VisibilityContext,
        owner: u8,
        audience: RevealAudience,
    ) -> bool {
        if !ctx.is_public() {
            return true;
        }
        match audience {
            RevealAudience::Public | RevealAudience::BothPlayers => true,
            RevealAudience::OwnerOnly | RevealAudience::ControllerOnly => {
                ctx.viewer.map(|viewer| viewer == owner).unwrap_or(false)
            }
            RevealAudience::ReplayOnly => false,
        }
    }

    fn sanitize_target_ref(&self, ctx: VisibilityContext, target: TargetRef) -> TargetRef {
        if !self.target_hidden_for_viewer(ctx, target.player, target.zone) {
            return target;
        }
        TargetRef {
            player: target.player,
            zone: target.zone,
            index: 0,
            card_id: 0,
            instance_id: 0,
        }
    }

    fn sanitize_stack_item(&self, ctx: VisibilityContext, item: &StackItem) -> StackItem {
        if !ctx.is_public() {
            return item.clone();
        }
        let hide_source = match ctx.viewer {
            Some(viewer) => viewer != item.controller,
            None => true,
        };
        let source_id = if hide_source { 0 } else { item.source_id };
        let targets = item
            .payload
            .targets
            .iter()
            .copied()
            .map(|t| self.sanitize_target_ref(ctx, t))
            .collect();
        StackItem {
            id: item.id,
            controller: item.controller,
            source_id,
            effect_id: item.effect_id,
            payload: EffectPayload {
                spec: item.payload.spec.clone(),
                targets,
            },
        }
    }

    fn sanitize_event_for_viewer(&self, event: &Event, ctx: VisibilityContext) -> ReplayEvent {
        match event {
            Event::Draw { player, card } => {
                let hide = self.zone_hidden_for_viewer(ctx, *player, Zone::Deck)
                    || self.zone_hidden_for_viewer(ctx, *player, Zone::Hand);
                let card = if hide { 0 } else { *card };
                ReplayEvent::Draw {
                    player: *player,
                    card,
                }
            }
            Event::Damage { player, card } => ReplayEvent::Damage {
                player: *player,
                card: *card,
            },
            Event::DamageCancel { player } => ReplayEvent::DamageCancel { player: *player },
            Event::DamageIntent {
                event_id,
                source_player,
                source_slot,
                target,
                amount,
                damage_type,
                cancelable,
            } => ReplayEvent::DamageIntent {
                event_id: *event_id,
                source_player: *source_player,
                source_slot: *source_slot,
                target: *target,
                amount: *amount,
                damage_type: *damage_type,
                cancelable: *cancelable,
            },
            Event::DamageModifierApplied {
                event_id,
                modifier,
                before_amount,
                after_amount,
                before_cancelable,
                after_cancelable,
                before_canceled,
                after_canceled,
            } => ReplayEvent::DamageModifierApplied {
                event_id: *event_id,
                modifier: *modifier,
                before_amount: *before_amount,
                after_amount: *after_amount,
                before_cancelable: *before_cancelable,
                after_cancelable: *after_cancelable,
                before_canceled: *before_canceled,
                after_canceled: *after_canceled,
            },
            Event::DamageModified {
                event_id,
                target,
                original,
                modified,
                canceled,
                damage_type,
            } => ReplayEvent::DamageModified {
                event_id: *event_id,
                target: *target,
                original: *original,
                modified: *modified,
                canceled: *canceled,
                damage_type: *damage_type,
            },
            Event::DamageCommitted {
                event_id,
                target,
                card,
                damage_type,
            } => ReplayEvent::DamageCommitted {
                event_id: *event_id,
                target: *target,
                card: *card,
                damage_type: *damage_type,
            },
            Event::ReversalCommitted {
                player,
                slot,
                cause_damage_event,
            } => ReplayEvent::ReversalCommitted {
                player: *player,
                slot: *slot,
                cause_damage_event: *cause_damage_event,
            },
            Event::Reveal {
                player,
                card,
                reason,
                audience,
            } => {
                let visible = self.reveal_visible_to_viewer(ctx, *player, *audience);
                ReplayEvent::Reveal {
                    player: *player,
                    card: if visible { *card } else { 0 },
                    reason: *reason,
                    audience: *audience,
                }
            }
            Event::TriggerQueued {
                trigger_id,
                group_id,
                player,
                source,
                effect,
            } => ReplayEvent::TriggerQueued {
                trigger_id: *trigger_id,
                group_id: *group_id,
                player: *player,
                source: *source,
                effect: *effect,
            },
            Event::TriggerResolved {
                trigger_id,
                player,
                effect,
            } => ReplayEvent::TriggerResolved {
                trigger_id: *trigger_id,
                player: *player,
                effect: *effect,
            },
            Event::TriggerCanceled {
                trigger_id,
                player,
                reason,
            } => ReplayEvent::TriggerCanceled {
                trigger_id: *trigger_id,
                player: *player,
                reason: *reason,
            },
            Event::TimingWindowEntered { window, player } => {
                ReplayEvent::TimingWindowEntered {
                    window: *window,
                    player: *player,
                }
            }
            Event::PriorityGranted { window, player } => ReplayEvent::PriorityGranted {
                window: *window,
                player: *player,
            },
            Event::PriorityPassed {
                player,
                window,
                pass_count,
            } => ReplayEvent::PriorityPassed {
                player: *player,
                window: *window,
                pass_count: *pass_count,
            },
            Event::StackGroupPresented {
                group_id,
                controller,
                items,
            } => ReplayEvent::StackGroupPresented {
                group_id: *group_id,
                controller: *controller,
                items: items
                    .iter()
                    .map(|item| self.sanitize_stack_item(ctx, item))
                    .collect(),
            },
            Event::StackOrderChosen {
                group_id,
                controller,
                stack_id,
            } => ReplayEvent::StackOrderChosen {
                group_id: *group_id,
                controller: *controller,
                stack_id: *stack_id,
            },
            Event::StackPushed { item } => ReplayEvent::StackPushed {
                item: self.sanitize_stack_item(ctx, item),
            },
            Event::StackResolved { item } => ReplayEvent::StackResolved {
                item: self.sanitize_stack_item(ctx, item),
            },
            Event::AutoResolveCapExceeded {
                cap,
                stack_len,
                window,
            } => ReplayEvent::AutoResolveCapExceeded {
                cap: *cap,
                stack_len: *stack_len,
                window: *window,
            },
            Event::WindowAdvanced { from, to } => ReplayEvent::WindowAdvanced {
                from: *from,
                to: *to,
            },
            Event::ChoicePresented {
                choice_id,
                player,
                reason,
                options,
                total_candidates,
                page_start,
            } => {
                let summaries = self.summarize_choice_options_for_event(
                    *reason,
                    *player,
                    options,
                    *page_start,
                    *choice_id,
                    ctx,
                );
                ReplayEvent::ChoicePresented {
                    choice_id: *choice_id,
                    player: *player,
                    reason: *reason,
                    options: summaries,
                    total_candidates: *total_candidates,
                    page_start: *page_start,
                }
            }
            Event::ChoicePageChanged {
                choice_id,
                player,
                from_start,
                to_start,
            } => ReplayEvent::ChoicePageChanged {
                choice_id: *choice_id,
                player: *player,
                from_start: *from_start,
                to_start: *to_start,
            },
            Event::ChoiceMade {
                choice_id,
                player,
                reason,
                option,
            } => {
                let sanitized =
                    self.sanitize_choice_option_for_event(*reason, *player, ctx, option);
                ReplayEvent::ChoiceMade {
                    choice_id: *choice_id,
                    player: *player,
                    reason: *reason,
                    option: sanitized,
                }
            }
            Event::ChoiceAutopicked {
                choice_id,
                player,
                reason,
                option,
            } => {
                let sanitized =
                    self.sanitize_choice_option_for_event(*reason, *player, ctx, option);
                ReplayEvent::ChoiceAutopicked {
                    choice_id: *choice_id,
                    player: *player,
                    reason: *reason,
                    option: sanitized,
                }
            }
            Event::ChoiceSkipped {
                choice_id,
                player,
                reason,
                skip_reason,
            } => ReplayEvent::ChoiceSkipped {
                choice_id: *choice_id,
                player: *player,
                reason: *reason,
                skip_reason: *skip_reason,
            },
            Event::ZoneMove {
                player,
                card,
                from,
                to,
                from_slot,
                to_slot,
            } => {
                let hide_from = self.zone_hidden_for_viewer(ctx, *player, *from);
                let hide_to = self.zone_hidden_for_viewer(ctx, *player, *to);
                ReplayEvent::ZoneMove {
                    player: *player,
                    card: if hide_from && hide_to { 0 } else { *card },
                    from: *from,
                    to: *to,
                    from_slot: if hide_from { None } else { *from_slot },
                    to_slot: if hide_to { None } else { *to_slot },
                }
            }
            Event::ControlChanged {
                card,
                owner,
                from_controller,
                to_controller,
                from_slot,
                to_slot,
            } => ReplayEvent::ControlChanged {
                card: *card,
                owner: *owner,
                from_controller: *from_controller,
                to_controller: *to_controller,
                from_slot: *from_slot,
                to_slot: *to_slot,
            },
            Event::ModifierAdded {
                id,
                source,
                target_player,
                target_slot,
                target_card,
                kind,
                magnitude,
                duration,
            } => ReplayEvent::ModifierAdded {
                id: *id,
                source: *source,
                target_player: *target_player,
                target_slot: *target_slot,
                target_card: *target_card,
                kind: *kind,
                magnitude: *magnitude,
                duration: *duration,
            },
            Event::ModifierRemoved { id, reason } => ReplayEvent::ModifierRemoved {
                id: *id,
                reason: *reason,
            },
            Event::Play { player, card, slot } => ReplayEvent::Play {
                player: *player,
                card: *card,
                slot: *slot,
            },
            Event::PlayEvent { player, card } => ReplayEvent::PlayEvent {
                player: *player,
                card: *card,
            },
            Event::PlayClimax { player, card } => ReplayEvent::PlayClimax {
                player: *player,
                card: *card,
            },
            Event::Trigger { player, icon, card } => {
                let reveal = if self.replay_config.include_trigger_card_id {
                    *card
                } else {
                    None
                };
                if ctx.is_public() && reveal.is_some() {
                    // Trigger checks are public, so no additional masking.
                }
                ReplayEvent::Trigger {
                    player: *player,
                    icon: *icon,
                    card: reveal,
                }
            }
            Event::Attack { player, slot } => ReplayEvent::Attack {
                player: *player,
                slot: *slot,
            },
            Event::AttackType {
                player,
                attacker_slot,
                attack_type,
            } => ReplayEvent::AttackType {
                player: *player,
                attacker_slot: *attacker_slot,
                attack_type: *attack_type,
            },
            Event::Counter {
                player,
                card,
                power,
            } => ReplayEvent::Counter {
                player: *player,
                card: *card,
                power: *power,
            },
            Event::Clock { player, card } => ReplayEvent::Clock {
                player: *player,
                card: *card,
            },
            Event::Shuffle { player, zone } => ReplayEvent::Shuffle {
                player: *player,
                zone: *zone,
            },
            Event::Refresh { player } => ReplayEvent::Refresh { player: *player },
            Event::RefreshPenalty { player, card } => ReplayEvent::RefreshPenalty {
                player: *player,
                card: *card,
            },
            Event::LevelUpChoice { player, card } => ReplayEvent::LevelUpChoice {
                player: *player,
                card: *card,
            },
            Event::Encore { player, slot, kept } => ReplayEvent::Encore {
                player: *player,
                slot: *slot,
                kept: *kept,
            },
            Event::Stand { player } => ReplayEvent::Stand { player: *player },
            Event::EndTurn { player } => ReplayEvent::EndTurn { player: *player },
            Event::Terminal { winner } => ReplayEvent::Terminal { winner: *winner },
        }
    }

    pub fn finish_episode_replay(&mut self) {
        if !self.recording {
            return;
        }
        if self.state.terminal.is_some() {
            let need_terminal = !self
                .replay_events
                .iter()
                .any(|e| matches!(e, ReplayEvent::Terminal { .. }));
            if need_terminal {
                let winner = match self.state.terminal {
                    Some(TerminalResult::Win { winner }) => Some(winner),
                    Some(TerminalResult::Draw | TerminalResult::Timeout) => None,
                    None => None,
                };
                self.log_event(Event::Terminal { winner });
            }
        }
        let writer = self.replay_writer.clone();
        if let Some(writer) = writer {
            let header = EpisodeHeader {
                obs_version: OBS_ENCODING_VERSION,
                action_version: ACTION_ENCODING_VERSION,
                replay_version: REPLAY_SCHEMA_VERSION,
                seed: self.episode_seed,
                starting_player: self.state.turn.starting_player,
                deck_ids: self.config.deck_ids,
                curriculum_id: "default".to_string(),
                config_hash: self.config.config_hash(&self.curriculum),
                fingerprint_algo: crate::fingerprint::FINGERPRINT_ALGO.to_string(),
            };
            let body = EpisodeBody {
                actions: self.replay_actions.clone(),
                events: Some(self.replay_events.clone()),
                steps: self.replay_steps.clone(),
                final_state: Some(ReplayFinal {
                    terminal: self.state.terminal,
                    state_hash: crate::fingerprint::state_fingerprint(&self.state),
                    decision_count: self.state.turn.decision_count,
                    tick_count: self.state.turn.tick_count,
                }),
            };
            writer.send(ReplayData { header, body });
        }
        self.recording = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CurriculumConfig, EnvConfig, ErrorPolicy, ObservationVisibility, RewardConfig,
        SimultaneousLossPolicy,
    };
    use crate::db::{CardColor, CardDb, CardId, CardStatic, CardType};
    use crate::encode::{encode_observation, OBS_LEN};
    use crate::effects::{EffectId, EffectKind, EffectSourceKind, EffectSpec};
    use crate::replay::ReplayConfig;
    use crate::replay::ReplayEvent;
    use crate::state::{
        CardInstance, PendingTargetEffect, TargetSelectionState, TargetSide, TargetSlotFilter,
        TargetSpec, TargetZone, TerminalResult,
    };
    use std::sync::Arc;

    fn make_instance(id: CardId, owner: u8, next_id: &mut u32) -> CardInstance {
        let instance = CardInstance::new(id, owner, *next_id);
        *next_id = next_id.wrapping_add(1);
        instance
    }

    fn make_env() -> GameEnv {
        let cards = vec![
            CardStatic {
                id: 1,
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
            },
            CardStatic {
                id: 2,
                card_set: None,
                card_type: CardType::Character,
                color: CardColor::Blue,
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
            },
        ];
        let db = Arc::new(CardDb::new(cards).expect("db"));
        let config = EnvConfig {
            deck_lists: [vec![1; 10], vec![2; 10]],
            deck_ids: [1, 2],
            max_decisions: 100,
            max_ticks: 1000,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        };
        GameEnv::new(
            db,
            config,
            CurriculumConfig::default(),
            1,
            Default::default(),
            None,
        )
    }

    fn make_env_with_replay(replay_config: ReplayConfig) -> GameEnv {
        let cards = vec![
            CardStatic {
                id: 1,
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
            },
            CardStatic {
                id: 2,
                card_set: None,
                card_type: CardType::Character,
                color: CardColor::Blue,
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
            },
        ];
        let db = Arc::new(CardDb::new(cards).expect("db"));
        let config = EnvConfig {
            deck_lists: [vec![1; 10], vec![2; 10]],
            deck_ids: [1, 2],
            max_decisions: 100,
            max_ticks: 1000,
            reward: RewardConfig::default(),
            error_policy: ErrorPolicy::Strict,
            observation_visibility: ObservationVisibility::Public,
            end_condition_policy: Default::default(),
        };
        GameEnv::new(
            db,
            config,
            CurriculumConfig::default(),
            2,
            replay_config,
            None,
        )
    }

    fn enumerate_targets_for_test(
        env: &GameEnv,
        controller: u8,
        spec: &TargetSpec,
        selected: &[TargetRef],
    ) -> Vec<TargetRef> {
        let mut out = Vec::new();
        GameEnv::enumerate_target_candidates_into(
            &env.state,
            &env.db,
            &env.curriculum,
            controller,
            spec,
            selected,
            &mut out,
        );
        out
    }

    #[test]
    fn stack_group_ordering_stable() {
        let mut env = make_env();
        let spec_a = EffectSpec {
            id: EffectId::new(EffectSourceKind::System, 2, 0, 0),
            kind: EffectKind::Draw { count: 1 },
            target: None,
        };
        let spec_b = EffectSpec {
            id: EffectId::new(EffectSourceKind::System, 1, 0, 0),
            kind: EffectKind::Draw { count: 1 },
            target: None,
        };
        let item_a = StackItem {
            id: 2,
            controller: 0,
            source_id: 2,
            effect_id: spec_a.id,
            payload: EffectPayload {
                spec: spec_a,
                targets: Vec::new(),
            },
        };
        let item_b = StackItem {
            id: 1,
            controller: 0,
            source_id: 1,
            effect_id: spec_b.id,
            payload: EffectPayload {
                spec: spec_b,
                targets: Vec::new(),
            },
        };
        env.enqueue_stack_items(vec![item_a, item_b]);
        let order = env.state.turn.stack_order.as_ref().expect("stack order");
        assert_eq!(order.items[0].source_id, 1);
        assert_eq!(order.items[1].source_id, 2);
    }

    #[test]
    fn target_candidate_ordering_by_zone() {
        let mut env = make_env();
        let p = 0usize;
        let owner = p as u8;
        let mut next_id = 1u32;
        env.state.players[p].hand = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].waiting_room = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].clock = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
        ];
        env.state.players[p].level = vec![
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].stock = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
        ];
        env.state.players[p].memory = vec![make_instance(1, owner, &mut next_id)];
        env.state.players[p].climax = vec![make_instance(2, owner, &mut next_id)];
        env.state.players[p].deck = vec![
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
            make_instance(1, owner, &mut next_id),
            make_instance(2, owner, &mut next_id),
        ];
        env.state.players[p].stage = [
            {
                let mut s = StageSlot::empty();
                s.card = Some(make_instance(1, owner, &mut next_id));
                s
            },
            {
                let mut s = StageSlot::empty();
                s.card = Some(make_instance(2, owner, &mut next_id));
                s
            },
            StageSlot::empty(),
            StageSlot::empty(),
            StageSlot::empty(),
        ];

        let spec = |zone| TargetSpec {
            zone,
            side: TargetSide::SelfSide,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 3,
        };

        let stage = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Stage), &[]);
        assert_eq!(
            stage.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let waiting = enumerate_targets_for_test(&env, owner, &spec(TargetZone::WaitingRoom), &[]);
        assert_eq!(
            waiting.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let hand = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Hand), &[]);
        assert_eq!(
            hand.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let deck = enumerate_targets_for_test(&env, owner, &spec(TargetZone::DeckTop), &[]);
        assert_eq!(
            deck.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );

        let clock = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Clock), &[]);
        assert_eq!(
            clock.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let level = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Level), &[]);
        assert_eq!(
            level.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let stock = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Stock), &[]);
        assert_eq!(
            stock.iter().map(|t| t.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let memory = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Memory), &[]);
        assert_eq!(memory.iter().map(|t| t.index).collect::<Vec<_>>(), vec![0]);

        let climax = enumerate_targets_for_test(&env, owner, &spec(TargetZone::Climax), &[]);
        assert_eq!(climax.iter().map(|t| t.index).collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn visibility_policy_masks_opponent_hidden_choices() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        let mut next_id = 1u32;
        env.state.players[1].hand = vec![
            make_instance(1, 1, &mut next_id),
            make_instance(2, 1, &mut next_id),
        ];

        let spec = TargetSpec {
            zone: TargetZone::Hand,
            side: TargetSide::Opponent,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 1,
        };
        let effect_spec = EffectSpec {
            id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
            kind: EffectKind::MoveToHand,
            target: Some(spec.clone()),
        };
        env.state.turn.target_selection = Some(TargetSelectionState {
            controller: 0,
            source_id: 1,
            remaining: 1,
            spec,
            selected: Vec::new(),
            effect: PendingTargetEffect::EffectPending {
                instance_id: 1,
                payload: EffectPayload {
                    spec: effect_spec,
                    targets: Vec::new(),
                },
            },
        });
        env.present_target_choice();

        let (choice_id, options) = env
            .replay_events
            .iter()
            .find_map(|e| {
                if let ReplayEvent::ChoicePresented {
                    reason: ChoiceReason::TargetSelect,
                    choice_id,
                    options,
                    ..
                } = e
                {
                    Some((*choice_id, options))
                } else {
                    None
                }
            })
            .expect("choice presented");
        assert!(options.iter().all(|opt| opt.reference.card_id == 0));
        assert!(options.iter().all(|opt| opt.reference.index.is_none()));
        assert!(options
            .iter()
            .all(|opt| opt.option_id >> 32 == choice_id as u64));
        let mut unique = std::collections::BTreeSet::new();
        for opt in options {
            assert!(unique.insert(opt.option_id));
        }

        env.replay_events.clear();
        env.state.turn.choice = None;
        let revealed = env.state.players[1].hand[1];
        env.reveal_card(1, &revealed, RevealReason::TriggerCheck, RevealAudience::Public);
        env.present_target_choice();

        let options = env
            .replay_events
            .iter()
            .find_map(|e| {
                if let ReplayEvent::ChoicePresented {
                    reason: ChoiceReason::TargetSelect,
                    options,
                    ..
                } = e
                {
                    Some(options)
                } else {
                    None
                }
            })
            .expect("choice presented");
        assert!(options.iter().any(|opt| opt.reference.card_id == 2));
        assert!(options.iter().any(|opt| opt.reference.card_id == 0));
    }

    #[test]
    fn public_replay_masks_hidden_action_params() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.replay_actions.clear();

        env.log_action(
            1,
            ActionDesc::MainPlayCharacter {
                hand_index: 3,
                stage_slot: 2,
            },
        );

        let last = env.replay_actions.last().expect("action logged");
        match last {
            ActionDesc::MainPlayCharacter {
                hand_index,
                stage_slot,
            } => {
                assert_eq!(*hand_index, u8::MAX);
                assert_eq!(*stage_slot, 2);
            }
            _ => panic!("unexpected action: {last:?}"),
        }
    }

    #[test]
    fn public_observation_masks_opponent_last_action_params() {
        let mut env = make_env();
        env.curriculum.enable_visibility_policies = true;
        env.last_action_desc = Some(ActionDesc::MainPlayCharacter {
            hand_index: 4,
            stage_slot: 1,
        });
        env.last_action_player = Some(1);
        let mut obs = vec![0; OBS_LEN];
        encode_observation(
            &env.state,
            &env.db,
            &env.curriculum,
            0,
            env.decision.as_ref(),
            env.last_action_desc.as_ref(),
            env.last_action_player,
            env.config.observation_visibility,
            env.curriculum.enable_visibility_policies,
            &mut obs,
        );
        assert_eq!(obs[5], 6);
        assert_eq!(obs[6], -1);
        assert_eq!(obs[7], 1);
    }

    #[test]
    fn public_replay_masks_hidden_draws() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.recording = true;
        env.replay_events.clear();

        env.log_event(Event::Draw { player: 1, card: 99 });

        let last = env.replay_events.last().expect("draw event");
        match last {
            ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
            _ => panic!("unexpected event: {last:?}"),
        }
    }

    #[test]
    fn public_replay_no_hidden_zone_leaks() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.recording = true;
        env.replay_events.clear();

        env.draw_to_hand(1, 1);

        let mut next_id = 1u32;
        env.state.players[1].hand.clear();
        env.state.players[1]
            .hand
            .push(make_instance(2, 1, &mut next_id));

        let spec = TargetSpec {
            zone: TargetZone::Hand,
            side: TargetSide::Opponent,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 1,
        };
        let effect_spec = EffectSpec {
            id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
            kind: EffectKind::MoveToHand,
            target: Some(spec.clone()),
        };
        env.state.turn.target_selection = Some(TargetSelectionState {
            controller: 0,
            source_id: 1,
            remaining: 1,
            spec,
            selected: Vec::new(),
            effect: PendingTargetEffect::EffectPending {
                instance_id: 1,
                payload: EffectPayload {
                    spec: effect_spec,
                    targets: Vec::new(),
                },
            },
        });
        env.present_target_choice();

        for event in &env.replay_events {
            match event {
                ReplayEvent::Draw { card, .. } => assert_eq!(*card, 0),
                ReplayEvent::ZoneMove {
                    card,
                    from,
                    to,
                    from_slot,
                    to_slot,
                    ..
                } => {
                    let hidden_from = matches!(
                        from,
                        Zone::Deck | Zone::Hand | Zone::Stock | Zone::Memory
                    );
                    let hidden_to =
                        matches!(to, Zone::Deck | Zone::Hand | Zone::Stock | Zone::Memory);
                    if hidden_from && hidden_to {
                        assert_eq!(*card, 0);
                        assert_eq!(*from_slot, None);
                        assert_eq!(*to_slot, None);
                    }
                }
                ReplayEvent::ChoicePresented { options, .. } => {
                    for opt in options {
                        if matches!(
                            opt.reference.zone,
                            ChoiceZone::Hand
                                | ChoiceZone::DeckTop
                                | ChoiceZone::Stock
                                | ChoiceZone::Memory
                        ) {
                            assert_eq!(opt.reference.card_id, 0);
                            assert_eq!(opt.reference.instance_id, 0);
                            assert!(opt.reference.index.is_none());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn reveal_one_copy_does_not_unmask_duplicates() {
        let replay_config = ReplayConfig {
            enabled: true,
            sample_rate: 1.0,
            ..Default::default()
        };
        let mut env = make_env_with_replay(replay_config);
        env.curriculum.enable_visibility_policies = true;
        env.replay_events.clear();

        let mut next_id = 1u32;
        let first = make_instance(1, 1, &mut next_id);
        let second = make_instance(1, 1, &mut next_id);
        env.state.players[1].hand = vec![first, second];

        env.reveal_card(1, &first, RevealReason::TriggerCheck, RevealAudience::Public);

        let spec = TargetSpec {
            zone: TargetZone::Hand,
            side: TargetSide::Opponent,
            slot_filter: TargetSlotFilter::Any,
            card_type: None,
            count: 1,
        };
        let effect_spec = EffectSpec {
            id: EffectId::new(EffectSourceKind::Activated, 1, 0, 0),
            kind: EffectKind::MoveToHand,
            target: Some(spec.clone()),
        };
        env.state.turn.target_selection = Some(TargetSelectionState {
            controller: 0,
            source_id: 1,
            remaining: 1,
            spec,
            selected: Vec::new(),
            effect: PendingTargetEffect::EffectPending {
                instance_id: 1,
                payload: EffectPayload {
                    spec: effect_spec,
                    targets: Vec::new(),
                },
            },
        });
        env.present_target_choice();

        let options = env
            .replay_events
            .iter()
            .find_map(|e| {
                if let ReplayEvent::ChoicePresented {
                    reason: ChoiceReason::TargetSelect,
                    options,
                    ..
                } = e
                {
                    Some(options)
                } else {
                    None
                }
            })
            .expect("choice presented");
        let revealed = options.iter().filter(|opt| opt.reference.card_id == 1).count();
        let hidden = options.iter().filter(|opt| opt.reference.card_id == 0).count();
        assert_eq!(revealed, 1);
        assert_eq!(hidden, 1);
        assert!(options.iter().all(|opt| opt.reference.instance_id == 0));
    }

    #[test]
    fn alternate_end_conditions_simultaneous_loss_policies() {
        let mut env = make_env();
        env.curriculum.use_alternate_end_conditions = true;

        env.state.turn.active_player = 0;
        env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::Draw;
        env.config
            .end_condition_policy
            .allow_draw_on_simultaneous_loss = true;
        env.state.turn.pending_losses = [true, true];
        env.resolve_pending_losses();
        assert!(matches!(env.state.terminal, Some(TerminalResult::Draw)));

        env.state.terminal = None;
        env.state.turn.pending_losses = [true, true];
        env.config.end_condition_policy.simultaneous_loss =
            SimultaneousLossPolicy::ActivePlayerWins;
        env.resolve_pending_losses();
        assert!(matches!(
            env.state.terminal,
            Some(TerminalResult::Win { winner: 0 })
        ));

        env.state.terminal = None;
        env.state.turn.pending_losses = [true, true];
        env.config.end_condition_policy.simultaneous_loss =
            SimultaneousLossPolicy::NonActivePlayerWins;
        env.resolve_pending_losses();
        assert!(matches!(
            env.state.terminal,
            Some(TerminalResult::Win { winner: 1 })
        ));

        env.state.terminal = None;
        env.state.turn.pending_losses = [true, true];
        env.config.end_condition_policy.simultaneous_loss = SimultaneousLossPolicy::Draw;
        env.config
            .end_condition_policy
            .allow_draw_on_simultaneous_loss = false;
        env.resolve_pending_losses();
        assert!(matches!(
            env.state.terminal,
            Some(TerminalResult::Win { winner: 0 })
        ));
    }
}
