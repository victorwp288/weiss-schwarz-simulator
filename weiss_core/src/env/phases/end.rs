use super::super::{GameEnv, HAND_LIMIT};
use crate::db::AbilityTiming;
use crate::events::{Event, ModifierRemoveReason, Zone};
use crate::state::{ChoiceOptionRef, ChoiceReason, ChoiceZone, ModifierDuration, TimingWindow};

impl GameEnv {
    pub(in crate::env) fn resolve_end_phase(&mut self, player: u8) -> bool {
        if !self.state.turn.end_phase_pending {
            self.run_check_timing(AbilityTiming::EndPhase);
            self.state.turn.end_phase_pending = true;
            self.state.turn.end_phase_window_done = false;
            self.state.turn.end_phase_discard_done = false;
            self.state.turn.end_phase_climax_done = false;
            self.state.turn.end_phase_cleanup_done = false;
        }
        if self.state.turn.pending_level_up.is_some() {
            return false;
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
        if !self.state.turn.end_phase_discard_done {
            let hand_len = self.state.players[player as usize].hand.len();
            if hand_len > HAND_LIMIT {
                return self.start_end_phase_discard_choice(player);
            }
            self.state.turn.end_phase_discard_done = true;
        }
        if !self.state.turn.end_phase_climax_done {
            let p = player as usize;
            if let Some(card) = self.state.players[p].climax.pop() {
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Climax,
                    Zone::WaitingRoom,
                    None,
                    None,
                );
            }
            self.state.turn.end_phase_climax_done = true;
        }
        if !self.state.turn.end_phase_cleanup_done {
            self.run_check_timing(AbilityTiming::EndPhaseCleanup);
            self.state.turn.end_phase_cleanup_done = true;
            if self.state.turn.pending_level_up.is_some() {
                return false;
            }
            if !self.state.turn.pending_triggers.is_empty() {
                return false;
            }
        }
        self.expire_end_of_turn_effects();
        self.finish_end_phase(player);
        self.state.turn.end_phase_pending = false;
        true
    }

    pub(in crate::env) fn start_end_phase_discard_choice(&mut self, player: u8) -> bool {
        self.scratch.choice_options.clear();
        for (idx, card) in self.state.players[player as usize].hand.iter().enumerate() {
            debug_assert!(idx <= u16::MAX as usize, "end-phase hand index exceeds u16");
            let index = if idx <= u16::MAX as usize {
                Some(idx as u16)
            } else {
                None
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: card.id,
                instance_id: card.instance_id,
                zone: ChoiceZone::Hand,
                index,
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(ChoiceReason::EndPhaseDiscard, player, options, None)
    }

    pub(in crate::env) fn expire_end_of_turn_effects(&mut self) {
        for pid in 0..2 {
            for slot in &mut self.state.players[pid].stage {
                slot.power_mod_battle = 0;
                slot.power_mod_turn = 0;
                slot.cannot_attack = false;
                slot.attack_cost = 0;
            }
        }
        self.mark_all_slot_power_dirty();
        let mut removed: Vec<u32> = Vec::new();
        self.state.modifiers.retain(|m| {
            if m.duration == ModifierDuration::UntilEndOfTurn {
                removed.push(m.id);
                false
            } else {
                true
            }
        });
        if !removed.is_empty() {
            self.bump_modifiers_version();
        }
        for id in removed {
            self.log_event(Event::ModifierRemoved {
                id,
                reason: ModifierRemoveReason::EndOfTurn,
            });
        }
        self.state.turn.derived_attack = None;
        self.mark_continuous_modifiers_dirty();
        self.maybe_validate_state("end_phase_expire");
    }

    pub(in crate::env) fn finish_end_phase(&mut self, player: u8) {
        self.state.turn.pending_triggers.clear();
        self.state.turn.pending_triggers_sorted = true;
        self.state.turn.trigger_order = None;
        self.state.turn.choice = None;
        self.state.turn.priority = None;
        self.state.turn.stack.clear();
        self.state.turn.pending_stack_groups.clear();
        self.state.turn.stack_order = None;
        self.state.turn.derived_attack = None;
        self.state.turn.attack = None;
        self.state.turn.encore_queue.clear();
        self.state.turn.encore_step_player = None;
        self.state.turn.pending_level_up = None;
        self.state.turn.main_passed = false;
        self.state.turn.active_window = None;
        self.state.turn.end_phase_window_done = false;
        self.state.turn.end_phase_discard_done = false;
        self.state.turn.end_phase_climax_done = false;
        self.state.turn.end_phase_cleanup_done = false;
        self.state.turn.encore_window_done = false;
        self.state.turn.pending_losses = [false; 2];
        self.state.turn.attack_subphase_count = 0;
        self.state.turn.phase_step = 0;
        self.state.turn.attack_phase_begin_done = false;
        self.state.turn.attack_decl_check_done = false;
        self.state.turn.encore_begin_done = false;
        self.state.turn.pending_resolution_cleanup.clear();
        self.state.turn.turn_number = self.state.turn.turn_number.saturating_add(1);
        self.log_event(Event::EndTurn { player });
        self.maybe_validate_state("end_phase_finish");
    }
}
