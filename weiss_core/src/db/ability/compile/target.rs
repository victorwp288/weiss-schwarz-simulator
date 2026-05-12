use super::*;

/// Convert a DB target template into an executable runtime target spec.
pub(super) fn target_spec_from_template(
    template: TargetTemplate,
    count: u8,
) -> crate::state::TargetSpec {
    let zone = match template {
        TargetTemplate::OppFrontRow
        | TargetTemplate::OppBackRow
        | TargetTemplate::OppStage
        | TargetTemplate::OppStageSlot { .. }
        | TargetTemplate::SelfFrontRow
        | TargetTemplate::SelfBackRow
        | TargetTemplate::SelfStage
        | TargetTemplate::SelfStageSlot { .. }
        | TargetTemplate::This => crate::state::TargetZone::Stage,
        TargetTemplate::SelfWaitingRoom => crate::state::TargetZone::WaitingRoom,
        TargetTemplate::SelfHand => crate::state::TargetZone::Hand,
        TargetTemplate::SelfDeckTop => crate::state::TargetZone::DeckTop,
        TargetTemplate::SelfClock => crate::state::TargetZone::Clock,
        TargetTemplate::SelfLevel => crate::state::TargetZone::Level,
        TargetTemplate::SelfStock => crate::state::TargetZone::Stock,
        TargetTemplate::SelfMemory => crate::state::TargetZone::Memory,
    };
    let card_type = match zone {
        crate::state::TargetZone::Stage => Some(CardType::Character),
        _ => None,
    };
    crate::state::TargetSpec {
        zone,
        side: match template {
            TargetTemplate::OppFrontRow
            | TargetTemplate::OppBackRow
            | TargetTemplate::OppStage
            | TargetTemplate::OppStageSlot { .. } => crate::state::TargetSide::Opponent,
            _ => crate::state::TargetSide::SelfSide,
        },
        slot_filter: match template {
            TargetTemplate::OppFrontRow | TargetTemplate::SelfFrontRow => {
                crate::state::TargetSlotFilter::FrontRow
            }
            TargetTemplate::OppBackRow | TargetTemplate::SelfBackRow => {
                crate::state::TargetSlotFilter::BackRow
            }
            TargetTemplate::OppStageSlot { slot } | TargetTemplate::SelfStageSlot { slot } => {
                crate::state::TargetSlotFilter::SpecificSlot(slot)
            }
            _ => crate::state::TargetSlotFilter::Any,
        },
        card_type,
        card_trait: None,
        level_max: None,
        cost_max: None,
        card_ids: Vec::new(),
        count,
        limit: None,
        source_only: matches!(template, TargetTemplate::This),
        reveal_to_controller: false,
    }
}

pub(super) fn template_effect_spec(
    source_kind: crate::effects::EffectSourceKind,
    card_id: CardId,
    ability_index: u8,
    kind: crate::effects::EffectKind,
    target: Option<crate::state::TargetSpec>,
    optional: bool,
) -> crate::effects::EffectSpec {
    crate::effects::EffectSpec {
        id: crate::effects::EffectId::new(source_kind, card_id, ability_index, 0),
        kind,
        target,
        optional,
    }
}
