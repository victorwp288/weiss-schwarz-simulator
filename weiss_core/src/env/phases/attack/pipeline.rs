use super::*;

impl GameEnv {
    #[inline]
    pub(in crate::env) fn resolve_attack_pipeline(&mut self) {
        // Invariants:
        // - Preserve deterministic damage modifier application order.
        //   See `weiss_core/tests/determinism_tests.rs`.
        loop {
            let Some(ctx) = self.state.turn.attack.take() else {
                return;
            };
            let flow = match ctx.step {
                AttackStep::Trigger => self.resolve_attack_pipeline_trigger_step(ctx),
                AttackStep::Counter => self.resolve_attack_pipeline_counter_step(ctx),
                AttackStep::Damage => self.resolve_attack_pipeline_damage_step(ctx),
                AttackStep::Battle => self.resolve_attack_pipeline_battle_step(ctx),
                AttackStep::Encore => self.resolve_attack_pipeline_encore_step(ctx),
            };
            match flow {
                AttackPipelineFlow::Continue => {}
                AttackPipelineFlow::Break => break,
                AttackPipelineFlow::Return => return,
            }
            if self.maybe_validate_state("attack_pipeline") {
                return;
            }
        }
    }

    #[inline]
    fn resolve_attack_pipeline_trigger_step(
        &mut self,
        mut ctx: AttackContext,
    ) -> AttackPipelineFlow {
        if self.curriculum.enable_priority_windows && !ctx.decl_window_done {
            ctx.decl_window_done = true;
            self.state.turn.attack = Some(ctx);
            self.attack_enter_timing_window_if_idle(
                TimingWindow::AttackDeclarationWindow,
                self.state.turn.active_player,
            );
            return AttackPipelineFlow::Break;
        }
        if !ctx.auto_trigger_enqueued {
            self.enqueue_other_attack_declaration_auto_effects(&ctx, self.state.turn.active_player);
            self.enqueue_attack_auto_effects(
                &ctx,
                self.state.turn.active_player,
                AttackAutoResolvePhase::TriggerStep,
            );
            ctx.auto_trigger_enqueued = true;
            if self.attack_has_pending_resolution_work() {
                self.state.turn.attack = Some(ctx);
                if self.maybe_validate_state("attack_decl_auto_pause") {
                    return AttackPipelineFlow::Return;
                }
                return AttackPipelineFlow::Break;
            }
        }
        self.resolve_trigger_step(&mut ctx);
        ctx.trigger_checks_resolved = ctx.trigger_checks_resolved.saturating_add(1);
        let trigger_checks_total = ctx.trigger_checks_total.max(1);
        if ctx.trigger_checks_resolved >= trigger_checks_total {
            if ctx.counter_allowed && self.curriculum.enable_counters {
                ctx.step = AttackStep::Counter;
            } else {
                ctx.step = AttackStep::Damage;
            }
        } else {
            // Re-enter trigger step for additional trigger checks granted this attack.
            ctx.step = AttackStep::Trigger;
            ctx.trigger_window_done = false;
        }
        if self.attack_has_pending_level_or_trigger() {
            self.state.turn.attack = Some(ctx);
            if self.maybe_validate_state("attack_trigger_pause") {
                return AttackPipelineFlow::Return;
            }
            return AttackPipelineFlow::Break;
        }
        if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
            ctx.trigger_window_done = true;
            self.state.turn.attack = Some(ctx);
            self.attack_enter_timing_window_if_idle(
                TimingWindow::TriggerResolutionWindow,
                self.state.turn.active_player,
            );
            return AttackPipelineFlow::Break;
        }
        self.state.turn.attack = Some(ctx);
        AttackPipelineFlow::Continue
    }

    #[inline]
    fn resolve_attack_pipeline_counter_step(
        &mut self,
        mut ctx: AttackContext,
    ) -> AttackPipelineFlow {
        if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
            ctx.trigger_window_done = true;
            self.state.turn.attack = Some(ctx);
            self.attack_enter_timing_window_if_idle(
                TimingWindow::TriggerResolutionWindow,
                self.state.turn.active_player,
            );
            return AttackPipelineFlow::Break;
        }
        let defender = 1 - self.state.turn.active_player;
        self.state.turn.attack = Some(ctx);
        self.attack_enter_timing_window_if_idle(TimingWindow::CounterWindow, defender);
        if self.maybe_validate_state("attack_counter_window") {
            return AttackPipelineFlow::Return;
        }
        AttackPipelineFlow::Break
    }

    #[inline]
    fn resolve_attack_pipeline_damage_step(
        &mut self,
        mut ctx: AttackContext,
    ) -> AttackPipelineFlow {
        if self.curriculum.enable_priority_windows && !ctx.trigger_window_done {
            ctx.trigger_window_done = true;
            self.state.turn.attack = Some(ctx);
            self.attack_enter_timing_window_if_idle(
                TimingWindow::TriggerResolutionWindow,
                self.state.turn.active_player,
            );
            return AttackPipelineFlow::Break;
        }
        if !ctx.auto_damage_enqueued {
            self.enqueue_attack_auto_effects(
                &ctx,
                self.state.turn.active_player,
                AttackAutoResolvePhase::DamageStep,
            );
            ctx.auto_damage_enqueued = true;
            if self.attack_has_pending_resolution_work() {
                self.state.turn.attack = Some(ctx);
                if self.maybe_validate_state("attack_damage_auto_pause") {
                    return AttackPipelineFlow::Return;
                }
                return AttackPipelineFlow::Break;
            }
        }
        let pause = self.resolve_damage_step(&mut ctx);
        if pause {
            self.state.turn.attack = Some(ctx);
            if self.maybe_validate_state("attack_damage_pause") {
                return AttackPipelineFlow::Return;
            }
            return AttackPipelineFlow::Break;
        }
        if ctx.attack_type == AttackType::Direct {
            self.finish_attack_and_run_end_of_attack_timing();
            if self.attack_has_pending_level_or_trigger() {
                return AttackPipelineFlow::Break;
            }
            if self.maybe_validate_state("attack_direct_done") {
                return AttackPipelineFlow::Return;
            }
            return AttackPipelineFlow::Break;
        }
        ctx.step = AttackStep::Battle;
        if self.curriculum.enable_priority_windows && !ctx.damage_window_done {
            ctx.damage_window_done = true;
            self.state.turn.attack = Some(ctx);
            self.attack_enter_timing_window_if_idle(
                TimingWindow::DamageResolutionWindow,
                self.state.turn.active_player,
            );
            return AttackPipelineFlow::Break;
        }
        self.state.turn.attack = Some(ctx);
        AttackPipelineFlow::Continue
    }

    #[inline]
    fn resolve_attack_pipeline_battle_step(
        &mut self,
        mut ctx: AttackContext,
    ) -> AttackPipelineFlow {
        if self.curriculum.enable_priority_windows && !ctx.damage_window_done {
            ctx.damage_window_done = true;
            self.state.turn.attack = Some(ctx);
            self.attack_enter_timing_window_if_idle(
                TimingWindow::DamageResolutionWindow,
                self.state.turn.active_player,
            );
            return AttackPipelineFlow::Break;
        }
        self.resolve_battle_step(&ctx);
        self.finish_attack_and_run_end_of_attack_timing();
        if self.attack_has_pending_level_or_trigger() {
            return AttackPipelineFlow::Break;
        }
        if self.maybe_validate_state("attack_battle_done") {
            return AttackPipelineFlow::Return;
        }
        AttackPipelineFlow::Break
    }

    #[inline]
    fn resolve_attack_pipeline_encore_step(&mut self, ctx: AttackContext) -> AttackPipelineFlow {
        self.state.turn.attack = Some(ctx);
        if self.maybe_validate_state("attack_encore_hold") {
            return AttackPipelineFlow::Return;
        }
        AttackPipelineFlow::Break
    }

    fn attack_enter_timing_window_if_idle(&mut self, window: TimingWindow, player: u8) {
        if self.state.turn.priority.is_none() {
            self.enter_timing_window(window, player);
        }
    }

    fn attack_has_pending_resolution_work(&self) -> bool {
        !self.state.turn.stack.is_empty()
            || self.state.turn.pending_level_up.is_some()
            || !self.state.turn.pending_triggers.is_empty()
            || self.state.turn.pending_cost.is_some()
            || self.state.turn.choice.is_some()
    }

    fn attack_has_pending_level_or_trigger(&self) -> bool {
        self.state.turn.pending_level_up.is_some() || !self.state.turn.pending_triggers.is_empty()
    }

    fn finish_attack_and_run_end_of_attack_timing(&mut self) {
        self.clear_battle_mods();
        self.state.turn.attack = None;
        self.state.turn.attack_decl_check_done = false;
        self.run_check_timing(crate::db::AbilityTiming::EndOfAttack);
    }
}
