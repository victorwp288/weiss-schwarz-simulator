use super::target::{target_spec_from_template, template_effect_spec};
use super::*;

fn compile_template_continuous_effect(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Option<crate::effects::EffectSpec> {
    match template {
        AbilityTemplate::ContinuousPower { amount } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Continuous,
            card_id,
            ability_index,
            crate::effects::EffectKind::AddModifier {
                kind: crate::state::ModifierKind::Power,
                magnitude: *amount,
                duration: crate::state::ModifierDuration::WhileOnStage,
            },
            Some(target_spec_from_template(TargetTemplate::This, 1)),
            false,
        )),
        AbilityTemplate::ContinuousCannotAttack => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Continuous,
            card_id,
            ability_index,
            crate::effects::EffectKind::AddModifier {
                kind: crate::state::ModifierKind::CannotAttack,
                magnitude: 1,
                duration: crate::state::ModifierDuration::WhileOnStage,
            },
            Some(target_spec_from_template(TargetTemplate::This, 1)),
            false,
        )),
        AbilityTemplate::ContinuousAttackCost { cost } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Continuous,
            card_id,
            ability_index,
            crate::effects::EffectKind::AddModifier {
                kind: crate::state::ModifierKind::AttackCost,
                magnitude: *cost as i32,
                duration: crate::state::ModifierDuration::WhileOnStage,
            },
            Some(target_spec_from_template(TargetTemplate::This, 1)),
            false,
        )),
        _ => None,
    }
}

fn compile_template_auto_effect(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Option<crate::effects::EffectSpec> {
    match template {
        AbilityTemplate::AutoOnPlayDraw { count }
        | AbilityTemplate::AutoEndPhaseDraw { count }
        | AbilityTemplate::AutoOnReverseDraw { count } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Auto,
            card_id,
            ability_index,
            crate::effects::EffectKind::Draw { count: *count },
            None,
            false,
        )),
        AbilityTemplate::AutoOnPlaySalvage {
            count,
            optional,
            card_type,
        }
        | AbilityTemplate::AutoOnReverseSalvage {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_type = *card_type;
            Some(template_effect_spec(
                crate::effects::EffectSourceKind::Auto,
                card_id,
                ability_index,
                crate::effects::EffectKind::MoveToHand,
                Some(spec),
                *optional,
            ))
        }
        AbilityTemplate::AutoOnPlaySearchDeckTop {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfDeckTop, 1);
            spec.card_type = *card_type;
            spec.limit = Some(*count);
            spec.reveal_to_controller = true;
            Some(template_effect_spec(
                crate::effects::EffectSourceKind::Auto,
                card_id,
                ability_index,
                crate::effects::EffectKind::MoveToHand,
                Some(spec),
                *optional,
            ))
        }
        AbilityTemplate::AutoOnPlayRevealDeckTop { count } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Auto,
            card_id,
            ability_index,
            crate::effects::EffectKind::RevealDeckTop {
                count: *count,
                audience: crate::events::RevealAudience::ControllerOnly,
            },
            None,
            false,
        )),
        AbilityTemplate::AutoOnPlayStockCharge { count } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Auto,
            card_id,
            ability_index,
            crate::effects::EffectKind::StockCharge { count: *count },
            None,
            false,
        )),
        AbilityTemplate::AutoOnPlayMillTop { count } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Auto,
            card_id,
            ability_index,
            crate::effects::EffectKind::MillTop {
                target: crate::state::TargetSide::SelfSide,
                count: *count,
            },
            None,
            false,
        )),
        AbilityTemplate::AutoOnPlayHeal { count } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Auto,
            card_id,
            ability_index,
            crate::effects::EffectKind::Heal,
            Some(target_spec_from_template(TargetTemplate::SelfClock, *count)),
            false,
        )),
        AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } => {
            Some(template_effect_spec(
                crate::effects::EffectSourceKind::Auto,
                card_id,
                ability_index,
                crate::effects::EffectKind::Damage {
                    amount: *amount as i32,
                    cancelable: *cancelable,
                    damage_type: crate::state::DamageType::Effect,
                },
                None,
                false,
            ))
        }
        AbilityTemplate::Bond {
            count, target_ids, ..
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_ids = target_ids.clone();
            Some(template_effect_spec(
                crate::effects::EffectSourceKind::Auto,
                card_id,
                ability_index,
                crate::effects::EffectKind::MoveToHand,
                Some(spec),
                false,
            ))
        }
        _ => None,
    }
}

