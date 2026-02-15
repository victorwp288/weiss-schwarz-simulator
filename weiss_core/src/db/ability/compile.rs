pub(crate) fn target_spec_from_template(
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

pub(crate) fn compile_effects_from_template(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Vec<crate::effects::EffectSpec> {
    let mut out = Vec::new();
    match template {
        AbilityTemplate::ContinuousPower { amount } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Continuous,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(TargetTemplate::This, 1)),
                optional: false,
            });
        }
        AbilityTemplate::ContinuousCannotAttack => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Continuous,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::CannotAttack,
                    magnitude: 1,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(TargetTemplate::This, 1)),
                optional: false,
            });
        }
        AbilityTemplate::ContinuousAttackCost { cost } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Continuous,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::AttackCost,
                    magnitude: *cost as i32,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(TargetTemplate::This, 1)),
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayDraw { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Draw { count: *count },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlaySalvage {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_type = *card_type;
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: *optional,
            });
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
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: *optional,
            });
        }
        AbilityTemplate::AutoOnPlayRevealDeckTop { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::RevealDeckTop {
                    count: *count,
                    audience: crate::events::RevealAudience::ControllerOnly,
                },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayStockCharge { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::StockCharge { count: *count },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayMillTop { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MillTop {
                    target: crate::state::TargetSide::SelfSide,
                    count: *count,
                },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayHeal { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Heal,
                target: Some(target_spec_from_template(TargetTemplate::SelfClock, *count)),
                optional: false,
            });
        }
        AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Damage {
                    amount: *amount as i32,
                    cancelable: *cancelable,
                    damage_type: crate::state::DamageType::Effect,
                },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoEndPhaseDraw { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Draw { count: *count },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnReverseDraw { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Draw { count: *count },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnReverseSalvage {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_type = *card_type;
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: *optional,
            });
        }
        AbilityTemplate::EventDealDamage { amount, cancelable } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::EventPlay,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Damage {
                    amount: *amount as i32,
                    cancelable: *cancelable,
                    damage_type: crate::state::DamageType::Effect,
                },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::ActivatedTargetedPower {
            amount,
            count,
            target,
        } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: crate::state::ModifierDuration::UntilEndOfTurn,
                },
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
            });
        }
        AbilityTemplate::ActivatedPaidTargetedPower {
            amount,
            count,
            target,
            ..
        } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
            });
        }
        AbilityTemplate::ActivatedTargetedMoveToHand { count, target } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
            });
        }
        AbilityTemplate::ActivatedPaidTargetedMoveToHand { count, target, .. } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
            });
        }
        AbilityTemplate::ActivatedChangeController { count, target } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
            });
        }
        AbilityTemplate::ActivatedPaidChangeController { count, target, .. } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
            });
        }
        AbilityTemplate::CounterBackup { power } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::CounterBackup { power: *power },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::CounterDamageReduce { amount } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::CounterDamageReduce { amount: *amount },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::CounterDamageCancel => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::CounterDamageCancel,
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::Bond {
            count, target_ids, ..
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_ids = target_ids.clone();
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: false,
            });
        }
        AbilityTemplate::AbilityDef(_)
        | AbilityTemplate::EncoreVariant { .. }
        | AbilityTemplate::Vanilla
        | AbilityTemplate::Unsupported { .. } => {}
        AbilityTemplate::ActivatedPlaceholder => {}
    }
    out
}

