use anyhow::{anyhow, Result};

use crate::config::RewardConfig;
use crate::events::Event;
use crate::legal::{ActionDesc, DecisionKind};
use crate::state::{Phase, StageStatus, TerminalResult, TimingWindow};

use super::super::{GameEnv, MAX_CHOICE_OPTIONS};
use crate::env::core::ProgressSignature;

impl GameEnv {
    /// Apply an action by id and return the resulting outcome.
    ///
    /// This is the primary stepping entry point for RL-style loops.
    pub fn apply_action_id(&mut self, action_id: usize) -> Result<super::super::StepOutcome> {
        self.apply_action_id_internal(action_id, true)
    }

    /// Apply an action by id without copying the observation buffer.
    ///
    /// The returned `StepOutcome.obs` will be empty; use output buffers instead.
    pub fn apply_action_id_no_copy(
        &mut self,
        action_id: usize,
    ) -> Result<super::super::StepOutcome> {
        self.apply_action_id_internal(action_id, false)
    }

    fn apply_action_id_internal(
        &mut self,
        action_id: usize,
        copy_obs: bool,
    ) -> Result<super::super::StepOutcome> {
        if self.is_fault_latched() {
            return Ok(self.build_fault_step_outcome(copy_obs));
        }
        self.last_illegal_action = false;
        self.last_engine_error = false;
        self.last_engine_error_code = super::super::EngineErrorCode::None;
        let Some(decision) = self.decision.as_ref() else {
            return Err(anyhow!("No pending decision"));
        };
        self.last_perspective = decision.player;
        let action = match self.action_for_id(action_id) {
            Some(action) => action,
            None => {
                return self.handle_illegal_action(decision.player, "Invalid action id", copy_obs);
            }
        };
        self.apply_action_internal(action, copy_obs)
    }

    /// Apply a canonical action descriptor.
    pub fn apply_action(&mut self, action: ActionDesc) -> Result<super::super::StepOutcome> {
        self.apply_action_internal(action, true)
    }

