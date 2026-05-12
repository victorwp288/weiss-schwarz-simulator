use super::*;

impl GameEnv {
    #[inline]
    pub(in crate::env) fn resolve_trigger_step(&mut self, ctx: &mut AttackContext) {
        let active = self.state.turn.active_player as usize;
        let card = self.draw_from_deck(active as u8);
        if let Some(card_inst) = card {
            let card_id = card_inst.id;
            let instance_id = card_inst.instance_id;
            ctx.trigger_card = Some(card_id);
            ctx.trigger_instance_id = Some(instance_id);
            let _ = self.reveal_cards(
                active as u8,
                &[card_inst],
                RevealReason::TriggerCheck,
                RevealAudience::Public,
            );
            self.move_card_between_zones(
                active as u8,
                card_inst,
                Zone::Deck,
                Zone::Resolution,
                None,
                None,
            );
            self.state.turn.attack = Some(ctx.clone());
            self.queue_timing_triggers(crate::db::AbilityTiming::TriggerResolution);
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
                            TriggerIcon::Choice => effects.push(TriggerEffect::Choice),
                            TriggerIcon::Pool => effects.push(TriggerEffect::Pool),
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
                    let has_delayed_trigger_card_move = effects
                        .iter()
                        .any(|e| matches!(e, TriggerEffect::Treasure | TriggerEffect::Pool));
                    self.queue_trigger_group(active as u8, card_id, effects);
                    if has_delayed_trigger_card_move {
                        return;
                    }
                }
            }
            if let Some(resolved) = self.take_resolution_card(active as u8, instance_id) {
                self.move_card_between_zones(
                    active as u8,
                    resolved,
                    Zone::Resolution,
                    Zone::Stock,
                    None,
                    None,
                );
            }
        }
    }
}
