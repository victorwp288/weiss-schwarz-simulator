use super::*;

impl GameEnv {
    pub(in crate::env) fn trigger_effect_id(
        &self,
        source_card: CardId,
        effect_index: u8,
    ) -> EffectId {
        EffectId::new(EffectSourceKind::Trigger, source_card, 0, effect_index)
    }

    pub(in crate::env) fn compile_trigger_icon_effects(
        &self,
        icon: TriggerIcon,
        ctx: TriggerCompileContext,
    ) -> Vec<EffectSpec> {
        match icon {
            TriggerIcon::Soul => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SOUL),
                kind: EffectKind::ModifyPendingAttackDamage { delta: 1 },
                target: None,
                optional: false,
            }],
            TriggerIcon::Draw => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_DRAW),
                kind: EffectKind::Draw { count: 1 },
                target: None,
                optional: false,
            }],
            TriggerIcon::Shot => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_SHOT),
                kind: EffectKind::EnableShotDamage { amount: 1 },
                target: None,
                optional: false,
            }],
            TriggerIcon::Choice => Vec::new(),
            TriggerIcon::Pool => vec![
                EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_POOL_MOVE),
                    kind: EffectKind::MoveTriggerCardToStock,
                    target: None,
                    optional: false,
                },
                EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_POOL_STOCK),
                    kind: EffectKind::StockCharge { count: 1 },
                    target: None,
                    optional: false,
                },
            ],
            TriggerIcon::Gate => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_GATE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::WaitingRoom,
                    side: TargetSide::SelfSide,
                    slot_filter: TargetSlotFilter::Any,
                    card_type: Some(CardType::Climax),
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
            }],
            TriggerIcon::Bounce => vec![EffectSpec {
                id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_BOUNCE),
                kind: EffectKind::MoveToHand,
                target: Some(TargetSpec {
                    zone: TargetZone::Stage,
                    side: TargetSide::Opponent,
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
            }],
            TriggerIcon::Standby => {
                let Some(slot) = ctx.standby_slot else {
                    return Vec::new();
                };
                vec![EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_STANDBY),
                    kind: EffectKind::Standby { target_slot: slot },
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
                    optional: false,
                }]
            }
            TriggerIcon::Treasure => {
                let Some(take_stock) = ctx.treasure_take_stock else {
                    return Vec::new();
                };
                let mut effects = Vec::new();
                if take_stock {
                    // Stack is LIFO; enqueue move first so stock resolves before hand move.
                    effects.push(EffectSpec {
                        id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_MOVE),
                        kind: EffectKind::MoveTriggerCardToHand,
                        target: None,
                        optional: false,
                    });
                    effects.push(EffectSpec {
                        id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_STOCK),
                        kind: EffectKind::TreasureStock { take_stock },
                        target: None,
                        optional: false,
                    });
                    return effects;
                }
                effects.push(EffectSpec {
                    id: self.trigger_effect_id(ctx.source_card, TRIGGER_EFFECT_TREASURE_MOVE),
                    kind: EffectKind::MoveTriggerCardToHand,
                    target: None,
                    optional: false,
                });
                effects
            }
        }
    }
}
