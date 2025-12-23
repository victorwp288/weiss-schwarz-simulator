use super::{GameEnv, TriggerCompileContext, VisibilityContext, MAX_CHOICE_OPTIONS};
use anyhow::{anyhow, Result};
use crate::config::*;
use crate::db::*;
use crate::effects::*;
use crate::encode::*;
use crate::events::*;
use crate::legal::*;
use crate::state::*;

impl GameEnv {
    pub(super) fn allocate_trigger_group(&mut self) -> u32 {
        let group_id = self.state.turn.next_trigger_group_id;
        self.state.turn.next_trigger_group_id =
            self.state.turn.next_trigger_group_id.wrapping_add(1);
        group_id
    }

    pub(super) fn allocate_choice_id(&mut self) -> u32 {
        let choice_id = self.state.turn.next_choice_id;
        self.state.turn.next_choice_id = self.state.turn.next_choice_id.wrapping_add(1);
        choice_id
    }

    pub(super) fn allocate_stack_group_id(&mut self) -> u32 {
        let group_id = self.state.turn.next_stack_group_id;
        self.state.turn.next_stack_group_id = self.state.turn.next_stack_group_id.wrapping_add(1);
        group_id
    }

    pub(super) fn choice_option_id(&self, option: &ChoiceOptionRef, choice_id: u32, global_index: usize) -> u64 {
        let zone_id = match option.zone {
            ChoiceZone::WaitingRoom => 1u64,
            ChoiceZone::Stage => 2u64,
            ChoiceZone::DeckTop => 3u64,
            ChoiceZone::Hand => 4u64,
            ChoiceZone::Clock => 5u64,
            ChoiceZone::Level => 6u64,
            ChoiceZone::Stock => 7u64,
            ChoiceZone::Memory => 8u64,
            ChoiceZone::Climax => 9u64,
            ChoiceZone::Stack => 10u64,
            ChoiceZone::PriorityCounter => 11u64,
            ChoiceZone::PriorityAct => 12u64,
        };
        let index = option.index.unwrap_or(0) as u64;
        let target = option.target_slot.unwrap_or(0) as u64;
        let hidden_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::Memory
                | ChoiceZone::PriorityCounter
        );
        if option.instance_id != 0 {
            (option.instance_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
        } else if option.card_id != 0 && !hidden_zone {
            (option.card_id as u64) << 32 | (zone_id << 24) | (index << 8) | target
        } else {
            let choice_tag = (choice_id as u64) << 32;
            let global_tag = (global_index as u64 & 0xFFFF) << 8;
            choice_tag | (zone_id << 24) | global_tag | target
        }
    }

    pub(super) fn summarize_choice_options_for_event(
        &self,
        reason: ChoiceReason,
        player: u8,
        options: &[ChoiceOptionSnapshot],
        page_start: u16,
        choice_id: u32,
        ctx: VisibilityContext,
    ) -> Vec<ChoiceOptionSnapshot> {
        options
            .iter()
            .enumerate()
            .map(|(idx, opt)| {
                let global_index = page_start as usize + idx;
                let sanitized =
                    self.sanitize_choice_option_for_event(reason, player, ctx, &opt.reference);
                ChoiceOptionSnapshot {
                    option_id: self.choice_option_id(&sanitized, choice_id, global_index),
                    reference: sanitized,
                }
            })
            .collect()
    }

