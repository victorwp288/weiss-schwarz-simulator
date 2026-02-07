use super::super::GameEnv;
use crate::db::CardId;
use crate::effects::*;
use crate::encode::MAX_STAGE;
use crate::events::{Event, RevealReason, Zone};
use crate::state::*;

impl GameEnv {
    pub(in crate::env) fn allocate_effect_instance_id(&mut self) -> u32 {
        let id = self.state.turn.next_effect_instance_id;
        self.state.turn.next_effect_instance_id =
            self.state.turn.next_effect_instance_id.wrapping_add(1);
        id
    }

    pub(in crate::env) fn enqueue_effect_spec(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
    ) {
        self.enqueue_effect_spec_with_source(controller, source_id, spec, None);
    }

    pub(in crate::env) fn enqueue_effect_spec_with_source(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
        source: Option<TargetRef>,
    ) {
        let instance_id = self.allocate_effect_instance_id();
        if spec.kind.expects_target() {
            if let Some(target_spec) = spec.target.clone() {
                if target_spec.source_only {
                    if let Some(source_ref) = source {
                        if self.source_ref_matches_spec(controller, &target_spec, &source_ref) {
                            self.enqueue_effect_with_targets(
                                controller,
                                source_id,
                                spec,
                                vec![source_ref],
                            );
                        }
                    }
                    return;
                }
                let allow_skip = spec.optional;
                self.start_target_selection(
                    controller,
                    source_id,
                    target_spec,
                    PendingTargetEffect::EffectPending {
                        instance_id,
                        payload: EffectPayload {
                            spec,
                            targets: Vec::new(),
                        },
                    },
                    allow_skip,
                );
                return;
            }
        }
        let item = StackItem {
            id: instance_id,
            controller,
            source_id,
            effect_id: spec.id,
            payload: EffectPayload {
                spec,
                targets: Vec::new(),
            },
        };
        self.enqueue_stack_items(vec![item]);
    }

    pub(in crate::env) fn enqueue_effect_with_targets(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
        targets: Vec<TargetRef>,
    ) {
        let instance_id = self.allocate_effect_instance_id();
        let item = StackItem {
            id: instance_id,
            controller,
            source_id,
            effect_id: spec.id,
            payload: EffectPayload { spec, targets },
        };
        self.enqueue_stack_items(vec![item]);
    }

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
            EffectKind::MoveTriggerCardToHand => {
                let _ = self.move_trigger_card_from_stock_to_hand(controller, source_id);
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
                    let mut moved_card = moved_slot.card.take().expect("card present");
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
            EffectKind::TriggerIcon { .. } => {}
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
        }
    }

    fn source_ref_matches_spec(
        &self,
        controller: u8,
        spec: &TargetSpec,
        source: &TargetRef,
    ) -> bool {
        let target_player = match spec.side {
            TargetSide::SelfSide => controller,
            TargetSide::Opponent => 1 - controller,
        };
        if source.player != target_player {
            return false;
        }
        if source.zone != spec.zone {
            return false;
        }
        match spec.slot_filter {
            TargetSlotFilter::FrontRow if source.index >= 3 => return false,
            TargetSlotFilter::BackRow if source.index < 3 => return false,
            TargetSlotFilter::SpecificSlot(slot) if source.index != slot => return false,
            _ => {}
        }
        if let Some(card_type) = spec.card_type {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if card.card_type != card_type {
                return false;
            }
        }
        if let Some(trait_id) = spec.card_trait {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if !card.traits.contains(&trait_id) {
                return false;
            }
        }
        if let Some(level_max) = spec.level_max {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if card.level > level_max {
                return false;
            }
        }
        if let Some(cost_max) = spec.cost_max {
            let Some(card) = self.db.get(source.card_id) else {
                return false;
            };
            if card.cost > cost_max {
                return false;
            }
        }
        true
    }
}
