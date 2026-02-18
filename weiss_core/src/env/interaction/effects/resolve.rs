use crate::db::{CardId, CardType, TriggerIcon};
use crate::effects::*;
use crate::encode::MAX_STAGE;
use crate::env::GameEnv;
use crate::events::{Event, RevealAudience, RevealReason, Zone};
use crate::state::*;

impl GameEnv {
    pub(in crate::env) fn resolve_effect_payload(
        &mut self,
        controller: u8,
        source_id: CardId,
        payload: &EffectPayload,
    ) {
        match &payload.spec.kind {
            EffectKind::Draw { count } => {
                self.draw_to_hand(controller, *count as usize);
            }
            EffectKind::Brainstorm {
                reveal_count,
                per_climax,
                mode,
            } => {
                let mut climax_hits: u32 = 0;
                for _ in 0..*reveal_count {
                    let Some(card) = self.draw_from_deck(controller) else {
                        break;
                    };
                    let is_climax = self
                        .db
                        .get(card.id)
                        .map(|c| c.card_type == CardType::Climax)
                        .unwrap_or(false);
                    self.reveal_card(
                        controller,
                        &card,
                        RevealReason::AbilityEffect,
                        RevealAudience::Public,
                    );
                    self.move_card_between_zones(
                        controller,
                        card,
                        Zone::Deck,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                    if is_climax {
                        climax_hits = climax_hits.saturating_add(1);
                    }
                }
                let total = climax_hits.saturating_mul(*per_climax as u32);
                if total == 0 {
                    return;
                }
                match mode {
                    crate::db::BrainstormMode::Draw => {
                        let spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::BrainstormDrawChoice,
                            target: None,
                            optional: false,
                        };
                        for _ in 0..total {
                            self.enqueue_effect_spec(controller, source_id, spec.clone());
                        }
                    }
                    crate::db::BrainstormMode::SalvageCharacter => {
                        let spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::MoveToHand,
                            target: Some(TargetSpec {
                                zone: TargetZone::WaitingRoom,
                                side: TargetSide::SelfSide,
                                slot_filter: TargetSlotFilter::Any,
                                card_type: Some(CardType::Character),
                                card_trait: None,
                                level_max: None,
                                cost_max: None,
                                card_ids: Vec::new(),
                                count: 1,
                                limit: None,
                                source_only: false,
                                reveal_to_controller: false,
                            }),
                            optional: true,
                        };
                        for _ in 0..total {
                            self.enqueue_effect_spec(controller, source_id, spec.clone());
                        }
                    }
                    crate::db::BrainstormMode::LookTopToHand => {
                        let spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::MoveToHand,
                            target: Some(TargetSpec {
                                zone: TargetZone::DeckTop,
                                side: TargetSide::SelfSide,
                                slot_filter: TargetSlotFilter::Any,
                                card_type: None,
                                card_trait: None,
                                level_max: None,
                                cost_max: None,
                                card_ids: Vec::new(),
                                count: 1,
                                limit: Some(3),
                                source_only: false,
                                reveal_to_controller: false,
                            }),
                            optional: true,
                        };
                        for _ in 0..total {
                            self.enqueue_effect_spec(controller, source_id, spec.clone());
                        }
                    }
                    crate::db::BrainstormMode::LookTopToHandThenDiscard => {
                        let look_spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::MoveToHand,
                            target: Some(TargetSpec {
                                zone: TargetZone::DeckTop,
                                side: TargetSide::SelfSide,
                                slot_filter: TargetSlotFilter::Any,
                                card_type: None,
                                card_trait: None,
                                level_max: None,
                                cost_max: None,
                                card_ids: Vec::new(),
                                count: 1,
                                limit: Some(3),
                                source_only: false,
                                reveal_to_controller: false,
                            }),
                            optional: true,
                        };
                        let discard_spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::MoveToWaitingRoom,
                            target: Some(TargetSpec {
                                zone: TargetZone::Hand,
                                side: TargetSide::SelfSide,
                                slot_filter: TargetSlotFilter::Any,
                                card_type: None,
                                card_trait: None,
                                level_max: None,
                                cost_max: None,
                                card_ids: Vec::new(),
                                count: 1,
                                limit: None,
                                source_only: false,
                                reveal_to_controller: false,
                            }),
                            optional: false,
                        };
                        for _ in 0..total {
                            // Enqueue discard first so look-to-hand resolves before discard.
                            self.enqueue_effect_spec(controller, source_id, discard_spec.clone());
                            self.enqueue_effect_spec(controller, source_id, look_spec.clone());
                        }
                    }
                    crate::db::BrainstormMode::SalvageCharacterThenDiscard => {
                        let salvage_spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::MoveToHand,
                            target: Some(TargetSpec {
                                zone: TargetZone::WaitingRoom,
                                side: TargetSide::SelfSide,
                                slot_filter: TargetSlotFilter::Any,
                                card_type: Some(CardType::Character),
                                card_trait: None,
                                level_max: None,
                                cost_max: None,
                                card_ids: Vec::new(),
                                count: 1,
                                limit: None,
                                source_only: false,
                                reveal_to_controller: false,
                            }),
                            optional: true,
                        };
                        let discard_spec = EffectSpec {
                            id: payload.spec.id,
                            kind: EffectKind::MoveToWaitingRoom,
                            target: Some(TargetSpec {
                                zone: TargetZone::Hand,
                                side: TargetSide::SelfSide,
                                slot_filter: TargetSlotFilter::Any,
                                card_type: None,
                                card_trait: None,
                                level_max: None,
                                cost_max: None,
                                card_ids: Vec::new(),
                                count: 1,
                                limit: None,
                                source_only: false,
                                reveal_to_controller: false,
                            }),
                            optional: false,
                        };
                        for _ in 0..total {
                            // Enqueue discard first so salvage resolves before discard.
                            self.enqueue_effect_spec(controller, source_id, discard_spec.clone());
                            self.enqueue_effect_spec(controller, source_id, salvage_spec.clone());
                        }
                    }
                }
            }
            EffectKind::BrainstormDrawChoice => {
                self.scratch.choice_options.clear();
                self.scratch.choice_options.push(ChoiceOptionRef {
                    card_id: 0,
                    instance_id: 0,
                    zone: ChoiceZone::DeckTop,
                    index: Some(0),
                    target_slot: None,
                });
                self.scratch.choice_options.push(ChoiceOptionRef {
                    card_id: 0,
                    instance_id: 0,
                    zone: ChoiceZone::Skip,
                    index: Some(1),
                    target_slot: None,
                });
                let options = std::mem::take(&mut self.scratch.choice_options);
                let _ = self.start_choice(
                    ChoiceReason::BrainstormDrawSelect,
                    controller,
                    options,
                    None,
                );
            }
            EffectKind::RandomDiscardFromHand { target, count } => {
                let target_player = match target {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                let p = target_player as usize;
                for _ in 0..*count {
                    let hand_len = self.state.players[p].hand.len();
                    if hand_len == 0 {
                        break;
                    }
                    let idx = self.state.rng.gen_range(hand_len);
                    if idx >= self.state.players[p].hand.len() {
                        break;
                    }
                    let card = self.state.players[p].hand.remove(idx);
                    let from_slot = if idx <= u8::MAX as usize {
                        Some(idx as u8)
                    } else {
                        None
                    };
                    self.move_card_between_zones(
                        target_player,
                        card,
                        Zone::Hand,
                        Zone::WaitingRoom,
                        from_slot,
                        None,
                    );
                }
            }
            EffectKind::RandomMill { target, count } => {
                let target_player = match target {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                for _ in 0..*count {
                    let Some(card) = self.draw_from_deck(target_player) else {
                        break;
                    };
                    self.move_card_between_zones(
                        target_player,
                        card,
                        Zone::Deck,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
            }
            EffectKind::RevealDeckTop { count, audience } => {
                let p = controller as usize;
                let deck_len = self.state.players[p].deck.len();
                let reveal_count = std::cmp::min(deck_len, *count as usize);
                for offset in 0..reveal_count {
                    let deck_idx = deck_len.saturating_sub(1 + offset);
                    let Some(card) = self.state.players[p].deck.get(deck_idx).copied() else {
                        continue;
                    };
                    self.reveal_card(controller, &card, RevealReason::AbilityEffect, *audience);
                }
            }
            EffectKind::RevealZoneTop {
                target,
                zone,
                count,
                audience,
            } => {
                let target_player = match target {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                match zone {
                    TargetZone::DeckTop => {
                        let p = target_player as usize;
                        let deck_len = self.state.players[p].deck.len();
                        let reveal_count = std::cmp::min(deck_len, *count as usize);
                        for offset in 0..reveal_count {
                            let deck_idx = deck_len.saturating_sub(1 + offset);
                            let Some(card) = self.state.players[p].deck.get(deck_idx).copied()
                            else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Hand => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].hand.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].hand.get(idx).copied() else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::WaitingRoom => {
                        let p = target_player as usize;
                        let reveal_count = std::cmp::min(
                            self.state.players[p].waiting_room.len(),
                            *count as usize,
                        );
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].waiting_room.get(idx).copied()
                            else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Clock => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].clock.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].clock.get(idx).copied() else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Level => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].level.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].level.get(idx).copied() else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Stock => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].stock.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].stock.get(idx).copied() else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Memory => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].memory.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].memory.get(idx).copied() else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Climax => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].climax.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].climax.get(idx).copied() else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Resolution => {
                        let p = target_player as usize;
                        let reveal_count =
                            std::cmp::min(self.state.players[p].resolution.len(), *count as usize);
                        for idx in 0..reveal_count {
                            let Some(card) = self.state.players[p].resolution.get(idx).copied()
                            else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                        }
                    }
                    TargetZone::Stage => {
                        let p = target_player as usize;
                        let reveal_count = if self.curriculum.reduced_stage_mode {
                            1
                        } else {
                            MAX_STAGE
                        };
                        let mut revealed = 0usize;
                        for slot in 0..reveal_count {
                            if revealed >= *count as usize {
                                break;
                            }
                            let Some(card) = self.state.players[p].stage[slot].card else {
                                continue;
                            };
                            self.reveal_card(
                                target_player,
                                &card,
                                RevealReason::AbilityEffect,
                                *audience,
                            );
                            revealed = revealed.saturating_add(1);
                        }
                    }
                }
            }
            EffectKind::RevealTopIfLevelAtLeastMoveThisToHand { min_level } => {
                if !self.reveal_top_deck_card_passes_level_gate(controller, *min_level) {
                    return;
                }
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let option = ChoiceOptionRef {
                    card_id: source_ref.card_id,
                    instance_id: source_ref.instance_id,
                    zone: ChoiceZone::Stage,
                    index: Some(source_ref.index as u16),
                    target_slot: None,
                };
                self.move_stage_to_hand(source_ref.player, option);
            }
            EffectKind::RevealTopIfLevelAtLeastRestThis { min_level } => {
                if !self.reveal_top_deck_card_passes_level_gate(controller, *min_level) {
                    return;
                }
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let slot = source_ref.index as usize;
                if slot >= self.state.players[p].stage.len() {
                    return;
                }
                let Some(card_inst) = self.state.players[p].stage[slot].card else {
                    return;
                };
                if card_inst.instance_id != source_ref.instance_id {
                    return;
                }
                self.state.players[p].stage[slot].status = StageStatus::Rest;
                self.mark_slot_power_dirty(source_ref.player, source_ref.index);
                self.mark_continuous_modifiers_dirty();
                self.touch_player_obs(source_ref.player);
            }
            EffectKind::RevealTopIfLevelAtLeastMoveTopToStock { min_level } => {
                if !self.reveal_top_deck_card_passes_level_gate(controller, *min_level) {
                    return;
                }
                let p = controller as usize;
                let Some(card) = self.state.players[p].deck.pop() else {
                    return;
                };
                self.move_card_between_zones(controller, card, Zone::Deck, Zone::Stock, None, None);
            }
            EffectKind::LookTopDeckReorder { count: _ } => {
                if payload.targets.is_empty() {
                    return;
                }
                let p = controller as usize;
                if self.state.players[p].deck.is_empty() {
                    return;
                }
                let mut reordered: Vec<CardInstance> = Vec::with_capacity(payload.targets.len());
                for instance_id in payload.targets.iter().map(|target| target.instance_id) {
                    if let Some(pos) = self.state.players[p]
                        .deck
                        .iter()
                        .position(|card| card.instance_id == instance_id)
                    {
                        reordered.push(self.state.players[p].deck.remove(pos));
                    }
                }
                if reordered.is_empty() {
                    return;
                }
                for card in reordered.iter().rev().copied() {
                    self.state.players[p].deck.push(card);
                }
                self.touch_player_obs(controller);
                self.mark_rule_actions_dirty();
            }
            EffectKind::LookTopCardTopOrWaitingRoom => {
                if let Some(target) = payload.targets.first().copied() {
                    if target.zone != TargetZone::DeckTop {
                        return;
                    }
                    let p = target.player as usize;
                    let offset = target.index as usize;
                    if offset >= self.state.players[p].deck.len() {
                        return;
                    }
                    let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                    if deck_idx >= self.state.players[p].deck.len() {
                        return;
                    }
                    if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                        return;
                    }
                    let card = self.state.players[p].deck.remove(deck_idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Deck,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
            }
            EffectKind::LookTopCardTopOrBottom => {
                if let Some(target) = payload.targets.first().copied() {
                    self.move_target_to_deck_bottom(target);
                }
            }
            EffectKind::SearchTopDeckToHandLevelAtLeastMillRest {
                look_count,
                choose_count,
                min_level,
            } => {
                let p = controller as usize;
                let reveal_count =
                    std::cmp::min(*look_count as usize, self.state.players[p].deck.len());
                let mut looked: Vec<CardInstance> = Vec::with_capacity(reveal_count);
                for _ in 0..reveal_count {
                    let Some(card) = self.draw_from_deck(controller) else {
                        break;
                    };
                    looked.push(card);
                }
                let mut moved_to_hand = 0u8;
                for card in looked {
                    let eligible = self
                        .db
                        .get(card.id)
                        .map(|static_card| {
                            let effective_level = if static_card.card_type == CardType::Climax {
                                0
                            } else {
                                static_card.level
                            };
                            effective_level >= *min_level
                        })
                        .unwrap_or(false);
                    if eligible && moved_to_hand < *choose_count {
                        self.reveal_card(
                            controller,
                            &card,
                            RevealReason::AbilityEffect,
                            RevealAudience::Public,
                        );
                        moved_to_hand = moved_to_hand.saturating_add(1);
                        self.move_card_between_zones(
                            controller,
                            card,
                            Zone::Deck,
                            Zone::Hand,
                            None,
                            None,
                        );
                    } else {
                        self.move_card_between_zones(
                            controller,
                            card,
                            Zone::Deck,
                            Zone::WaitingRoom,
                            None,
                            None,
                        );
                    }
                }
            }
            EffectKind::RevealTopAndSalvageByRevealedLevel {
                count,
                climax_level,
            } => {
                let p = controller as usize;
                let Some(top_card) = self.state.players[p].deck.last().copied() else {
                    return;
                };
                self.reveal_card(
                    controller,
                    &top_card,
                    RevealReason::AbilityEffect,
                    RevealAudience::Public,
                );
                let revealed_level = self
                    .db
                    .get(top_card.id)
                    .map(|static_card| {
                        if static_card.card_type == CardType::Climax {
                            *climax_level
                        } else {
                            static_card.level
                        }
                    })
                    .unwrap_or(*climax_level);
                let mut selected: Vec<usize> = Vec::new();
                for idx in (0..self.state.players[p].waiting_room.len()).rev() {
                    if selected.len() >= *count as usize {
                        break;
                    }
                    let Some(card_inst) = self.state.players[p].waiting_room.get(idx).copied()
                    else {
                        continue;
                    };
                    let Some(static_card) = self.db.get(card_inst.id) else {
                        continue;
                    };
                    if static_card.card_type != CardType::Character {
                        continue;
                    }
                    if static_card.level <= revealed_level {
                        selected.push(idx);
                    }
                }
                for idx in selected {
                    if idx >= self.state.players[p].waiting_room.len() {
                        continue;
                    }
                    let card = self.state.players[p].waiting_room.remove(idx);
                    self.move_card_between_zones(
                        controller,
                        card,
                        Zone::WaitingRoom,
                        Zone::Hand,
                        None,
                        None,
                    );
                }
            }
            EffectKind::Damage {
                amount,
                cancelable,
                damage_type: _,
            } => {
                let target_player = if let Some(target) = payload.targets.first() {
                    target.player
                } else if let Some(spec) = payload.spec.target.as_ref() {
                    match spec.side {
                        TargetSide::SelfSide => controller,
                        TargetSide::Opponent => 1 - controller,
                    }
                } else if payload.spec.id.source_kind == EffectSourceKind::System {
                    controller
                } else {
                    1 - controller
                };
                let (amount, target_player) =
                    self.apply_replacements_to_damage(controller, target_player, *amount);
                let refresh_penalty = payload.spec.id.source_kind == EffectSourceKind::System
                    && payload.spec.id.source_card == 0
                    && payload.spec.id.ability_index == 0
                    && payload.spec.id.effect_index == 0
                    && !*cancelable;
                if amount > 0 {
                    let _ = self.resolve_effect_damage(
                        controller,
                        target_player,
                        amount,
                        *cancelable,
                        refresh_penalty,
                        Some(source_id),
                        payload.source_ref,
                    );
                }
            }
            EffectKind::AddModifier {
                kind,
                magnitude,
                duration,
            } => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let s = target.index as usize;
                    if s >= self.state.players[p].stage.len() {
                        continue;
                    }
                    if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                        != Some(target.instance_id)
                    {
                        continue;
                    }
                    let _ = self.add_modifier(
                        source_id,
                        target.player,
                        target.index,
                        *kind,
                        *magnitude,
                        *duration,
                    );
                }
            }
            EffectKind::GrantAbilityDef { ability, duration } => {
                for target in &payload.targets {
                    self.add_granted_ability_to_target(controller, target, ability, *duration);
                }
            }
            EffectKind::AddPowerIfTargetLevelAtLeast {
                amount,
                min_level,
                duration,
            } => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let s = target.index as usize;
                    if s >= self.state.players[p].stage.len() {
                        continue;
                    }
                    if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                        != Some(target.instance_id)
                    {
                        continue;
                    }
                    let level = self.compute_slot_level(p, s);
                    if level < i32::from(*min_level) {
                        continue;
                    }
                    let _ = self.add_modifier(
                        source_id,
                        target.player,
                        target.index,
                        ModifierKind::Power,
                        *amount,
                        *duration,
                    );
                }
            }
            EffectKind::AddPowerByTargetLevel {
                multiplier,
                duration,
            } => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let s = target.index as usize;
                    if s >= self.state.players[p].stage.len() {
                        continue;
                    }
                    if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                        != Some(target.instance_id)
                    {
                        continue;
                    }
                    let level = self.compute_slot_level(p, s);
                    let magnitude = (*multiplier).saturating_mul(level);
                    if magnitude == 0 {
                        continue;
                    }
                    let _ = self.add_modifier(
                        source_id,
                        target.player,
                        target.index,
                        ModifierKind::Power,
                        magnitude,
                        *duration,
                    );
                }
            }
            EffectKind::AddPowerIfBattleOpponentLevelAtLeast {
                amount,
                min_level,
                duration,
            } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let Some(opponent_ref) = self.battle_opponent_target_from_source(&source_ref)
                else {
                    return;
                };
                let opponent_level = self
                    .compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
                if opponent_level < i32::from(*min_level) {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    source_ref.player,
                    source_ref.index,
                    ModifierKind::Power,
                    *amount,
                    *duration,
                );
            }
            EffectKind::AddSoulIfBattleOpponentLevelAtLeast {
                amount,
                min_level,
                duration,
            } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let Some(opponent_ref) = self.battle_opponent_target_from_source(&source_ref)
                else {
                    return;
                };
                let opponent_level = self
                    .compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
                if opponent_level < i32::from(*min_level) {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    source_ref.player,
                    source_ref.index,
                    ModifierKind::Soul,
                    *amount,
                    *duration,
                );
            }
            EffectKind::AddPowerIfBattleOpponentLevelExact {
                amount,
                level,
                duration,
            } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let Some(opponent_ref) = self.battle_opponent_target_from_source(&source_ref)
                else {
                    return;
                };
                let opponent_level = self
                    .compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
                if opponent_level != i32::from(*level) {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    source_ref.player,
                    source_ref.index,
                    ModifierKind::Power,
                    *amount,
                    *duration,
                );
            }
            EffectKind::AddPowerIfOtherAttackerMatches {
                amount,
                duration,
                attacker_card_ids,
            } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let Some(attack) = self.state.turn.attack.as_ref() else {
                    return;
                };
                let active_player = self.state.turn.active_player as usize;
                if source_ref.player as usize != active_player {
                    return;
                }
                let attacker_slot = attack.attacker_slot as usize;
                if attacker_slot >= self.state.players[active_player].stage.len() {
                    return;
                }
                if source_ref.index as usize == attacker_slot {
                    return;
                }
                let Some(attacker_card) =
                    self.state.players[active_player].stage[attacker_slot].card
                else {
                    return;
                };
                if !attacker_card_ids.is_empty() && !attacker_card_ids.contains(&attacker_card.id) {
                    return;
                }
                let source_slot = source_ref.index as usize;
                if source_slot >= self.state.players[active_player].stage.len() {
                    return;
                }
                if self.state.players[active_player].stage[source_slot]
                    .card
                    .map(|c| c.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    source_ref.player,
                    source_ref.index,
                    ModifierKind::Power,
                    *amount,
                    *duration,
                );
            }
            EffectKind::AddSoulIfMiddleCenter { amount } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let middle_slot = if self.curriculum.reduced_stage_mode {
                    0
                } else {
                    1
                };
                if s != middle_slot {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    source_ref.player,
                    source_ref.index,
                    ModifierKind::Soul,
                    *amount,
                    ModifierDuration::WhileOnStage,
                );
            }
            EffectKind::FacingOpponentAddSoul { amount } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let Some(opponent_ref) = self.battle_opponent_target_from_source(&source_ref)
                else {
                    return;
                };
                let p = opponent_ref.player as usize;
                let s = opponent_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(opponent_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    opponent_ref.player,
                    opponent_ref.index,
                    ModifierKind::Soul,
                    *amount,
                    ModifierDuration::WhileOnStage,
                );
            }
            EffectKind::FacingOpponentAddModifier {
                kind,
                magnitude,
                duration,
            } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let Some(opponent_ref) = self.battle_opponent_target_from_source(&source_ref)
                else {
                    return;
                };
                let p = opponent_ref.player as usize;
                let s = opponent_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(opponent_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    opponent_ref.player,
                    opponent_ref.index,
                    *kind,
                    *magnitude,
                    *duration,
                );
            }
            EffectKind::SelfAddModifierIfFacingOpponent {
                kind,
                magnitude,
                duration,
                max_level,
                max_cost,
                level_gt_source_level,
            } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                let Some(opponent_ref) = self.battle_opponent_target_from_source(&source_ref)
                else {
                    return;
                };
                let Some(opponent_card) = self.db.get(opponent_ref.card_id) else {
                    return;
                };
                let opponent_level = self
                    .compute_slot_level(opponent_ref.player as usize, opponent_ref.index as usize);
                if let Some(level_cap) = max_level {
                    if opponent_level > i32::from(*level_cap) {
                        return;
                    }
                }
                if let Some(cost_cap) = max_cost {
                    if opponent_card.cost > *cost_cap {
                        return;
                    }
                }
                if *level_gt_source_level {
                    let source_level = self
                        .compute_slot_level(source_ref.player as usize, source_ref.index as usize);
                    if opponent_level <= source_level {
                        return;
                    }
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let _ = self.add_modifier(
                    source_id,
                    source_ref.player,
                    source_ref.index,
                    *kind,
                    *magnitude,
                    *duration,
                );
            }
            EffectKind::ConditionalAddModifier {
                kind,
                magnitude,
                duration,
                turn,
                zone_count,
                require_source_marker,
                per_source_marker,
                per_zone_count,
                exclude_source,
            } => {
                if !self.conditional_modifier_context_satisfied(
                    controller,
                    payload.source_ref.as_ref(),
                    *turn,
                    zone_count.as_ref(),
                    *require_source_marker,
                ) {
                    return;
                }
                let mut resolved_magnitude = *magnitude;
                if *per_source_marker {
                    let marker_count = self.source_marker_count(payload.source_ref.as_ref()) as i32;
                    resolved_magnitude = resolved_magnitude.saturating_mul(marker_count);
                }
                if *per_zone_count {
                    let Some(zone_condition) = zone_count.as_ref() else {
                        return;
                    };
                    let count = self.zone_count_value(controller, zone_condition) as i32;
                    resolved_magnitude = resolved_magnitude.saturating_mul(count);
                }
                if resolved_magnitude == 0 {
                    return;
                }
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let s = target.index as usize;
                    if s >= self.state.players[p].stage.len() {
                        continue;
                    }
                    if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                        != Some(target.instance_id)
                    {
                        continue;
                    }
                    if *exclude_source
                        && payload
                            .source_ref
                            .as_ref()
                            .map(|source| {
                                source.player == target.player
                                    && source.zone == target.zone
                                    && source.index == target.index
                                    && source.instance_id == target.instance_id
                            })
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    let _ = self.add_modifier(
                        source_id,
                        target.player,
                        target.index,
                        *kind,
                        resolved_magnitude,
                        *duration,
                    );
                }
            }
            EffectKind::MoveToHand => {
                let mut waiting_room_targets: Vec<TargetRef> = Vec::new();
                let mut deck_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => {
                            let option = ChoiceOptionRef {
                                card_id: target.card_id,
                                instance_id: target.instance_id,
                                zone: ChoiceZone::Stage,
                                index: Some(target.index as u16),
                                target_slot: None,
                            };
                            self.move_stage_to_hand(target.player, option);
                        }
                        TargetZone::WaitingRoom => {
                            waiting_room_targets.push(*target);
                        }
                        TargetZone::DeckTop => {
                            deck_targets.push(*target);
                        }
                        _ => {}
                    }
                }
                waiting_room_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in waiting_room_targets {
                    let option = ChoiceOptionRef {
                        card_id: target.card_id,
                        instance_id: target.instance_id,
                        zone: ChoiceZone::WaitingRoom,
                        index: Some(target.index as u16),
                        target_slot: None,
                    };
                    self.move_waiting_room_to_hand(target.player, option);
                }
                deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in deck_targets {
                    let p = target.player as usize;
                    let offset = target.index as usize;
                    if offset >= self.state.players[p].deck.len() {
                        continue;
                    }
                    let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                    if deck_idx >= self.state.players[p].deck.len() {
                        continue;
                    }
                    if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].deck.remove(deck_idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Deck,
                        Zone::Hand,
                        None,
                        None,
                    );
                }
            }
            EffectKind::MoveToWaitingRoom => {
                let mut stage_targets: Vec<TargetRef> = Vec::new();
                let mut hand_targets: Vec<TargetRef> = Vec::new();
                let mut deck_targets: Vec<TargetRef> = Vec::new();
                let mut clock_targets: Vec<TargetRef> = Vec::new();
                let mut level_targets: Vec<TargetRef> = Vec::new();
                let mut stock_targets: Vec<TargetRef> = Vec::new();
                let mut memory_targets: Vec<TargetRef> = Vec::new();
                let mut climax_targets: Vec<TargetRef> = Vec::new();
                let mut resolution_targets: Vec<TargetRef> = Vec::new();
                let mut waiting_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => stage_targets.push(*target),
                        TargetZone::Hand => hand_targets.push(*target),
                        TargetZone::DeckTop => deck_targets.push(*target),
                        TargetZone::Clock => clock_targets.push(*target),
                        TargetZone::Level => level_targets.push(*target),
                        TargetZone::Stock => stock_targets.push(*target),
                        TargetZone::Memory => memory_targets.push(*target),
                        TargetZone::Climax => climax_targets.push(*target),
                        TargetZone::Resolution => resolution_targets.push(*target),
                        TargetZone::WaitingRoom => waiting_targets.push(*target),
                    }
                }
                for target in stage_targets {
                    let p = target.player as usize;
                    let slot = target.index as usize;
                    if slot >= self.state.players[p].stage.len() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[p].stage[slot].card else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.remove_modifiers_for_slot(target.player, target.index);
                    self.drain_stage_markers_to_waiting_room(target.player, target.index);
                    self.state.players[p].stage[slot] = StageSlot::empty();
                    self.mark_slot_power_dirty(target.player, target.index);
                    self.move_card_between_zones(
                        target.player,
                        card_inst,
                        Zone::Stage,
                        Zone::WaitingRoom,
                        Some(target.index),
                        None,
                    );
                }
                hand_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in hand_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].hand.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].hand.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].hand.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Hand,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                clock_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in clock_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].clock.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].clock.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].clock.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Clock,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                    self.check_level_up(target.player);
                }
                level_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in level_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].level.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].level.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].level.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Level,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                stock_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in stock_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].stock.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].stock.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].stock.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Stock,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                memory_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in memory_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].memory.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].memory.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].memory.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Memory,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                climax_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in climax_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].climax.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].climax.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].climax.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Climax,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                resolution_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in resolution_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].resolution.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].resolution.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].resolution.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Resolution,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                waiting_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in waiting_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].waiting_room.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].waiting_room.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].waiting_room.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::WaitingRoom,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in deck_targets {
                    let p = target.player as usize;
                    let offset = target.index as usize;
                    if offset >= self.state.players[p].deck.len() {
                        continue;
                    }
                    let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                    if deck_idx >= self.state.players[p].deck.len() {
                        continue;
                    }
                    if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].deck.remove(deck_idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Deck,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
            }
            EffectKind::MoveToStock => {
                let mut stage_targets: Vec<TargetRef> = Vec::new();
                let mut hand_targets: Vec<TargetRef> = Vec::new();
                let mut deck_targets: Vec<TargetRef> = Vec::new();
                let mut clock_targets: Vec<TargetRef> = Vec::new();
                let mut level_targets: Vec<TargetRef> = Vec::new();
                let mut waiting_targets: Vec<TargetRef> = Vec::new();
                let mut memory_targets: Vec<TargetRef> = Vec::new();
                let mut climax_targets: Vec<TargetRef> = Vec::new();
                let mut resolution_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => stage_targets.push(*target),
                        TargetZone::Hand => hand_targets.push(*target),
                        TargetZone::DeckTop => deck_targets.push(*target),
                        TargetZone::Clock => clock_targets.push(*target),
                        TargetZone::Level => level_targets.push(*target),
                        TargetZone::WaitingRoom => waiting_targets.push(*target),
                        TargetZone::Memory => memory_targets.push(*target),
                        TargetZone::Climax => climax_targets.push(*target),
                        TargetZone::Resolution => resolution_targets.push(*target),
                        TargetZone::Stock => {}
                    }
                }
                for target in stage_targets {
                    let p = target.player as usize;
                    let slot = target.index as usize;
                    if slot >= self.state.players[p].stage.len() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[p].stage[slot].card else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.remove_modifiers_for_slot(target.player, target.index);
                    self.drain_stage_markers_to_waiting_room(target.player, target.index);
                    self.state.players[p].stage[slot] = StageSlot::empty();
                    self.mark_slot_power_dirty(target.player, target.index);
                    self.move_card_between_zones(
                        target.player,
                        card_inst,
                        Zone::Stage,
                        Zone::Stock,
                        Some(target.index),
                        None,
                    );
                }
                hand_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in hand_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].hand.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].hand.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].hand.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Hand,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
                clock_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in clock_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].clock.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].clock.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].clock.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Clock,
                        Zone::Stock,
                        None,
                        None,
                    );
                    self.check_level_up(target.player);
                }
                level_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in level_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].level.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].level.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].level.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Level,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
                waiting_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in waiting_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].waiting_room.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].waiting_room.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].waiting_room.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::WaitingRoom,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
                memory_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in memory_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].memory.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].memory.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].memory.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Memory,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
                climax_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in climax_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].climax.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].climax.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].climax.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Climax,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
                resolution_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in resolution_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].resolution.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].resolution.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].resolution.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Resolution,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
                deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in deck_targets {
                    let p = target.player as usize;
                    let offset = target.index as usize;
                    if offset >= self.state.players[p].deck.len() {
                        continue;
                    }
                    let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                    if deck_idx >= self.state.players[p].deck.len() {
                        continue;
                    }
                    if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].deck.remove(deck_idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Deck,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
            }
            EffectKind::MoveToClock => {
                let mut stage_targets: Vec<TargetRef> = Vec::new();
                let mut hand_targets: Vec<TargetRef> = Vec::new();
                let mut deck_targets: Vec<TargetRef> = Vec::new();
                let mut waiting_targets: Vec<TargetRef> = Vec::new();
                let mut resolution_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => stage_targets.push(*target),
                        TargetZone::Hand => hand_targets.push(*target),
                        TargetZone::DeckTop => deck_targets.push(*target),
                        TargetZone::WaitingRoom => waiting_targets.push(*target),
                        TargetZone::Resolution => resolution_targets.push(*target),
                        TargetZone::Clock => {}
                        TargetZone::Level => {}
                        TargetZone::Stock => {}
                        TargetZone::Memory => {}
                        TargetZone::Climax => {}
                    }
                }
                for target in stage_targets {
                    let p = target.player as usize;
                    let slot = target.index as usize;
                    if slot >= self.state.players[p].stage.len() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[p].stage[slot].card else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.remove_modifiers_for_slot(target.player, target.index);
                    self.drain_stage_markers_to_waiting_room(target.player, target.index);
                    self.state.players[p].stage[slot] = StageSlot::empty();
                    self.mark_slot_power_dirty(target.player, target.index);
                    self.move_card_between_zones(
                        target.player,
                        card_inst,
                        Zone::Stage,
                        Zone::Clock,
                        Some(target.index),
                        None,
                    );
                    self.check_level_up(target.player);
                }
                hand_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in hand_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].hand.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].hand.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].hand.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Hand,
                        Zone::Clock,
                        None,
                        None,
                    );
                    self.check_level_up(target.player);
                }
                waiting_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in waiting_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].waiting_room.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].waiting_room.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].waiting_room.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::WaitingRoom,
                        Zone::Clock,
                        None,
                        None,
                    );
                    self.check_level_up(target.player);
                }
                resolution_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in resolution_targets {
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].resolution.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].resolution.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].resolution.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Resolution,
                        Zone::Clock,
                        None,
                        None,
                    );
                    self.check_level_up(target.player);
                }
                deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in deck_targets {
                    let p = target.player as usize;
                    let offset = target.index as usize;
                    if offset >= self.state.players[p].deck.len() {
                        continue;
                    }
                    let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                    if deck_idx >= self.state.players[p].deck.len() {
                        continue;
                    }
                    if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].deck.remove(deck_idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Deck,
                        Zone::Clock,
                        None,
                        None,
                    );
                    self.check_level_up(target.player);
                }
            }
            EffectKind::MoveToMemory => {
                let mut stage_targets: Vec<TargetRef> = Vec::new();
                let mut deck_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => stage_targets.push(*target),
                        TargetZone::DeckTop => deck_targets.push(*target),
                        _ => {}
                    }
                }
                for target in stage_targets {
                    let _ = self.move_stage_target_to_memory(target);
                }
                deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in deck_targets {
                    let p = target.player as usize;
                    let offset = target.index as usize;
                    if offset >= self.state.players[p].deck.len() {
                        continue;
                    }
                    let deck_idx = self.state.players[p].deck.len().saturating_sub(1 + offset);
                    if deck_idx >= self.state.players[p].deck.len() {
                        continue;
                    }
                    if self.state.players[p].deck[deck_idx].instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].deck.remove(deck_idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Deck,
                        Zone::Memory,
                        None,
                        None,
                    );
                }
            }
            EffectKind::MoveToDeckBottom => {
                let mut stage_targets: Vec<TargetRef> = Vec::new();
                let mut deck_targets: Vec<TargetRef> = Vec::new();
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => stage_targets.push(*target),
                        TargetZone::DeckTop => deck_targets.push(*target),
                        _ => {}
                    }
                }
                for target in stage_targets {
                    self.move_target_to_deck_bottom(target);
                }
                deck_targets.sort_by_key(|t| std::cmp::Reverse(t.index));
                for target in deck_targets {
                    self.move_target_to_deck_bottom(target);
                }
            }
            EffectKind::MoveWaitingRoomCardToSourceSlot => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                let mut waiting_targets: Vec<TargetRef> = payload
                    .targets
                    .iter()
                    .copied()
                    .filter(|target| {
                        target.zone == TargetZone::WaitingRoom && target.player == source_ref.player
                    })
                    .collect();
                waiting_targets.sort_by_key(|target| std::cmp::Reverse(target.index));
                let Some(target) = waiting_targets.into_iter().next() else {
                    return;
                };
                let tp = target.player as usize;
                let idx = target.index as usize;
                if idx >= self.state.players[tp].waiting_room.len() {
                    return;
                }
                let Some(card_inst) = self.state.players[tp].waiting_room.get(idx).copied() else {
                    return;
                };
                if card_inst.instance_id != target.instance_id {
                    return;
                }
                let card = self.state.players[tp].waiting_room.remove(idx);
                self.place_card_on_stage(
                    target.player,
                    card,
                    source_ref.index,
                    StageStatus::Stand,
                    Zone::WaitingRoom,
                    Some(target.index),
                );
            }
            EffectKind::RecycleWaitingRoomToDeckShuffle => {
                let p = controller as usize;
                while let Some(card) = self.state.players[p].waiting_room.pop() {
                    self.move_card_between_zones(
                        controller,
                        card,
                        Zone::WaitingRoom,
                        Zone::Deck,
                        None,
                        None,
                    );
                }
                self.shuffle_deck(controller);
            }
            EffectKind::ResetStockFromDeckTop { target } => {
                let target_player = match target {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                let p = target_player as usize;
                let stock_count = self.state.players[p].stock.len();
                while let Some(card) = self.state.players[p].stock.pop() {
                    self.move_card_between_zones(
                        target_player,
                        card,
                        Zone::Stock,
                        Zone::WaitingRoom,
                        None,
                        None,
                    );
                }
                for _ in 0..stock_count {
                    if let Some(card) = self.draw_from_deck(target_player) {
                        self.move_card_between_zones(
                            target_player,
                            card,
                            Zone::Deck,
                            Zone::Stock,
                            None,
                            None,
                        );
                    }
                }
            }
            EffectKind::MoveToMarker => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let source_player = source_ref.player as usize;
                let source_slot = source_ref.index as usize;
                if source_slot >= self.state.players[source_player].stage.len() {
                    return;
                }
                if self.state.players[source_player].stage[source_slot]
                    .card
                    .map(|card| card.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let mut waiting_targets: Vec<TargetRef> = payload
                    .targets
                    .iter()
                    .copied()
                    .filter(|target| target.zone == TargetZone::WaitingRoom)
                    .collect();
                waiting_targets.sort_by_key(|target| std::cmp::Reverse(target.index));
                for target in waiting_targets {
                    let _ = self.move_waiting_room_to_marker(
                        target.player,
                        target.index,
                        target.instance_id,
                        source_ref.player,
                        source_ref.index,
                    );
                }
            }
            EffectKind::MoveTopDeckToMarker => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s]
                    .card
                    .map(|card| card.instance_id)
                    != Some(source_ref.instance_id)
                {
                    return;
                }
                let Some(card) = self.draw_from_deck(source_ref.player) else {
                    return;
                };
                self.state.players[p].stage[s].markers.push(card);
                self.touch_player_obs(source_ref.player);
                self.mark_continuous_modifiers_dirty();
                self.mark_slot_power_dirty(source_ref.player, source_ref.index);
            }
            EffectKind::Heal => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Clock {
                        continue;
                    }
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].clock.len() {
                        continue;
                    }
                    let Some(card) = self.state.players[p].clock.get(idx).copied() else {
                        continue;
                    };
                    if card.instance_id != target.instance_id {
                        continue;
                    }
                    let card = self.state.players[p].clock.remove(idx);
                    self.move_card_between_zones(
                        target.player,
                        card,
                        Zone::Clock,
                        Zone::WaitingRoom,
                        Some(target.index),
                        None,
                    );
                }
            }
            EffectKind::HealIfSourcePlayedFromHandThisTurn => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let s = source_ref.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                let slot_state = &self.state.players[p].stage[s];
                if slot_state.card.map(|card| card.instance_id) != Some(source_ref.instance_id) {
                    return;
                }
                if !slot_state.played_from_hand_this_turn {
                    return;
                }
                let clock_len = self.state.players[p].clock.len();
                if clock_len == 0 {
                    return;
                }
                let idx = clock_len - 1;
                let Ok(idx_u8) = u8::try_from(idx) else {
                    return;
                };
                let card = self.state.players[p].clock.remove(idx);
                self.move_card_between_zones(
                    source_ref.player,
                    card,
                    Zone::Clock,
                    Zone::WaitingRoom,
                    Some(idx_u8),
                    None,
                );
            }
            EffectKind::MoveTriggerCardToHand => {
                let _ = self.move_trigger_card_from_stock_to_hand(controller, source_id);
            }
            EffectKind::MoveTriggerCardToStock => {
                let p = controller as usize;
                let Some(instance_id) = self
                    .state
                    .turn
                    .attack
                    .as_ref()
                    .and_then(|ctx| ctx.trigger_instance_id)
                else {
                    return;
                };
                if let Some(pos) = self.state.players[p]
                    .resolution
                    .iter()
                    .position(|card| card.instance_id == instance_id)
                {
                    let card = self.state.players[p].resolution.remove(pos);
                    self.move_card_between_zones(
                        controller,
                        card,
                        Zone::Resolution,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
            }
            EffectKind::MillTop { target, count } => {
                let target_player = match target {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                for _ in 0..*count {
                    if let Some(card) = self.draw_from_deck(target_player) {
                        self.move_card_between_zones(
                            target_player,
                            card,
                            Zone::Deck,
                            Zone::WaitingRoom,
                            None,
                            None,
                        );
                    }
                }
            }
            EffectKind::MoveStageSlot { slot } => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let idx = target.index as usize;
                    if idx >= self.state.players[p].stage.len() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[p].stage[idx].card else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.swap_stage_slots(target.player, target.index, *slot);
                }
            }
            EffectKind::MoveThisToOpenCenter { require_facing } => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let source_slot = source_ref.index as usize;
                if source_slot >= self.state.players[p].stage.len() {
                    return;
                }
                let Some(card_inst) = self.state.players[p].stage[source_slot].card else {
                    return;
                };
                if card_inst.instance_id != source_ref.instance_id {
                    return;
                }
                let center_slots: &[u8] = if self.curriculum.reduced_stage_mode {
                    &[0]
                } else {
                    &[0, 1, 2]
                };
                let dest = center_slots.iter().copied().find(|&slot| {
                    let idx = slot as usize;
                    if idx >= self.state.players[p].stage.len() {
                        return false;
                    }
                    if self.state.players[p].stage[idx].card.is_some() {
                        return false;
                    }
                    if !*require_facing {
                        return true;
                    }
                    let opp = 1 - p;
                    idx < self.state.players[opp].stage.len()
                        && self.state.players[opp].stage[idx].card.is_some()
                });
                if let Some(dest_slot) = dest {
                    self.swap_stage_slots(source_ref.player, source_ref.index, dest_slot);
                }
            }
            EffectKind::MoveThisToOpenBack => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let source_slot = source_ref.index as usize;
                if source_slot >= self.state.players[p].stage.len() {
                    return;
                }
                let Some(card_inst) = self.state.players[p].stage[source_slot].card else {
                    return;
                };
                if card_inst.instance_id != source_ref.instance_id {
                    return;
                }
                let back_slots: &[u8] = if self.curriculum.reduced_stage_mode {
                    &[]
                } else {
                    &[3, 4]
                };
                let dest = back_slots.iter().copied().find(|&slot| {
                    let idx = slot as usize;
                    idx < self.state.players[p].stage.len()
                        && self.state.players[p].stage[idx].card.is_none()
                });
                if let Some(dest_slot) = dest {
                    self.swap_stage_slots(source_ref.player, source_ref.index, dest_slot);
                }
            }
            EffectKind::SwapStageSlots => {
                let mut stage_targets: Vec<TargetRef> = payload
                    .targets
                    .iter()
                    .copied()
                    .filter(|t| t.zone == TargetZone::Stage)
                    .collect();
                if stage_targets.len() < 2 {
                    return;
                }
                stage_targets.sort_by_key(|t| (t.player, t.index, t.instance_id));
                let first = stage_targets[0];
                let second = stage_targets[1];
                if first.player != second.player {
                    return;
                }
                let p = first.player as usize;
                let f_idx = first.index as usize;
                let s_idx = second.index as usize;
                if f_idx >= self.state.players[p].stage.len()
                    || s_idx >= self.state.players[p].stage.len()
                {
                    return;
                }
                let Some(f_card) = self.state.players[p].stage[f_idx].card else {
                    return;
                };
                let Some(s_card) = self.state.players[p].stage[s_idx].card else {
                    return;
                };
                if f_card.instance_id != first.instance_id
                    || s_card.instance_id != second.instance_id
                {
                    return;
                }
                self.swap_stage_slots(first.player, first.index, second.index);
            }
            EffectKind::ChangeController { new_controller } => {
                let to_player = match new_controller {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let from_player = target.player;
                    if from_player == to_player {
                        continue;
                    }
                    let from_slot = target.index as usize;
                    let to_slot = target.index as usize;
                    if from_slot >= self.state.players[from_player as usize].stage.len()
                        || to_slot >= self.state.players[to_player as usize].stage.len()
                    {
                        continue;
                    }
                    if self.state.players[to_player as usize].stage[to_slot]
                        .card
                        .is_some()
                    {
                        continue;
                    }
                    let Some(card_inst) =
                        self.state.players[from_player as usize].stage[from_slot].card
                    else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.remove_modifiers_for_slot(from_player, target.index);
                    let mut moved_slot = std::mem::replace(
                        &mut self.state.players[from_player as usize].stage[from_slot],
                        StageSlot::empty(),
                    );
                    let Some(mut moved_card) = moved_slot.card.take() else {
                        continue;
                    };
                    moved_card.controller = to_player;
                    moved_slot.card = Some(moved_card);
                    self.state.players[to_player as usize].stage[to_slot] = moved_slot;
                    self.mark_slot_power_dirty(from_player, target.index);
                    self.mark_slot_power_dirty(to_player, target.index);
                    self.mark_rule_actions_dirty();
                    self.mark_continuous_modifiers_dirty();
                    self.log_event(Event::ControlChanged {
                        card: moved_card.id,
                        owner: moved_card.owner,
                        from_controller: from_player,
                        to_controller: to_player,
                        from_slot: target.index,
                        to_slot: target.index,
                    });
                }
            }
            EffectKind::Standby { target_slot } => {
                let Some(target) = payload.targets.first() else {
                    return;
                };
                if target.zone != TargetZone::WaitingRoom {
                    return;
                }
                let option = ChoiceOptionRef {
                    card_id: target.card_id,
                    instance_id: target.instance_id,
                    zone: ChoiceZone::WaitingRoom,
                    index: Some(target.index as u16),
                    target_slot: Some(*target_slot),
                };
                self.move_waiting_room_to_stage_standby(controller, option);
            }
            EffectKind::TreasureStock { take_stock } => {
                if *take_stock {
                    if let Some(card) = self.draw_from_deck(controller) {
                        self.move_card_between_zones(
                            controller,
                            card,
                            Zone::Deck,
                            Zone::Stock,
                            None,
                            None,
                        );
                    }
                }
            }
            EffectKind::ModifyPendingAttackDamage { delta } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.damage = ctx.damage.saturating_add(*delta);
                }
            }
            EffectKind::EnableShotDamage { amount } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.pending_shot_damage = ctx.pending_shot_damage.saturating_add(*amount);
                }
            }
            EffectKind::SetTriggerCheckCount { count } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    let desired = (*count).max(1);
                    let floor = ctx.trigger_checks_resolved.max(1);
                    ctx.trigger_checks_total = ctx.trigger_checks_total.max(desired).max(floor);
                }
            }
            EffectKind::RestThisIfNoOtherRestCenter => {
                let Some(source_ref) = payload.source_ref else {
                    return;
                };
                if source_ref.zone != TargetZone::Stage {
                    return;
                }
                let p = source_ref.player as usize;
                let source_slot = source_ref.index as usize;
                if source_slot >= self.state.players[p].stage.len() {
                    return;
                }
                let Some(card_inst) = self.state.players[p].stage[source_slot].card else {
                    return;
                };
                if card_inst.instance_id != source_ref.instance_id {
                    return;
                }
                let center_slots: &[u8] = if self.curriculum.reduced_stage_mode {
                    &[0]
                } else {
                    &[0, 1, 2]
                };
                let has_other_rest = center_slots.iter().copied().any(|slot| {
                    let idx = slot as usize;
                    if idx == source_slot || idx >= self.state.players[p].stage.len() {
                        return false;
                    }
                    let slot_state = &self.state.players[p].stage[idx];
                    slot_state.card.is_some() && slot_state.status == StageStatus::Rest
                });
                if has_other_rest {
                    return;
                }
                let slot_state = &mut self.state.players[p].stage[source_slot];
                if slot_state.card.is_none() {
                    return;
                }
                slot_state.status = StageStatus::Rest;
                self.mark_slot_power_dirty(source_ref.player, source_ref.index);
                self.mark_continuous_modifiers_dirty();
                self.touch_player_obs(source_ref.player);
            }
            EffectKind::RestTarget => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let slot = target.index as usize;
                    if slot >= self.state.players[p].stage.len() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[p].stage[slot].card else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.state.players[p].stage[slot].status = StageStatus::Rest;
                    self.mark_slot_power_dirty(target.player, target.index);
                    self.mark_continuous_modifiers_dirty();
                }
            }
            EffectKind::StandTarget => {
                for target in &payload.targets {
                    if target.zone != TargetZone::Stage {
                        continue;
                    }
                    let p = target.player as usize;
                    let slot = target.index as usize;
                    if slot >= self.state.players[p].stage.len() {
                        continue;
                    }
                    let Some(card_inst) = self.state.players[p].stage[slot].card else {
                        continue;
                    };
                    if card_inst.instance_id != target.instance_id {
                        continue;
                    }
                    self.state.players[p].stage[slot].status = StageStatus::Stand;
                    self.mark_slot_power_dirty(target.player, target.index);
                    self.mark_continuous_modifiers_dirty();
                }
            }
            EffectKind::StockCharge { count } => {
                for _ in 0..*count {
                    if let Some(card) = self.draw_from_deck(controller) {
                        self.move_card_between_zones(
                            controller,
                            card,
                            Zone::Deck,
                            Zone::Stock,
                            None,
                            None,
                        );
                    }
                }
            }
            EffectKind::CannotUseAutoEncoreForPlayer { target } => {
                let target_player = match target {
                    TargetSide::SelfSide => controller,
                    TargetSide::Opponent => 1 - controller,
                };
                self.state.turn.cannot_use_auto_encore[target_player as usize] = true;
            }
            EffectKind::TriggerIcon { icon } => {
                let trigger_id = self.state.turn.next_trigger_id;
                self.state.turn.next_trigger_id = self.state.turn.next_trigger_id.wrapping_add(1);
                let trigger = PendingTrigger {
                    id: trigger_id,
                    group_id: 0,
                    player: controller,
                    source_card: source_id,
                    effect: match icon {
                        TriggerIcon::Soul => TriggerEffect::Soul,
                        TriggerIcon::Draw => TriggerEffect::Draw,
                        TriggerIcon::Shot => TriggerEffect::Shot,
                        TriggerIcon::Bounce => TriggerEffect::Bounce,
                        TriggerIcon::Choice => TriggerEffect::Choice,
                        TriggerIcon::Pool => TriggerEffect::Pool,
                        TriggerIcon::Treasure => TriggerEffect::Treasure,
                        TriggerIcon::Gate => TriggerEffect::Gate,
                        TriggerIcon::Standby => TriggerEffect::Standby,
                    },
                    effect_id: Some(payload.spec.id),
                };
                let trigger_effect = trigger.effect;
                let resolved = match icon {
                    TriggerIcon::Draw => self.resolve_trigger_draw(trigger),
                    TriggerIcon::Choice => self.resolve_trigger_choice(trigger),
                    TriggerIcon::Pool
                    | TriggerIcon::Soul
                    | TriggerIcon::Shot
                    | TriggerIcon::Gate
                    | TriggerIcon::Bounce => {
                        let _ = self.resolve_trigger(trigger);
                        false
                    }
                    TriggerIcon::Treasure => self.resolve_trigger_treasure(trigger),
                    TriggerIcon::Standby => self.resolve_trigger_standby(trigger),
                };
                if !resolved && self.state.turn.choice.is_none() {
                    self.log_event(Event::TriggerResolved {
                        trigger_id,
                        player: controller,
                        effect: trigger_effect,
                    });
                }
            }
            EffectKind::BattleOpponentReverseIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                let p = target.player as usize;
                let s = target.index as usize;
                if s >= self.state.players[p].stage.len() {
                    return;
                }
                if self.state.players[p].stage[s].status == StageStatus::Reverse {
                    return;
                }
                if self.state.players[p].stage[s].card.map(|c| c.instance_id)
                    != Some(target.instance_id)
                {
                    return;
                }
                self.state.players[p].stage[s].status = StageStatus::Reverse;
                self.mark_slot_power_dirty(target.player, target.index);
                let cause_damage_event = self
                    .state
                    .turn
                    .attack
                    .as_ref()
                    .and_then(|ctx| ctx.last_damage_event_id);
                self.log_event(Event::ReversalCommitted {
                    player: target.player,
                    slot: target.index,
                    cause_damage_event,
                });
                self.queue_on_reverse_triggers(&[(target.player, target.card_id)]);
                self.queue_battle_opponent_reverse_triggers(&[(
                    source_ref.player,
                    source_ref.card_id,
                )]);
            }
            EffectKind::BattleOpponentMoveToDeckBottomIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                self.move_target_to_deck_bottom(target);
            }
            EffectKind::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                if !self.move_stage_target_to_stock(target) {
                    return;
                }
                let _ = self.move_bottom_stock_to_waiting_room(target.player);
            }
            EffectKind::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                if !self.move_top_clock_to_waiting_room(target.player) {
                    return;
                }
                let _ = self.move_stage_target_to_clock(target);
            }
            EffectKind::BattleOpponentMoveToMemoryIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                let _ = self.move_stage_target_to_memory(target);
            }
            EffectKind::BattleOpponentMoveToClockIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                let _ = self.move_stage_target_to_clock(target);
            }
            EffectKind::BattleOpponentMove {
                destination,
                prelude,
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if !self.battle_opponent_condition_met(
                    &target,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                ) {
                    return;
                }
                if matches!(
                    prelude,
                    Some(crate::db::BattleOpponentMovePreludeAction::OpponentClockTopToWaitingRoom)
                ) && !self.move_top_clock_to_waiting_room(target.player)
                {
                    return;
                }
                match destination {
                    crate::db::BattleOpponentMoveDestination::DeckBottom => {
                        self.move_target_to_deck_bottom(target);
                    }
                    crate::db::BattleOpponentMoveDestination::StockThenBottomStockToWaitingRoom => {
                        if self.move_stage_target_to_stock(target) {
                            let _ = self.move_bottom_stock_to_waiting_room(target.player);
                        }
                    }
                    crate::db::BattleOpponentMoveDestination::Clock => {
                        let _ = self.move_stage_target_to_clock(target);
                    }
                    crate::db::BattleOpponentMoveDestination::Memory => {
                        let _ = self.move_stage_target_to_memory(target);
                    }
                }
            }
            EffectKind::BattleOpponentTopDeckToStockIf { min_level } => {
                let Some(source_ref) = payload.source_ref.as_ref() else {
                    return;
                };
                let Some(target) = self.battle_opponent_target_from_source(source_ref) else {
                    return;
                };
                if target.zone != TargetZone::Stage {
                    return;
                }
                let target_level =
                    self.compute_slot_level(target.player as usize, target.index as usize);
                if target_level < i32::from(*min_level) {
                    return;
                }
                if let Some(card) = self.draw_from_deck(controller) {
                    self.move_card_between_zones(
                        controller,
                        card,
                        Zone::Deck,
                        Zone::Stock,
                        None,
                        None,
                    );
                }
            }
            EffectKind::CounterBackup { power } => {
                let mut dirty_slot = None;
                if let Some(ctx) = &mut self.state.turn.attack {
                    if let Some(def_slot) = ctx.defender_slot {
                        let slot_state =
                            &mut self.state.players[controller as usize].stage[def_slot as usize];
                        slot_state.power_mod_battle += *power;
                        ctx.counter_power += *power;
                        dirty_slot = Some(def_slot);
                    }
                }
                if let Some(def_slot) = dirty_slot {
                    self.mark_slot_power_dirty(controller, def_slot);
                }
                self.log_event(Event::Counter {
                    player: controller,
                    card: source_id,
                    power: *power,
                });
            }
            EffectKind::CounterDamageReduce { amount } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    if *amount > 0 {
                        Self::push_attack_damage_modifier(
                            ctx,
                            DamageModifierKind::AddAmount {
                                delta: -(*amount as i32),
                            },
                            source_id,
                        );
                    }
                }
            }
            EffectKind::CounterDamageCancel => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    Self::push_attack_damage_modifier(
                        ctx,
                        DamageModifierKind::CancelNext,
                        source_id,
                    );
                }
            }
            EffectKind::SetTerminalOutcome { outcome } => {
                self.state.terminal = Some(match outcome {
                    crate::effects::TerminalOutcomeSpec::WinSelf => {
                        TerminalResult::Win { winner: controller }
                    }
                    crate::effects::TerminalOutcomeSpec::WinOpponent => TerminalResult::Win {
                        winner: 1 - controller,
                    },
                    crate::effects::TerminalOutcomeSpec::Draw => TerminalResult::Draw,
                    crate::effects::TerminalOutcomeSpec::Timeout => TerminalResult::Timeout,
                });
            }
            EffectKind::ApplyRuleOverride { kind } => {
                if !self.state.turn.rule_overrides.contains(kind) {
                    self.state.turn.rule_overrides.push(*kind);
                }
            }
        }
    }
}