    fn apply_action_internal(
        &mut self,
        action: ActionDesc,
        copy_obs: bool,
    ) -> Result<super::super::StepOutcome> {
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
            .ok_or_else(|| anyhow!("No decision to apply"))?;
        self.last_action_decision_kind = Some(decision_kind);
        let action_clone = action.clone();
        if self.should_validate_state() {
            if let Some(decision) = &self.decision {
                let legal = super::legal_actions_cached(
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
                crate::config::ErrorPolicy::Strict => Err(err),
                crate::config::ErrorPolicy::LenientTerminate => {
                    self.last_engine_error = true;
                    self.last_engine_error_code = super::super::EngineErrorCode::ActionError;
                    self.last_perspective = acting_player;
                    self.state.terminal = Some(TerminalResult::Win {
                        winner: 1 - acting_player,
                    });
                    self.decision = None;
                    self.update_action_cache();
                    Ok(self
                        .build_outcome_with_obs(self.terminal_reward_for(acting_player), copy_obs))
                }
                crate::config::ErrorPolicy::LenientNoop => {
                    self.last_engine_error = true;
                    self.last_engine_error_code = super::super::EngineErrorCode::ActionError;
                    self.last_perspective = acting_player;
                    self.update_action_cache();
                    Ok(self.build_outcome_with_obs(0.0, copy_obs))
                }
            },
        }?;
        if self.recording || self.should_validate_state() {
            let main_move_action = matches!(decision_kind, DecisionKind::Main)
                && matches!(action_clone, ActionDesc::MainMove { .. });
            let main_pass_action = matches!(decision_kind, DecisionKind::Main)
                && matches!(action_clone, ActionDesc::Pass);
            self.log_action(acting_player, action_clone);
            self.replay_steps.push(crate::replay::StepMeta {
                actor: acting_player,
                decision_kind,
                illegal_action: self.last_illegal_action,
                engine_error: self.last_engine_error,
                main_move_action,
                main_pass_action,
            });
        }
        Ok(outcome)
    }

    fn apply_action_impl(
        &mut self,
        action: ActionDesc,
        copy_obs: bool,
    ) -> Result<super::super::StepOutcome> {
        let decision = self
            .decision
            .clone()
            .ok_or_else(|| anyhow!("No decision to apply"))?;
        self.last_perspective = decision.player;
        self.last_action_desc = Some(action.clone());
        self.last_action_player = Some(decision.player);
        let progress_before = self.progress_signature();

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
            if self.maybe_validate_state("post_concede") || self.is_fault_latched() {
                return Ok(self.build_fault_step_outcome(copy_obs));
            }
            reward += self.compute_reward(
                decision.player,
                &self.pending_damage_delta,
                &progress_before,
            );
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
                            crate::events::Zone::Hand,
                            crate::events::Zone::WaitingRoom,
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
                            crate::events::Zone::Hand,
                            crate::events::Zone::Clock,
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
                    if self.slot_has_active_modifier_kind(
                        decision.player,
                        from_slot,
                        crate::state::ModifierKind::CannotMoveStagePosition,
                    ) {
                        return self.handle_illegal_action(
                            decision.player,
                            "Source slot card cannot move",
                            copy_obs,
                        );
                    }
                    if self.state.players[p].stage[ts].card.is_some()
                        && self.slot_has_active_modifier_kind(
                            decision.player,
                            to_slot,
                            crate::state::ModifierKind::CannotMoveStagePosition,
                        )
                    {
                        return self.handle_illegal_action(
                            decision.player,
                            "Destination slot card cannot move",
                            copy_obs,
                        );
                    }
                    if self.state.turn.main_move_used {
                        return self.handle_illegal_action(
                            decision.player,
                            "Main move already used this turn",
                            copy_obs,
                        );
                    }
                    self.state.players[p].stage.swap(fs, ts);
                    self.state.turn.main_move_used = true;
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
                        if let Err(err) = self.resolve_trigger(trigger) {
                            let msg = format!("Trigger resolve failed: {err}");
                            return self.handle_illegal_action(decision.player, &msg, copy_obs);
                        }
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
        if self.maybe_validate_state("post_action") || self.is_fault_latched() {
            return Ok(self.build_fault_step_outcome(copy_obs));
        }

        self.update_no_progress_counter(progress_before);
        reward += self.compute_reward(
            decision.player,
            &self.pending_damage_delta,
            &progress_before,
        );
        Ok(self.build_outcome_with_obs(reward, copy_obs))
    }

    pub(in crate::env) fn progress_signature(&self) -> ProgressSignature {
        let mut signature = ProgressSignature {
            active_player: self.state.turn.active_player,
            turn_number: self.state.turn.turn_number,
            phase: self.state.turn.phase,
            choice_id: self.state.turn.choice.as_ref().map(|choice| choice.id),
            choice_page_start: self
                .state
                .turn
                .choice
                .as_ref()
                .map_or(0, |choice| choice.page_start),
            choice_total_candidates: self
                .state
                .turn
                .choice
                .as_ref()
                .map_or(0, |choice| choice.total_candidates),
            ..ProgressSignature::default()
        };
        for player in 0..2usize {
            let state = &self.state.players[player];
            signature.deck_counts[player] = state.deck.len() as u16;
            signature.hand_counts[player] = state.hand.len() as u16;
            signature.waiting_room_counts[player] = state.waiting_room.len() as u16;
            signature.clock_counts[player] = state.clock.len() as u16;
            signature.level_counts[player] = state.level.len() as u16;
            signature.stock_counts[player] = state.stock.len() as u16;
            signature.memory_counts[player] = state.memory.len() as u16;
            signature.climax_counts[player] = state.climax.len() as u16;
            signature.resolution_counts[player] = state.resolution.len() as u16;
            signature.occupied_stage_counts[player] = state
                .stage
                .iter()
                .filter(|slot| slot.card.is_some())
                .count() as u16;
            signature.reversed_stage_counts[player] = state
                .stage
                .iter()
                .filter(|slot| slot.card.is_some() && slot.status == StageStatus::Reverse)
                .count() as u16;
            signature.live_stage_counts[player] = state
                .stage
                .iter()
                .filter(|slot| slot.card.is_some() && slot.status != StageStatus::Reverse)
                .count() as u16;
        }
        signature
    }

    pub(crate) fn last_action_main_flags(&self) -> (bool, bool) {
        match (
            self.last_action_decision_kind,
            self.last_action_desc.as_ref(),
        ) {
            (Some(DecisionKind::Main), Some(ActionDesc::MainMove { .. })) => (true, false),
            (Some(DecisionKind::Main), Some(ActionDesc::Pass)) => (false, true),
            _ => (false, false),
        }
    }

    pub(in crate::env) fn update_no_progress_counter(&mut self, before: ProgressSignature) {
        if self.state.terminal.is_some() {
            self.no_progress_decisions = 0;
            return;
        }
        let limit = self.curriculum.max_no_progress_decisions;
        if limit == 0 {
            self.no_progress_decisions = 0;
            return;
        }
        let after = self.progress_signature();
        if after != before {
            self.no_progress_decisions = 0;
            return;
        }
        self.no_progress_decisions = self.no_progress_decisions.saturating_add(1);
        if self.no_progress_decisions >= limit {
            self.state.terminal = Some(TerminalResult::Timeout);
            self.decision = None;
            self.update_action_cache();
        }
    }

    pub(in crate::env) fn compute_reward(
        &self,
        perspective: u8,
        damage_delta: &[i32; 2],
        progress_before: &ProgressSignature,
    ) -> f32 {
        let RewardConfig {
            terminal_win,
            terminal_loss,
            terminal_draw,
            terminal_timeout,
            enable_shaping,
            damage_reward,
            level_reward,
            board_reward,
            no_progress_penalty,
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
                TerminalResult::Draw => *terminal_draw,
                TerminalResult::Timeout => *terminal_timeout,
            };
        }
        if *enable_shaping {
            let mut reward = 0.0;
            let p = perspective as usize;
            let opp = 1 - p;
            reward += *damage_reward * damage_delta[opp] as f32;
            reward -= *damage_reward * damage_delta[p] as f32;
            let progress_after = self.progress_signature();
            let level_delta = (progress_after.level_counts[opp] as i32
                - progress_before.level_counts[opp] as i32)
                - (progress_after.level_counts[p] as i32 - progress_before.level_counts[p] as i32);
            reward += *level_reward * level_delta as f32;
            let board_delta = (progress_after.live_stage_counts[p] as i32
                - progress_before.live_stage_counts[p] as i32)
                - (progress_after.live_stage_counts[opp] as i32
                    - progress_before.live_stage_counts[opp] as i32);
            reward += *board_reward * board_delta as f32;
            if progress_after == *progress_before {
                reward -= *no_progress_penalty;
            }
            return reward;
        }
        0.0
    }
}
