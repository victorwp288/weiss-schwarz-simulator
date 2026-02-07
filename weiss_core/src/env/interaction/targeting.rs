use super::super::GameEnv;
use crate::config::CurriculumConfig;
use crate::db::{CardDb, CardId};
use crate::encode::MAX_STAGE;
use crate::events::{RevealAudience, RevealReason};
use crate::state::{
    CardInstance, ChoiceOptionRef, ChoiceReason, ChoiceZone, GameState, PendingTargetEffect,
    StackItem, TargetRef, TargetSelectionState, TargetSide, TargetSlotFilter, TargetSpec,
    TargetZone,
};

impl GameEnv {
    pub(in crate::env) fn start_target_selection(
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

    pub(in crate::env) fn enumerate_target_candidates_into(
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

    pub(in crate::env) fn present_target_choice(&mut self) {
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
                index: Some(target.index as u16),
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

    pub(in crate::env) fn apply_target_choice(&mut self, player: u8, option: ChoiceOptionRef) {
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
        let Ok(index) = u8::try_from(index) else {
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
            TargetSide::Opponent => {
                debug_assert!(
                    selection.controller <= 1,
                    "invalid target selection controller {}",
                    selection.controller
                );
                match selection.controller {
                    0 => 1,
                    1 => 0,
                    _ => 0,
                }
            }
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
}
