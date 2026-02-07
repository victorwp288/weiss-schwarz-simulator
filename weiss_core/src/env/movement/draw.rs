use super::super::GameEnv;
use crate::db::CardType;
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::state::{CardInstance, CardInstanceId};

impl GameEnv {
    pub(in crate::env) fn shuffle_deck(&mut self, player: u8) {
        let p = player as usize;
        self.touch_player_obs(player);
        self.state.rng.shuffle(&mut self.state.players[p].deck);
        self.log_event(Event::Shuffle {
            player,
            zone: Zone::Deck,
        });
        if self.curriculum.enable_visibility_policies {
            let instance_ids: Vec<CardInstanceId> = self.state.players[p]
                .deck
                .iter()
                .map(|c| c.instance_id)
                .collect();
            for instance_id in instance_ids {
                self.forget_instance_revealed(instance_id);
            }
        }
    }

    pub(in crate::env) fn draw_to_hand(&mut self, player: u8, count: usize) {
        for _ in 0..count {
            if let Some(card) = self.draw_from_deck(player) {
                let card_id = card.id;
                self.move_card_between_zones(player, card, Zone::Deck, Zone::Hand, None, None);
                self.log_event(Event::Draw {
                    player,
                    card: card_id,
                });
            }
        }
    }

    pub(in crate::env) fn draw_from_deck(&mut self, player: u8) -> Option<CardInstance> {
        let p = player as usize;
        if self.state.players[p].deck.is_empty() {
            if self.state.turn.cost_payment_depth > 0 {
                return None;
            }
            if !self.refresh_deck(player) {
                return None;
            }
        }
        let card = self.state.players[p].deck.pop()?;
        Some(card)
    }

    pub(in crate::env) fn refresh_deck(&mut self, player: u8) -> bool {
        let p = player as usize;
        if self.state.players[p].waiting_room.is_empty() {
            let in_damage = self.state.turn.damage_resolution_target == Some(player);
            if in_damage {
                let has_climax = self.state.players[p].resolution.iter().any(|card| {
                    self.db
                        .get(card.id)
                        .map(|c| c.card_type == CardType::Climax)
                        .unwrap_or(false)
                });
                if has_climax {
                    return false;
                }
            }
            self.register_loss(player);
            return false;
        }
        debug_assert!(
            self.state.players[p].deck.is_empty(),
            "refresh_deck called with non-empty deck"
        );
        self.state.players[p].deck = std::mem::take(&mut self.state.players[p].waiting_room);
        self.shuffle_deck(player);
        self.log_event(Event::Refresh { player });
        if self.curriculum.enable_refresh_penalty {
            if let Some(card) = self.state.players[p].deck.pop() {
                self.reveal_card(
                    player,
                    &card,
                    RevealReason::RefreshPenalty,
                    RevealAudience::Public,
                );
                let card_id = card.id;
                self.move_card_between_zones(player, card, Zone::Deck, Zone::Clock, None, None);
                self.log_event(Event::RefreshPenalty {
                    player,
                    card: card_id,
                });
                self.check_level_up(player);
            }
        }
        true
    }
}
