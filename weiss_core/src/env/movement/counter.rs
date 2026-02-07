use super::super::GameEnv;
use crate::db::{AbilityTemplate, CardStatic};

impl GameEnv {
    pub(in crate::env) fn is_counter_card(&self, card: &CardStatic) -> bool {
        if !card.counter_timing {
            return false;
        }
        self.db
            .iter_card_abilities_in_canonical_order(card.id)
            .iter()
            .any(|spec| {
                matches!(
                    spec.template,
                    AbilityTemplate::CounterBackup { .. }
                        | AbilityTemplate::CounterDamageReduce { .. }
                        | AbilityTemplate::CounterDamageCancel
                )
            })
    }

    pub(in crate::env) fn counter_power(&self, card: &CardStatic) -> i32 {
        for spec in self.db.iter_card_abilities_in_canonical_order(card.id) {
            if let AbilityTemplate::CounterBackup { power } = spec.template {
                return power;
            }
        }
        0
    }

    pub(in crate::env) fn counter_damage_reductions(&self, card: &CardStatic) -> Vec<i32> {
        let mut out = Vec::new();
        for spec in self.db.iter_card_abilities_in_canonical_order(card.id) {
            if let AbilityTemplate::CounterDamageReduce { amount } = spec.template {
                out.push(amount as i32);
            }
        }
        out
    }

    pub(in crate::env) fn counter_damage_cancel(&self, card: &CardStatic) -> bool {
        self.db
            .iter_card_abilities_in_canonical_order(card.id)
            .iter()
            .any(|spec| matches!(spec.template, AbilityTemplate::CounterDamageCancel))
    }
}