    pub(super) fn sanitize_choice_option_for_event(
        &self,
        reason: ChoiceReason,
        player: u8,
        ctx: VisibilityContext,
        option: &ChoiceOptionRef,
    ) -> ChoiceOptionRef {
        if !ctx.is_public() {
            return *option;
        }
        let option_player = if reason == ChoiceReason::TargetSelect {
            self.state
                .turn
                .target_selection
                .as_ref()
                .map(|selection| match selection.spec.side {
                    TargetSide::SelfSide => selection.controller,
                    TargetSide::Opponent => 1 - selection.controller,
                })
                .unwrap_or(player)
        } else {
            player
        };
        let hide_for_viewer = match ctx.viewer {
            Some(viewer) => viewer != option_player,
            None => true,
        };
        if !hide_for_viewer {
            return *option;
        }
        let hide_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::Memory
                | ChoiceZone::PriorityCounter
        );
        if !hide_zone {
            return *option;
        }
        let revealed = self.instance_revealed_to_viewer(ctx, option.instance_id);
        ChoiceOptionRef {
            card_id: if revealed { option.card_id } else { 0 },
            instance_id: 0,
            zone: option.zone,
            index: None,
            target_slot: option.target_slot,
        }
    }

    pub(super) fn choice_page_bounds(&self, total: usize, page_start: usize) -> (usize, usize) {
        let start = page_start.min(total);
        let end = total.min(start + MAX_CHOICE_OPTIONS);
        (start, end)
    }

    pub(super) fn recycle_choice_options(&mut self, options: Vec<ChoiceOptionRef>) {
        self.scratch.choice_options = options;
    }

    pub(super) fn start_choice(
        &mut self,
        reason: ChoiceReason,
        player: u8,
        candidates: Vec<ChoiceOptionRef>,
        pending_trigger: Option<PendingTrigger>,
    ) -> bool {
        let total = candidates.len();
        let choice_id = self.allocate_choice_id();
        if total == 0 {
            if self.recording {
                self.log_event(Event::ChoiceSkipped {
                    choice_id,
                    player,
                    reason,
                    skip_reason: ChoiceSkipReason::NoCandidates,
                });
            }
            if let Some(trigger) = pending_trigger {
                self.log_event(Event::TriggerResolved {
                    trigger_id: trigger.id,
                    player: trigger.player,
                    effect: trigger.effect,
                });
            }
            self.recycle_choice_options(candidates);
            return false;
        }
        if total == 1 {
            let option = candidates[0];
            if self.recording {
                self.log_event(Event::ChoiceAutopicked {
                    choice_id,
                    player,
                    reason,
                    option,
                });
            }
            self.recycle_choice_options(candidates);
            self.apply_choice_effect(reason, player, option, pending_trigger);
            return false;
        }
        let page_start = 0u16;
        let (page_start_idx, page_end_idx) = self.choice_page_bounds(total, 0);
        let page_slice = &candidates[page_start_idx..page_end_idx];
        let total_candidates = total.min(u16::MAX as usize) as u16;
        if self.recording {
            let mut options = Vec::with_capacity(page_slice.len());
            for (idx, opt) in page_slice.iter().enumerate() {
                options.push(ChoiceOptionSnapshot {
                    option_id: self.choice_option_id(
                        opt,
                        choice_id,
                        page_start as usize + idx,
                    ),
                    reference: *opt,
                });
            }
            self.log_event(Event::ChoicePresented {
                choice_id,
                player,
                reason,
                options,
                total_candidates,
                page_start,
            });
        }
        self.state.turn.choice = Some(ChoiceState {
            id: choice_id,
            reason,
            player,
            options: candidates,
            total_candidates,
            page_start,
            pending_trigger,
        });
        true
    }

    pub(super) fn apply_choice_effect(
        &mut self,
        reason: ChoiceReason,
        player: u8,
        option: ChoiceOptionRef,
        pending_trigger: Option<PendingTrigger>,
    ) {
        match reason {
            ChoiceReason::TriggerStandbySelect => {
                let Some(target_slot) = option.target_slot else {
                    return;
                };
                let ctx = TriggerCompileContext {
                    source_card: pending_trigger
                        .as_ref()
                        .map(|t| t.source_card)
                        .unwrap_or(option.card_id),
                    standby_slot: Some(target_slot),
                    treasure_take_stock: None,
                };
                let effects = self.compile_trigger_icon_effects(TriggerIcon::Standby, ctx);
                if effects.is_empty() {
                    return;
                }
                let Some(index) = option.index else {
                    return;
                };
                let targets = vec![TargetRef {
                    player,
                    zone: TargetZone::WaitingRoom,
                    index,
                    card_id: option.card_id,
                    instance_id: option.instance_id,
                }];
                for effect in effects {
                    self.enqueue_effect_with_targets(
                        player,
                        ctx.source_card,
                        effect,
                        targets.clone(),
                    );
                }
            }
            ChoiceReason::TriggerTreasureSelect => {
                let take_stock = option.index.unwrap_or(1) == 0;
                let ctx = TriggerCompileContext {
                    source_card: pending_trigger.as_ref().map(|t| t.source_card).unwrap_or(0),
                    standby_slot: None,
                    treasure_take_stock: Some(take_stock),
                };
                let effects = self.compile_trigger_icon_effects(TriggerIcon::Treasure, ctx);
                for effect in effects {
                    self.enqueue_effect_spec(player, ctx.source_card, effect);
                }
            }
            ChoiceReason::StackOrderSelect => {
                self.apply_stack_order_choice(player, option);
            }
            ChoiceReason::PriorityActionSelect => {
                self.apply_priority_action_choice(player, option);
            }
            ChoiceReason::TargetSelect => {
                self.apply_target_choice(player, option);
            }
        }
        if let Some(trigger) = pending_trigger {
            self.log_event(Event::TriggerResolved {
                trigger_id: trigger.id,
                player: trigger.player,
                effect: trigger.effect,
            });
        }
    }

    pub(super) fn start_target_selection(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: TargetSpec,
        effect: PendingTargetEffect,
    ) {
        self.state.turn.target_selection = Some(TargetSelectionState {
            controller,
            source_id,
            remaining: spec.count,
            spec,
            selected: Vec::new(),
            effect,
        });
        self.present_target_choice();
    }

    pub(super) fn allocate_effect_instance_id(&mut self) -> u32 {
        let id = self.state.turn.next_effect_instance_id;
        self.state.turn.next_effect_instance_id =
            self.state.turn.next_effect_instance_id.wrapping_add(1);
        id
    }

    pub(super) fn enqueue_effect_spec(&mut self, controller: u8, source_id: CardId, spec: EffectSpec) {
        let instance_id = self.allocate_effect_instance_id();
        if spec.kind.expects_target() {
            if let Some(target_spec) = spec.target.clone() {
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

    pub(super) fn enqueue_effect_with_targets(
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

    pub(super) fn enumerate_target_candidates_into(
        state: &GameState,
        db: &CardDb,
        curriculum: &CurriculumConfig,
        controller: u8,
        spec: &TargetSpec,
        selected: &[TargetRef],
        out: &mut Vec<TargetRef>,
    ) {
        let target_player = match spec.side {
            TargetSide::SelfSide => controller,
            TargetSide::Opponent => 1 - controller,
        };
        out.clear();
        match spec.zone {
            TargetZone::Stage => {
                let max_slot = if curriculum.reduced_stage_mode {
                    1
                } else {
                    MAX_STAGE
                };
                // Deterministic target ordering: stage slot ascending (front row is slots 0..2, then back row).
                for slot in 0..max_slot {
                    if spec.slot_filter == TargetSlotFilter::FrontRow && slot >= 3 {
                        continue;
                    }
                    let slot_state = &state.players[target_player as usize].stage[slot];
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Stage
                            && t.index as usize == slot
                    }) {
                        continue;
                    }
                    let index = slot as u8;
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Stage,
                        index,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::WaitingRoom => {
                // Deterministic target ordering: waiting room index ascending.
                for (idx, card_inst) in state.players[target_player as usize]
                    .waiting_room
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::WaitingRoom
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::WaitingRoom,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Hand => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .hand
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Hand
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Hand,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::DeckTop => {
                let deck = &state.players[target_player as usize].deck;
                for offset in 0..deck.len() {
                    if offset > u8::MAX as usize {
                        break;
                    }
                    let deck_idx = deck.len().saturating_sub(1 + offset);
                    let card_inst = deck.get(deck_idx).copied();
                    let Some(card_inst) = card_inst else {
                        continue;
                    };
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::DeckTop
                            && t.index as usize == offset
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::DeckTop,
                        index: offset as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Clock => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .clock
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Clock
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Clock,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Level => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .level
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Level
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Level,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Stock => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .stock
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Stock
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Stock,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Memory => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .memory
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Memory
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Memory,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
            TargetZone::Climax => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .climax
                    .iter()
                    .copied()
                    .enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    let Some(card) = db.get(card_inst.id) else {
                        continue;
                    };
                    if let Some(card_type) = spec.card_type {
                        if card.card_type != card_type {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Climax
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Climax,
                        index: idx as u8,
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                    });
                }
            }
        }
    }

    pub(super) fn present_target_choice(&mut self) {
        let controller = {
            let Some(selection) = self.state.turn.target_selection.as_ref() else {
                return;
            };
            Self::enumerate_target_candidates_into(
                &self.state,
                &self.db,
                &self.curriculum,
                selection.controller,
                &selection.spec,
                &selection.selected,
                &mut self.scratch.targets,
            );
            selection.controller
        };
        let candidates = self.scratch.targets.as_slice();
        if candidates.is_empty() {
            let _ = self.start_choice(
                ChoiceReason::TargetSelect,
                controller,
                Vec::new(),
                None,
            );
            self.state.turn.target_selection = None;
            return;
        }
        self.scratch.choice_options.clear();
        for target in candidates {
            let zone = match target.zone {
                TargetZone::Stage => ChoiceZone::Stage,
                TargetZone::WaitingRoom => ChoiceZone::WaitingRoom,
                TargetZone::Hand => ChoiceZone::Hand,
                TargetZone::DeckTop => ChoiceZone::DeckTop,
                TargetZone::Clock => ChoiceZone::Clock,
                TargetZone::Level => ChoiceZone::Level,
                TargetZone::Stock => ChoiceZone::Stock,
                TargetZone::Memory => ChoiceZone::Memory,
                TargetZone::Climax => ChoiceZone::Climax,
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: target.card_id,
                instance_id: target.instance_id,
                zone,
                index: Some(target.index),
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        let _ = self.start_choice(
            ChoiceReason::TargetSelect,
            controller,
            options,
            None,
        );
    }

    pub(super) fn apply_target_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let Some(mut selection) = self.state.turn.target_selection.take() else {
            return;
        };
        if selection.controller != player {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        let Some(index) = option.index else {
            self.state.turn.target_selection = Some(selection);
            return;
        };
        let zone = match option.zone {
            ChoiceZone::Stage => TargetZone::Stage,
            ChoiceZone::WaitingRoom => TargetZone::WaitingRoom,
            ChoiceZone::Hand => TargetZone::Hand,
            ChoiceZone::DeckTop => TargetZone::DeckTop,
            ChoiceZone::Clock => TargetZone::Clock,
            ChoiceZone::Level => TargetZone::Level,
            ChoiceZone::Stock => TargetZone::Stock,
            ChoiceZone::Memory => TargetZone::Memory,
            ChoiceZone::Climax => TargetZone::Climax,
            _ => {
                self.state.turn.target_selection = Some(selection);
                return;
            }
        };
        if zone != selection.spec.zone {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        let target_player = match selection.spec.side {
            TargetSide::SelfSide => selection.controller,
            TargetSide::Opponent => 1 - selection.controller,
        };
        let valid = match zone {
            TargetZone::Stage => {
                let slot = index as usize;
                if slot >= self.state.players[target_player as usize].stage.len() {
                    false
                } else {
                    self.state.players[target_player as usize].stage[slot]
                        .card
                        .map(|c| c.instance_id)
                        == Some(option.instance_id)
                }
            }
            TargetZone::WaitingRoom => {
                let idx = index as usize;
                if idx
                    >= self.state.players[target_player as usize]
                        .waiting_room
                        .len()
                {
                    false
                } else {
                    self.state.players[target_player as usize].waiting_room[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Hand => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].hand.len() {
                    false
                } else {
                    self.state.players[target_player as usize].hand[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::DeckTop => {
                let offset = index as usize;
                let deck = &self.state.players[target_player as usize].deck;
                let deck_idx = deck.len().saturating_sub(1 + offset);
                if deck_idx >= deck.len() {
                    false
                } else {
                    deck[deck_idx].instance_id == option.instance_id
                }
            }
            TargetZone::Clock => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].clock.len() {
                    false
                } else {
                    self.state.players[target_player as usize].clock[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Level => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].level.len() {
                    false
                } else {
                    self.state.players[target_player as usize].level[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Stock => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].stock.len() {
                    false
                } else {
                    self.state.players[target_player as usize].stock[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Memory => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].memory.len() {
                    false
                } else {
                    self.state.players[target_player as usize].memory[idx].instance_id
                        == option.instance_id
                }
            }
            TargetZone::Climax => {
                let idx = index as usize;
                if idx >= self.state.players[target_player as usize].climax.len() {
                    false
                } else {
                    self.state.players[target_player as usize].climax[idx].instance_id
                        == option.instance_id
                }
            }
        };
        if !valid {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        let target = TargetRef {
            player: target_player,
            zone,
            index,
            card_id: option.card_id,
            instance_id: option.instance_id,
        };
        if selection
            .selected
            .iter()
            .any(|t| t.player == target.player && t.zone == target.zone && t.index == target.index)
        {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        selection.selected.push(target);
        if selection.remaining > 0 {
            selection.remaining -= 1;
        }
        if selection.remaining == 0 {
            let targets = selection.selected.clone();
            match selection.effect {
                PendingTargetEffect::EffectPending {
                    instance_id,
                    mut payload,
                } => {
                    payload.targets = targets;
                    let item = StackItem {
                        id: instance_id,
                        controller: selection.controller,
                        source_id: selection.source_id,
                        effect_id: payload.spec.id,
                        payload,
                    };
                    self.enqueue_stack_items(vec![item]);
                }
            }
            self.state.turn.target_selection = None;
            return;
        }
        self.state.turn.target_selection = Some(selection);
        self.present_target_choice();
    }

    pub(super) fn enter_timing_window(&mut self, window: TimingWindow, holder: u8) {
        self.state.turn.priority = Some(PriorityState {
            holder,
            passes: 0,
            window,
            used_act_mask: 0,
        });
        self.state.turn.active_window = Some(window);
        self.log_event(Event::TimingWindowEntered {
            window,
            player: holder,
        });
        self.log_event(Event::PriorityGranted {
            window,
            player: holder,
        });
    }

    pub(super) fn collect_priority_actions(&mut self, player: u8) {
        self.scratch.priority_actions.clear();
        let Some(priority) = self.state.turn.priority.as_ref() else {
            return;
        };
        if priority.holder != player {
            return;
        }
        match priority.window {
            TimingWindow::MainWindow => {
                if !self.curriculum.enable_activated_abilities {
                    return;
                }
                let p = &self.state.players[player as usize];
                let max_slot = if self.curriculum.reduced_stage_mode {
                    1
                } else {
                    MAX_STAGE
                };
                // Deterministic priority ordering: stage slot ascending, then ability index ascending.
                for slot in 0..max_slot {
                    let slot_state = &p.stage[slot];
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    let card_id = card_inst.id;
                    if self.db.get(card_id).is_none() {
                        continue;
                    }
                    let specs = self.db.iter_card_abilities_in_canonical_order(card_id);
                    for (idx, spec) in specs.iter().enumerate() {
                        if idx >= MAX_ABILITIES_PER_CARD || idx > u8::MAX as usize {
                            break;
                        }
                        if spec.kind != AbilityKind::Activated {
                            continue;
                        }
                        if self
                            .db
                            .compiled_effects_for_ability(card_id, idx)
                            .is_empty()
                        {
                            continue;
                        }
                        let bit = (slot * MAX_ABILITIES_PER_CARD + idx) as u32;
                        if priority.used_act_mask & (1u32 << bit) != 0 {
                            continue;
                        }
                        self.scratch.priority_actions.push(ActionDesc::MainActivateAbility {
                            slot: slot as u8,
                            ability_index: idx as u8,
                        });
                    }
                }
            }
            TimingWindow::CounterWindow => {
                let Some(ctx) = &self.state.turn.attack else {
                    return;
                };
                if ctx.attack_type != AttackType::Frontal
                    || ctx.defender_slot.is_none()
                    || ctx.counter_played
                {
                    return;
                }
                if self.curriculum.enable_counters {
                    let p = &self.state.players[player as usize];
                    // Deterministic priority ordering: hand index ascending.
                    for (hand_index, card_inst) in p.hand.iter().enumerate() {
                        if hand_index >= crate::encode::MAX_HAND || hand_index > u8::MAX as usize {
                            break;
                        }
                        let Some(card) = self.db.get(card_inst.id) else {
                            continue;
                        };
                        if !self.card_set_allowed(card) {
                            continue;
                        }
                        if self.is_counter_card(card)
                            && self.meets_level_requirement(player, card)
                            && self.meets_color_requirement(player, card)
                            && self.meets_cost_requirement(player, card)
                        {
                            self.scratch.priority_actions.push(ActionDesc::CounterPlay {
                                hand_index: hand_index as u8,
                            });
                        }
                    }
                }
            }
            TimingWindow::ClimaxWindow
            | TimingWindow::AttackDeclarationWindow
            | TimingWindow::TriggerResolutionWindow
            | TimingWindow::DamageResolutionWindow
            | TimingWindow::EncoreWindow
            | TimingWindow::EndPhaseWindow => {}
        }
    }

    pub(super) fn start_priority_choice(&mut self, player: u8) {
        self.scratch.choice_options.clear();
        for action in self.scratch.priority_actions.iter() {
            match *action {
                ActionDesc::CounterPlay { hand_index } => {
                    let (card_id, instance_id) = self.state.players[player as usize]
                        .hand
                        .get(hand_index as usize)
                        .map(|c| (c.id, c.instance_id))
                        .unwrap_or((0, 0));
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id,
                        instance_id,
                        zone: ChoiceZone::PriorityCounter,
                        index: Some(hand_index),
                        target_slot: None,
                    });
                }
                ActionDesc::MainActivateAbility {
                    slot,
                    ability_index,
                } => {
                    let (card_id, instance_id) = self.state.players[player as usize]
                        .stage
                        .get(slot as usize)
                        .and_then(|s| s.card)
                        .map(|c| (c.id, c.instance_id))
                        .unwrap_or((0, 0));
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id,
                        instance_id,
                        zone: ChoiceZone::PriorityAct,
                        index: Some(slot),
                        target_slot: Some(ability_index),
                    });
                }
                _ => {}
            }
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(ChoiceReason::PriorityActionSelect, player, options, None);
    }

    pub(super) fn apply_priority_action_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let action = match option.zone {
            ChoiceZone::PriorityCounter => option
                .index
                .map(|idx| ActionDesc::CounterPlay { hand_index: idx }),
            ChoiceZone::PriorityAct => {
                if let (Some(slot), Some(ability)) = (option.index, option.target_slot) {
                    Some(ActionDesc::MainActivateAbility {
                        slot,
                        ability_index: ability,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(action) = action {
            let _ = self.apply_priority_action(player, action);
        }
    }

    pub(super) fn apply_priority_action(&mut self, player: u8, action: ActionDesc) -> Result<()> {
        let Some(priority) = self.state.turn.priority.as_ref() else {
            return Err(anyhow!("Priority window not active"));
        };
        if priority.holder != player {
            return Err(anyhow!("Priority holder mismatch"));
        }
        let window = priority.window;
        match action {
            ActionDesc::MainActivateAbility {
                slot,
                ability_index,
            } => {
                if window != TimingWindow::MainWindow {
                    return Err(anyhow!("Activated abilities not allowed in this window"));
                }
                self.queue_activated_ability_stack_item(player, slot, ability_index)?;
                let bit = slot as u32 * MAX_ABILITIES_PER_CARD as u32 + ability_index as u32;
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.used_act_mask |= 1u32 << bit;
                    priority.holder = 1 - player;
                    priority.passes = 0;
                    new_holder = Some(priority.holder);
                }
                if let Some(holder) = new_holder {
                    self.log_event(Event::PriorityGranted {
                        window,
                        player: holder,
                    });
                }
            }
            ActionDesc::CounterPlay { hand_index } => {
                if window != TimingWindow::CounterWindow {
                    return Err(anyhow!("Counter play not allowed in this window"));
                }
                self.queue_counter_stack_item(player, hand_index)?;
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.holder = 1 - player;
                    priority.passes = 0;
                    new_holder = Some(priority.holder);
                }
                if let Some(holder) = new_holder {
                    self.log_event(Event::PriorityGranted {
                        window,
                        player: holder,
                    });
                }
            }
            ActionDesc::MainPass | ActionDesc::CounterPass => {
                self.collect_priority_actions(player);
                if !self.scratch.priority_actions.is_empty() {
                    return Err(anyhow!(
                        "Explicit pass not allowed when priority actions exist"
                    ));
                }
                self.priority_pass(player);
            }
            _ => return Err(anyhow!("Invalid priority action")),
        }
        Ok(())
    }

    pub(super) fn priority_pass(&mut self, player: u8) {
        let (window, pass_count, should_check_stack, new_holder) = {
            let Some(priority) = &mut self.state.turn.priority else {
                return;
            };
            if priority.holder != player {
                return;
            }
            priority.passes = priority.passes.saturating_add(1);
            let window = priority.window;
            let pass_count = priority.passes;
            let mut new_holder = None;
            if pass_count < 2 {
                priority.holder = 1 - player;
                new_holder = Some(priority.holder);
            }
            (window, pass_count, pass_count >= 2, new_holder)
        };
        self.log_event(Event::PriorityPassed {
            player,
            window,
            pass_count,
        });
        if let Some(holder) = new_holder {
            self.log_event(Event::PriorityGranted {
                window,
                player: holder,
            });
        }
        if should_check_stack {
            if let Some(item) = self.state.turn.stack.pop() {
                self.resolve_stack_item(&item);
                self.log_event(Event::StackResolved { item });
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.passes = 0;
                    priority.holder = self.state.turn.active_player;
                    new_holder = Some(priority.holder);
                }
                if let Some(holder) = new_holder {
                    self.log_event(Event::PriorityGranted {
                        window,
                        player: holder,
                    });
                }
            } else {
                self.close_priority_window(window);
            }
        }
    }

    pub(super) fn close_priority_window(&mut self, window: TimingWindow) {
        self.state.turn.priority = None;
        self.state.turn.active_window = None;
        match window {
            TimingWindow::MainWindow => {
                if self.state.turn.main_passed {
                    self.state.turn.main_passed = false;
                    self.state.turn.phase = Phase::Climax;
                }
            }
            TimingWindow::CounterWindow => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.step = AttackStep::Damage;
                }
            }
            TimingWindow::ClimaxWindow => {
                self.state.turn.phase = Phase::Attack;
            }
            TimingWindow::AttackDeclarationWindow => {}
            TimingWindow::TriggerResolutionWindow => {}
            TimingWindow::DamageResolutionWindow => {}
            TimingWindow::EncoreWindow => {}
            TimingWindow::EndPhaseWindow => {}
        }
        self.log_event(Event::WindowAdvanced {
            from: window,
            to: self.state.turn.active_window,
        });
    }

    pub(super) fn stack_effect_key(effect: &EffectKind) -> u8 {
        match effect {
            EffectKind::CounterBackup { .. } => 0,
            EffectKind::CounterDamageReduce { .. } => 1,
            EffectKind::CounterDamageCancel => 2,
            EffectKind::AddModifier { .. } => 3,
            EffectKind::MoveToHand => 4,
            EffectKind::MoveTriggerCardToHand => 5,
            EffectKind::ChangeController { .. } => 6,
            EffectKind::Standby { .. } => 7,
            EffectKind::TreasureStock { .. } => 8,
            EffectKind::ModifyPendingAttackDamage { .. } => 9,
            EffectKind::Damage { .. } => 10,
            EffectKind::Draw { .. } => 11,
            EffectKind::TriggerIcon { .. } => 12,
        }
    }

    pub(super) fn enqueue_stack_items(&mut self, items: Vec<StackItem>) {
        if items.is_empty() {
            return;
        }
        let active = self.state.turn.active_player;
        let mut per_player: [Vec<StackItem>; 2] = [Vec::new(), Vec::new()];
        for item in items {
            per_player[item.controller as usize].push(item);
        }
        for controller in [active, 1 - active] {
            let list = &mut per_player[controller as usize];
            if list.is_empty() {
                continue;
            }
            // Deterministic ordering for simultaneous stack items: source id, effect kind, then stack id.
            list.sort_by_key(|item| {
                (
                    item.source_id,
                    Self::stack_effect_key(&item.payload.spec.kind),
                    item.id,
                )
            });
            let group_id = self.allocate_stack_group_id();
            let items = std::mem::take(list);
            let group = StackOrderState {
                group_id,
                controller,
                items,
            };
            self.state.turn.pending_stack_groups.push(group);
        }
        self.process_next_stack_group();
    }

    pub(super) fn process_next_stack_group(&mut self) {
        if self.state.turn.stack_order.is_some() {
            return;
        }
        if self.state.turn.pending_stack_groups.is_empty() {
            return;
        }
        let group = self.state.turn.pending_stack_groups.remove(0);
        if group.items.len() == 1 {
            let item = group.items.into_iter().next().expect("group item");
            self.push_stack_item(item);
            self.process_next_stack_group();
            return;
        }
        self.log_event(Event::StackGroupPresented {
            group_id: group.group_id,
            controller: group.controller,
            items: group.items.clone(),
        });
        self.state.turn.stack_order = Some(group);
        self.present_stack_order_choice();
    }

    pub(super) fn present_stack_order_choice(&mut self) {
        let Some(order) = &self.state.turn.stack_order else {
            return;
        };
        self.scratch.choice_options.clear();
        for (idx, item) in order.items.iter().enumerate() {
            let index = if idx <= u8::MAX as usize {
                Some(idx as u8)
            } else {
                None
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: item.source_id,
                instance_id: 0,
                zone: ChoiceZone::Stack,
                index,
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(
            ChoiceReason::StackOrderSelect,
            order.controller,
            options,
            None,
        );
    }

    pub(super) fn apply_stack_order_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        if option.zone != ChoiceZone::Stack {
            return;
        }
        let Some(idx) = option.index else {
            return;
        };
        let Some(mut order) = self.state.turn.stack_order.take() else {
            return;
        };
        if order.controller != player {
            self.state.turn.stack_order = Some(order);
            return;
        }
        let index = idx as usize;
        if index >= order.items.len() {
            self.state.turn.stack_order = Some(order);
            return;
        }
        let item = order.items.remove(index);
        self.log_event(Event::StackOrderChosen {
            group_id: order.group_id,
            controller: order.controller,
            stack_id: item.id,
        });
        self.push_stack_item(item);
        if !order.items.is_empty() {
            self.state.turn.stack_order = Some(order);
            self.present_stack_order_choice();
        } else {
            self.state.turn.stack_order = None;
            self.process_next_stack_group();
        }
    }

    pub(super) fn push_stack_item(&mut self, item: StackItem) {
        self.state.turn.stack.push(item.clone());
        self.log_event(Event::StackPushed { item });
    }

    pub(super) fn resolve_stack_item(&mut self, item: &StackItem) {
        self.resolve_effect_payload(item.controller, item.source_id, &item.payload);
    }

    pub(super) fn resolve_effect_payload(
        &mut self,
        controller: u8,
        source_id: CardId,
        payload: &EffectPayload,
    ) {
        match &payload.spec.kind {
            EffectKind::Draw { count } => {
                self.draw_to_hand(controller, *count as usize);
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
                    if self.state.players[p].stage[s]
                        .card
                        .map(|c| c.instance_id)
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
                for target in &payload.targets {
                    match target.zone {
                        TargetZone::Stage => {
                            let option = ChoiceOptionRef {
                                card_id: target.card_id,
                                instance_id: target.instance_id,
                                zone: ChoiceZone::Stage,
                                index: Some(target.index),
                                target_slot: None,
                            };
                            self.move_stage_to_hand(target.player, option);
                        }
                        TargetZone::WaitingRoom => {
                            waiting_room_targets.push(*target);
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
                        index: Some(target.index),
                        target_slot: None,
                    };
                    self.move_waiting_room_to_hand(target.player, option);
                }
            }
            EffectKind::MoveTriggerCardToHand => {
                let _ = self.move_trigger_card_from_stock_to_hand(controller, source_id);
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
                    self.apply_continuous_modifiers_for_slot(
                        to_player,
                        target.index,
                        moved_card.id,
                    );
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
                    index: Some(target.index),
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
            EffectKind::TriggerIcon { .. } => {}
            EffectKind::CounterBackup { power } => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    if let Some(def_slot) = ctx.defender_slot {
                        let slot_state =
                            &mut self.state.players[controller as usize].stage[def_slot as usize];
                        slot_state.power_mod_battle += *power;
                        ctx.counter_power += *power;
                    }
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

    pub(super) fn apply_replacements_to_damage(
        &mut self,
        source_player: u8,
        target_player: u8,
        amount: i32,
    ) -> (i32, u8) {
        let mut amount = amount;
        let mut target = target_player;
        if amount <= 0 {
            return (0, target);
        }
        self.scratch_replacement_indices.clear();
        for (idx, replacement) in self.state.replacements.iter().enumerate() {
            if matches!(replacement.hook, ReplacementHook::Damage) {
                self.scratch_replacement_indices.push(idx);
            }
        }
        self.scratch_replacement_indices.sort_by_key(|idx| {
            let replacement = &self.state.replacements[*idx];
            (
                replacement.priority,
                replacement.insertion,
                replacement.source,
            )
        });
        for idx in self.scratch_replacement_indices.iter().copied() {
            let replacement = &self.state.replacements[idx];
            match replacement.kind {
                ReplacementKind::CancelDamage => {
                    amount = 0;
                    break;
                }
                ReplacementKind::RedirectDamage { new_target } => {
                    target = match new_target {
                        TargetSide::SelfSide => source_player,
                        TargetSide::Opponent => 1 - source_player,
                    };
                }
            }
        }
        (amount, target)
    }

    pub(super) fn queue_activated_ability_stack_item(
        &mut self,
        player: u8,
        slot: u8,
        ability_index: u8,
    ) -> Result<()> {
        if !self.curriculum.enable_activated_abilities {
            return Err(anyhow!("Activated abilities disabled"));
        }
        let p = player as usize;
        let s = slot as usize;
        if s >= self.state.players[p].stage.len() {
            return Err(anyhow!("Ability slot out of range"));
        }
        let card_inst = self.state.players[p].stage[s]
            .card
            .ok_or_else(|| anyhow!("No card in ability slot"))?;
        let card_id = card_inst.id;
        let db = self.db.clone();
        if db.get(card_id).is_none() {
            return Err(anyhow!("Card missing in db"));
        }
        let idx = ability_index as usize;
        let spec_kind = db
            .iter_card_abilities_in_canonical_order(card_id)
            .get(idx)
            .map(|spec| spec.kind);
        if idx >= MAX_ABILITIES_PER_CARD {
            return Err(anyhow!("Ability index out of range"));
        }
        let Some(spec_kind) = spec_kind else {
            return Err(anyhow!("Ability index out of range"));
        };
        if spec_kind != AbilityKind::Activated {
            return Err(anyhow!("Ability is not activated"));
        }
        let effects = db.compiled_effects_for_ability(card_id, idx);
        if effects.is_empty() {
            return Err(anyhow!("Activated ability has no effects"));
        }
        for effect in effects {
            self.enqueue_effect_spec(player, card_id, effect.clone());
        }
        Ok(())
    }

    pub(super) fn queue_counter_stack_item(&mut self, player: u8, hand_index: u8) -> Result<()> {
        if !self.curriculum.enable_counters {
            return Err(anyhow!("Counters disabled"));
        }
        let Some(ctx) = &self.state.turn.attack else {
            return Err(anyhow!("No attack context for counter"));
        };
        if ctx.attack_type != AttackType::Frontal
            || ctx.defender_slot.is_none()
            || ctx.counter_played
        {
            return Err(anyhow!("Counter not allowed for this attack"));
        }
        let p = player as usize;
        let hi = hand_index as usize;
        if hi >= self.state.players[p].hand.len() {
            return Err(anyhow!("Counter hand index out of range"));
        }
        let card_inst = self.state.players[p].hand[hi];
        let card_id = card_inst.id;
        let card = self
            .db
            .get(card_id)
            .ok_or_else(|| anyhow!("Card missing in db"))?;
        if !self.card_set_allowed(card) {
            return Err(anyhow!("Card set not allowed"));
        }
        if !self.is_counter_card(card) {
            return Err(anyhow!("Card is not a counter"));
        }
        if !self.meets_level_requirement(player, card)
            || !self.meets_color_requirement(player, card)
            || !self.meets_cost_requirement(player, card)
        {
            return Err(anyhow!("Counter requirements not met"));
        }
        let power = self.counter_power(card);
        let damage_reductions = self.counter_damage_reductions(card);
        let damage_cancel = self.counter_damage_cancel(card);
        self.pay_cost(player, card.cost as usize)?;
        let card_inst = self.state.players[p].hand.remove(hi);
        let card_id = card_inst.id;
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::WaitingRoom,
            Some(hand_index),
            None,
        );
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.counter_played = true;
        }
        if power != 0 {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, 0),
                kind: EffectKind::CounterBackup { power },
                target: None,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        for (idx, reduce) in damage_reductions.into_iter().enumerate() {
            if reduce > 0 {
                let spec = EffectSpec {
                    id: EffectId::new(EffectSourceKind::Counter, card_id, 0, idx as u8),
                    kind: EffectKind::CounterDamageReduce {
                        amount: reduce as u8,
                    },
                    target: None,
                };
                self.enqueue_effect_spec(player, card_id, spec);
            }
        }
        if damage_cancel {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, 10),
                kind: EffectKind::CounterDamageCancel,
                target: None,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        Ok(())
    }

    pub(super) fn enumerate_open_stage_slots(&self, player: u8) -> Vec<u8> {
        let p = player as usize;
        let max_slot = if self.curriculum.reduced_stage_mode {
            1
        } else {
            MAX_STAGE
        };
        let mut slots = Vec::new();
        for slot in 0..max_slot {
            if self.state.players[p].stage[slot].card.is_none() {
                slots.push(slot as u8);
            }
        }
        slots
    }
}
