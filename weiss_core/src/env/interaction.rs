use super::{GameEnv, TriggerCompileContext, VisibilityContext, MAX_CHOICE_OPTIONS};
use crate::config::*;
use crate::db::*;
use crate::effects::*;
use crate::encode::*;
use crate::events::*;
use crate::legal::*;
use crate::state::*;
use anyhow::{anyhow, Result};

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

    pub(super) fn choice_option_id(
        &self,
        option: &ChoiceOptionRef,
        choice_id: u32,
        global_index: usize,
    ) -> u64 {
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
            ChoiceZone::Resolution => 10u64,
            ChoiceZone::Stack => 11u64,
            ChoiceZone::PriorityCounter => 12u64,
            ChoiceZone::PriorityAct => 13u64,
            ChoiceZone::PriorityPass => 14u64,
            ChoiceZone::Skip => 15u64,
        };
        let index = option.index.unwrap_or(0) as u64;
        let target = option.target_slot.unwrap_or(0) as u64;
        let hidden_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::PriorityCounter
        ) || (option.zone == ChoiceZone::Memory
            && !self.curriculum.memory_is_public);
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
        let sanitize_instance = ctx.viewer.is_none();
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
            if sanitize_instance && option.instance_id != 0 {
                let mut sanitized = *option;
                sanitized.instance_id = 0;
                return sanitized;
            }
            return *option;
        }
        let hide_zone = matches!(
            option.zone,
            ChoiceZone::Hand
                | ChoiceZone::DeckTop
                | ChoiceZone::Stock
                | ChoiceZone::PriorityCounter
        ) || (option.zone == ChoiceZone::Memory
            && !self.curriculum.memory_is_public);
        if !hide_zone {
            if sanitize_instance && option.instance_id != 0 {
                let mut sanitized = *option;
                sanitized.instance_id = 0;
                return sanitized;
            }
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
                    option_id: self.choice_option_id(opt, choice_id, page_start as usize + idx),
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
                if option.zone != ChoiceZone::Skip {
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
            ChoiceReason::CostPayment => {
                self.apply_cost_payment_choice(player, option);
            }
            ChoiceReason::TargetSelect => {
                self.apply_target_choice(player, option);
            }
            ChoiceReason::EndPhaseDiscard => {
                if option.zone != ChoiceZone::Hand {
                    return;
                }
                let Some(index) = option.index else {
                    return;
                };
                let p = player as usize;
                let idx = index as usize;
                if idx >= self.state.players[p].hand.len() {
                    return;
                }
                let card = self.state.players[p].hand[idx];
                if card.instance_id != option.instance_id {
                    return;
                }
                let card = self.state.players[p].hand.remove(idx);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Hand,
                    Zone::WaitingRoom,
                    Some(index),
                    None,
                );
                if self.state.players[p].hand.len() > super::HAND_LIMIT {
                    let _ = self.start_end_phase_discard_choice(player);
                } else {
                    self.state.turn.end_phase_discard_done = true;
                }
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
        allow_skip: bool,
    ) {
        Self::enumerate_target_candidates_into(
            &self.state,
            &self.db,
            &self.curriculum,
            controller,
            &spec,
            &[],
            &mut self.scratch.targets,
        );
        let candidates = self.scratch.targets.to_vec();
        if spec.reveal_to_controller {
            for target in candidates.iter().copied() {
                if target.zone != TargetZone::DeckTop {
                    continue;
                }
                let card = CardInstance {
                    id: target.card_id,
                    instance_id: target.instance_id,
                    owner: target.player,
                    controller: target.player,
                };
                self.reveal_card(
                    controller,
                    &card,
                    RevealReason::AbilityEffect,
                    RevealAudience::ControllerOnly,
                );
            }
        }
        self.state.turn.target_selection = Some(TargetSelectionState {
            controller,
            source_id,
            remaining: spec.count,
            spec,
            selected: Vec::new(),
            candidates,
            effect,
            allow_skip,
        });
        self.present_target_choice();
    }

    pub(super) fn allocate_effect_instance_id(&mut self) -> u32 {
        let id = self.state.turn.next_effect_instance_id;
        self.state.turn.next_effect_instance_id =
            self.state.turn.next_effect_instance_id.wrapping_add(1);
        id
    }

    pub(super) fn enqueue_effect_spec(
        &mut self,
        controller: u8,
        source_id: CardId,
        spec: EffectSpec,
    ) {
        self.enqueue_effect_spec_with_source(controller, source_id, spec, None);
    }

    pub(super) fn enqueue_effect_spec_with_source(
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
        if spec.source_only {
            out.clear();
            return;
        }
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
                    match spec.slot_filter {
                        TargetSlotFilter::FrontRow if slot >= 3 => continue,
                        TargetSlotFilter::BackRow if slot < 3 => continue,
                        TargetSlotFilter::SpecificSlot(target_slot)
                            if slot != target_slot as usize =>
                        {
                            continue
                        }
                        _ => {}
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                let max_offset = match spec.limit {
                    Some(limit) => std::cmp::min(deck.len(), limit as usize),
                    None => deck.len(),
                };
                for offset in 0..max_offset {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
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
            TargetZone::Resolution => {
                for (idx, card_inst) in state.players[target_player as usize]
                    .resolution
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
                    if let Some(trait_id) = spec.card_trait {
                        if !card.traits.contains(&trait_id) {
                            continue;
                        }
                    }
                    if let Some(level_max) = spec.level_max {
                        if card.level > level_max {
                            continue;
                        }
                    }
                    if let Some(cost_max) = spec.cost_max {
                        if card.cost > cost_max {
                            continue;
                        }
                    }
                    if selected.iter().any(|t| {
                        t.player == target_player
                            && t.zone == TargetZone::Resolution
                            && t.index as usize == idx
                    }) {
                        continue;
                    }
                    out.push(TargetRef {
                        player: target_player,
                        zone: TargetZone::Resolution,
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
            self.scratch.targets.clear();
            for candidate in selection.candidates.iter().copied() {
                if selection.selected.iter().any(|t| t == &candidate) {
                    continue;
                }
                self.scratch.targets.push(candidate);
            }
            selection.controller
        };
        let candidates = self.scratch.targets.as_slice();
        let allow_skip = self
            .state
            .turn
            .target_selection
            .as_ref()
            .map(|s| s.allow_skip)
            .unwrap_or(false);
        if candidates.is_empty() {
            let _ = self.start_choice(ChoiceReason::TargetSelect, controller, Vec::new(), None);
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
                TargetZone::Resolution => ChoiceZone::Resolution,
            };
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: target.card_id,
                instance_id: target.instance_id,
                zone,
                index: Some(target.index),
                target_slot: None,
            });
        }
        if allow_skip {
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: 0,
                instance_id: 0,
                zone: ChoiceZone::Skip,
                index: None,
                target_slot: None,
            });
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        let _ = self.start_choice(ChoiceReason::TargetSelect, controller, options, None);
    }

    pub(super) fn apply_target_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let Some(mut selection) = self.state.turn.target_selection.take() else {
            return;
        };
        if selection.controller != player {
            self.state.turn.target_selection = Some(selection);
            return;
        }
        if option.zone == ChoiceZone::Skip {
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
            ChoiceZone::Resolution => TargetZone::Resolution,
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
        let Some(target) = selection.candidates.iter().copied().find(|candidate| {
            candidate.player == target_player
                && candidate.zone == zone
                && candidate.index == index
                && candidate.instance_id == option.instance_id
                && candidate.card_id == option.card_id
        }) else {
            self.state.turn.target_selection = Some(selection);
            return;
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
                        let cost = spec.template.activation_cost_spec();
                        if !self.can_pay_ability_cost(player, slot as u8, card_inst, cost) {
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
                        self.scratch
                            .priority_actions
                            .push(ActionDesc::MainActivateAbility {
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

    pub(super) fn start_priority_choice(&mut self, player: u8) -> bool {
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
                ActionDesc::Pass => {
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id: 0,
                        instance_id: 0,
                        zone: ChoiceZone::PriorityPass,
                        index: None,
                        target_slot: None,
                    });
                }
                _ => {}
            }
        }
        let options = std::mem::take(&mut self.scratch.choice_options);
        self.start_choice(ChoiceReason::PriorityActionSelect, player, options, None)
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
            ChoiceZone::PriorityPass => {
                self.priority_pass(player);
                None
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
                let pending_cost =
                    self.queue_activated_ability_stack_item(player, slot, ability_index)?;
                let bit = slot as u32 * MAX_ABILITIES_PER_CARD as u32 + ability_index as u32;
                let mut new_holder = None;
                if let Some(priority) = &mut self.state.turn.priority {
                    priority.used_act_mask |= 1u32 << bit;
                    if !pending_cost {
                        priority.holder = 1 - player;
                        priority.passes = 0;
                        new_holder = Some(priority.holder);
                    }
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
            ActionDesc::Pass => {
                if self.curriculum.strict_priority_mode || !self.curriculum.priority_allow_pass {
                    self.collect_priority_actions(player);
                    if !self.scratch.priority_actions.is_empty() {
                        return Err(anyhow!(
                            "Explicit pass not allowed when priority actions exist"
                        ));
                    }
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
                    self.state.turn.phase_step = 0;
                }
            }
            TimingWindow::CounterWindow => {
                if let Some(ctx) = &mut self.state.turn.attack {
                    ctx.step = AttackStep::Damage;
                }
            }
            TimingWindow::ClimaxWindow => {
                self.state.turn.phase_step = 2;
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
            EffectKind::RevealDeckTop { .. } => 12,
            EffectKind::TriggerIcon { .. } => 13,
            EffectKind::MoveToWaitingRoom => 14,
            EffectKind::MoveToStock => 15,
            EffectKind::MoveToClock => 16,
            EffectKind::Heal => 17,
            EffectKind::RestTarget => 18,
            EffectKind::StandTarget => 19,
            EffectKind::StockCharge { .. } => 20,
            EffectKind::MillTop { .. } => 21,
            EffectKind::MoveStageSlot { .. } => 22,
            EffectKind::SwapStageSlots => 23,
            EffectKind::RandomDiscardFromHand { .. } => 24,
            EffectKind::RandomMill { .. } => 25,
            EffectKind::RevealZoneTop { .. } => 26,
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
            self.state.turn.pending_stack_groups.push_back(group);
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
        let Some(group) = self.state.turn.pending_stack_groups.pop_front() else {
            return;
        };
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
                                index: Some(target.index),
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
                        index: Some(target.index),
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
                    let card = self.state.players[p].hand.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].clock.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].level.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].stock.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].memory.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].climax.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].resolution.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].waiting_room.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].hand.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].clock.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].level.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].waiting_room.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].memory.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].climax.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].resolution.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].hand.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].waiting_room.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].resolution.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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
                    let card = self.state.players[p].clock.remove(idx);
                    if card.instance_id != target.instance_id {
                        continue;
                    }
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

    fn ability_cost_for_spec(&self, spec: &AbilitySpec) -> AbilityCost {
        spec.template.activation_cost_spec()
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

    fn can_pay_ability_cost(
        &self,
        player: u8,
        slot: u8,
        source: CardInstance,
        cost: AbilityCost,
    ) -> bool {
        let p = player as usize;
        if cost.rest_self {
            let slot_idx = slot as usize;
            if slot_idx >= self.state.players[p].stage.len() {
                return false;
            }
            let slot_state = &self.state.players[p].stage[slot_idx];
            if slot_state.card.map(|c| c.instance_id) != Some(source.instance_id) {
                return false;
            }
            if slot_state.status != StageStatus::Stand {
                return false;
            }
        }
        if cost.rest_other > 0 {
            let mut available = 0usize;
            for (idx, slot_state) in self.state.players[p].stage.iter().enumerate() {
                if idx == slot as usize {
                    continue;
                }
                if slot_state.card.is_some() && slot_state.status == StageStatus::Stand {
                    available += 1;
                }
            }
            if available < cost.rest_other as usize {
                return false;
            }
        }
        if cost.stock > 0
            && self.curriculum.enforce_cost_requirement
            && self.state.players[p].stock.len() < cost.stock as usize
        {
            return false;
        }
        let required_hand = cost.discard_from_hand as usize
            + cost.clock_from_hand as usize
            + cost.reveal_from_hand as usize;
        if required_hand > self.state.players[p].hand.len() {
            return false;
        }
        if cost.clock_from_deck_top > 0
            && self.state.players[p].deck.len() < cost.clock_from_deck_top as usize
        {
            return false;
        }
        true
    }

    fn pay_ability_cost_immediate(
        &mut self,
        player: u8,
        slot: u8,
        source: CardInstance,
        cost: &mut AbilityCost,
    ) -> Result<()> {
        let p = player as usize;
        if cost.rest_self {
            let slot_idx = slot as usize;
            if slot_idx >= self.state.players[p].stage.len() {
                return Err(anyhow!("Cost rest slot out of range"));
            }
            let slot_state = &mut self.state.players[p].stage[slot_idx];
            if slot_state.card.map(|c| c.instance_id) != Some(source.instance_id) {
                return Err(anyhow!("Cost rest target mismatch"));
            }
            if slot_state.status != StageStatus::Stand {
                return Err(anyhow!("Cost rest requires stand"));
            }
            slot_state.status = StageStatus::Rest;
            self.mark_slot_power_dirty(player, slot);
            cost.rest_self = false;
        }
        if cost.stock > 0 && self.curriculum.enforce_cost_requirement {
            self.pay_cost(player, cost.stock as usize)?;
            cost.stock = 0;
        } else {
            cost.stock = 0;
        }
        Ok(())
    }

    fn next_cost_step(cost: &AbilityCost) -> Option<CostStepKind> {
        if cost.rest_other > 0 {
            Some(CostStepKind::RestOther)
        } else if cost.discard_from_hand > 0 {
            Some(CostStepKind::DiscardFromHand)
        } else if cost.clock_from_hand > 0 {
            Some(CostStepKind::ClockFromHand)
        } else if cost.clock_from_deck_top > 0 {
            Some(CostStepKind::ClockFromDeckTop)
        } else if cost.reveal_from_hand > 0 {
            Some(CostStepKind::RevealFromHand)
        } else {
            None
        }
    }

    fn start_cost_choice(&mut self) {
        let Some(cost_state) = self.state.turn.pending_cost.as_mut() else {
            return;
        };
        let Some(step) = Self::next_cost_step(&cost_state.remaining) else {
            let cost_state = self.state.turn.pending_cost.take();
            if let Some(cost_state) = cost_state {
                self.finish_cost_payment(cost_state);
            }
            return;
        };
        cost_state.current_step = Some(step);
        let player = cost_state.controller;
        match step {
            CostStepKind::RestOther => {
                self.scratch.choice_options.clear();
                let source_slot = cost_state.source_slot;
                let p = player as usize;
                for (idx, slot_state) in self.state.players[p].stage.iter().enumerate() {
                    if Some(idx as u8) == source_slot {
                        continue;
                    }
                    let Some(card_inst) = slot_state.card else {
                        continue;
                    };
                    if slot_state.status != StageStatus::Stand {
                        continue;
                    }
                    if idx > u8::MAX as usize {
                        break;
                    }
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                        zone: ChoiceZone::Stage,
                        index: Some(idx as u8),
                        target_slot: None,
                    });
                }
                let options = std::mem::take(&mut self.scratch.choice_options);
                let _ = self.start_choice(ChoiceReason::CostPayment, player, options, None);
            }
            CostStepKind::ClockFromDeckTop => {
                if let Some(card) = self.draw_from_deck(player) {
                    let card_id = card.id;
                    self.move_card_between_zones(player, card, Zone::Deck, Zone::Clock, None, None);
                    self.log_event(Event::Clock {
                        player,
                        card: Some(card_id),
                    });
                }
                if let Some(cost_state) = self.state.turn.pending_cost.as_mut() {
                    cost_state.remaining.clock_from_deck_top =
                        cost_state.remaining.clock_from_deck_top.saturating_sub(1);
                    cost_state.current_step = None;
                }
                self.start_cost_choice();
            }
            _ => {
                self.scratch.choice_options.clear();
                for (idx, card_inst) in self.state.players[player as usize].hand.iter().enumerate()
                {
                    if idx > u8::MAX as usize {
                        break;
                    }
                    self.scratch.choice_options.push(ChoiceOptionRef {
                        card_id: card_inst.id,
                        instance_id: card_inst.instance_id,
                        zone: ChoiceZone::Hand,
                        index: Some(idx as u8),
                        target_slot: None,
                    });
                }
                let options = std::mem::take(&mut self.scratch.choice_options);
                let _ = self.start_choice(ChoiceReason::CostPayment, player, options, None);
            }
        }
    }

    fn apply_cost_payment_choice(&mut self, player: u8, option: ChoiceOptionRef) {
        let Some(mut cost_state) = self.state.turn.pending_cost.take() else {
            return;
        };
        if cost_state.controller != player {
            self.state.turn.pending_cost = Some(cost_state);
            return;
        }
        let Some(step) = cost_state.current_step else {
            self.state.turn.pending_cost = Some(cost_state);
            return;
        };
        let p = player as usize;
        match step {
            CostStepKind::RestOther => {
                if option.zone != ChoiceZone::Stage {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                if idx >= self.state.players[p].stage.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(card_inst) = self.state.players[p].stage[idx].card else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                if self.state.players[p].stage[idx].status != StageStatus::Stand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                self.state.players[p].stage[idx].status = StageStatus::Rest;
                self.mark_slot_power_dirty(player, index);
                self.mark_continuous_modifiers_dirty();
                cost_state.remaining.rest_other = cost_state.remaining.rest_other.saturating_sub(1);
            }
            CostStepKind::DiscardFromHand => {
                if option.zone != ChoiceZone::Hand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                if idx >= self.state.players[p].hand.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card_inst = self.state.players[p].hand[idx];
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card = self.state.players[p].hand.remove(idx);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Hand,
                    Zone::WaitingRoom,
                    Some(index),
                    None,
                );
                cost_state.remaining.discard_from_hand =
                    cost_state.remaining.discard_from_hand.saturating_sub(1);
            }
            CostStepKind::ClockFromHand => {
                if option.zone != ChoiceZone::Hand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                if idx >= self.state.players[p].hand.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card_inst = self.state.players[p].hand[idx];
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card = self.state.players[p].hand.remove(idx);
                self.move_card_between_zones(
                    player,
                    card,
                    Zone::Hand,
                    Zone::Clock,
                    Some(index),
                    None,
                );
                cost_state.remaining.clock_from_hand =
                    cost_state.remaining.clock_from_hand.saturating_sub(1);
            }
            CostStepKind::RevealFromHand => {
                if option.zone != ChoiceZone::Hand {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let Some(index) = option.index else {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                };
                let idx = index as usize;
                if idx >= self.state.players[p].hand.len() {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card_inst = self.state.players[p].hand[idx];
                if card_inst.instance_id != option.instance_id {
                    self.state.turn.pending_cost = Some(cost_state);
                    return;
                }
                let card = self.state.players[p].hand[idx];
                self.reveal_card(
                    player,
                    &card,
                    RevealReason::AbilityEffect,
                    RevealAudience::Public,
                );
                cost_state.remaining.reveal_from_hand =
                    cost_state.remaining.reveal_from_hand.saturating_sub(1);
            }
            CostStepKind::ClockFromDeckTop => {
                self.state.turn.pending_cost = Some(cost_state);
                return;
            }
        }
        cost_state.current_step = None;
        self.state.turn.pending_cost = Some(cost_state);
        self.start_cost_choice();
    }

    fn finish_cost_payment(&mut self, cost_state: CostPaymentState) {
        self.state.turn.cost_payment_depth = self.state.turn.cost_payment_depth.saturating_sub(1);
        let effects: Vec<_> = self
            .db
            .compiled_effects_for_ability(cost_state.source_id, cost_state.ability_index as usize)
            .to_vec();
        let source_ref = cost_state.source_slot.and_then(|slot| {
            let p = cost_state.controller as usize;
            let slot_state = self.state.players[p].stage.get(slot as usize)?;
            let card_inst = slot_state.card?;
            if card_inst.instance_id != cost_state.source_instance_id {
                return None;
            }
            Some(TargetRef {
                player: cost_state.controller,
                zone: TargetZone::Stage,
                index: slot,
                card_id: card_inst.id,
                instance_id: card_inst.instance_id,
            })
        });
        for effect in effects {
            self.enqueue_effect_spec_with_source(
                cost_state.controller,
                cost_state.source_id,
                effect.clone(),
                source_ref,
            );
        }
        let mut grant_priority = None;
        if let Some(priority) = &mut self.state.turn.priority {
            if priority.holder == cost_state.controller {
                priority.holder = 1 - cost_state.controller;
                priority.passes = 0;
                grant_priority = Some((priority.window, priority.holder));
            }
        }
        if let Some((window, player)) = grant_priority {
            self.log_event(Event::PriorityGranted { window, player });
        }
    }

    pub(super) fn queue_activated_ability_stack_item(
        &mut self,
        player: u8,
        slot: u8,
        ability_index: u8,
    ) -> Result<bool> {
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
        let spec = db
            .iter_card_abilities_in_canonical_order(card_id)
            .get(idx)
            .ok_or_else(|| anyhow!("Ability index out of range"))?;
        let mut cost = self.ability_cost_for_spec(spec);
        if !self.can_pay_ability_cost(player, slot, card_inst, cost) {
            return Err(anyhow!("Activated ability cost not payable"));
        }
        self.pay_ability_cost_immediate(player, slot, card_inst, &mut cost)?;
        let effects = db.compiled_effects_for_ability(card_id, idx);
        if effects.is_empty() {
            return Err(anyhow!("Activated ability has no effects"));
        }
        if Self::next_cost_step(&cost).is_some() {
            self.state.turn.cost_payment_depth =
                self.state.turn.cost_payment_depth.saturating_add(1);
            self.state.turn.pending_cost = Some(CostPaymentState {
                controller: player,
                source_id: card_id,
                source_instance_id: card_inst.instance_id,
                source_slot: Some(slot),
                ability_index,
                remaining: cost,
                current_step: None,
            });
            self.start_cost_choice();
            return Ok(true);
        }
        let source_ref = Some(TargetRef {
            player,
            zone: TargetZone::Stage,
            index: slot,
            card_id,
            instance_id: card_inst.instance_id,
        });
        for effect in effects {
            self.enqueue_effect_spec_with_source(player, card_id, effect.clone(), source_ref);
        }
        Ok(false)
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
        self.reveal_card(
            player,
            &card_inst,
            RevealReason::Play,
            RevealAudience::Public,
        );
        self.move_card_between_zones(
            player,
            card_inst,
            Zone::Hand,
            Zone::Resolution,
            Some(hand_index),
            None,
        );
        self.state
            .turn
            .pending_resolution_cleanup
            .push((player, card_inst.instance_id));
        if let Some(ctx) = &mut self.state.turn.attack {
            ctx.counter_played = true;
        }
        if power != 0 {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, 0),
                kind: EffectKind::CounterBackup { power },
                target: None,
                optional: false,
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
                    optional: false,
                };
                self.enqueue_effect_spec(player, card_id, spec);
            }
        }
        if damage_cancel {
            let spec = EffectSpec {
                id: EffectId::new(EffectSourceKind::Counter, card_id, 0, 10),
                kind: EffectKind::CounterDamageCancel,
                target: None,
                optional: false,
            };
            self.enqueue_effect_spec(player, card_id, spec);
        }
        Ok(())
    }
}
