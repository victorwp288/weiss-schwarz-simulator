use super::GameEnv;
use crate::db::{AbilitySpec, CardId};
use crate::effects::EffectSpec;
use crate::state::CardInstanceId;

pub(in crate::env) struct LiveAbilityRef<'a> {
    pub spec: &'a AbilitySpec,
    pub effects: &'a [EffectSpec],
    pub grant_id: Option<u64>,
}

impl GameEnv {
    pub(in crate::env) fn prune_granted_abilities(&mut self) {
        self.state.turn.granted_abilities.retain(|grant| {
            if grant.expires_turn_number < self.state.turn.turn_number {
                return false;
            }
            let p = grant.target_player as usize;
            let s = grant.target_slot as usize;
            self.state
                .players
                .get(p)
                .and_then(|player| player.stage.get(s))
                .and_then(|slot| slot.card)
                .map(|card| card.instance_id == grant.target_instance_id)
                .unwrap_or(false)
        });
    }

    pub(in crate::env) fn remove_granted_abilities_for_stage_instance(
        &mut self,
        player: u8,
        slot: u8,
        instance_id: CardInstanceId,
    ) {
        self.state.turn.granted_abilities.retain(|grant| {
            !(grant.target_player == player
                && grant.target_slot == slot
                && grant.target_instance_id == instance_id)
        });
    }

    pub(in crate::env) fn live_stage_ability_at<'a>(
        &'a self,
        player: u8,
        slot: u8,
        ability_index: usize,
    ) -> Option<LiveAbilityRef<'a>> {
        let p = player as usize;
        let s = slot as usize;
        let card_inst = self.state.players.get(p)?.stage.get(s)?.card?;
        let card_id: CardId = card_inst.id;

        let static_specs = self.db.iter_card_abilities_in_canonical_order(card_id);
        if ability_index < static_specs.len() {
            let spec = static_specs.get(ability_index)?;
            let effects = self.db.compiled_effects_for_ability(card_id, ability_index);
            return Some(LiveAbilityRef {
                spec,
                effects,
                grant_id: None,
            });
        }

        let mut remaining = ability_index.saturating_sub(static_specs.len());
        for grant in &self.state.turn.granted_abilities {
            if grant.target_player != player
                || grant.target_slot != slot
                || grant.target_instance_id != card_inst.instance_id
            {
                continue;
            }
            if remaining == 0 {
                return Some(LiveAbilityRef {
                    spec: &grant.spec,
                    effects: grant.compiled_effects.as_slice(),
                    grant_id: Some(grant.grant_id),
                });
            }
            remaining = remaining.saturating_sub(1);
        }
        None
    }

    pub(in crate::env) fn live_stage_ability_count(&self, player: u8, slot: u8) -> usize {
        let p = player as usize;
        let s = slot as usize;
        let Some(card_inst) = self
            .state
            .players
            .get(p)
            .and_then(|state| state.stage.get(s))
            .and_then(|slot_state| slot_state.card)
        else {
            return 0;
        };
        let static_count = self
            .db
            .iter_card_abilities_in_canonical_order(card_inst.id)
            .len();
        let granted_count = self
            .state
            .turn
            .granted_abilities
            .iter()
            .filter(|grant| {
                grant.target_player == player
                    && grant.target_slot == slot
                    && grant.target_instance_id == card_inst.instance_id
            })
            .count();
        static_count + granted_count
    }
}
