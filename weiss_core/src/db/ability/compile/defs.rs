use super::target::target_spec_from_template;
use super::*;

/// Compile structured ability definitions into executable effect specs.
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
                EffectTemplate::ConditionalAddSoul {
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
                        kind: crate::state::ModifierKind::Soul,
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
                EffectTemplate::SetTerminalOutcome { outcome } => {
                    crate::effects::EffectKind::SetTerminalOutcome {
                        outcome: match outcome {
                            TerminalOutcomeSpec::WinSelf => {
                                crate::effects::TerminalOutcomeSpec::WinSelf
                            }
                            TerminalOutcomeSpec::WinOpponent => {
                                crate::effects::TerminalOutcomeSpec::WinOpponent
                            }
                            TerminalOutcomeSpec::Draw => {
                                crate::effects::TerminalOutcomeSpec::Draw
                            }
                            TerminalOutcomeSpec::Timeout => {
                                crate::effects::TerminalOutcomeSpec::Timeout
                            }
                        },
                    }
                }
                EffectTemplate::ApplyRuleOverride { kind } => {
                    crate::effects::EffectKind::ApplyRuleOverride {
                        kind: match kind {
                            RuleOverrideKind::SkipDeckRefreshOrLoss => {
                                crate::effects::RuleOverrideKind::SkipDeckRefreshOrLoss
                            }
                            RuleOverrideKind::SkipLevelFourLoss => {
                                crate::effects::RuleOverrideKind::SkipLevelFourLoss
                            }
                            RuleOverrideKind::SkipNonCharacterStageCleanup => {
                                crate::effects::RuleOverrideKind::SkipNonCharacterStageCleanup
                            }
                            RuleOverrideKind::SkipZeroOrNegativePowerCleanup => {
                                crate::effects::RuleOverrideKind::SkipZeroOrNegativePowerCleanup
                            }
                        },
                    }
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
