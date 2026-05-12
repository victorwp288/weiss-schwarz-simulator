use super::*;
use smallvec::SmallVec;

impl GameEnv {
    #[inline]
    pub(in crate::env) fn resolve_battle_step(&mut self, ctx: &AttackContext) {
        let attacker = self.state.turn.active_player as usize;
        let defender = 1 - attacker;
        let atk_slot = ctx.attacker_slot as usize;
        let def_slot = match ctx.defender_slot {
            Some(s) => s as usize,
            None => return,
        };
        let collect_reverse_triggers =
            self.curriculum.enable_triggers && self.curriculum.enable_on_reverse_triggers;
        let mut reversed: SmallVec<[(u8, CardId); 2]> = SmallVec::new();
        let mut battle_opponent_reversed_sources: SmallVec<[(u8, CardId); 2]> = SmallVec::new();
        let atk_power = self.compute_slot_power(attacker, atk_slot);
        let def_power = self.compute_slot_power(defender, def_slot);

        let can_become_reverse = |player: usize, slot: usize| -> bool {
            if slot >= self.state.players[player].stage.len() {
                return false;
            }
            let Some(card_inst) = self.state.players[player].stage[slot].card else {
                return false;
            };
            !self.state.modifiers.iter().any(|modifier| {
                modifier.target_player as usize == player
                    && modifier.target_slot as usize == slot
                    && modifier.target_card == card_inst.id
                    && modifier.kind == ModifierKind::CannotBecomeReverse
                    && modifier.magnitude != 0
            })
        };

        let defender_can_reverse = can_become_reverse(defender, def_slot);
        let attacker_can_reverse = can_become_reverse(attacker, atk_slot);

        let maybe_move_reversed_to_memory =
            |source_player: usize,
             source_slot: usize,
             target_player: usize,
             target_slot: usize,
             this: &mut GameEnv| {
                if !this.slot_has_active_modifier_kind(
                    source_player as u8,
                    source_slot as u8,
                    ModifierKind::BattleOpponentMoveToMemoryOnReverse,
                ) {
                    return;
                }
                if target_slot >= this.state.players[target_player].stage.len() {
                    return;
                }
                let Some(target_card_inst) =
                    this.state.players[target_player].stage[target_slot].card
                else {
                    return;
                };
                let target_ref = TargetRef {
                    player: target_player as u8,
                    zone: TargetZone::Stage,
                    index: target_slot as u8,
                    card_id: target_card_inst.id,
                    instance_id: target_card_inst.instance_id,
                };
                let _ = this.move_stage_target_to_memory(target_ref);
            };

        if atk_power > def_power {
            if defender_can_reverse {
                self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: defender as u8,
                    slot: def_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if collect_reverse_triggers {
                    if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                        reversed.push((defender as u8, card_inst.id));
                    }
                    if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                        battle_opponent_reversed_sources.push((attacker as u8, card_inst.id));
                    }
                }
                maybe_move_reversed_to_memory(attacker, atk_slot, defender, def_slot, self);
            }
        } else if atk_power < def_power {
            if attacker_can_reverse {
                self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: attacker as u8,
                    slot: atk_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if collect_reverse_triggers {
                    if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                        reversed.push((attacker as u8, card_inst.id));
                    }
                    if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                        battle_opponent_reversed_sources.push((defender as u8, card_inst.id));
                    }
                }
                maybe_move_reversed_to_memory(defender, def_slot, attacker, atk_slot, self);
            }
        } else {
            if defender_can_reverse {
                self.state.players[defender].stage[def_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: defender as u8,
                    slot: def_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if collect_reverse_triggers {
                    if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                        reversed.push((defender as u8, card_inst.id));
                    }
                    if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                        battle_opponent_reversed_sources.push((attacker as u8, card_inst.id));
                    }
                }
                maybe_move_reversed_to_memory(attacker, atk_slot, defender, def_slot, self);
            }
            if attacker_can_reverse {
                self.state.players[attacker].stage[atk_slot].status = StageStatus::Reverse;
                self.log_event(Event::ReversalCommitted {
                    player: attacker as u8,
                    slot: atk_slot as u8,
                    cause_damage_event: ctx.last_damage_event_id,
                });
                if collect_reverse_triggers {
                    if let Some(card_inst) = self.state.players[attacker].stage[atk_slot].card {
                        reversed.push((attacker as u8, card_inst.id));
                    }
                    if let Some(card_inst) = self.state.players[defender].stage[def_slot].card {
                        battle_opponent_reversed_sources.push((defender as u8, card_inst.id));
                    }
                }
                maybe_move_reversed_to_memory(defender, def_slot, attacker, atk_slot, self);
            }
        }
        if !reversed.is_empty() {
            self.queue_on_reverse_triggers(&reversed);
        }
        if !battle_opponent_reversed_sources.is_empty() {
            self.queue_battle_opponent_reverse_triggers(&battle_opponent_reversed_sources);
        }
    }
}
