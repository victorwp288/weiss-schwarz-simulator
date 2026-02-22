use super::prelude::*;

pub(super) fn brainstorm(
    env: &mut GameEnv,
    controller: u8,
    source_id: CardId,
    payload: &EffectPayload,
    reveal_count: u8,
    per_climax: u8,
    mode: crate::db::BrainstormMode,
) {
    let mut climax_hits: u32 = 0;
    for _ in 0..reveal_count {
        let Some(card) = env.draw_from_deck(controller) else {
            break;
        };
        let is_climax = env
            .db
            .get(card.id)
            .map(|c| c.card_type == CardType::Climax)
            .unwrap_or(false);
        env.reveal_card(
            controller,
            &card,
            RevealReason::AbilityEffect,
            RevealAudience::Public,
        );
        env.move_card_between_zones(controller, card, Zone::Deck, Zone::WaitingRoom, None, None);
        if is_climax {
            climax_hits = climax_hits.saturating_add(1);
        }
    }
    let total = climax_hits.saturating_mul(per_climax as u32);
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
                env.enqueue_effect_spec(controller, source_id, spec.clone());
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
                env.enqueue_effect_spec(controller, source_id, spec.clone());
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
                env.enqueue_effect_spec(controller, source_id, spec.clone());
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
                env.enqueue_effect_spec(controller, source_id, discard_spec.clone());
                env.enqueue_effect_spec(controller, source_id, look_spec.clone());
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
                env.enqueue_effect_spec(controller, source_id, discard_spec.clone());
                env.enqueue_effect_spec(controller, source_id, salvage_spec.clone());
            }
        }
    }
}

pub(super) fn brainstorm_draw_choice(env: &mut GameEnv, controller: u8) {
    env.scratch.choice_options.clear();
    env.scratch.choice_options.push(ChoiceOptionRef {
        card_id: 0,
        instance_id: 0,
        zone: ChoiceZone::DeckTop,
        index: Some(0),
        target_slot: None,
    });
    env.scratch.choice_options.push(ChoiceOptionRef {
        card_id: 0,
        instance_id: 0,
        zone: ChoiceZone::Skip,
        index: Some(1),
        target_slot: None,
    });
    let options = std::mem::take(&mut env.scratch.choice_options);
    let _ = env.start_choice(
        ChoiceReason::BrainstormDrawSelect,
        controller,
        options,
        None,
    );
}