fn compile_template_event_effect(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Option<crate::effects::EffectSpec> {
    match template {
        AbilityTemplate::EventDealDamage { amount, cancelable } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::EventPlay,
            card_id,
            ability_index,
            crate::effects::EffectKind::Damage {
                amount: *amount as i32,
                cancelable: *cancelable,
                damage_type: crate::state::DamageType::Effect,
            },
            None,
            false,
        )),
        _ => None,
    }
}

fn compile_template_activated_effect(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Option<crate::effects::EffectSpec> {
    match template {
        AbilityTemplate::ActivatedTargetedPower {
            amount,
            count,
            target,
        } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Activated,
            card_id,
            ability_index,
            crate::effects::EffectKind::AddModifier {
                kind: crate::state::ModifierKind::Power,
                magnitude: *amount,
                duration: crate::state::ModifierDuration::UntilEndOfTurn,
            },
            Some(target_spec_from_template(*target, *count)),
            false,
        )),
        AbilityTemplate::ActivatedPaidTargetedPower {
            amount,
            count,
            target,
            ..
        } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Activated,
            card_id,
            ability_index,
            crate::effects::EffectKind::AddModifier {
                kind: crate::state::ModifierKind::Power,
                magnitude: *amount,
                duration: crate::state::ModifierDuration::WhileOnStage,
            },
            Some(target_spec_from_template(*target, *count)),
            false,
        )),
        AbilityTemplate::ActivatedTargetedMoveToHand { count, target }
        | AbilityTemplate::ActivatedPaidTargetedMoveToHand { count, target, .. } => {
            Some(template_effect_spec(
                crate::effects::EffectSourceKind::Activated,
                card_id,
                ability_index,
                crate::effects::EffectKind::MoveToHand,
                Some(target_spec_from_template(*target, *count)),
                false,
            ))
        }
        AbilityTemplate::ActivatedChangeController { count, target }
        | AbilityTemplate::ActivatedPaidChangeController { count, target, .. } => {
            Some(template_effect_spec(
                crate::effects::EffectSourceKind::Activated,
                card_id,
                ability_index,
                crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                Some(target_spec_from_template(*target, *count)),
                false,
            ))
        }
        AbilityTemplate::CounterBackup { power } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Activated,
            card_id,
            ability_index,
            crate::effects::EffectKind::CounterBackup { power: *power },
            None,
            false,
        )),
        AbilityTemplate::CounterDamageReduce { amount } => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Activated,
            card_id,
            ability_index,
            crate::effects::EffectKind::CounterDamageReduce { amount: *amount },
            None,
            false,
        )),
        AbilityTemplate::CounterDamageCancel => Some(template_effect_spec(
            crate::effects::EffectSourceKind::Activated,
            card_id,
            ability_index,
            crate::effects::EffectKind::CounterDamageCancel,
            None,
            false,
        )),
        _ => None,
    }
}

fn is_noop_template(template: &AbilityTemplate) -> bool {
    matches!(
        template,
        AbilityTemplate::AbilityDef(_)
            | AbilityTemplate::EncoreVariant { .. }
            | AbilityTemplate::Vanilla
            | AbilityTemplate::Unsupported { .. }
            | AbilityTemplate::ActivatedPlaceholder
    )
}

/// Compile template-only abilities into executable effect specs.
pub(crate) fn compile_effects_from_template(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Vec<crate::effects::EffectSpec> {
    // Invariants:
    // - Template expansion behavior is validated in
    //   weiss_core/tests/ability_template_expansion_tests.rs.
    // - Template compilation emits at most one effect and keeps deterministic output.
    // - Template-generated EffectId indices stay fixed at 0.
    if let Some(spec) = compile_template_continuous_effect(card_id, ability_index, template)
        .or_else(|| compile_template_auto_effect(card_id, ability_index, template))
        .or_else(|| compile_template_event_effect(card_id, ability_index, template))
        .or_else(|| compile_template_activated_effect(card_id, ability_index, template))
    {
        return vec![spec];
    }

    debug_assert!(
        is_noop_template(template),
        "compile_effects_from_template missing variant coverage"
    );
    Vec::new()
}
