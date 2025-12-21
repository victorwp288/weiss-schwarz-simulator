use std::sync::Arc;

use anyhow::{Result, anyhow};

use crate::config::{CurriculumConfig, EnvConfig, ErrorPolicy, RewardConfig};
use crate::db::{AbilityKind, AbilitySpec, AbilityTemplate, AbilityTemplateTag, CardDb, CardId, CardColor, CardStatic, CardType, TargetTemplate, TriggerIcon};
use crate::encode::{fill_action_mask, encode_observation, ACTION_ENCODING_VERSION, OBS_ENCODING_VERSION, OBS_LEN, MAX_ABILITIES_PER_CARD, MAX_STAGE};
use crate::events::{ChoiceOptionSummary, ChoiceSkipReason, Event, ModifierRemoveReason, RevealAudience, RevealReason, TriggerCancelReason, Zone};
use crate::legal::{ActionDesc, Decision, DecisionKind};
use crate::replay::{EpisodeBody, EpisodeHeader, ReplayConfig, ReplayData, ReplayWriter, StepMeta, ReplayFinal, ReplayEvent, REPLAY_SCHEMA_VERSION};
use crate::state::{AttackContext, AttackStep, AttackType, CardInstance, ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone, DamageModifier, DamageModifierKind, DamageType, EncoreRequest, GameState, ModifierDuration, ModifierKind, PendingTrigger, Phase, PendingTargetEffect, PriorityState, StackEffectKind, StackItem, StackOrderState, StageSlot, StageStatus, TargetRef, TargetSelectionState, TargetSide, TargetSlotFilter, TargetSpec, TargetZone, TerminalResult, TimingWindow, TriggerEffect, TriggerOrderState};
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
    pub last_illegal_action: bool,
    pub last_engine_error: bool,
    pub last_perspective: u8,
    pub pending_damage_delta: [i32; 2],
    pub obs_buf: Vec<i32>,
    pub replay_config: ReplayConfig,
    pub replay_writer: Option<ReplayWriter>,
    pub replay_actions: Vec<ActionDesc>,
    pub replay_events: Vec<ReplayEvent>,
    pub replay_steps: Vec<StepMeta>,
    pub recording: bool,
    pub meta_rng: Rng64,
    pub episode_seed: u64,
}

#[derive(Clone, Copy, Debug)]
struct DamageIntentLocal {
    source_player: u8,
    source_slot: Option<u8>,
    target: u8,
    amount: i32,
    damage_type: DamageType,
    cancelable: bool,
}

const MAX_CHOICE_OPTIONS: usize = crate::encode::CHOICE_COUNT;

#[derive(Clone, Copy)]
struct ActivatedAbilityHandler {
    tag: AbilityTemplateTag,
    queue: fn(&mut GameEnv, player: u8, source_id: CardId, slot: u8, ability_index: u8, template: &AbilityTemplate) -> Result<()>,
}

#[derive(Clone, Copy)]
struct ContinuousAbilityHandler {
    tag: AbilityTemplateTag,
    apply: fn(&mut GameEnv, player: u8, source_id: CardId, slot: u8, template: &AbilityTemplate),
}

const ACTIVATED_ABILITY_HANDLERS: &[ActivatedAbilityHandler] = &[
    ActivatedAbilityHandler {
        tag: AbilityTemplateTag::ActivatedPlaceholder,
        queue: GameEnv::queue_activated_placeholder,
    },
    ActivatedAbilityHandler {
        tag: AbilityTemplateTag::ActivatedTargetedPower,
        queue: GameEnv::queue_activated_targeted_power,
    },
    ActivatedAbilityHandler {
        tag: AbilityTemplateTag::ActivatedTargetedMoveToHand,
        queue: GameEnv::queue_activated_targeted_move_to_hand,
    },
    ActivatedAbilityHandler {
        tag: AbilityTemplateTag::ActivatedChangeController,
        queue: GameEnv::queue_activated_change_controller,
    },
];

const CONTINUOUS_ABILITY_HANDLERS: &[ContinuousAbilityHandler] = &[
    ContinuousAbilityHandler {
        tag: AbilityTemplateTag::ContinuousPower,
        apply: GameEnv::apply_continuous_power_modifier,
    },
];

impl GameEnv {
    pub fn add_modifier(&mut self, source: CardId, target_player: u8, target_slot: u8, kind: ModifierKind, magnitude: i32, duration: ModifierDuration) -> Option<u32> {
        self.add_modifier_instance(source, target_player, target_slot, kind, magnitude, duration)
    }

