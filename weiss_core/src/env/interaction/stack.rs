use super::super::GameEnv;
use crate::effects::EffectKind;
use crate::events::Event;
use crate::state::{ChoiceOptionRef, ChoiceReason, ChoiceZone, StackItem, StackOrderState};

impl GameEnv {
    pub(in crate::env) fn allocate_stack_group_id(&mut self) -> u32 {
        let group_id = self.state.turn.next_stack_group_id;
        self.state.turn.next_stack_group_id = self.state.turn.next_stack_group_id.wrapping_add(1);
        group_id
    }

    pub(in crate::env) fn stack_effect_key(effect: &EffectKind) -> u8 {
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

    pub(in crate::env) fn enqueue_stack_items(&mut self, items: Vec<StackItem>) {
        if items.is_empty() {
            return;
        }
        let active = self.state.turn.active_player;
        let mut per_player: [Vec<StackItem>; 2] = [Vec::new(), Vec::new()];
        for item in items {
            if item.controller > 1 {
                eprintln!(
                    "Stack item has invalid controller {} (source {})",
                    item.controller, item.source_id
                );
                continue;
            }
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

    pub(in crate::env) fn process_next_stack_group(&mut self) {
        if self.state.turn.stack_order.is_some() {
            return;
        }
        while let Some(group) = self.state.turn.pending_stack_groups.pop_front() {
            if group.items.len() == 1 {
                let item = group.items.into_iter().next().expect("group item");
                self.push_stack_item(item);
                continue;
            }
            self.log_event(Event::StackGroupPresented {
                group_id: group.group_id,
                controller: group.controller,
                items: group.items.clone(),
            });
            self.state.turn.stack_order = Some(group);
            self.present_stack_order_choice();
            return;
        }
    }

    pub(in crate::env) fn present_stack_order_choice(&mut self) {
        let Some(order) = &self.state.turn.stack_order else {
            return;
        };
        assert!(
            order.items.len() <= u16::MAX as usize + 1,
            "stack order too large"
        );
        self.scratch.choice_options.clear();
        for (idx, item) in order.items.iter().enumerate() {
            self.scratch.choice_options.push(ChoiceOptionRef {
                card_id: item.source_id,
                instance_id: 0,
                zone: ChoiceZone::Stack,
                index: Some(idx as u16),
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

    pub(in crate::env) fn apply_stack_order_choice(&mut self, player: u8, option: ChoiceOptionRef) {
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

    pub(in crate::env) fn push_stack_item(&mut self, item: StackItem) {
        self.state.turn.stack.push(item.clone());
        self.log_event(Event::StackPushed { item });
    }

    pub(in crate::env) fn resolve_stack_item(&mut self, item: &StackItem) {
        self.resolve_effect_payload(item.controller, item.source_id, &item.payload);
    }
}
