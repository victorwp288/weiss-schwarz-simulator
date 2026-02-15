use crate::events::Event;
use crate::legal::{Decision, DecisionKind};
use crate::state::{Phase, TerminalResult};

use super::super::{EngineErrorCode, GameEnv, STACK_AUTO_RESOLVE_CAP};

impl GameEnv {
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
            if !self.advance_tick() {
                break;
            }

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
            } else {
                auto_resolve_steps = 0;
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
                            crate::state::TimingWindow::EncoreWindow,
                            self.state.turn.active_player,
                        );
                    }
                    break;
                }
                if self.state.turn.encore_step_player.is_none() {
                    self.state.turn.encore_step_player = Some(self.state.turn.active_player);
                }
                let Some(current) = self.state.turn.encore_step_player else {
                    continue;
                };
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
                            self.state.turn.phase_step = 1;
                            continue;
                        }
                        1 => {
                            if self.state.turn.pending_level_up.is_some()
                                || !self.state.turn.pending_triggers.is_empty()
                            {
                                continue;
                            }
                            self.run_check_timing(crate::db::AbilityTiming::BeginStandPhase);
                            self.state.turn.phase_step = 2;
                            continue;
                        }
                        2 => {
                            self.resolve_stand_phase(p);
                            self.state.turn.phase_step = 3;
                            continue;
                        }
                        3 => {
                            self.run_check_timing(crate::db::AbilityTiming::AfterStandPhase);
                            self.state.turn.phase_step = 4;
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
            if self.maybe_validate_state("advance_loop") {
                break;
            }
        }
    }
}
