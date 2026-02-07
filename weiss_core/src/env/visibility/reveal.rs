use super::super::GameEnv;
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::state::{CardInstance, CardInstanceId};
use crate::visibility_policy::{zone_identity_visibility, ZoneIdentityVisibility};

impl GameEnv {
    pub(in crate::env) fn reveal_card(
        &mut self,
        player: u8,
        card: &CardInstance,
        reason: RevealReason,
        audience: RevealAudience,
    ) {
        let mut viewers = [0u8; 2];
        let mut count = 0usize;
        match audience {
            RevealAudience::Public | RevealAudience::BothPlayers => {
                viewers[0] = 0;
                viewers[1] = 1;
                count = 2;
            }
            RevealAudience::OwnerOnly => {
                viewers[0] = card.owner;
                count = 1;
            }
            RevealAudience::ControllerOnly => {
                viewers[0] = card.controller;
                count = 1;
            }
            RevealAudience::ReplayOnly => {}
        }
        for &viewer in viewers[..count].iter() {
            if let Some(history) = self.state.reveal_history.get_mut(viewer as usize) {
                history.push(card.id);
            }
        }
        if self.curriculum.enable_visibility_policies && count > 0 {
            self.mark_instance_revealed(&viewers[..count], card.instance_id);
        }
        self.log_event(Event::Reveal {
            player,
            card: card.id,
            reason,
            audience,
        });
    }

    pub(in crate::env) fn reveal_cards(
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

    pub(in crate::env) fn mark_instance_revealed(
        &mut self,
        viewers: &[u8],
        instance_id: CardInstanceId,
    ) {
        if instance_id == 0 {
            return;
        }
        for &viewer in viewers {
            if let Some(set) = self.revealed_to_viewer.get_mut(viewer as usize) {
                set.insert(instance_id);
            }
        }
    }

    pub(in crate::env) fn forget_instance_revealed(&mut self, instance_id: CardInstanceId) {
        if instance_id == 0 {
            return;
        }
        for set in &mut self.revealed_to_viewer {
            set.remove(&instance_id);
        }
    }

    pub(in crate::env) fn on_card_enter_zone(&mut self, card: &CardInstance, zone: Zone) {
        if !self.curriculum.enable_visibility_policies {
            return;
        }
        match zone_identity_visibility(zone, &self.curriculum) {
            ZoneIdentityVisibility::Public => {
                self.mark_instance_revealed(&[0, 1], card.instance_id);
            }
            ZoneIdentityVisibility::OwnerOnly => {
                self.forget_instance_revealed(card.instance_id);
                self.mark_instance_revealed(&[card.owner], card.instance_id);
            }
        }
    }
}