    pub fn new(db: Arc<CardDb>, config: EnvConfig, curriculum: CurriculumConfig, seed: u64, replay_config: ReplayConfig, replay_writer: Option<ReplayWriter>) -> Self {
        let starting_player = (seed as u8) & 1;
        let state = GameState::new(config.deck_lists[0].clone(), config.deck_lists[1].clone(), seed, starting_player);
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
            last_illegal_action: false,
            last_engine_error: false,
            last_perspective: 0,
            pending_damage_delta: [0, 0],
            obs_buf: vec![0; OBS_LEN],
            replay_config,
            replay_writer,
            replay_actions: Vec::new(),
            replay_events: Vec::new(),
            replay_steps: Vec::new(),
            recording: false,
            meta_rng: Rng64::new(seed ^ 0xABCDEF1234567890),
            episode_seed: seed,
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

    fn reset_with_obs(&mut self, copy_obs: bool) -> StepOutcome {
        let episode_seed = self.meta_rng.next_u64();
        let starting_player = if (episode_seed & 1) == 1 { 1 } else { 0 };
        self.episode_seed = episode_seed;
        self.state = GameState::new(self.config.deck_lists[0].clone(), self.config.deck_lists[1].clone(), episode_seed, starting_player);
        self.decision = None;
        if self.last_action_lookup.len() != crate::encode::ACTION_SPACE_SIZE {
            self.last_action_lookup.resize(crate::encode::ACTION_SPACE_SIZE, None);
        }
        for slot in self.last_action_lookup.iter_mut() {
            *slot = None;
        }
        if self.last_action_mask.len() != crate::encode::ACTION_SPACE_SIZE {
            self.last_action_mask.resize(crate::encode::ACTION_SPACE_SIZE, 0);
        }
        self.last_action_mask.fill(0);
        self.last_legal_actions.clear();
        self.last_action_desc = None;
        self.last_illegal_action = false;
        self.last_engine_error = false;
        self.last_perspective = self.state.turn.starting_player;
        self.pending_damage_delta = [0, 0];
        if self.obs_buf.len() != OBS_LEN {
            self.obs_buf.resize(OBS_LEN, 0);
        }
        self.replay_actions.clear();
        self.replay_events.clear();
        self.replay_steps.clear();
        self.recording = self.replay_config.enabled && self.meta_rng.next_u32() as f32 / u32::MAX as f32 <= self.replay_config.sample_rate;

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

    fn apply_action_id_internal(&mut self, action_id: usize, copy_obs: bool) -> Result<StepOutcome> {
        self.last_illegal_action = false;
        self.last_engine_error = false;
        if self.decision.is_none() {
            return Err(anyhow!("No pending decision"));
        }
        self.last_perspective = self.decision.as_ref().unwrap().player;
        let action = match self.last_action_lookup.get(action_id).and_then(|a| a.clone()) {
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
        let acting_player = self.decision.as_ref().map(|d| d.player).unwrap_or(self.last_perspective);
        self.last_perspective = acting_player;
        self.pending_damage_delta = [0, 0];
        let decision_kind = self.decision.as_ref().map(|d| d.kind).unwrap_or(DecisionKind::Main);
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
                    return self.handle_illegal_action(decision.player, "Action not in legal set", copy_obs);
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
                    self.state.terminal = Some(TerminalResult::Win { winner: 1 - acting_player });
                    self.decision = None;
                    self.update_action_cache();
                    Ok(self.build_outcome_with_obs(self.terminal_reward_for(acting_player), copy_obs))
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
            self.replay_actions.push(action_clone);
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
        let decision = self.decision.clone().ok_or_else(|| anyhow!("No decision to apply"))?;
        self.last_perspective = decision.player;
        self.last_action_desc = Some(action.clone());

        let mut reward = 0.0f32;

        match decision.kind {
            DecisionKind::Mulligan => {
                match action {
                    ActionDesc::MulliganKeep => {
                        self.state.turn.mulligan_done[decision.player as usize] = true;
                    }
                    ActionDesc::MulliganAll => {
                        let p = decision.player as usize;
                        let hand_len = self.state.players[p].hand.len();
                        let mut new_hand = Vec::with_capacity(hand_len);
                        let mut discarded: Vec<CardInstance> = Vec::new();
                        std::mem::swap(&mut discarded, &mut self.state.players[p].hand);
                        self.state.players[p].waiting_room.extend(discarded);
                        for _ in 0..hand_len {
                            if let Some(card) = self.draw_from_deck(p as u8) {
                                new_hand.push(card);
                                self.log_event(Event::Draw { player: p as u8, card: card.id });
                            }
                        }
                        self.state.players[p].hand = new_hand;
                        self.shuffle_deck(p as u8);
                        self.state.turn.mulligan_done[p] = true;
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid mulligan action", copy_obs),
                }
            }
            DecisionKind::Clock => {
                match action {
                    ActionDesc::ClockPass => {
                        self.log_event(Event::Clock { player: decision.player, card: None });
                    }
                    ActionDesc::Clock { hand_index } => {
                        let p = decision.player as usize;
                        let hi = hand_index as usize;
                        if hi >= self.state.players[p].hand.len() {
                            return self.handle_illegal_action(decision.player, "Clock hand index out of range", copy_obs);
                        }
                        let card = self.state.players[p].hand.remove(hi);
                        self.state.players[p].clock.push(card);
                        self.log_event(Event::Clock { player: decision.player, card: Some(card.id) });
                        self.draw_to_hand(decision.player, 2);
                        self.check_level_up(decision.player);
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid clock action", copy_obs),
                }
                self.state.turn.phase = Phase::Main;
            }
            DecisionKind::Main => {
                match action {
                    ActionDesc::MainPass => {
                        self.state.turn.main_passed = true;
                        if self.state.turn.priority.is_none() {
                            self.enter_timing_window(TimingWindow::MainWindow, decision.player);
                        }
                    }
                    ActionDesc::MainPlayCharacter { hand_index, stage_slot } => {
                        if let Err(err) = self.play_character(decision.player, hand_index, stage_slot) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                    }
                    ActionDesc::MainPlayEvent { hand_index } => {
                        if let Err(err) = self.play_event(decision.player, hand_index) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                    }
                    ActionDesc::MainMove { from_slot, to_slot } => {
                        let p = decision.player as usize;
                        let fs = from_slot as usize;
                        let ts = to_slot as usize;
                        if fs >= self.state.players[p].stage.len() || ts >= self.state.players[p].stage.len() || fs == ts {
                            return self.handle_illegal_action(decision.player, "Invalid move slots", copy_obs);
                        }
                        if self.state.players[p].stage[fs].card.is_none() || self.state.players[p].stage[ts].card.is_none() {
                            return self.handle_illegal_action(decision.player, "Move requires two occupied slots", copy_obs);
                        }
                        self.state.players[p].stage.swap(fs, ts);
                        self.remove_modifiers_for_slot(decision.player, from_slot);
                        self.remove_modifiers_for_slot(decision.player, to_slot);
                        if let Some(card) = self.state.players[p].stage[fs].card {
                            self.apply_continuous_modifiers_for_slot(decision.player, from_slot, card.id);
                        }
                        if let Some(card) = self.state.players[p].stage[ts].card {
                            self.apply_continuous_modifiers_for_slot(decision.player, to_slot, card.id);
                        }
                    }
                    ActionDesc::MainActivateAbility { slot, ability_index } => {
                        let _ = (slot, ability_index);
                        return self.handle_illegal_action(decision.player, "Activated abilities only via priority window", copy_obs);
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid main action", copy_obs),
                }
            }
            DecisionKind::Climax => {
                match action {
                    ActionDesc::ClimaxPass => {
                        self.state.turn.phase = Phase::Attack;
                    }
                    ActionDesc::ClimaxPlay { hand_index } => {
                        if let Err(err) = self.play_climax(decision.player, hand_index) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                        self.state.turn.phase = Phase::Attack;
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid climax action", copy_obs),
                }
            }
            DecisionKind::AttackDeclaration => {
                match action {
                    ActionDesc::AttackPass => {
                        if self.has_attackers(decision.player) {
                            return self.handle_illegal_action(decision.player, "Attack pass not allowed", copy_obs);
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
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid attack action", copy_obs),
                }
            }
            DecisionKind::Counter => {
                if self.state.turn.attack.is_none() {
                    return self.handle_illegal_action(decision.player, "No attack context for counter", copy_obs);
                }
                match action {
                    ActionDesc::CounterPass => {
                        if let Some(ctx) = &mut self.state.turn.attack {
                            ctx.step = AttackStep::Damage;
                        }
                    }
                    ActionDesc::CounterPlay { hand_index } => {
                        if let Err(err) = self.play_counter(decision.player, hand_index) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                        if let Some(ctx) = &mut self.state.turn.attack {
                            ctx.step = AttackStep::Damage;
                        }
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid counter action", copy_obs),
                }
            }
            DecisionKind::LevelUp => {
                match action {
                    ActionDesc::LevelUp { index } => {
                        if self.state.turn.pending_level_up != Some(decision.player) {
                            return self.handle_illegal_action(decision.player, "No pending level up", copy_obs);
                        }
                        if let Err(err) = self.resolve_level_up(decision.player, index) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid level up action", copy_obs),
                }
            }
            DecisionKind::Encore => {
                match action {
                    ActionDesc::EncoreYes => {
                        if let Err(err) = self.resolve_encore(decision.player, true) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                    }
                    ActionDesc::EncoreNo => {
                        if let Err(err) = self.resolve_encore(decision.player, false) {
                            return self.handle_illegal_action(decision.player, &err.to_string(), copy_obs);
                        }
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid encore action", copy_obs),
                }
            }
            DecisionKind::TriggerOrder => {
                let Some(order) = self.state.turn.trigger_order.clone() else {
                    return self.handle_illegal_action(decision.player, "No trigger order pending", copy_obs);
                };
                if order.player != decision.player {
                    return self.handle_illegal_action(decision.player, "Trigger order player mismatch", copy_obs);
                }
                match action {
                    ActionDesc::TriggerOrder { index } => {
                        let idx = index as usize;
                        if idx >= order.choices.len() {
                            return self.handle_illegal_action(decision.player, "Trigger order index out of range", copy_obs);
                        }
                        let trigger_id = order.choices[idx];
                        let trigger_index = self.state.turn.pending_triggers.iter().position(|t| t.id == trigger_id);
                        let Some(trigger_index) = trigger_index else {
                            return self.handle_illegal_action(decision.player, "Trigger already resolved", copy_obs);
                        };
                        let trigger = self.state.turn.pending_triggers.remove(trigger_index);
                        let _ = self.resolve_trigger(trigger);
                        self.state.turn.trigger_order = None;
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid trigger order action", copy_obs),
                }
            }
            DecisionKind::Choice => {
                let Some(choice) = self.state.turn.choice.take() else {
                    return self.handle_illegal_action(decision.player, "No choice pending", copy_obs);
                };
                if choice.player != decision.player {
                    return self.handle_illegal_action(decision.player, "Choice player mismatch", copy_obs);
                }
                match action {
                    ActionDesc::ChoiceSelect { index } => {
                        let idx = index as usize;
                        if idx >= choice.options.len() {
                            return self.handle_illegal_action(decision.player, "Choice index out of range", copy_obs);
                        }
                        let option = choice.options[idx];
                        self.log_event(Event::ChoiceMade { choice_id: choice.id, player: decision.player, option });
                        self.apply_choice_effect(choice.reason, choice.player, option, choice.pending_trigger);
                    }
                    _ => return self.handle_illegal_action(decision.player, "Invalid choice action", copy_obs),
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
        let RewardConfig { terminal_win, terminal_loss, terminal_draw, enable_shaping, damage_reward } = &self.config.reward;
        if let Some(term) = self.state.terminal {
            return match term {
                TerminalResult::Win { winner } => if winner == perspective { *terminal_win } else { *terminal_loss },
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
            if decision_kind == Some(DecisionKind::AttackDeclaration) && self.state.turn.derived_attack.is_none() {
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
            fill_action_mask(&actions, &mut self.last_action_mask, &mut self.last_action_lookup);
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
        use std::collections::HashMap;
        let mut errors = Vec::new();

        let mut counts: [HashMap<CardId, i32>; 2] = [HashMap::new(), HashMap::new()];
        for (owner, owner_counts) in counts.iter_mut().enumerate() {
            let deck_list = &self.config.deck_lists[owner];
            for card in deck_list.iter().copied() {
                *owner_counts.entry(card).or_insert(0) += 1;
            }
        }

        let mut consume = |owner: u8, card: CardId, zone: &str| {
            let owner_idx = owner as usize;
            let entry = counts[owner_idx].entry(card).or_insert(0);
            *entry -= 1;
            if *entry < 0 {
                errors.push(format!("Owner {owner} has extra card {card} in {zone}"));
            }
        };

        for zone_player in 0..2 {
            let p = &self.state.players[zone_player];
            for card in &p.deck {
                consume(card.owner, card.id, &format!("p{zone_player} deck"));
            }
            for card in &p.hand {
                consume(card.owner, card.id, &format!("p{zone_player} hand"));
            }
            for card in &p.waiting_room {
                consume(card.owner, card.id, &format!("p{zone_player} waiting_room"));
            }
            for card in &p.clock {
                consume(card.owner, card.id, &format!("p{zone_player} clock"));
            }
            for card in &p.level {
                consume(card.owner, card.id, &format!("p{zone_player} level"));
            }
            for card in &p.stock {
                consume(card.owner, card.id, &format!("p{zone_player} stock"));
            }
            for card in &p.memory {
                consume(card.owner, card.id, &format!("p{zone_player} memory"));
            }
            for card in &p.climax {
                consume(card.owner, card.id, &format!("p{zone_player} climax"));
            }
            for (slot_idx, slot) in p.stage.iter().enumerate() {
                if let Some(card) = slot.card {
                    consume(card.owner, card.id, &format!("p{zone_player} stage[{slot_idx}]"));
                }
            }
        }

        for (owner, owner_counts) in counts.iter().enumerate() {
            for (card, remaining) in owner_counts.iter() {
                if *remaining != 0 {
                    errors.push(format!("Owner {owner} card {card} count mismatch ({remaining})"));
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
                DecisionKind::Counter => {
                    if let Some(ctx) = &self.state.turn.attack {
                        if ctx.step != AttackStep::Counter {
                            errors.push("Counter decision without counter step".to_string());
                        }
                    } else {
                        errors.push("Counter decision without attack context".to_string());
                    }
                }
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

        let state_hash = crate::util::hash_value(&self.state);
        let phase = self.state.turn.phase;
        let attack_step = self.state.turn.attack.as_ref().map(|c| c.step);
        let tail_len = 8usize;
        let actions_tail: Vec<String> = self.replay_actions.iter().rev().take(tail_len).rev().map(|a| format!("{a:?}")).collect();
        let decisions_tail: Vec<String> = self.replay_steps.iter().rev().take(tail_len).rev().map(|s| format!("{:?}/{:?}", s.decision_kind, s.actor)).collect();
        let fallback_action = self.last_action_desc.as_ref().map(|a| format!("{a:?}")).unwrap_or_else(|| "None".to_string());
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
        let perspective = self.decision.as_ref().map(|d| d.player).unwrap_or(self.last_perspective);
        encode_observation(
            &self.state,
            &self.db,
            &self.curriculum,
            perspective,
            self.decision.as_ref(),
            self.last_action_desc.as_ref(),
            self.config.observation_visibility,
            &mut self.obs_buf,
        );
        let obs = if copy_obs { self.obs_buf.clone() } else { Vec::new() };
        let info = EnvInfo {
            obs_version: OBS_ENCODING_VERSION,
            action_version: ACTION_ENCODING_VERSION,
            decision_kind: self.decision.as_ref().map(|d| match d.kind {
                DecisionKind::Mulligan => 0,
                DecisionKind::Clock => 1,
                DecisionKind::Main => 2,
                DecisionKind::Climax => 3,
                DecisionKind::AttackDeclaration => 4,
                DecisionKind::Counter => 5,
                DecisionKind::LevelUp => 6,
                DecisionKind::Encore => 7,
                DecisionKind::TriggerOrder => 8,
                DecisionKind::Choice => 9,
            }).unwrap_or(-1),
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
        loop {
            if self.state.terminal.is_some() {
                break;
            }
            if self.decision.is_some() {
                break;
            }
            if self.state.turn.tick_count >= self.config.max_ticks {
                self.state.terminal = Some(TerminalResult::Timeout);
                break;
            }
            self.state.turn.tick_count += 1;

            if let Some(player) = self.state.turn.pending_level_up {
                self.decision = Some(Decision { player, kind: DecisionKind::LevelUp, focus_slot: None });
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

            if let Some(req) = self.state.turn.encore_queue.first().copied() {
                self.decision = Some(Decision { player: req.player, kind: DecisionKind::Encore, focus_slot: Some(req.slot) });
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
                    let next = if !self.state.turn.mulligan_done[sp] { sp } else { 1 - sp };
                    self.decision = Some(Decision { player: next as u8, kind: DecisionKind::Mulligan, focus_slot: None });
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
                    self.state.turn.phase = if self.curriculum.enable_clock_phase { Phase::Clock } else { Phase::Main };
                }
                Phase::Clock => {
                    if !self.curriculum.enable_clock_phase {
                        self.state.turn.phase = Phase::Main;
                        continue;
                    }
                    let p = self.state.turn.active_player;
                    self.decision = Some(Decision { player: p, kind: DecisionKind::Clock, focus_slot: None });
                    break;
                }
                Phase::Main => {
                    let p = self.state.turn.active_player;
                    self.decision = Some(Decision { player: p, kind: DecisionKind::Main, focus_slot: None });
                    break;
                }
                Phase::Climax => {
                    if !self.curriculum.enable_climax_phase {
                        self.state.turn.phase = Phase::Attack;
                        continue;
                    }
                    let p = self.state.turn.active_player;
                    self.decision = Some(Decision { player: p, kind: DecisionKind::Climax, focus_slot: None });
                    break;
                }
                Phase::Attack => {
                    if self.state.turn.attack.is_none() {
                        let p = self.state.turn.active_player;
                        self.recompute_derived_attack();
                        self.decision = Some(Decision { player: p, kind: DecisionKind::AttackDeclaration, focus_slot: None });
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
            self.decision = Some(Decision { player: choice.player, kind: DecisionKind::Choice, focus_slot: None });
            self.maybe_validate_state("choice_decision");
            return true;
        }
        if self.state.turn.pending_triggers.is_empty() {
            self.state.turn.trigger_order = None;
            return false;
        }

        if let Some(order) = &self.state.turn.trigger_order {
            self.decision = Some(Decision { player: order.player, kind: DecisionKind::TriggerOrder, focus_slot: None });
            self.maybe_validate_state("trigger_order_decision");
            return true;
        }

        let group_id = match self.state.turn.pending_triggers.iter().map(|t| t.group_id).min() {
            Some(id) => id,
            None => return false,
        };
        let active = self.state.turn.active_player;
        for player in [active, 1 - active] {
            let mut choices: Vec<u32> = self.state.turn.pending_triggers
                .iter()
                .filter(|t| t.group_id == group_id && t.player == player)
                .map(|t| t.id)
                .collect();
            if choices.len() > 1 {
                choices.sort_unstable();
                self.state.turn.trigger_order = Some(TriggerOrderState { group_id, player, choices });
                self.decision = Some(Decision { player, kind: DecisionKind::TriggerOrder, focus_slot: None });
                self.maybe_validate_state("trigger_order_decision");
                return true;
            }
            if choices.len() == 1 {
                let trigger_id = choices[0];
                if let Some(index) = self.state.turn.pending_triggers.iter().position(|t| t.id == trigger_id) {
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
        let Some(priority) = self.state.turn.priority.clone() else { return false; };
        if self.decision.is_some() {
            return true;
        }
        let actions = self.collect_priority_actions(priority.holder);
        if actions.is_empty() {
            self.priority_pass(priority.holder);
            return true;
        }
        if actions.len() == 1 {
            let action = actions[0].clone();
            let _ = self.apply_priority_action(priority.holder, action);
            return true;
        }
        self.start_priority_choice(priority.holder, actions);
        true
    }

    fn allocate_trigger_group(&mut self) -> u32 {
        let group_id = self.state.turn.next_trigger_group_id;
        self.state.turn.next_trigger_group_id = self.state.turn.next_trigger_group_id.wrapping_add(1);
        group_id
    }

    fn allocate_choice_id(&mut self) -> u32 {
        let choice_id = self.state.turn.next_choice_id;
        self.state.turn.next_choice_id = self.state.turn.next_choice_id.wrapping_add(1);
        choice_id
    }

    fn allocate_stack_id(&mut self) -> u32 {
        let stack_id = self.state.turn.next_stack_id;
        self.state.turn.next_stack_id = self.state.turn.next_stack_id.wrapping_add(1);
        stack_id
    }

    fn allocate_stack_group_id(&mut self) -> u32 {
        let group_id = self.state.turn.next_stack_group_id;
        self.state.turn.next_stack_group_id = self.state.turn.next_stack_group_id.wrapping_add(1);
        group_id
    }

    fn choice_option_id(&self, option: &ChoiceOptionRef) -> u64 {
        let zone_id = match option.zone {
            ChoiceZone::WaitingRoom => 1u64,
            ChoiceZone::Stage => 2u64,
            ChoiceZone::DeckTop => 3u64,
            ChoiceZone::Stack => 4u64,
            ChoiceZone::PriorityCounter => 5u64,
            ChoiceZone::PriorityAct => 6u64,
        };
        let index = option.index.unwrap_or(0) as u64;
        let target = option.target_slot.unwrap_or(0) as u64;
        (option.card_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
    }

    fn choice_option_label(&self, option: &ChoiceOptionRef) -> String {
        match option.zone {
            ChoiceZone::WaitingRoom => {
                let idx = option.index.unwrap_or(0);
                if let Some(slot) = option.target_slot {
                    format!("WR[{idx}] -> ST[{slot}] card {}", option.card_id)
                } else {
                    format!("WR[{idx}] card {}", option.card_id)
                }
            }
            ChoiceZone::Stage => {
                let slot = option.index.unwrap_or(0);
                format!("ST[{slot}] card {}", option.card_id)
            }
            ChoiceZone::DeckTop => {
                match option.index.unwrap_or(0) {
                    0 => "Treasure: Stock top card".to_string(),
                    _ => "Treasure: Skip".to_string(),
                }
            }
            ChoiceZone::Stack => {
                let idx = option.index.unwrap_or(0);
                format!("Stack order [{idx}] card {}", option.card_id)
            }
            ChoiceZone::PriorityCounter => {
                let idx = option.index.unwrap_or(0);
                format!("Counter hand[{idx}] card {}", option.card_id)
            }
            ChoiceZone::PriorityAct => {
                let slot = option.index.unwrap_or(0);
                let ability = option.target_slot.unwrap_or(0);
                format!("Act ST[{slot}] ability {ability} card {}", option.card_id)
            }
        }
    }

    fn summarize_choice_options(&self, options: &[ChoiceOptionRef]) -> Vec<ChoiceOptionSummary> {
        options
            .iter()
            .map(|opt| ChoiceOptionSummary {
                option_id: self.choice_option_id(opt),
                label: self.choice_option_label(opt),
                reference: *opt,
            })
            .collect()
    }

    fn start_choice(&mut self, reason: ChoiceReason, player: u8, mut candidates: Vec<ChoiceOptionRef>, pending_trigger: Option<PendingTrigger>) -> bool {
        let total = candidates.len();
        let choice_id = self.allocate_choice_id();
        if total == 0 {
            self.log_event(Event::ChoiceSkipped { choice_id, player, reason, skip_reason: ChoiceSkipReason::NoCandidates });
            if let Some(trigger) = pending_trigger {
                self.log_event(Event::TriggerResolved { trigger_id: trigger.id, player: trigger.player, effect: trigger.effect });
            }
            return false;
        }
        if total == 1 {
            let option = candidates[0];
            self.log_event(Event::ChoiceAutopicked { choice_id, player, option });
            self.apply_choice_effect(reason, player, option, pending_trigger);
            return false;
        }
        if candidates.len() > MAX_CHOICE_OPTIONS {
            candidates.truncate(MAX_CHOICE_OPTIONS);
        }
        let summaries = self.summarize_choice_options(&candidates);
        let total_candidates = total.min(u16::MAX as usize) as u16;
        self.log_event(Event::ChoicePresented { choice_id, player, reason, options: summaries, total_candidates });
        self.state.turn.choice = Some(ChoiceState {
            id: choice_id,
            reason,
            player,
            options: candidates,
            total_candidates,
            pending_trigger,
        });
        true
    }

    fn apply_choice_effect(&mut self, reason: ChoiceReason, player: u8, option: ChoiceOptionRef, pending_trigger: Option<PendingTrigger>) {
        match reason {
            ChoiceReason::TriggerGateSelect => {
                self.move_waiting_room_to_hand(player, option);
            }
            ChoiceReason::TriggerBounceSelect => {
                self.move_stage_to_hand(player, option);
            }
            ChoiceReason::TriggerStandbySelect => {
                self.move_waiting_room_to_stage_standby(player, option);
            }
            ChoiceReason::TriggerTreasureSelect => {
                self.apply_treasure_choice(player, option);
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
            self.log_event(Event::TriggerResolved { trigger_id: trigger.id, player: trigger.player, effect: trigger.effect });
        }
    }

    fn target_spec_from_template(&self, template: TargetTemplate, count: u8) -> TargetSpec {
        match template {
            TargetTemplate::OppFrontRow => TargetSpec {
                zone: TargetZone::Stage,
                side: TargetSide::Opponent,
                slot_filter: TargetSlotFilter::FrontRow,
                card_type: Some(CardType::Character),
                count,
            },
            TargetTemplate::SelfStage => TargetSpec {
                zone: TargetZone::Stage,
                side: TargetSide::SelfSide,
                slot_filter: TargetSlotFilter::Any,
                card_type: Some(CardType::Character),
                count,
            },
            TargetTemplate::SelfWaitingRoom => TargetSpec {
                zone: TargetZone::WaitingRoom,
                side: TargetSide::SelfSide,
                slot_filter: TargetSlotFilter::Any,
                card_type: None,
                count,
            },
        }
    }

    fn start_target_selection(&mut self, controller: u8, source_id: CardId, spec: TargetSpec, effect: PendingTargetEffect) {
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

    fn enumerate_target_candidates(&self, controller: u8, spec: &TargetSpec, selected: &[TargetRef]) -> Vec<TargetRef> {
        let target_player = match spec.side {
            TargetSide::SelfSide => controller,
            TargetSide::Opponent => 1 - controller,
        };
        let mut candidates = Vec::new();
        match spec.zone {
            TargetZone::Stage => {
                let max_slot = if self.curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
                // Deterministic target ordering: stage slot ascending (front row is slots 0..2, then back row).
                for slot in 0..max_slot {
                    if spec.slot_filter == TargetSlotFilter::FrontRow && slot >= 3 {
                        continue;
                    }
                    let slot_state = &self.state.players[target_player as usize].stage[slot];
                    let Some(card_inst) = slot_state.card else { continue; };
                    let Some(card) = self.db.get(card_inst.id) else { continue; };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| t.player == target_player && t.zone == TargetZone::Stage && t.index as usize == slot) {
                        continue;
                    }
                    let index = slot as u8;
                    candidates.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Stage,
                        index,
                        card_id: card_inst.id,
                    });
                }
            }
            TargetZone::WaitingRoom => {
                // Deterministic target ordering: waiting room index ascending.
                for (idx, card_inst) in self.state.players[target_player as usize].waiting_room.iter().copied().enumerate() {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = self.db.get(card_inst.id) else { continue; };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| t.player == target_player && t.zone == TargetZone::WaitingRoom && t.index as usize == idx) {
                        continue;
                    }
                    candidates.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::WaitingRoom,
                        index: idx as u8,
                        card_id: card_inst.id,
                    });
                }
            }
        }
        candidates
    }

    fn present_target_choice(&mut self) {
        let Some(selection) = &self.state.turn.target_selection else { return; };
        let candidates = self.enumerate_target_candidates(selection.controller, &selection.spec, &selection.selected);
        if candidates.is_empty() {
            let _ = self.start_choice(ChoiceReason::TargetSelect, selection.controller, Vec::new(), None);
            self.state.turn.target_selection = None;
            return;
        }
        let mut options = Vec::new();
        for target in candidates {
            let zone = match target.zone {
                TargetZone::Stage => ChoiceZone::Stage,
                TargetZone::WaitingRoom => ChoiceZone::WaitingRoom,
            };
            options.push(ChoiceOptionRef {
                card_id: target.card_id,
                zone,
                index: Some(target.index),
                target_slot: None,
            });
        }
        let _ = self.start_choice(ChoiceReason::TargetSelect, selection.controller, options, None);
    }

    fn apply_target_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let Some(mut selection) = self.state.turn.target_selection.take() else { return; };
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
                        .map(|c| c.id)
                        == Some(option.card_id)
                }
            }
            TargetZone::WaitingRoom => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].waiting_room.len() {
                    false
                } else {
                    self.state.players[target_player as usize].waiting_room[idx].id == option.card_id
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
        };
        if selection.selected.iter().any(|t| t.player == target.player && t.zone == target.zone && t.index == target.index) {
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
                PendingTargetEffect::StackTargetedPower { magnitude, duration } => {
                    let item = StackItem {
                        id: self.allocate_stack_id(),
                        controller: selection.controller,
                        source_id: selection.source_id,
                        effect: StackEffectKind::TargetedPower { targets, magnitude, duration },
                    };
                    self.enqueue_stack_items(vec![item]);
                }
                PendingTargetEffect::StackMoveToHand => {
                    let item = StackItem {
                        id: self.allocate_stack_id(),
                        controller: selection.controller,
                        source_id: selection.source_id,
                        effect: StackEffectKind::TargetedMoveToHand { targets },
                    };
                    self.enqueue_stack_items(vec![item]);
                }
                PendingTargetEffect::StackChangeController { new_controller } => {
                    let item = StackItem {
                        id: self.allocate_stack_id(),
                        controller: selection.controller,
                        source_id: selection.source_id,
                        effect: StackEffectKind::TargetedChangeController { targets, new_controller },
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
        self.log_event(Event::TimingWindowEntered { window, player: holder });
    }

    fn collect_priority_actions(&self, player: u8) -> Vec<ActionDesc> {
        let mut actions = Vec::new();
        let Some(priority) = self.state.turn.priority.as_ref() else { return actions; };
        if priority.holder != player {
            return actions;
        }
        match priority.window {
            TimingWindow::MainWindow => {
                if !self.curriculum.enable_activated_abilities {
                    return actions;
                }
                let p = &self.state.players[player as usize];
                let max_slot = if self.curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
                // Deterministic priority ordering: stage slot ascending, then ability index ascending.
                for slot in 0..max_slot {
                    let slot_state = &p.stage[slot];
                    let Some(card_inst) = slot_state.card else { continue; };
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_none() {
                        continue;
                    }
                    let specs = self.db.ability_specs(card_id);
                    for (idx, spec) in specs.iter().enumerate() {
                        if idx >= MAX_ABILITIES_PER_CARD || idx > u8::MAX as usize {
                            break;
                        }
                        if spec.kind != AbilityKind::Activated {
                            continue;
                        }
                        let template = &spec.template;
                        if Self::activated_handler_for(template).is_none() {
                            continue;
                        }
                        let bit = (slot * MAX_ABILITIES_PER_CARD + idx) as u32;
                        if priority.used_act_mask & (1u32 << bit) != 0 {
                            continue;
                        }
                        actions.push(ActionDesc::MainActivateAbility { slot: slot as u8, ability_index: idx as u8 });
                    }
                }
            }
            TimingWindow::CounterWindow => {
                let Some(ctx) = &self.state.turn.attack else { return actions; };
                if ctx.attack_type != AttackType::Frontal || ctx.defender_slot.is_none() || ctx.counter_played {
                    return actions;
                }
                if self.curriculum.enable_counters {
                    let p = &self.state.players[player as usize];
                    // Deterministic priority ordering: hand index ascending.
                    for (hand_index, card_inst) in p.hand.iter().enumerate() {
                        if hand_index >= crate::encode::MAX_HAND || hand_index > u8::MAX as usize {
                            break;
                        }
                        let Some(card) = self.db.get(card_inst.id) else { continue; };
                        if !self.card_set_allowed(card) {
                            continue;
                        }
                        if self.is_counter_card(card)
                            && self.meets_level_requirement(player, card)
                            && self.meets_color_requirement(player, card)
                            && self.meets_cost_requirement(player, card)
                        {
                            actions.push(ActionDesc::CounterPlay { hand_index: hand_index as u8 });
                        }
                    }
                }
            }
            TimingWindow::EndOfTurnWindow => {}
        }
        actions
    }

    fn start_priority_choice(&mut self, player: u8, actions: Vec<ActionDesc>) {
        let mut options = Vec::new();
        for action in actions {
            match action {
                ActionDesc::CounterPlay { hand_index } => {
                    let card_id = self.state.players[player as usize]
                        .hand
                        .get(hand_index as usize)
                        .map(|c| c.id)
                        .unwrap_or(0);
                    options.push(ChoiceOptionRef {
                        card_id,
                        zone: ChoiceZone::PriorityCounter,
                        index: Some(hand_index),
                        target_slot: None,
                    });
                }
                ActionDesc::MainActivateAbility { slot, ability_index } => {
                    let card_id = self.state.players[player as usize]
                        .stage
                        .get(slot as usize)
                        .and_then(|s| s.card)
                        .map(|c| c.id)
                        .unwrap_or(0);
                    options.push(ChoiceOptionRef {
                        card_id,
                        zone: ChoiceZone::PriorityAct,
                        index: Some(slot),
                        target_slot: Some(ability_index),
                    });
                }
                _ => {}
            }
        }
        self.start_choice(ChoiceReason::PriorityActionSelect, player, options, None);
    }

    fn apply_priority_action_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let action = match option.zone {
            ChoiceZone::PriorityCounter => option.index.map(|idx| ActionDesc::CounterPlay { hand_index: idx }),
            ChoiceZone::PriorityAct => {
                if let (Some(slot), Some(ability)) = (option.index, option.target_slot) {
                    Some(ActionDesc::MainActivateAbility { slot, ability_index: ability })
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
            ActionDesc::MainActivateAbility { slot, ability_index } => {
                if window != TimingWindow::MainWindow {
                    return Err(anyhow!("Activated abilities not allowed in this window"));
                }
                self.queue_activated_ability_stack_item(player, slot, ability_index)?;
                let bit = slot as u32 * MAX_ABILITIES_PER_CARD as u32 + ability_index as u32;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.used_act_mask |= 1u32 << bit;
                    priority.holder = 1 - player;
                    priority.passes = 0;
                }
            }
            ActionDesc::CounterPlay { hand_index } => {
                if window != TimingWindow::CounterWindow {
                    return Err(anyhow!("Counter play not allowed in this window"));
                }
                self.queue_counter_stack_item(player, hand_index)?;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.holder = 1 - player;
                    priority.passes = 0;
                }
            }
            ActionDesc::MainPass | ActionDesc::CounterPass => {
                let actions = self.collect_priority_actions(player);
                if !actions.is_empty() {
                    return Err(anyhow!("Explicit pass not allowed when priority actions exist"));
                }
                self.priority_pass(player);
            }
            _ => return Err(anyhow!("Invalid priority action")),
        }
        Ok(())
    }

    fn priority_pass(&mut self, player: u8) {
        let (window, pass_count, should_check_stack) = {
            let Some(priority) = &mut self.state.turn.priority else { return; };
            if priority.holder != player {
                return;
            }
            priority.passes = priority.passes.saturating_add(1);
            let window = priority.window;
            let pass_count = priority.passes;
            if pass_count < 2 {
                priority.holder = 1 - player;
            }
            (window, pass_count, pass_count >= 2)
        };
        self.log_event(Event::PriorityPassed { player, window, pass_count });
        if should_check_stack {
            if let Some(item) = self.state.turn.stack.pop() {
                self.resolve_stack_item(&item);
                self.log_event(Event::StackResolved { item });
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.passes = 0;
                    priority.holder = self.state.turn.active_player;
                }
            } else {
                self.close_priority_window(window);
            }
        }
    }

    fn close_priority_window(&mut self, window: TimingWindow) {
        self.state.turn.priority = None;
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
            TimingWindow::EndOfTurnWindow => {}
        }
    }

    fn stack_effect_key(effect: &StackEffectKind) -> u8 {
        match effect {
            StackEffectKind::ActivatedPlaceholder { .. } => 0,
            StackEffectKind::Counter { .. } => 1,
            StackEffectKind::TargetedPower { .. } => 2,
            StackEffectKind::TargetedMoveToHand { .. } => 3,
            StackEffectKind::TargetedChangeController { .. } => 4,
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
            list.sort_by_key(|item| (item.source_id, Self::stack_effect_key(&item.effect), item.id));
            let group_id = self.allocate_stack_group_id();
            let items = std::mem::take(list);
            let group = StackOrderState { group_id, controller, items };
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
        self.log_event(Event::StackGroupPresented { group_id: group.group_id, controller: group.controller, items: group.items.clone() });
        self.state.turn.stack_order = Some(group);
        self.present_stack_order_choice();
    }

    fn present_stack_order_choice(&mut self) {
        let Some(order) = &self.state.turn.stack_order else { return; };
        let mut options = Vec::new();
        for (idx, item) in order.items.iter().enumerate() {
            let index = if idx <= u8::MAX as usize { Some(idx as u8) } else { None };
            options.push(ChoiceOptionRef {
                card_id: item.source_id,
                zone: ChoiceZone::Stack,
                index,
                target_slot: None,
            });
        }
        self.start_choice(ChoiceReason::StackOrderSelect, order.controller, options, None);
    }

    fn apply_stack_order_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stack {
            return;
        }
        let Some(idx) = option.index else { return; };
        let Some(mut order) = self.state.turn.stack_order.take() else { return; };
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
        self.log_event(Event::StackOrderChosen { group_id: order.group_id, controller: order.controller, stack_id: item.id });
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
        match &item.effect {
            StackEffectKind::ActivatedPlaceholder { slot, .. } => {
                let p = item.controller as usize;
                let s = *slot as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.id) != Some(item.source_id) {
                    return;
                }
                let _ = self.add_modifier(item.source_id, item.controller, *slot, ModifierKind::Power, 1000, ModifierDuration::UntilEndOfTurn);
            }
            StackEffectKind::Counter { card_id, power, damage_reduce, damage_cancel } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    if let Some(def_slot) = ctx.defender_slot {
                        let slot_state = &mut self.state.players[item.controller as usize].stage[def_slot as usize];
                        slot_state.power_mod_battle += *power;
                        ctx.counter_power += *power;
                    }
                    if *damage_reduce > 0 {
                        Self::push_attack_damage_modifier(ctx, DamageModifierKind::AddAmount { delta: -(*damage_reduce) }, *card_id);
                    }
                    if *damage_cancel {
                        Self::push_attack_damage_modifier(ctx, DamageModifierKind::CancelNext, *card_id);
                    }
                }
                self.log_event(Event::Counter { player: item.controller, card: *card_id, power: *power });
            }
            StackEffectKind::TargetedPower { targets, magnitude, duration } => {
                for target in targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let s = target.index as usize;
                    if s >= self.state.players[p].stage.len() {
                        continue;
                    }
                    if self.state.players[p].stage[s].card.map(|c| c.id) != Some(target.card_id) {
                        continue;
                    }
                    let _ = self.add_modifier(item.source_id, target.player, target.index, ModifierKind::Power, *magnitude, *duration);
                }
            }
            StackEffectKind::TargetedMoveToHand { targets } => {
                let mut waiting_room_targets: Vec<TargetRef> = Vec::new();
                for target in targets {
                    match target.zone {
                        TargetZone::Stage => {
                            let option = ChoiceOptionRef {
                                card_id: target.card_id,
                                zone: ChoiceZone::Stage,
                                index: Some(target.index),
                                target_slot: None,
                            };
                            self.move_stage_to_hand(target.player, option);
                        }
                        TargetZone::WaitingRoom => {
                            waiting_room_targets.push(*target);
                        }
                    }
                }
                // Remove waiting room targets in descending index order to avoid index shifts.
                waiting_room_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in waiting_room_targets {
                    let option = ChoiceOptionRef {
                        card_id: target.card_id,
                        zone: ChoiceZone::WaitingRoom,
                        index: Some(target.index),
                        target_slot: None,
                    };
                    self.move_waiting_room_to_hand(target.player, option);
                }
            }
            StackEffectKind::TargetedChangeController { targets, new_controller } => {
                for target in targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let from_player = target.player;
                    let to_player = *new_controller;
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
                    if self.state.players[to_player as usize].stage[to_slot].card.is_some() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[from_player as usize].stage[from_slot].card else {
                        continue;
                    };
                    if card_inst.id != target.card_id {
                        continue;
                    }
                    self.remove_modifiers_for_slot(from_player, target.index);
                    let mut moved_slot = std::mem::replace(&mut self.state.players[from_player as usize].stage[from_slot], StageSlot::empty());
                    let mut moved_card = moved_slot.card.take().expect("card present");
                    moved_card.controller = to_player;
                    moved_slot.card = Some(moved_card);
                    self.state.players[to_player as usize].stage[to_slot] = moved_slot;
                    self.apply_continuous_modifiers_for_slot(to_player, target.index, moved_card.id);
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
        }
    }

    fn activated_handler_for(template: &AbilityTemplate) -> Option<&'static ActivatedAbilityHandler> {
        let tag = template.tag();
        ACTIVATED_ABILITY_HANDLERS.iter().find(|h| h.tag == tag)
    }

    fn continuous_handler_for(template: &AbilityTemplate) -> Option<&'static ContinuousAbilityHandler> {
        let tag = template.tag();
        CONTINUOUS_ABILITY_HANDLERS.iter().find(|h| h.tag == tag)
    }

    fn queue_activated_placeholder(&mut self, player: u8, source_id: CardId, slot: u8, ability_index: u8, _template: &AbilityTemplate) -> Result<()> {
        let item = StackItem {
            id: self.allocate_stack_id(),
            controller: player,
            source_id,
            effect: StackEffectKind::ActivatedPlaceholder { slot, ability_index },
        };
        self.enqueue_stack_items(vec![item]);
        Ok(())
    }

    fn queue_activated_targeted_power(&mut self, player: u8, source_id: CardId, _slot: u8, _ability_index: u8, template: &AbilityTemplate) -> Result<()> {
        let AbilityTemplate::ActivatedTargetedPower { amount, count, target } = template else {
            return Err(anyhow!("Ability template mismatch"));
        };
        if *count == 0 {
            return Ok(());
        }
        let spec = self.target_spec_from_template(*target, *count);
        self.start_target_selection(player, source_id, spec, PendingTargetEffect::StackTargetedPower {
            magnitude: *amount,
            duration: ModifierDuration::UntilEndOfTurn,
        });
        Ok(())
    }

    fn queue_activated_targeted_move_to_hand(&mut self, player: u8, source_id: CardId, _slot: u8, _ability_index: u8, template: &AbilityTemplate) -> Result<()> {
        let AbilityTemplate::ActivatedTargetedMoveToHand { count, target } = template else {
            return Err(anyhow!("Ability template mismatch"));
        };
        if *count == 0 {
            return Ok(());
        }
        let spec = self.target_spec_from_template(*target, *count);
        self.start_target_selection(player, source_id, spec, PendingTargetEffect::StackMoveToHand);
        Ok(())
    }

    fn queue_activated_change_controller(&mut self, player: u8, source_id: CardId, _slot: u8, _ability_index: u8, template: &AbilityTemplate) -> Result<()> {
        let AbilityTemplate::ActivatedChangeController { count, target } = template else {
            return Err(anyhow!("Ability template mismatch"));
        };
        if *count == 0 {
            return Ok(());
        }
        let spec = self.target_spec_from_template(*target, *count);
        self.start_target_selection(player, source_id, spec, PendingTargetEffect::StackChangeController { new_controller: player });
        Ok(())
    }

    fn apply_continuous_power_modifier(&mut self, player: u8, source_id: CardId, slot: u8, template: &AbilityTemplate) {
        let AbilityTemplate::ContinuousPower { amount } = template else { return; };
        if *amount == 0 {
            return;
        }
        let _ = self.add_modifier(source_id, player, slot, ModifierKind::Power, *amount, ModifierDuration::WhileOnStage);
    }

    fn apply_continuous_modifiers_for_slot(&mut self, player: u8, slot: u8, card_id: CardId) {
        if !self.curriculum.enable_continuous_modifiers {
            return;
        }
        let specs_len = self.db.ability_specs(card_id).len();
        for idx in 0..specs_len {
            let spec = self.db.ability_specs(card_id)[idx].clone();
            if spec.kind != AbilityKind::Continuous {
                continue;
            }
            if let Some(handler) = Self::continuous_handler_for(&spec.template) {
                (handler.apply)(self, player, card_id, slot, &spec.template);
            }
        }
    }

    fn queue_activated_ability_stack_item(&mut self, player: u8, slot: u8, ability_index: u8) -> Result<()> {
        if !self.curriculum.enable_activated_abilities {
            return Err(anyhow!("Activated abilities disabled"));
        }
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return Err(anyhow!("Ability slot out of range"));
        }
        let card_inst = self.state.players[p].stage[s].card.ok_or_else(|| anyhow!("No card in ability slot"))?;
        let card_id = card_inst.id;
        if self.db.get(card_id).is_none() {
            return Err(anyhow!("Card missing in db"));
        }
        let idx = ability_index as usize;
        let specs_len = self.db.ability_specs(card_id).len();
        if idx >= specs_len || idx >= MAX_ABILITIES_PER_CARD {
            return Err(anyhow!("Ability index out of range"));
        }
        let spec = self.db.ability_specs(card_id)[idx].clone();
        if spec.kind != AbilityKind::Activated {
            return Err(anyhow!("Ability is not activated"));
        }
        let template = &spec.template;
        let Some(handler) = Self::activated_handler_for(template) else {
            return Err(anyhow!("Unsupported activated ability"));
        };
        (handler.queue)(self, player, card_id, slot, ability_index, template)
    }

    fn queue_counter_stack_item(&mut self, player: u8, hand_index: u8) -> Result<()> {
        if !self.curriculum.enable_counters {
            return Err(anyhow!("Counters disabled"));
        }
        let Some(ctx) = &self.state.turn.attack else {
            return Err(anyhow!("No attack context for counter"));
        };
        if ctx.attack_type != AttackType::Frontal || ctx.defender_slot.is_none() || ctx.counter_played {
            return Err(anyhow!("Counter not allowed for this attack"));
        }
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Counter hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card = self.db.get(card_inst.id).ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.is_counter_card(card) {
            return Err(anyhow!("Card is not a counter"));
        }
        if !self.meets_level_requirement(player, card) || !self.meets_color_requirement(player, card) || !self.meets_cost_requirement(player, card) {
            return Err(anyhow!("Counter requirements not met"));
        }
        let power = self.counter_power(card);
        let damage_reduce = self.counter_damage_reductions(card).iter().sum::<i32>();
        let damage_cancel = self.counter_damage_cancel(card);
        self.pay_cost(player, card.cost as usize)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.state.players[p].waiting_room.push(card_inst);
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.counter_played = true;
        }
        let item = StackItem {
            id: self.allocate_stack_id(),
            controller: player,
            source_id: card_inst.id,
            effect: StackEffectKind::Counter {
                card_id: card_inst.id,
                power,
                damage_reduce,
                damage_cancel,
            },
        };
        self.enqueue_stack_items(vec![item]);
        Ok(())
    }

    fn enumerate_open_stage_slots(&self, player: u8) -> Vec<u8> {
        let p = player as usize;
        let max_slot = if self.curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
        let mut slots = Vec::new();
        for slot in 0..max_slot {
            if self.state.players[p].stage[slot].card.is_none() {
                slots.push(slot as u8);
            }
        }
        slots
    }

    fn enumerate_stage_characters(&self, player: u8) -> Vec<ChoiceOptionRef> {
        let p = player as usize;
        let max_slot = if self.curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
        let mut options = Vec::new();
        for slot in 0..max_slot {
            let slot_state = &self.state.players[p].stage[slot];
            let Some(card_inst) = slot_state.card else { continue; };
            let Some(card) = self.db.get(card_inst.id) else { continue; };
            if card.card_type != CardType::Character {
                continue;
            }
            options.push(ChoiceOptionRef {
                card_id: card_inst.id,
                zone: ChoiceZone::Stage,
                index: Some(slot as u8),
                target_slot: None,
            });
        }
        options
    }

    fn enumerate_waiting_room_characters(&self, player: u8, target_slots: Option<&[u8]>) -> Vec<ChoiceOptionRef> {
        let p = player as usize;
        let mut options = Vec::new();
        for (idx, card_inst) in self.state.players[p].waiting_room.iter().copied().enumerate() {
            let Some(card) = self.db.get(card_inst.id) else { continue; };
            if card.card_type != CardType::Character {
                continue;
            }
            let index = if idx <= u8::MAX as usize { Some(idx as u8) } else { None };
            if let Some(slots) = target_slots {
                for slot in slots {
                    options.push(ChoiceOptionRef {
                        card_id: card_inst.id,
                        zone: ChoiceZone::WaitingRoom,
                        index,
                        target_slot: Some(*slot),
                    });
                }
            } else {
                options.push(ChoiceOptionRef {
                    card_id: card_inst.id,
                    zone: ChoiceZone::WaitingRoom,
                    index,
                    target_slot: None,
                });
            }
        }
        options
    }

    fn queue_trigger_group(&mut self, player: u8, source: CardId, effects: Vec<TriggerEffect>) {
        if effects.is_empty() {
            return;
        }
        let group_id = self.allocate_trigger_group();
        self.queue_trigger_group_with_group(group_id, player, source, effects);
    }

    fn queue_trigger_group_with_group(&mut self, group_id: u32, player: u8, source: CardId, effects: Vec<TriggerEffect>) {
        for effect in effects {
            let id = self.state.turn.next_trigger_id;
            self.state.turn.next_trigger_id = self.state.turn.next_trigger_id.wrapping_add(1);
            let pending = PendingTrigger {
                id,
                group_id,
                player,
                source_card: source,
                effect,
            };
            self.state.turn.pending_triggers.push(pending);
            self.log_event(Event::TriggerQueued { trigger_id: id, group_id, player, source, effect });
        }
    }

    fn resolve_trigger(&mut self, trigger: PendingTrigger) -> bool {
        if self.db.get(trigger.source_card).is_none() {
            self.log_event(Event::TriggerCanceled { trigger_id: trigger.id, player: trigger.player, reason: TriggerCancelReason::InvalidSource });
            return false;
        }
        match trigger.effect {
            TriggerEffect::Soul => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.damage += 1;
                }
            }
            TriggerEffect::Draw => {
                self.draw_to_hand(trigger.player, 1);
            }
            TriggerEffect::Shot => {
                let target = 1 - trigger.player;
                let _ = self.resolve_effect_damage(trigger.player, target, 1, true, Some(trigger.source_card));
            }
            TriggerEffect::Gate => {
                return self.resolve_trigger_gate(trigger);
            }
            TriggerEffect::Bounce => {
                return self.resolve_trigger_bounce(trigger);
            }
            TriggerEffect::Treasure => {
                return self.resolve_trigger_treasure(trigger);
            }
            TriggerEffect::Standby => {
                return self.resolve_trigger_standby(trigger);
            }
            TriggerEffect::EndPhaseDraw { count } => {
                self.draw_to_hand(trigger.player, count as usize);
            }
        }
        self.log_event(Event::TriggerResolved { trigger_id: trigger.id, player: trigger.player, effect: trigger.effect });
        self.maybe_validate_state("trigger_resolve");
        false
    }

    fn resolve_trigger_gate(&mut self, trigger: PendingTrigger) -> bool {
        let candidates = self.enumerate_waiting_room_characters(trigger.player, None);
        self.start_choice(ChoiceReason::TriggerGateSelect, trigger.player, candidates, Some(trigger))
    }

    fn resolve_trigger_bounce(&mut self, trigger: PendingTrigger) -> bool {
        let candidates = self.enumerate_stage_characters(trigger.player);
        self.start_choice(ChoiceReason::TriggerBounceSelect, trigger.player, candidates, Some(trigger))
    }

    fn resolve_trigger_standby(&mut self, trigger: PendingTrigger) -> bool {
        let open_slots = self.enumerate_open_stage_slots(trigger.player);
        let target_slots = if open_slots.is_empty() {
            let max_slot = if self.curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
            (0..max_slot).map(|slot| slot as u8).collect::<Vec<_>>()
        } else {
            open_slots
        };
        let level_limit = self.state.players[trigger.player as usize].level.len().saturating_add(1);
        let mut candidates = Vec::new();
        // Deterministic ordering: waiting room order, then slot order (ascending).
        for (idx, card_inst) in self.state.players[trigger.player as usize].waiting_room.iter().copied().enumerate() {
            let Some(card) = self.db.get(card_inst.id) else { continue; };
            if card.card_type != CardType::Character {
                continue;
            }
            if card.level as usize > level_limit {
                continue;
            }
            let index = if idx <= u8::MAX as usize { Some(idx as u8) } else { None };
            for slot in &target_slots {
                candidates.push(ChoiceOptionRef {
                    card_id: card_inst.id,
                    zone: ChoiceZone::WaitingRoom,
                    index,
                    target_slot: Some(*slot),
                });
            }
        }
        self.start_choice(ChoiceReason::TriggerStandbySelect, trigger.player, candidates, Some(trigger))
    }

    fn resolve_trigger_treasure(&mut self, trigger: PendingTrigger) -> bool {
        self.move_trigger_card_from_stock_to_hand(trigger.player, trigger.source_card);
        let mut options = Vec::new();
        if self.treasure_stock_available(trigger.player) {
            options.push(ChoiceOptionRef {
                card_id: 0,
                zone: ChoiceZone::DeckTop,
                index: Some(0),
                target_slot: None,
            });
        }
        options.push(ChoiceOptionRef {
            card_id: 0,
            zone: ChoiceZone::DeckTop,
            index: Some(1),
            target_slot: None,
        });
        self.start_choice(ChoiceReason::TriggerTreasureSelect, trigger.player, options, Some(trigger))
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
        }
        if !self.state.turn.pending_triggers.is_empty() {
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
            self.log_event(Event::ModifierRemoved { id, reason: ModifierRemoveReason::EndOfTurn });
        }
        self.state.turn.derived_attack = None;
        self.maybe_validate_state("end_phase_expire");
    }

    fn recompute_derived_attack(&mut self) {
        let mut derived = crate::state::DerivedAttackState::new();
        for player in 0..2usize {
            let max_slot = if self.curriculum.reduced_stage_mode { 1 } else { MAX_STAGE };
            for slot in 0..max_slot {
                let slot_state = &self.state.players[player].stage[slot];
                let mut entry = crate::state::DerivedAttackSlot::empty();
                entry.cannot_attack = slot_state.cannot_attack;
                entry.attack_cost = slot_state.attack_cost;
                if let Some(card_inst) = slot_state.card {
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_some() {
                        let specs = self.db.ability_specs(card_id);
                        for spec in specs {
                            match &spec.template {
                                AbilityTemplate::ContinuousCannotAttack => {
                                    entry.cannot_attack = true;
                                }
                                AbilityTemplate::ContinuousAttackCost { cost } => {
                                    entry.attack_cost = entry.attack_cost.saturating_add(*cost);
                                }
                                _ => {}
                            }
                        }
                    }
                    for modifier in &self.state.modifiers {
                        if modifier.target_player as usize != player || modifier.target_slot as usize != slot {
                            continue;
                        }
                        if modifier.target_card != card_id {
                            continue;
                        }
                        match modifier.kind {
                            ModifierKind::AttackCost => {
                                if modifier.magnitude > 0 {
                                    entry.attack_cost = entry.attack_cost.saturating_add(modifier.magnitude as u8);
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
                let Some(card_inst) = slot.card else { continue; };
                let card_id = card_inst.id;
                if self.db.get(card_id).is_none() {
                    continue;
                }
                let specs = self.db.ability_specs(card_id);
                for spec in specs {
                    if let AbilityTemplate::AutoEndPhaseDraw { count } = &spec.template {
                        pending.push((player as u8, card_id, TriggerEffect::EndPhaseDraw { count: *count }));
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
            self.state.players[p].waiting_room.push(card);
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
        self.log_event(Event::EndTurn { player });
        self.maybe_validate_state("end_phase_finish");
    }

    fn has_attackers(&self, player: u8) -> bool {
        !crate::legal::legal_attack_actions(&self.state, player, &self.curriculum).is_empty()
    }

    fn resolve_attack_pipeline(&mut self) {
        loop {
            let Some(mut ctx) = self.state.turn.attack.take() else { return; };
            match ctx.step {
                AttackStep::Trigger => {
                    self.resolve_trigger_step(&mut ctx);
                    if ctx.counter_allowed && self.curriculum.enable_counters {
                        ctx.step = AttackStep::Counter;
                    } else {
                        ctx.step = AttackStep::Damage;
                    }
                    self.state.turn.attack = Some(ctx);
                    if self.state.turn.pending_level_up.is_some() || !self.state.turn.pending_triggers.is_empty() {
                        self.maybe_validate_state("attack_trigger_pause");
                        break;
                    }
                }
                AttackStep::Counter => {
                    let defender = 1 - self.state.turn.active_player;
                    self.state.turn.attack = Some(ctx);
                    if self.state.turn.priority.is_none() {
                        self.enter_timing_window(TimingWindow::CounterWindow, defender);
                    }
                    self.maybe_validate_state("attack_counter_window");
                    break;
                }
                AttackStep::Damage => {
                    let pause = self.resolve_damage_step(&mut ctx);
                    if ctx.attack_type == AttackType::Direct {
                        self.clear_battle_mods();
                        self.state.turn.attack = None;
                        self.maybe_validate_state("attack_direct_done");
                        break;
                    }
                    ctx.step = AttackStep::Battle;
                    self.state.turn.attack = Some(ctx);
                    if pause {
                        self.maybe_validate_state("attack_damage_pause");
                        break;
                    }
                }
                AttackStep::Battle => {
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
            let _ = self.reveal_cards(active as u8, &[card_id], RevealReason::TriggerCheck, RevealAudience::Public);
            if self.curriculum.enable_triggers {
                if let Some(static_card) = self.db.get(card_id) {
                    let triggers = static_card.triggers.clone();
                    let mut effects = Vec::new();
                    for icon in triggers {
                        self.log_replay_trigger(active as u8, icon, Some(card_id));
                        match icon {
                            TriggerIcon::Soul if self.curriculum.enable_trigger_soul => effects.push(TriggerEffect::Soul),
                            TriggerIcon::Draw if self.curriculum.enable_trigger_draw => effects.push(TriggerEffect::Draw),
                            TriggerIcon::Shot if self.curriculum.enable_trigger_shot => effects.push(TriggerEffect::Shot),
                            TriggerIcon::Bounce if self.curriculum.enable_trigger_bounce => effects.push(TriggerEffect::Bounce),
                            TriggerIcon::Treasure if self.curriculum.enable_trigger_treasure => effects.push(TriggerEffect::Treasure),
                            TriggerIcon::Gate if self.curriculum.enable_trigger_gate => effects.push(TriggerEffect::Gate),
                            TriggerIcon::Standby if self.curriculum.enable_trigger_standby => effects.push(TriggerEffect::Standby),
                            _ => {},
                        }
                    }
                    self.queue_trigger_group(active as u8, card_id, effects);
                }
            }
            self.state.players[active].stock.push(card_inst);
        }
    }

    fn resolve_damage_step(&mut self, ctx: &mut AttackContext) -> bool {
        let attacker = self.state.turn.active_player;
        let defender = 1 - attacker;
        let mut intents = self.collect_attack_damage_intents(ctx, attacker, defender);
        for intent in intents.drain(..) {
            let event_id = self.resolve_damage_intent(intent, &mut ctx.damage_modifiers);
            ctx.last_damage_event_id = Some(event_id);
        }
        self.state.turn.pending_level_up.is_some()
    }

    fn collect_attack_damage_intents(&self, ctx: &AttackContext, attacker: u8, defender: u8) -> Vec<DamageIntentLocal> {
        let mut intents = Vec::new();
        let attacker_slot = ctx.attacker_slot as usize;
        if let Some(card_inst) = self.state.players[attacker as usize].stage[attacker_slot].card {
            let card_id = card_inst.id;
            if self.db.get(card_id).is_some() {
                let specs = self.db.ability_specs(card_id);
                for spec in specs {
                    if let AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } = &spec.template {
                        intents.push(DamageIntentLocal {
                            source_player: attacker,
                            source_slot: Some(ctx.attacker_slot),
                            target: defender,
                            amount: *amount as i32,
                            damage_type: DamageType::Effect,
                            cancelable: *cancelable,
                        });
                    }
                }
            }
        }
        intents.push(DamageIntentLocal {
            source_player: attacker,
            source_slot: Some(ctx.attacker_slot),
            target: defender,
            amount: ctx.damage,
            damage_type: DamageType::Battle,
            cancelable: true,
        });
        intents
    }

    fn resolve_effect_damage(&mut self, source_player: u8, target: u8, amount: i32, cancelable: bool, _source_card: Option<CardId>) -> bool {
        let mut modifiers: Vec<DamageModifier> = Vec::new();
        let intent = DamageIntentLocal {
            source_player,
            source_slot: None,
            target,
            amount,
            damage_type: DamageType::Effect,
            cancelable,
        };
        let _ = self.resolve_damage_intent(intent, &mut modifiers);
        self.state.turn.pending_level_up.is_some()
    }

    fn resolve_damage_intent(&mut self, intent: DamageIntentLocal, modifiers: &mut [DamageModifier]) -> u32 {
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
                    self.reveal_card(intent.target, card.id, RevealReason::DamageCheck, RevealAudience::Public);
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
            self.log_event(Event::DamageCancel { player: intent.target });
            self.state.players[target].waiting_room.extend(revealed);
            return event_id;
        }

        if cancelable {
            for card in revealed {
                self.state.players[target].clock.push(card);
                self.log_event(Event::DamageCommitted { event_id, target: intent.target, card: card.id, damage_type: intent.damage_type });
                self.log_event(Event::Damage { player: intent.target, card: card.id });
                self.pending_damage_delta[target] += 1;
            }
        } else {
            let count = amount as usize;
            for _ in 0..count {
                if let Some(card) = self.draw_from_deck(intent.target) {
                    self.state.players[target].clock.push(card);
                    self.log_event(Event::DamageCommitted { event_id, target: intent.target, card: card.id, damage_type: intent.damage_type });
                    self.log_event(Event::Damage { player: intent.target, card: card.id });
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
        let def_slot = match ctx.defender_slot { Some(s) => s as usize, None => return };
        let atk_power = self.compute_slot_power(attacker, atk_slot);
        let def_power = self.compute_slot_power(defender, def_slot);
        if atk_power > def_power {
            self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted { player: defender as u8, slot: def_slot as u8, cause_damage_event: ctx.last_damage_event_id });
        } else if atk_power < def_power {
            self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted { player: attacker as u8, slot: atk_slot as u8, cause_damage_event: ctx.last_damage_event_id });
        } else {
            self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
            self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
            self.log_event(Event::ReversalCommitted { player: defender as u8, slot: def_slot as u8, cause_damage_event: ctx.last_damage_event_id });
            self.log_event(Event::ReversalCommitted { player: attacker as u8, slot: atk_slot as u8, cause_damage_event: ctx.last_damage_event_id });
        }
    }

    fn queue_encore_requests(&mut self) {
        let mut queue = Vec::new();
        for player in 0..2 {
            for slot in 0..self.state.players[player].stage.len() {
                let slot_state = &self.state.players[player].stage[slot];
                if slot_state.card.is_some() && slot_state.status == StageStatus::Reverse {
                    queue.push(EncoreRequest { player: player as u8, slot: slot as u8 });
                }
            }
        }
        self.state.turn.encore_queue = queue;
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
        if ss >= MAX_STAGE || (self.curriculum.reduced_stage_mode && ss > 0) || self.state.players[p].stage[ss].card.is_some() {
            return Err(anyhow!("Stage slot invalid"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card = self.db.get(card_inst.id).ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if card.card_type != CardType::Character {
            return Err(anyhow!("Card is not a character"));
        }
        if !self.curriculum.allow_character {
            return Err(anyhow!("Character play disabled"));
        }
        if !self.meets_level_requirement(player, card) || !self.meets_color_requirement(player, card) || !self.meets_cost_requirement(player, card) {
            return Err(anyhow!("Play requirements not met"));
        }
        let specs: Vec<AbilitySpec> = self.db.ability_specs(card_inst.id).to_vec();
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        let mut slot = StageSlot::empty();
        slot.card = Some(card_inst);
        slot.status = StageStatus::Stand;
        self.state.players[p].stage[ss] = slot;
        self.log_event(Event::Play { player, card: card_inst.id, slot: stage_slot });
        self.apply_continuous_modifiers_for_slot(player, stage_slot, card_inst.id);
        self.resolve_on_play_abilities(player, &specs);
        Ok(())
    }

    fn play_event(&mut self, player: u8, hand_index: u8) -> Result<()> {
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Event hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card = self.db.get(card_inst.id).ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.looks_like_event(card) {
            return Err(anyhow!("Card is not an event"));
        }
        if !self.curriculum.allow_event {
            return Err(anyhow!("Event play disabled"));
        }
        if !self.meets_level_requirement(player, card) || !self.meets_color_requirement(player, card) || !self.meets_cost_requirement(player, card) {
            return Err(anyhow!("Event requirements not met"));
        }
        let specs: Vec<AbilitySpec> = self.db.ability_specs(card_inst.id).to_vec();
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.log_event(Event::PlayEvent { player, card: card_inst.id });
        self.resolve_on_play_abilities(player, &specs);
        for spec in &specs {
            if let AbilityTemplate::EventDealDamage { amount, cancelable } = &spec.template {
                let target = 1 - player;
                let _ = self.resolve_effect_damage(player, target, *amount as i32, *cancelable, Some(card_inst.id));
            }
        }
        self.state.players[p].waiting_room.push(card_inst);
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
        let card = self.db.get(card_inst.id).ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if card.card_type != CardType::Climax {
            return Err(anyhow!("Card is not a climax"));
        }
        if !self.meets_level_requirement(player, card) || !self.meets_color_requirement(player, card) || !self.meets_cost_requirement(player, card) {
            return Err(anyhow!("Climax requirements not met"));
        }
        let cost = card.cost as usize;
        self.pay_cost(player, cost)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.state.players[p].climax.push(card_inst);
        self.log_event(Event::PlayClimax { player, card: card_inst.id });
        Ok(())
    }

    fn declare_attack(&mut self, player: u8, slot: u8, attack_type: AttackType) -> Result<()> {
        if let Err(reason) = crate::legal::can_declare_attack(&self.state, player, slot, attack_type, &self.curriculum) {
            return Err(anyhow!(reason));
        }
        let p = player as usize;
        let s = slot as usize;
        let defender_player = 1 - p;
        let defender_slot = self.state.players[defender_player].stage[s].card.is_some();
        let attack_cost = self.state.turn.derived_attack.as_ref()
            .map(|d| d.per_player[p][s].attack_cost as usize)
            .unwrap_or(self.state.players[p].stage[s].attack_cost as usize);
        if attack_cost > 0 {
            self.pay_cost(player, attack_cost)?;
        }
        let attacker_slot = &mut self.state.players[p].stage[s];
        attacker_slot.status = StageStatus::Rest;
        attacker_slot.has_attacked = true;
        let card_inst = attacker_slot.card.ok_or_else(|| anyhow!("Missing attacker card"))?;
        let card = self.db.get(card_inst.id).ok_or_else(|| anyhow!("Card missing in db"))?;
        let mut damage = card.soul as i32;
        if attack_type == AttackType::Direct {
            damage += 1;
        } else if attack_type == AttackType::Side {
            let defender_level = self.state.players[defender_player].level.len() as i32;
            damage = (damage - defender_level).max(0);
        }
        self.log_event(Event::Attack { player, slot });
        self.log_event(Event::AttackType { player, attacker_slot: slot, attack_type });
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
            step: AttackStep::Trigger,
        };
        self.state.turn.attack = Some(ctx);
        Ok(())
    }

    fn play_counter(&mut self, player: u8, hand_index: u8) -> Result<()> {
        if !self.curriculum.enable_counters {
            return Err(anyhow!("Counters disabled"));
        }
        let Some(ctx) = &self.state.turn.attack else {
            return Err(anyhow!("No attack context for counter"));
        };
        if ctx.attack_type != AttackType::Frontal || ctx.defender_slot.is_none() {
            return Err(anyhow!("Counter not allowed for this attack"));
        }
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Counter hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card = self.db.get(card_inst.id).ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.is_counter_card(card) {
            return Err(anyhow!("Card is not a counter"));
        }
        if !self.meets_level_requirement(player, card) || !self.meets_color_requirement(player, card) || !self.meets_cost_requirement(player, card) {
            return Err(anyhow!("Counter requirements not met"));
        }
        let power = self.counter_power(card);
        let damage_reduces = self.counter_damage_reductions(card);
        let damage_cancel = self.counter_damage_cancel(card);
        self.pay_cost(player, card.cost as usize)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        self.state.players[p].waiting_room.push(card_inst);
        if let Some(ctx) = &mut self.state.turn.attack {
            if let Some(def_slot) = ctx.defender_slot {
                let slot_state = &mut self.state.players[p].stage[def_slot as usize];
                slot_state.power_mod_battle += power;
                ctx.counter_power += power;
            }
            for reduce in damage_reduces {
                if reduce > 0 {
                    Self::push_attack_damage_modifier(ctx, DamageModifierKind::AddAmount { delta: -reduce }, card_inst.id);
                }
            }
            if damage_cancel {
                Self::push_attack_damage_modifier(ctx, DamageModifierKind::CancelNext, card_inst.id);
            }
        }
        self.log_event(Event::Counter { player, card: card_inst.id, power });
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
        let chosen = top[idx];
        for (i, card) in top.into_iter().enumerate() {
            if i == idx {
                self.state.players[p].level.push(card);
            } else {
                self.state.players[p].waiting_room.push(card);
            }
        }
        self.log_event(Event::LevelUpChoice { player, card: chosen.id });
        self.state.turn.pending_level_up = None;
        if self.state.players[p].level.len() >= 4 {
            self.state.terminal = Some(TerminalResult::Win { winner: (1 - p) as u8 });
        }
        self.check_level_up(player);
        Ok(())
    }

    fn resolve_encore(&mut self, player: u8, keep: bool) -> Result<()> {
        let req = self.state.turn.encore_queue.first().copied().ok_or_else(|| anyhow!("No encore request"))?;
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
        self.log_event(Event::Encore { player, slot: req.slot, kept });
        self.state.turn.encore_queue.remove(0);
        Ok(())
    }

    fn send_stage_to_waiting_room(&mut self, player: u8, slot: u8) {
        let p = player as usize;
        let s = slot as usize;
        self.remove_modifiers_for_slot(player, slot);
        if let Some(card) = self.state.players[p].stage[s].card.take() {
            self.state.players[p].waiting_room.push(card);
        }
        self.state.players[p].stage[s] = StageSlot::empty();
    }

    fn move_waiting_room_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::WaitingRoom {
            return;
        }
        let Some(idx) = option.index else { return; };
        let p = player as usize;
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        if card.id != option.card_id {
            return;
        }
        self.state.players[p].hand.push(card);
        self.log_event(Event::ZoneMove { player, card: card.id, from: Zone::WaitingRoom, to: Zone::Hand, from_slot: None, to_slot: None });
    }

    fn move_stage_to_hand(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stage {
            return;
        }
        let Some(idx) = option.index else { return; };
        let p = player as usize;
        let slot = idx as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        self.remove_modifiers_for_slot(player, idx);
        let card = self.state.players[p].stage[slot].card.take();
        let Some(card) = card else { return; };
        if card.id != option.card_id {
            return;
        }
        self.state.players[p].stage[slot] = StageSlot::empty();
        self.state.players[p].hand.push(card);
        self.log_event(Event::ZoneMove { player, card: card.id, from: Zone::Stage, to: Zone::Hand, from_slot: Some(idx), to_slot: None });
    }

    fn move_waiting_room_to_stage_standby(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::WaitingRoom {
            return;
        }
        let Some(idx) = option.index else { return; };
        let Some(target_slot) = option.target_slot else { return; };
        let p = player as usize;
        let slot = target_slot as usize;
        if slot >= self.state.players[p].stage.len() {
            return;
        }
        if let Some(existing) = self.state.players[p].stage[slot].card {
            self.remove_modifiers_for_slot(player, target_slot);
            self.state.players[p].stage[slot] = StageSlot::empty();
            self.state.players[p].waiting_room.push(existing);
            self.log_event(Event::ZoneMove {
                player,
                card: existing.id,
                from: Zone::Stage,
                to: Zone::WaitingRoom,
                from_slot: Some(target_slot),
                to_slot: None,
            });
        }
        let index = idx as usize;
        if index >= self.state.players[p].waiting_room.len() {
            return;
        }
        let card = self.state.players[p].waiting_room.remove(index);
        if card.id != option.card_id {
            return;
        }
        let mut slot_state = StageSlot::empty();
        slot_state.card = Some(card);
        slot_state.status = StageStatus::Rest;
        self.state.players[p].stage[slot] = slot_state;
        self.apply_continuous_modifiers_for_slot(player, target_slot, card.id);
        self.log_event(Event::ZoneMove { player, card: card.id, from: Zone::WaitingRoom, to: Zone::Stage, from_slot: None, to_slot: Some(target_slot) });
    }

    fn move_trigger_card_from_stock_to_hand(&mut self, player: u8, card_id: CardId) -> bool {
        let p = player as usize;
        // Deterministic removal: take the most recent matching card from stock.
        if let Some(pos) = self.state.players[p].stock.iter().rposition(|c| c.id == card_id) {
            let card = self.state.players[p].stock.remove(pos);
            self.state.players[p].hand.push(card);
            self.log_event(Event::ZoneMove { player, card: card.id, from: Zone::Stock, to: Zone::Hand, from_slot: None, to_slot: None });
            return true;
        }
        false
    }

    fn treasure_stock_available(&self, player: u8) -> bool {
        let p = player as usize;
        !self.state.players[p].deck.is_empty() || !self.state.players[p].waiting_room.is_empty()
    }

    fn apply_treasure_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::DeckTop {
            return;
        }
        let take_stock = option.index.unwrap_or(0) == 0;
        if !take_stock {
            return;
        }
        if let Some(card) = self.draw_from_deck(player) {
            let p = player as usize;
            self.state.players[p].stock.push(card);
            self.log_event(Event::ZoneMove { player, card: card.id, from: Zone::Deck, to: Zone::Stock, from_slot: None, to_slot: None });
        }
    }

    fn push_attack_damage_modifier(ctx: &mut AttackContext, kind: DamageModifierKind, source_id: u32) {
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

    fn add_modifier_instance(&mut self, source: CardId, target_player: u8, target_slot: u8, kind: ModifierKind, magnitude: i32, duration: ModifierDuration) -> Option<u32> {
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
        self.log_event(Event::ModifierAdded { id, source, target_player, target_slot, target_card, kind, magnitude, duration });
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
            self.log_event(Event::ModifierRemoved { id, reason: ModifierRemoveReason::TargetLeftStage });
        }
    }

    fn resolve_on_play_abilities(&mut self, player: u8, specs: &[AbilitySpec]) {
        for spec in specs {
            if let AbilityTemplate::AutoOnPlayDraw { count } = &spec.template {
                self.draw_to_hand(player, *count as usize);
            }
        }
    }

    fn compute_slot_power(&self, player: usize, slot: usize) -> i32 {
        let slot_state = &self.state.players[player].stage[slot];
        let Some(card_inst) = slot_state.card else { return 0; };
        let card_id = card_inst.id;
        let Some(card) = self.db.get(card_id) else { return 0; };
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
                self.state.players[p].waiting_room.push(card);
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
        self.db.ability_specs(card.id).iter().any(|spec| matches!(spec.template, AbilityTemplate::CounterBackup { .. } | AbilityTemplate::CounterDamageReduce { .. } | AbilityTemplate::CounterDamageCancel))
    }

    fn counter_power(&self, card: &CardStatic) -> i32 {
        for spec in self.db.ability_specs(card.id) {
            if let AbilityTemplate::CounterBackup { power } = spec.template {
                return power;
            }
        }
        0
    }

    fn counter_damage_reductions(&self, card: &CardStatic) -> Vec<i32> {
        let mut out = Vec::new();
        for spec in self.db.ability_specs(card.id) {
            if let AbilityTemplate::CounterDamageReduce { amount } = spec.template {
                out.push(amount as i32);
            }
        }
        out
    }

    fn counter_damage_cancel(&self, card: &CardStatic) -> bool {
        self.db.ability_specs(card.id).iter().any(|spec| matches!(spec.template, AbilityTemplate::CounterDamageCancel))
    }

    fn shuffle_deck(&mut self, player: u8) {
        let p = player as usize;
        self.state.rng.shuffle(&mut self.state.players[p].deck);
    }

    fn draw_to_hand(&mut self, player: u8, count: usize) {
        for _ in 0..count {
            if let Some(card) = self.draw_from_deck(player) {
                let p = player as usize;
                self.state.players[p].hand.push(card);
                self.log_event(Event::Draw { player, card: card.id });
            }
        }
    }

    fn reveal_card(&mut self, player: u8, card: CardId, reason: RevealReason, audience: RevealAudience) {
        self.log_event(Event::Reveal { player, card, reason, audience });
    }

    fn reveal_cards(&mut self, player: u8, cards: &[CardId], reason: RevealReason, audience: RevealAudience) -> Vec<CardId> {
        for &card in cards {
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
            self.state.terminal = Some(TerminalResult::Win { winner: (1 - p) as u8 });
            return false;
        }
        let mut reshuffle = Vec::new();
        std::mem::swap(&mut reshuffle, &mut self.state.players[p].waiting_room);
        self.state.players[p].deck = reshuffle;
        self.shuffle_deck(player);
        self.log_event(Event::Refresh { player });
        if self.curriculum.enable_refresh_penalty {
            let mut modifiers: Vec<DamageModifier> = Vec::new();
            let intent = DamageIntentLocal {
                source_player: player,
                source_slot: None,
                target: player,
                amount: 1,
                damage_type: DamageType::Effect,
                cancelable: false,
            };
            let _ = self.resolve_damage_intent(intent, &mut modifiers);
            if let Some(card) = self.state.players[p].clock.last().copied() {
                self.log_event(Event::RefreshPenalty { player, card: card.id });
            }
        }
        true
    }

    fn log_event(&mut self, event: Event) {
        if self.recording {
            let replay_event = match event {
                Event::Draw { player, card } => ReplayEvent::Draw { player, card },
                Event::Damage { player, card } => ReplayEvent::Damage { player, card },
                Event::DamageCancel { player } => ReplayEvent::DamageCancel { player },
                Event::DamageIntent { event_id, source_player, source_slot, target, amount, damage_type, cancelable } => ReplayEvent::DamageIntent { event_id, source_player, source_slot, target, amount, damage_type, cancelable },
                Event::DamageModifierApplied { event_id, modifier, before_amount, after_amount, before_cancelable, after_cancelable, before_canceled, after_canceled } => ReplayEvent::DamageModifierApplied { event_id, modifier, before_amount, after_amount, before_cancelable, after_cancelable, before_canceled, after_canceled },
                Event::DamageModified { event_id, target, original, modified, canceled, damage_type } => ReplayEvent::DamageModified { event_id, target, original, modified, canceled, damage_type },
                Event::DamageCommitted { event_id, target, card, damage_type } => ReplayEvent::DamageCommitted { event_id, target, card, damage_type },
                Event::ReversalCommitted { player, slot, cause_damage_event } => ReplayEvent::ReversalCommitted { player, slot, cause_damage_event },
                Event::Reveal { player, card, reason, audience } => ReplayEvent::Reveal { player, card, reason, audience },
                Event::TriggerQueued { trigger_id, group_id, player, source, effect } => ReplayEvent::TriggerQueued { trigger_id, group_id, player, source, effect },
                Event::TriggerResolved { trigger_id, player, effect } => ReplayEvent::TriggerResolved { trigger_id, player, effect },
                Event::TriggerCanceled { trigger_id, player, reason } => ReplayEvent::TriggerCanceled { trigger_id, player, reason },
                Event::TimingWindowEntered { window, player } => ReplayEvent::TimingWindowEntered { window, player },
                Event::PriorityPassed { player, window, pass_count } => ReplayEvent::PriorityPassed { player, window, pass_count },
                Event::StackGroupPresented { group_id, controller, items } => ReplayEvent::StackGroupPresented { group_id, controller, items },
                Event::StackOrderChosen { group_id, controller, stack_id } => ReplayEvent::StackOrderChosen { group_id, controller, stack_id },
                Event::StackPushed { item } => ReplayEvent::StackPushed { item },
                Event::StackResolved { item } => ReplayEvent::StackResolved { item },
                Event::ChoicePresented { choice_id, player, reason, options, total_candidates } => ReplayEvent::ChoicePresented { choice_id, player, reason, options, total_candidates },
                Event::ChoiceMade { choice_id, player, option } => ReplayEvent::ChoiceMade { choice_id, player, option },
                Event::ChoiceAutopicked { choice_id, player, option } => ReplayEvent::ChoiceAutopicked { choice_id, player, option },
                Event::ChoiceSkipped { choice_id, player, reason, skip_reason } => ReplayEvent::ChoiceSkipped { choice_id, player, reason, skip_reason },
                Event::ZoneMove { player, card, from, to, from_slot, to_slot } => ReplayEvent::ZoneMove { player, card, from, to, from_slot, to_slot },
                Event::ControlChanged { card, owner, from_controller, to_controller, from_slot, to_slot } => ReplayEvent::ControlChanged {
                    card,
                    owner,
                    from_controller,
                    to_controller,
                    from_slot,
                    to_slot,
                },
                Event::ModifierAdded { id, source, target_player, target_slot, target_card, kind, magnitude, duration } => ReplayEvent::ModifierAdded { id, source, target_player, target_slot, target_card, kind, magnitude, duration },
                Event::ModifierRemoved { id, reason } => ReplayEvent::ModifierRemoved { id, reason },
                Event::Play { player, card, slot } => ReplayEvent::Play { player, card, slot },
                Event::PlayEvent { player, card } => ReplayEvent::PlayEvent { player, card },
                Event::PlayClimax { player, card } => ReplayEvent::PlayClimax { player, card },
                Event::Trigger { player, icon } => ReplayEvent::Trigger { player, icon, card: None },
                Event::Attack { player, slot } => ReplayEvent::Attack { player, slot },
                Event::AttackType { player, attacker_slot, attack_type } => ReplayEvent::AttackType { player, attacker_slot, attack_type },
                Event::Counter { player, card, power } => ReplayEvent::Counter { player, card, power },
                Event::Clock { player, card } => ReplayEvent::Clock { player, card },
                Event::Refresh { player } => ReplayEvent::Refresh { player },
                Event::RefreshPenalty { player, card } => ReplayEvent::RefreshPenalty { player, card },
                Event::LevelUpChoice { player, card } => ReplayEvent::LevelUpChoice { player, card },
                Event::Encore { player, slot, kept } => ReplayEvent::Encore { player, slot, kept },
                Event::Stand { player } => ReplayEvent::Stand { player },
                Event::EndTurn { player } => ReplayEvent::EndTurn { player },
                Event::Terminal { winner } => ReplayEvent::Terminal { winner },
            };
            self.replay_events.push(replay_event);
        }
    }

    fn log_replay_trigger(&mut self, player: u8, icon: TriggerIcon, card: Option<CardId>) {
        if self.recording {
            let reveal = if self.replay_config.include_trigger_card_id { card } else { None };
            self.replay_events.push(ReplayEvent::Trigger { player, icon, card: reveal });
        }
    }

    fn card_set_allowed(&self, card: &CardStatic) -> bool {
        match (&self.curriculum.allowed_card_sets_cache, &card.card_set) {
            (None, _) => true,
            (Some(set), Some(set_id)) => set.contains(set_id),
            (Some(_), None) => false,
        }
    }

    fn handle_illegal_action(&mut self, acting_player: u8, reason: &str, copy_obs: bool) -> Result<StepOutcome> {
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
        let RewardConfig { terminal_win, terminal_loss, terminal_draw, .. } = &self.config.reward;
        match self.state.terminal {
            Some(TerminalResult::Win { winner }) => if winner == perspective { *terminal_win } else { *terminal_loss },
            Some(TerminalResult::Draw | TerminalResult::Timeout) => *terminal_draw,
            None => 0.0,
        }
    }

    pub fn finish_episode_replay(&mut self) {
        if !self.recording {
            return;
        }
        if self.state.terminal.is_some() {
            let need_terminal = !self.replay_events.iter().any(|e| matches!(e, ReplayEvent::Terminal { .. }));
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
                config_hash: self.config.config_hash(),
            };
            let body = EpisodeBody {
                actions: self.replay_actions.clone(),
                events: Some(self.replay_events.clone()),
                steps: self.replay_steps.clone(),
                final_state: Some(ReplayFinal {
                    terminal: self.state.terminal,
                    state_hash: crate::util::hash_value(&self.state),
                    decision_count: self.state.turn.decision_count,
                    tick_count: self.state.turn.tick_count,
                }),
            };
            writer.send(ReplayData { header, body });
        }
        self.recording = false;
    }
}