pub(crate) fn compile_effects_from_def(
    card_id: CardId,
    ability_index: u8,
    def: &AbilityDef,
) -> Vec<crate::effects::EffectSpec> {
    let mut out = Vec::new();
    let max_len = def.effects.len().max(def.targets.len());
    let source_kind = match def.kind {
        AbilityKind::Activated => crate::effects::EffectSourceKind::Activated,
        AbilityKind::Auto => crate::effects::EffectSourceKind::Auto,
        AbilityKind::Continuous => crate::effects::EffectSourceKind::Continuous,
    };
    for idx in 0..max_len {
        let Some(effect) = def.effects.get(idx) else {
            continue;
        };
        // Ability defs can declare fewer targets than effects; when that happens we intentionally
        // reuse the first target spec to preserve existing card-db semantics.
        let target = def
            .targets
            .get(idx)
            .copied()
            .or_else(|| def.targets.first().copied());
        let mut target_spec = target.map(|t| {
            let mut spec = target_spec_from_template(t, 1);
            spec.card_type = def.target_card_type;
            spec.card_trait = def.target_trait;
            spec.level_max = def.target_level_max;
            spec.cost_max = def.target_cost_max;
            spec.card_ids = def.target_card_ids.clone();
            spec.limit = def.target_limit;
            spec
        });
        let effect_index = match u8::try_from(idx) {
            Ok(val) => val,
            Err(_) => {
                debug_assert!(false, "Effect index out of range for card {}", card_id);
                eprintln!(
                    "warning: skipping effect index {} for card {} (exceeds u8::MAX)",
                    idx, card_id
                );
                continue;
            }
        };
        out.push(crate::effects::EffectSpec {
            id: crate::effects::EffectId::new(source_kind, card_id, ability_index, effect_index),
            kind: match effect {
                EffectTemplate::Draw { count } => {
                    crate::effects::EffectKind::Draw { count: *count }
                }
                EffectTemplate::DealDamage { amount, cancelable } => {
                    crate::effects::EffectKind::Damage {
                        amount: *amount as i32,
                        cancelable: *cancelable,
                        damage_type: crate::state::DamageType::Effect,
                    }
                }
                EffectTemplate::AddPower {
                    amount,
                    duration_turn,
                } => crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddPowerIfTargetLevelAtLeast {
                    amount,
                    min_level,
                    duration_turn,
                } => crate::effects::EffectKind::AddPowerIfTargetLevelAtLeast {
                    amount: *amount,
                    min_level: *min_level,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddPowerByLevel {
                    multiplier,
                    duration_turn,
                } => crate::effects::EffectKind::AddPowerByTargetLevel {
                    multiplier: *multiplier,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddPowerIfBattleOpponentLevelAtLeast {
                    amount,
                    min_level,
                    duration_turn,
                } => crate::effects::EffectKind::AddPowerIfBattleOpponentLevelAtLeast {
                    amount: *amount,
                    min_level: *min_level,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddSoulIfBattleOpponentLevelAtLeast {
                    amount,
                    min_level,
                    duration_turn,
                } => crate::effects::EffectKind::AddSoulIfBattleOpponentLevelAtLeast {
                    amount: *amount,
                    min_level: *min_level,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddPowerIfBattleOpponentLevelExact {
                    amount,
                    level,
                    duration_turn,
                } => crate::effects::EffectKind::AddPowerIfBattleOpponentLevelExact {
                    amount: *amount,
                    level: *level,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddPowerIfOtherAttackerMatches {
                    amount,
                    duration_turn,
                    attacker_card_ids,
                } => crate::effects::EffectKind::AddPowerIfOtherAttackerMatches {
                    amount: *amount,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                    attacker_card_ids: attacker_card_ids.clone(),
                },
                EffectTemplate::AddSoul {
                    amount,
                    duration_turn,
                } => crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Soul,
                    magnitude: *amount,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::AddSoulIfMiddleCenter { amount } => {
                    crate::effects::EffectKind::AddSoulIfMiddleCenter { amount: *amount }
                }
                EffectTemplate::AddLevel {
                    amount,
                    duration_turn,
                } => crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Level,
                    magnitude: *amount,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::FacingOpponentAddSoul { amount } => {
                    crate::effects::EffectKind::FacingOpponentAddSoul { amount: *amount }
                }
                EffectTemplate::CannotFrontalAttack { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotFrontalAttack,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::CannotBecomeReverse { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotBecomeReverse,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::CannotBeChosenByOpponentEffects { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotBeChosenByOpponentEffects,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::CannotMoveStagePosition { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotMoveStagePosition,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::CannotPlayEventsFromHand { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotPlayEventsFromHand,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::CannotPlayBackupFromHand { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotPlayBackupFromHand,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::CannotStandDuringStandPhase { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotStandDuringStandPhase,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::BattleOpponentMoveToMemoryOnReverse { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::BattleOpponentMoveToMemoryOnReverse,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::EncoreStockCost {
                    cost,
                    duration_turn,
                } => crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::EncoreStockCost,
                    magnitude: *cost as i32,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::GrantAbilityDef { ability, duration } => {
                    crate::effects::EffectKind::GrantAbilityDef {
                        ability: ability.clone(),
                        duration: *duration,
                    }
                }
                EffectTemplate::FacingOpponentCannotMoveStagePosition => {
                    crate::effects::EffectKind::FacingOpponentAddModifier {
                        kind: crate::state::ModifierKind::CannotMoveStagePosition,
                        magnitude: 1,
                        duration: crate::state::ModifierDuration::WhileOnStage,
                    }
                }
                EffectTemplate::SelfCannotBecomeReverseIfFacingOpponent {
                    max_level,
                    max_cost,
                    level_gt_source_level,
                } => crate::effects::EffectKind::SelfAddModifierIfFacingOpponent {
                    kind: crate::state::ModifierKind::CannotBecomeReverse,
                    magnitude: 1,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_source_level: *level_gt_source_level,
                },
                EffectTemplate::SelfCannotFrontalAttackIfFacingOpponentHigherLevel => {
                    crate::effects::EffectKind::SelfAddModifierIfFacingOpponent {
                        kind: crate::state::ModifierKind::CannotFrontalAttack,
                        magnitude: 1,
                        duration: crate::state::ModifierDuration::WhileOnStage,
                        max_level: None,
                        max_cost: None,
                        level_gt_source_level: true,
                    }
                }
                EffectTemplate::CannotSideAttack { duration_turn } => {
                    crate::effects::EffectKind::AddModifier {
                        kind: crate::state::ModifierKind::CannotSideAttack,
                        magnitude: 1,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                    }
                }
                EffectTemplate::ConditionalAddPower {
                    amount,
                    turn,
                    zone_count,
                    require_source_marker,
                    per_source_marker,
                    per_zone_count,
                    exclude_source,
                    target_ids,
                } => {
                    if !target_ids.is_empty() {
                        if let Some(spec) = target_spec.as_mut() {
                            spec.card_ids = target_ids.clone();
                        }
                    }
                    crate::effects::EffectKind::ConditionalAddModifier {
                        kind: crate::state::ModifierKind::Power,
                        magnitude: *amount,
                        duration: crate::state::ModifierDuration::WhileOnStage,
                        turn: *turn,
                        zone_count: zone_count.clone(),
                        require_source_marker: *require_source_marker,
                        per_source_marker: *per_source_marker,
                        per_zone_count: *per_zone_count,
                        exclude_source: *exclude_source,
                    }
                }
                EffectTemplate::ConditionalAddLevel {
                    amount,
                    turn,
                    zone_count,
                    require_source_marker,
                    per_source_marker,
                    per_zone_count,
                    exclude_source,
                    target_ids,
                } => {
                    if !target_ids.is_empty() {
                        if let Some(spec) = target_spec.as_mut() {
                            spec.card_ids = target_ids.clone();
                        }
                    }
                    crate::effects::EffectKind::ConditionalAddModifier {
                        kind: crate::state::ModifierKind::Level,
                        magnitude: *amount,
                        duration: crate::state::ModifierDuration::WhileOnStage,
                        turn: *turn,
                        zone_count: zone_count.clone(),
                        require_source_marker: *require_source_marker,
                        per_source_marker: *per_source_marker,
                        per_zone_count: *per_zone_count,
                        exclude_source: *exclude_source,
                    }
                }
                EffectTemplate::ConditionalCannotSideAttack {
                    turn,
                    zone_count,
                    require_source_marker,
                    exclude_source,
                } => crate::effects::EffectKind::ConditionalAddModifier {
                    kind: crate::state::ModifierKind::CannotSideAttack,
                    magnitude: 1,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                    turn: *turn,
                    zone_count: zone_count.clone(),
                    require_source_marker: *require_source_marker,
                    per_source_marker: false,
                    per_zone_count: false,
                    exclude_source: *exclude_source,
                },
                EffectTemplate::TimedConditionalAddPower {
                    amount,
                    duration_turn,
                    turn,
                    zone_count,
                    require_source_marker,
                    per_source_marker,
                    per_zone_count,
                    exclude_source,
                    target_ids,
                } => {
                    if !target_ids.is_empty() {
                        if let Some(spec) = target_spec.as_mut() {
                            spec.card_ids = target_ids.clone();
                        }
                    }
                    crate::effects::EffectKind::ConditionalAddModifier {
                        kind: crate::state::ModifierKind::Power,
                        magnitude: *amount,
                        duration: if *duration_turn {
                            crate::state::ModifierDuration::UntilEndOfTurn
                        } else {
                            crate::state::ModifierDuration::WhileOnStage
                        },
                        turn: *turn,
                        zone_count: zone_count.clone(),
                        require_source_marker: *require_source_marker,
                        per_source_marker: *per_source_marker,
                        per_zone_count: *per_zone_count,
                        exclude_source: *exclude_source,
                    }
                }
                EffectTemplate::MoveToHand => crate::effects::EffectKind::MoveToHand,
                EffectTemplate::MoveToWaitingRoom => crate::effects::EffectKind::MoveToWaitingRoom,
                EffectTemplate::MoveToStock => crate::effects::EffectKind::MoveToStock,
                EffectTemplate::MoveToClock => crate::effects::EffectKind::MoveToClock,
                EffectTemplate::MoveToMemory => crate::effects::EffectKind::MoveToMemory,
                EffectTemplate::MoveToDeckBottom => crate::effects::EffectKind::MoveToDeckBottom,
                EffectTemplate::MoveWaitingRoomCardToSourceSlot { target_ids } => {
                    if !target_ids.is_empty() {
                        if let Some(spec) = target_spec.as_mut() {
                            spec.card_ids = target_ids.clone();
                        }
                    }
                    crate::effects::EffectKind::MoveWaitingRoomCardToSourceSlot
                }
                EffectTemplate::RecycleWaitingRoomToDeckShuffle => {
                    crate::effects::EffectKind::RecycleWaitingRoomToDeckShuffle
                }
                EffectTemplate::ResetStockFromDeckTop { target } => {
                    crate::effects::EffectKind::ResetStockFromDeckTop { target: *target }
                }
                EffectTemplate::MoveToMarker { target_ids } => {
                    if !target_ids.is_empty() {
                        if let Some(spec) = target_spec.as_mut() {
                            spec.card_ids = target_ids.clone();
                        }
                    }
                    crate::effects::EffectKind::MoveToMarker
                }
                EffectTemplate::MoveTopDeckToMarker => {
                    crate::effects::EffectKind::MoveTopDeckToMarker
                }
                EffectTemplate::Heal => crate::effects::EffectKind::Heal,
                EffectTemplate::HealIfSourcePlayedFromHandThisTurn => {
                    crate::effects::EffectKind::HealIfSourcePlayedFromHandThisTurn
                }
                EffectTemplate::RestTarget => crate::effects::EffectKind::RestTarget,
                EffectTemplate::StandTarget => crate::effects::EffectKind::StandTarget,
                EffectTemplate::StockCharge { count } => {
                    crate::effects::EffectKind::StockCharge { count: *count }
                }
                EffectTemplate::MillTop { target, count } => crate::effects::EffectKind::MillTop {
                    target: *target,
                    count: *count,
                },
                EffectTemplate::MoveStageSlot { slot } => {
                    crate::effects::EffectKind::MoveStageSlot { slot: *slot }
                }
                EffectTemplate::MoveThisToOpenCenter { require_facing } => {
                    crate::effects::EffectKind::MoveThisToOpenCenter {
                        require_facing: *require_facing,
                    }
                }
                EffectTemplate::MoveThisToOpenBack => crate::effects::EffectKind::MoveThisToOpenBack,
                EffectTemplate::SwapStageSlots => crate::effects::EffectKind::SwapStageSlots,
                EffectTemplate::RandomDiscardFromHand { target, count } => {
                    crate::effects::EffectKind::RandomDiscardFromHand {
                        target: *target,
                        count: *count,
                    }
                }
                EffectTemplate::RandomMill { target, count } => {
                    crate::effects::EffectKind::RandomMill {
                        target: *target,
                        count: *count,
                    }
                }
                EffectTemplate::RevealZoneTop {
                    target,
                    zone,
                    count,
                    audience,
                } => crate::effects::EffectKind::RevealZoneTop {
                    target: *target,
                    zone: *zone,
                    count: *count,
                    audience: *audience,
                },
                EffectTemplate::RevealTopIfLevelAtLeastMoveThisToHand { min_level } => {
                    crate::effects::EffectKind::RevealTopIfLevelAtLeastMoveThisToHand {
                        min_level: *min_level,
                    }
                }
                EffectTemplate::RevealTopIfLevelAtLeastRestThis { min_level } => {
                    crate::effects::EffectKind::RevealTopIfLevelAtLeastRestThis {
                        min_level: *min_level,
                    }
                }
                EffectTemplate::RevealTopIfLevelAtLeastMoveTopToStock { min_level } => {
                    crate::effects::EffectKind::RevealTopIfLevelAtLeastMoveTopToStock {
                        min_level: *min_level,
                    }
                }
                EffectTemplate::LookTopDeckReorder { count } => {
                    if let Some(spec) = target_spec.as_mut() {
                        spec.zone = TargetZone::DeckTop;
                        spec.side = TargetSide::SelfSide;
                        spec.count = *count;
                        spec.limit = Some(*count);
                        spec.reveal_to_controller = true;
                    } else {
                        target_spec = Some(TargetSpec {
                            zone: TargetZone::DeckTop,
                            side: TargetSide::SelfSide,
                            slot_filter: crate::state::TargetSlotFilter::Any,
                            card_type: None,
                            card_trait: None,
                            level_max: None,
                            cost_max: None,
                            card_ids: Vec::new(),
                            count: *count,
                            limit: Some(*count),
                            source_only: false,
                            reveal_to_controller: true,
                        });
                    }
                    crate::effects::EffectKind::LookTopDeckReorder { count: *count }
                }
                EffectTemplate::LookTopCardTopOrWaitingRoom => {
                    if let Some(spec) = target_spec.as_mut() {
                        spec.zone = TargetZone::DeckTop;
                        spec.side = TargetSide::SelfSide;
                        spec.count = 1;
                        spec.limit = Some(1);
                        spec.reveal_to_controller = true;
                    } else {
                        target_spec = Some(TargetSpec {
                            zone: TargetZone::DeckTop,
                            side: TargetSide::SelfSide,
                            slot_filter: crate::state::TargetSlotFilter::Any,
                            card_type: None,
                            card_trait: None,
                            level_max: None,
                            cost_max: None,
                            card_ids: Vec::new(),
                            count: 1,
                            limit: Some(1),
                            source_only: false,
                            reveal_to_controller: true,
                        });
                    }
                    crate::effects::EffectKind::LookTopCardTopOrWaitingRoom
                }
                EffectTemplate::LookTopCardTopOrBottom => {
                    if let Some(spec) = target_spec.as_mut() {
                        spec.zone = TargetZone::DeckTop;
                        spec.side = TargetSide::SelfSide;
                        spec.count = 1;
                        spec.limit = Some(1);
                        spec.reveal_to_controller = true;
                    } else {
                        target_spec = Some(TargetSpec {
                            zone: TargetZone::DeckTop,
                            side: TargetSide::SelfSide,
                            slot_filter: crate::state::TargetSlotFilter::Any,
                            card_type: None,
                            card_trait: None,
                            level_max: None,
                            cost_max: None,
                            card_ids: Vec::new(),
                            count: 1,
                            limit: Some(1),
                            source_only: false,
                            reveal_to_controller: true,
                        });
                    }
                    crate::effects::EffectKind::LookTopCardTopOrBottom
                }
                EffectTemplate::SearchTopDeckToHandLevelAtLeastMillRest {
                    look_count,
                    choose_count,
                    min_level,
                } => crate::effects::EffectKind::SearchTopDeckToHandLevelAtLeastMillRest {
                    look_count: *look_count,
                    choose_count: *choose_count,
                    min_level: *min_level,
                },
                EffectTemplate::RevealTopAndSalvageByRevealedLevel { count, climax_level } => {
                    crate::effects::EffectKind::RevealTopAndSalvageByRevealedLevel {
                        count: *count,
                        climax_level: *climax_level,
                    }
                }
                EffectTemplate::ChangeController => crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                EffectTemplate::CounterBackup { power } => {
                    crate::effects::EffectKind::CounterBackup { power: *power }
                }
                EffectTemplate::CounterDamageReduce { amount } => {
                    crate::effects::EffectKind::CounterDamageReduce { amount: *amount }
                }
                EffectTemplate::CounterDamageCancel => {
                    crate::effects::EffectKind::CounterDamageCancel
                }
                EffectTemplate::TriggerIcon { icon } => {
                    crate::effects::EffectKind::TriggerIcon { icon: *icon }
                }
                EffectTemplate::BattleOpponentReverseIf {
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentReverseIf {
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentMoveToDeckBottomIf {
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentMoveToDeckBottomIf {
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentMoveToMemoryIf {
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentMoveToMemoryIf {
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentMoveToClockIf {
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentMoveToClockIf {
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentMoveIf {
                    destination,
                    prelude,
                    max_level,
                    max_cost,
                    level_gt_opponent_level,
                } => crate::effects::EffectKind::BattleOpponentMove {
                    destination: *destination,
                    prelude: *prelude,
                    max_level: *max_level,
                    max_cost: *max_cost,
                    level_gt_opponent_level: *level_gt_opponent_level,
                },
                EffectTemplate::BattleOpponentTopDeckToStockIf { min_level } => {
                    crate::effects::EffectKind::BattleOpponentTopDeckToStockIf {
                        min_level: *min_level,
                    }
                }
                EffectTemplate::Brainstorm {
                    reveal_count,
                    per_climax,
                    mode,
                } => crate::effects::EffectKind::Brainstorm {
                    reveal_count: *reveal_count,
                    per_climax: *per_climax,
                    mode: *mode,
                },
                EffectTemplate::SetTriggerCheckCount { count } => {
                    crate::effects::EffectKind::SetTriggerCheckCount { count: *count }
                }
                EffectTemplate::RestThisIfNoOtherRestCenter => {
                    crate::effects::EffectKind::RestThisIfNoOtherRestCenter
                }
                EffectTemplate::CannotUseAutoEncoreForPlayer { target } => {
                    crate::effects::EffectKind::CannotUseAutoEncoreForPlayer { target: *target }
                }
            },
            target: target_spec,
            // Missing optional flags default to required unless there is no aligned target entry.
            // This matches the legacy "targetless tail effects are skippable" behavior.
            optional: def
                .effect_optional
                .get(idx)
                .copied()
                .unwrap_or(idx >= def.targets.len()),
        });
    }
    out
}
