use super::super::GameEnv;
use crate::db::AbilityKind;
use crate::encode::{MAX_ABILITIES_PER_CARD, MAX_HAND, MAX_STAGE};
use crate::events::Event;
use crate::legal::ActionDesc;
use crate::state::{
    AttackStep, AttackType, ChoiceOptionRef, ChoiceReason, ChoiceZone, Phase, PriorityState,
    TimingWindow,
};
use anyhow::{anyhow, Result};

impl GameEnv {
    pub(in crate::env) fn enter_timing_window(&mut self, window: TimingWindow, holder: u8) {
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

    pub(in crate::env) fn collect_priority_actions(&mut self, player: u8) {
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
                    let total_abilities = self.live_stage_ability_count(player, slot as u8);
                    for idx in 0..total_abilities {
                        if idx >= MAX_ABILITIES_PER_CARD || idx > u8::MAX as usize {
                            break;
                        }
                        let Some(live) = self.live_stage_ability_at(player, slot as u8, idx) else {
                            continue;
                        };
                        let spec = live.spec;
                        if spec.kind != AbilityKind::Activated {
                            continue;
                        }
                        let cost = spec.template.activation_cost_spec();
                        if !self.can_pay_ability_cost(player, Some(slot as u8), card_inst, &cost) {
                            continue;
                        }
                        if live.effects.is_empty() {
                            continue;
                        }
                        let bit = slot * MAX_ABILITIES_PER_CARD + idx;
                        if bit >= u32::BITS as usize {
                            debug_assert!(
                                false,
                                "priority ability bit out of range: slot={slot} idx={idx}"
                            );
                            continue;
                        }
                        if priority.used_act_mask & (1u32 << bit) != 0 {
                            continue;
                        }
                        self.scratch
                            .priority_actions
                            .push(ActionDesc::MainActivateAbility {
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
                    let backup_locked = p.stage.iter().enumerate().any(|(slot, slot_state)| {
                        slot_state.card.is_some()
                            && self.slot_has_active_modifier_kind(
                                player,
                                slot as u8,
                                crate::state::ModifierKind::CannotPlayBackupFromHand,
                            )
                    });
                    // Deterministic priority ordering: hand index ascending.
                    for (hand_index, card_inst) in p.hand.iter().enumerate() {
                        if hand_index >= MAX_HAND || hand_index > u8::MAX as usize {
                            break;
                        }
                        let Some(card) = self.db.get(card_inst.id) else {
                            continue;
                        };
                        if !self.card_set_allowed(card) {
                            continue;
                        }
                        if self.is_counter_card(card)
                            && !backup_locked
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
        if self.curriculum.priority_allow_pass && !self.curriculum.strict_priority_mode {
            self.scratch.priority_actions.push(ActionDesc::Pass);
        }
    }

    pub(in crate::env) fn start_priority_choice(&mut self, player: u8) -> bool {
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
                        index: Some(hand_index as u16),
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
                        index: Some(slot as u16),
                        target_slot: Some(ability_index),
                    });
                }
                ActionDesc::Pass => {
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id: 0,
                        instance_id: 0,
                        zone: ChoiceZone::PriorityPass,
                        index: None,
                        target_slot: None,
                    });
                }
                _ => {}
            }
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(ChoiceReason::PriorityActionSelect, player, options, None)
    }

    pub(in crate::env) fn apply_priority_action_choice(
        &mut self,
        player: u8,
        option: ChoiceOptionRef,
    ) {
        let action = match option.zone {
            ChoiceZone::PriorityCounter => option
                .index
                .and_then(|idx| u8::try_from(idx).ok())
                .map(|idx| ActionDesc::CounterPlay { hand_index: idx }),
            ChoiceZone::PriorityAct => {
                if let (Some(slot), Some(ability)) = (option.index, option.target_slot) {
                    let Ok(slot) = u8::try_from(slot) else {
                        return;
                    };
                    Some(ActionDesc::MainActivateAbility {
                        slot,
                        ability_index: ability,
                    })
                } else {
                    None
                }
            }
            ChoiceZone::PriorityPass => Some(ActionDesc::Pass),
            _ => None,
        };
        if let Some(action) = action {
            if let Err(err) = self.apply_priority_action(player, action) {
                self.last_engine_error = true;
                self.last_engine_error_code = super::super::EngineErrorCode::ActionError;
                eprintln!("Priority action failed: {err}");
            }
        }
    }

    pub(in crate::env) fn apply_priority_action(
        &mut self,
        player: u8,
        action: ActionDesc,
    ) -> Result<()> {
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
                let pending_cost =
                    self.queue_activated_ability_stack_item(player, slot, ability_index)?;
                let bit = slot as usize * MAX_ABILITIES_PER_CARD + ability_index as usize;
                if bit >= u32::BITS as usize {
                    return Err(anyhow!("Activated ability mask bit out of range"));
                }
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.used_act_mask |= 1u32 << bit;
                    if !pending_cost {
                        priority.holder = 1 - player;
                        priority.passes = 0;
                        new_holder = Some(priority.holder);
                    }
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
            ActionDesc::Pass => {
                if self.curriculum.strict_priority_mode || !self.curriculum.priority_allow_pass {
                    self.collect_priority_actions(player);
                    if !self.scratch.priority_actions.is_empty() {
                        return Err(anyhow!(
                            "Explicit pass not allowed when priority actions exist"
                        ));
                    }
                }
                self.priority_pass(player);
            }
            _ => return Err(anyhow!("Invalid priority action")),
        }
        Ok(())
    }

    pub(in crate::env) fn priority_pass(&mut self, player: u8) {
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

    pub(in crate::env) fn close_priority_window(&mut self, window: TimingWindow) {
        self.state.turn.priority = None;
        let mut to_window = None;
        match window {
            TimingWindow::MainWindow => {
                if self.state.turn.main_passed {
                    self.state.turn.main_passed = false;
                    self.state.turn.phase = Phase::Climax;
                    self.state.turn.phase_step = 0;
                    if self.curriculum.enable_priority_windows {
                        to_window = Some(TimingWindow::ClimaxWindow);
                    }
                }
            }
            TimingWindow::CounterWindow => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.step = AttackStep::Damage;
                }
            }
            TimingWindow::ClimaxWindow => {
                self.state.turn.phase_step = 2;
            }
            TimingWindow::AttackDeclarationWindow => {}
            TimingWindow::TriggerResolutionWindow => {}
            TimingWindow::DamageResolutionWindow => {}
            TimingWindow::EncoreWindow => {}
            TimingWindow::EndPhaseWindow => {}
        }
        self.state.turn.active_window = None;
        self.log_event(Event::WindowAdvanced {
            from: window,
            to: to_window,
        });
    }
}
