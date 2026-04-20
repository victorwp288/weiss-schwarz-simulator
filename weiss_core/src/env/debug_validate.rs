use anyhow::{anyhow, Result};

use crate::db::CardId;
use crate::legal::DecisionKind;
use crate::state::{CardInstance, CardInstanceId, Phase};

use super::debug_fingerprints;
use super::{EngineErrorCode, FaultSource, GameEnv};

impl GameEnv {
    /// Whether expensive state validation should run on this step/reset path.
    pub(crate) fn should_validate_state(&self) -> bool {
        if cfg!(debug_assertions) {
            return true;
        }
        self.validate_state_enabled
    }

    /// Run validation and latch an invariant fault when validation fails.
    ///
    /// Returns `true` when a fault was latched (validation failed).
    pub(crate) fn maybe_validate_state(&mut self, context: &str) -> bool {
        if !self.should_validate_state() {
            return false;
        }
        if let Err(err) = self.validate_state() {
            eprintln!("State validation failed in {context}: {err}");
            let actor = self.decision.as_ref().map(|d| d.player);
            self.latch_fault_deferred(
                EngineErrorCode::InvariantViolation,
                actor,
                FaultSource::Step,
            );
            return true;
        }
        false
    }

    /// Run expensive invariants checks over the full game state.
    ///
    /// Intended for debug builds or diagnostics; returns a detailed error.
    pub fn validate_state(&self) -> Result<()> {
        use std::collections::{HashMap, HashSet};
        let mut errors = Vec::new();

        let mut counts: [HashMap<CardId, i32>; 2] = [HashMap::new(), HashMap::new()];
        for (owner, owner_counts) in counts.iter_mut().enumerate() {
            let deck_list = &self.config.deck_lists[owner];
            for card in deck_list.iter().copied() {
                *owner_counts.entry(card).or_insert(0) += 1;
            }
        }

        fn consume(
            counts: &mut [HashMap<CardId, i32>; 2],
            errors: &mut Vec<String>,
            owner: u8,
            card: CardId,
            zone: &str,
        ) {
            let owner_idx = owner as usize;
            let entry = counts[owner_idx].entry(card).or_insert(0);
            *entry -= 1;
            if *entry < 0 {
                errors.push(format!("Owner {owner} has extra card {card} in {zone}"));
            }
        }

        let mut instance_ids: HashSet<CardInstanceId> = HashSet::new();
        fn check_instance(
            instance_ids: &mut HashSet<CardInstanceId>,
            errors: &mut Vec<String>,
            card: &CardInstance,
            zone: &str,
        ) {
            if card.instance_id == 0 {
                errors.push(format!("Card instance id 0 in {zone}"));
                return;
            }
            if !instance_ids.insert(card.instance_id) {
                errors.push(format!(
                    "Duplicate instance id {} in {zone}",
                    card.instance_id
                ));
            }
        }

        for zone_player in 0..2 {
            let p = &self.state.players[zone_player];
            for card in &p.deck {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} deck"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} deck"),
                );
            }
            for card in &p.hand {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} hand"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} hand"),
                );
            }
            for card in &p.waiting_room {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} waiting_room"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} waiting_room"),
                );
            }
            for card in &p.clock {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} clock"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} clock"),
                );
            }
            for card in &p.level {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} level"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} level"),
                );
            }
            for card in &p.stock {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} stock"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} stock"),
                );
            }
            for card in &p.memory {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} memory"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} memory"),
                );
            }
            for card in &p.climax {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} climax"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} climax"),
                );
            }
            for card in &p.resolution {
                consume(
                    &mut counts,
                    &mut errors,
                    card.owner,
                    card.id,
                    &format!("p{zone_player} resolution"),
                );
                check_instance(
                    &mut instance_ids,
                    &mut errors,
                    card,
                    &format!("p{zone_player} resolution"),
                );
            }
            for (slot_idx, slot) in p.stage.iter().enumerate() {
                if let Some(card) = slot.card {
                    consume(
                        &mut counts,
                        &mut errors,
                        card.owner,
                        card.id,
                        &format!("p{zone_player} stage[{slot_idx}]"),
                    );
                    check_instance(
                        &mut instance_ids,
                        &mut errors,
                        &card,
                        &format!("p{zone_player} stage[{slot_idx}]"),
                    );
                }
                for marker in &slot.markers {
                    consume(
                        &mut counts,
                        &mut errors,
                        marker.owner,
                        marker.id,
                        &format!("p{zone_player} stage[{slot_idx}] marker"),
                    );
                    check_instance(
                        &mut instance_ids,
                        &mut errors,
                        marker,
                        &format!("p{zone_player} stage[{slot_idx}] marker"),
                    );
                }
            }
        }

        for (owner, owner_counts) in counts.iter().enumerate() {
            for (card, remaining) in owner_counts.iter() {
                if *remaining != 0 {
                    errors.push(format!(
                        "Owner {owner} card {card} count mismatch ({remaining})"
                    ));
                }
            }
        }

        if let Some(decision) = &self.decision {
            if let Some(slot) = decision.focus_slot {
                if slot as usize >= self.state.players[decision.player as usize].stage.len() {
                    errors.push("Decision focus slot out of range".to_string());
                }
            }
            match decision.kind {
                DecisionKind::AttackDeclaration if self.state.turn.attack.is_some() => {
                    errors.push("Attack declaration while attack context active".to_string());
                }
                DecisionKind::LevelUp if self.state.turn.pending_level_up.is_none() => {
                    errors.push("Level up decision without pending level".to_string());
                }
                DecisionKind::Encore => {
                    let has = self
                        .state
                        .turn
                        .encore_queue
                        .iter()
                        .any(|r| r.player == decision.player);
                    if !has {
                        errors.push("Encore decision without reversed options".to_string());
                    }
                }
                DecisionKind::TriggerOrder if self.state.turn.trigger_order.is_none() => {
                    errors.push("Trigger order decision without pending order".to_string());
                }
                DecisionKind::Choice => {
                    if let Some(choice) = &self.state.turn.choice {
                        if choice.player != decision.player {
                            errors.push("Choice decision player mismatch".to_string());
                        }
                    } else {
                        errors.push("Choice decision without pending choice".to_string());
                    }
                }
                _ => {}
            }
        }

        if self.state.turn.attack.is_some() && self.state.turn.phase != Phase::Attack {
            errors.push("Attack context outside Attack phase".to_string());
        }

        if errors.is_empty() {
            return Ok(());
        }

        let state_hash = debug_fingerprints::state_fingerprint(&self.state);
        let phase = self.state.turn.phase;
        let attack_step = self.state.turn.attack.as_ref().map(|c| c.step);
        let tail_len = 8usize;
        let actions_tail: Vec<String> = self
            .replay_actions
            .iter()
            .rev()
            .take(tail_len)
            .rev()
            .map(|a| format!("{a:?}"))
            .collect();
        let decisions_tail: Vec<String> = self
            .replay_steps
            .iter()
            .rev()
            .take(tail_len)
            .rev()
            .map(|s| format!("{:?}/{:?}", s.decision_kind, s.actor))
            .collect();
        let fallback_action = self
            .last_action_desc
            .as_ref()
            .map(|a| format!("{a:?}"))
            .unwrap_or_else(|| "None".to_string());
        let payload = format!(
            "seed={}\nphase={:?}\nattack_step={:?}\nlast_action={}\nactions_tail={:?}\ndecisions_tail={:?}\nstate_hash={}",
            self.episode_seed,
            phase,
            attack_step,
            fallback_action,
            actions_tail,
            decisions_tail,
            state_hash,
        );
        Err(anyhow!("{}\n{}", payload, errors.join("; ")))
    }
}
