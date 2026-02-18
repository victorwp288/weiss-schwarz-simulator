fn ability_kind_key(kind: AbilityKind) -> u64 {
    match kind {
        AbilityKind::Continuous => 0,
        AbilityKind::Activated => 1,
        AbilityKind::Auto => 2,
    }
}

fn ability_timing_key(timing: Option<AbilityTiming>) -> u64 {
    match timing {
        None => u64::MAX,
        Some(AbilityTiming::BeginTurn) => 0,
        Some(AbilityTiming::BeginStandPhase) => 1,
        Some(AbilityTiming::AfterStandPhase) => 2,
        Some(AbilityTiming::BeginDrawPhase) => 3,
        Some(AbilityTiming::AfterDrawPhase) => 4,
        Some(AbilityTiming::BeginClockPhase) => 5,
        Some(AbilityTiming::AfterClockPhase) => 6,
        Some(AbilityTiming::BeginMainPhase) => 7,
        Some(AbilityTiming::BeginClimaxPhase) => 8,
        Some(AbilityTiming::AfterClimaxPhase) => 9,
        Some(AbilityTiming::BeginAttackPhase) => 10,
        Some(AbilityTiming::BeginAttackDeclarationStep) => 11,
        Some(AbilityTiming::BeginEncoreStep) => 12,
        Some(AbilityTiming::EndPhase) => 13,
        Some(AbilityTiming::EndPhaseCleanup) => 14,
        Some(AbilityTiming::EndOfAttack) => 15,
        Some(AbilityTiming::AttackDeclaration) => 16,
        Some(AbilityTiming::OtherAttackDeclaration) => 17,
        Some(AbilityTiming::TriggerResolution) => 18,
        Some(AbilityTiming::Counter) => 19,
        Some(AbilityTiming::DamageResolution) => 20,
        Some(AbilityTiming::Encore) => 21,
        Some(AbilityTiming::OnPlay) => 22,
        Some(AbilityTiming::OnReverse) => 23,
        Some(AbilityTiming::DamageDealtCanceled) => 24,
        Some(AbilityTiming::DamageReceivedCanceled) => 25,
        Some(AbilityTiming::BattleOpponentReverse) => 26,
        Some(AbilityTiming::UseAct) => 27,
        Some(AbilityTiming::DamageDealtNotCanceled) => 28,
        Some(AbilityTiming::LevelUp) => 29,
        Some(AbilityTiming::DamageReceivedNotCanceled) => 30,
    }
}

fn i32_key(value: i32) -> u64 {
    // Map signed i32 to u64 while preserving numeric order.
    (value as u32 ^ 0x8000_0000) as u64
}

fn push_opt_u8(out: &mut Vec<u64>, value: Option<u8>) {
    match value {
        None => {
            out.push(0);
            out.push(0);
        }
        Some(val) => {
            out.push(1);
            out.push(val as u64);
        }
    }
}

fn push_opt_u16(out: &mut Vec<u64>, value: Option<u16>) {
    match value {
        None => {
            out.push(0);
            out.push(0);
        }
        Some(val) => {
            out.push(1);
            out.push(val as u64);
        }
    }
}

fn target_template_key(target: TargetTemplate) -> u64 {
    match target {
        TargetTemplate::OppFrontRow => 0,
        TargetTemplate::OppBackRow => 1,
        TargetTemplate::OppStage => 2,
        TargetTemplate::OppStageSlot { slot } => 3 + slot as u64,
        TargetTemplate::SelfFrontRow => 10,
        TargetTemplate::SelfBackRow => 11,
        TargetTemplate::SelfStage => 12,
        TargetTemplate::SelfStageSlot { slot } => 13 + slot as u64,
        TargetTemplate::This => 20,
        TargetTemplate::SelfWaitingRoom => 21,
        TargetTemplate::SelfHand => 22,
        TargetTemplate::SelfDeckTop => 23,
        TargetTemplate::SelfClock => 24,
        TargetTemplate::SelfLevel => 25,
        TargetTemplate::SelfStock => 26,
        TargetTemplate::SelfMemory => 27,
    }
}

fn card_type_key(card_type: Option<CardType>) -> u64 {
    match card_type {
        None => 0,
        Some(CardType::Character) => 1,
        Some(CardType::Event) => 2,
        Some(CardType::Climax) => 3,
    }
}

fn target_side_key(side: TargetSide) -> u64 {
    match side {
        TargetSide::SelfSide => 0,
        TargetSide::Opponent => 1,
    }
}

fn target_zone_key(zone: TargetZone) -> u64 {
    match zone {
        TargetZone::Stage => 0,
        TargetZone::Hand => 1,
        TargetZone::DeckTop => 2,
        TargetZone::Clock => 3,
        TargetZone::Level => 4,
        TargetZone::Stock => 5,
        TargetZone::Memory => 6,
        TargetZone::WaitingRoom => 7,
        TargetZone::Climax => 8,
        TargetZone::Resolution => 9,
    }
}

fn reveal_audience_key(audience: RevealAudience) -> u64 {
    match audience {
        RevealAudience::Public => 0,
        RevealAudience::BothPlayers => 1,
        RevealAudience::OwnerOnly => 2,
        RevealAudience::ControllerOnly => 3,
        RevealAudience::ReplayOnly => 4,
    }
}

fn trigger_icon_key(icon: TriggerIcon) -> u8 {
    match icon {
        TriggerIcon::Soul => 0,
        TriggerIcon::Shot => 1,
        TriggerIcon::Bounce => 2,
        TriggerIcon::Draw => 3,
        TriggerIcon::Choice => 4,
        TriggerIcon::Pool => 5,
        TriggerIcon::Treasure => 6,
        TriggerIcon::Gate => 7,
        TriggerIcon::Standby => 8,
    }
}

fn condition_turn_key(turn: Option<ConditionTurn>) -> u64 {
    match turn {
        None => 0,
        Some(ConditionTurn::SelfTurn) => 1,
        Some(ConditionTurn::OpponentTurn) => 2,
    }
}

fn count_cmp_key(cmp: CountCmp) -> u64 {
    match cmp {
        CountCmp::AtLeast => 0,
        CountCmp::AtMost => 1,
    }
}

fn count_zone_key(zone: CountZone) -> u64 {
    match zone {
        CountZone::Stock => 0,
        CountZone::WaitingRoom => 1,
        CountZone::Hand => 2,
        CountZone::Stage => 3,
        CountZone::BackStage => 4,
        CountZone::WaitingRoomClimax => 5,
        CountZone::LevelTotal => 6,
    }
}

fn grant_duration_key(duration: crate::db::GrantDuration) -> u64 {
    match duration {
        crate::db::GrantDuration::UntilEndOfTurn => 0,
        crate::db::GrantDuration::UntilEndOfOpponentsNextTurn => 1,
    }
}

fn battle_opponent_move_destination_key(
    destination: crate::db::BattleOpponentMoveDestination,
) -> u64 {
    match destination {
        crate::db::BattleOpponentMoveDestination::DeckBottom => 0,
        crate::db::BattleOpponentMoveDestination::StockThenBottomStockToWaitingRoom => 1,
        crate::db::BattleOpponentMoveDestination::Clock => 2,
        crate::db::BattleOpponentMoveDestination::Memory => 3,
    }
}

fn battle_opponent_move_prelude_key(
    prelude: Option<crate::db::BattleOpponentMovePreludeAction>,
) -> u64 {
    match prelude {
        None => 0,
        Some(crate::db::BattleOpponentMovePreludeAction::OpponentClockTopToWaitingRoom) => 1,
    }
}

fn terminal_outcome_key(outcome: super::types::TerminalOutcomeSpec) -> u64 {
    match outcome {
        super::types::TerminalOutcomeSpec::WinSelf => 0,
        super::types::TerminalOutcomeSpec::WinOpponent => 1,
        super::types::TerminalOutcomeSpec::Draw => 2,
        super::types::TerminalOutcomeSpec::Timeout => 3,
    }
}

fn rule_override_key(kind: super::types::RuleOverrideKind) -> u64 {
    match kind {
        super::types::RuleOverrideKind::SkipDeckRefreshOrLoss => 0,
        super::types::RuleOverrideKind::SkipLevelFourLoss => 1,
        super::types::RuleOverrideKind::SkipNonCharacterStageCleanup => 2,
        super::types::RuleOverrideKind::SkipZeroOrNegativePowerCleanup => 3,
    }
}

fn ability_cost_step_key(step: AbilityCostStep) -> u64 {
    match step {
        AbilityCostStep::RestOther => 0,
        AbilityCostStep::SacrificeFromStage => 1,
        AbilityCostStep::DiscardFromHand => 2,
        AbilityCostStep::ClockFromHand => 3,
        AbilityCostStep::ClockFromDeckTop => 4,
        AbilityCostStep::RevealFromHand => 5,
    }
}

fn push_ability_cost_key(out: &mut Vec<u64>, cost: &AbilityCost) {
    out.push(cost.stock as u64);
    out.push(u64::from(cost.rest_self));
    out.push(cost.rest_other as u64);
    out.push(cost.sacrifice_from_stage as u64);
    out.push(cost.discard_from_hand as u64);
    out.push(cost.clock_from_hand as u64);
    out.push(cost.clock_from_deck_top as u64);
    out.push(cost.reveal_from_hand as u64);
    out.push(u64::from(cost.move_self_to_waiting_room));
    out.push(u64::from(cost.return_self_to_hand));
    out.push(cost.step_order.len() as u64);
    for step in &cost.step_order {
        out.push(ability_cost_step_key(*step));
    }
}

fn push_zone_count_key(out: &mut Vec<u64>, zone_count: Option<&ZoneCountCondition>) {
    match zone_count {
        None => out.push(0),
        Some(condition) => {
            out.push(1);
            out.push(target_side_key(condition.side));
            out.push(count_zone_key(condition.zone));
            out.push(count_cmp_key(condition.cmp));
            out.push(condition.value as u64);
            push_sorted_card_id_set_key(out, &condition.card_ids);
        }
    }
}

fn push_sorted_card_id_set_key(out: &mut Vec<u64>, card_ids: &[CardId]) {
    let mut sorted = card_ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    out.push(sorted.len() as u64);
    for id in sorted {
        out.push(id as u64);
    }
}

fn push_ability_def_conditions_key(out: &mut Vec<u64>, conditions: &AbilityDefConditions) {
    out.push(u64::from(conditions.requires_approx_effects));
    match &conditions.climax_area {
        None => out.push(0),
        Some(condition) => {
            out.push(1);
            out.push(target_side_key(condition.side));
            push_sorted_card_id_set_key(out, &condition.card_ids);
        }
    }
    out.push(condition_turn_key(conditions.turn));
    out.push(i32_key(conditions.hand_level_delta as i32));
    push_opt_u8(out, conditions.self_waiting_room_climax_at_most);
    push_sorted_card_id_set_key(out, &conditions.self_clock_card_ids_any);
    push_sorted_card_id_set_key(out, &conditions.self_memory_card_ids_any);
    push_opt_u8(out, conditions.self_memory_at_most);
    push_opt_u8(out, conditions.opponent_stage_has_level_at_least);
    out.push(u64::from(conditions.trigger_check_revealed_climax));
    push_opt_u8(
        out,
        conditions.trigger_check_revealed_icon.map(trigger_icon_key),
    );
    out.push(u64::from(conditions.ignore_color_requirement));
    push_zone_count_key(out, conditions.zone_count.as_ref());
}

fn effect_template_key(effect: &EffectTemplate, out: &mut Vec<u64>) {
    match effect {
        EffectTemplate::Draw { count } => {
            out.push(0);
            out.push(*count as u64);
        }
        EffectTemplate::DealDamage { amount, cancelable } => {
            out.push(1);
            out.push(*amount as u64);
            out.push(u64::from(*cancelable));
        }
        EffectTemplate::AddPower {
            amount,
            duration_turn,
        } => {
            out.push(2);
            out.push(i32_key(*amount));
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddPowerIfTargetLevelAtLeast {
            amount,
            min_level,
            duration_turn,
        } => {
            out.push(31);
            out.push(i32_key(*amount));
            out.push(*min_level as u64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddPowerByLevel {
            multiplier,
            duration_turn,
        } => {
            out.push(26);
            out.push(i32_key(*multiplier));
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddPowerIfBattleOpponentLevelAtLeast {
            amount,
            min_level,
            duration_turn,
        } => {
            out.push(35);
            out.push(i32_key(*amount));
            out.push(*min_level as u64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddSoulIfBattleOpponentLevelAtLeast {
            amount,
            min_level,
            duration_turn,
        } => {
            out.push(73);
            out.push(i32_key(*amount));
            out.push(*min_level as u64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddPowerIfBattleOpponentLevelExact {
            amount,
            level,
            duration_turn,
        } => {
            out.push(36);
            out.push(i32_key(*amount));
            out.push(*level as u64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddPowerIfOtherAttackerMatches {
            amount,
            duration_turn,
            attacker_card_ids,
        } => {
            out.push(67);
            out.push(i32_key(*amount));
            out.push(u64::from(*duration_turn));
            push_sorted_card_id_set_key(out, attacker_card_ids);
        }
        EffectTemplate::AddSoul {
            amount,
            duration_turn,
        } => {
            out.push(21);
            out.push(i32_key(*amount));
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::AddSoulIfMiddleCenter { amount } => {
            out.push(68);
            out.push(i32_key(*amount));
        }
        EffectTemplate::AddLevel {
            amount,
            duration_turn,
        } => {
            out.push(59);
            out.push(i32_key(*amount));
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::FacingOpponentAddSoul { amount } => {
            out.push(42);
            out.push(i32_key(*amount));
        }
        EffectTemplate::CannotFrontalAttack { duration_turn } => {
            out.push(50);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::CannotBecomeReverse { duration_turn } => {
            out.push(51);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::CannotBeChosenByOpponentEffects { duration_turn } => {
            out.push(52);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::CannotMoveStagePosition { duration_turn } => {
            out.push(53);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::CannotPlayEventsFromHand { duration_turn } => {
            out.push(63);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::CannotPlayBackupFromHand { duration_turn } => {
            out.push(64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::CannotStandDuringStandPhase { duration_turn } => {
            out.push(82);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::BattleOpponentMoveToMemoryOnReverse { duration_turn } => {
            out.push(57);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::EncoreStockCost {
            cost,
            duration_turn,
        } => {
            out.push(58);
            out.push(*cost as u64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::GrantAbilityDef { ability, duration } => {
            out.push(77);
            out.push(grant_duration_key(*duration));
            out.extend(ability_def_key(ability));
        }
        EffectTemplate::FacingOpponentCannotMoveStagePosition => {
            out.push(54);
        }
        EffectTemplate::SelfCannotBecomeReverseIfFacingOpponent {
            max_level,
            max_cost,
            level_gt_source_level,
        } => {
            out.push(55);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_source_level));
        }
        EffectTemplate::SelfCannotFrontalAttackIfFacingOpponentHigherLevel => {
            out.push(56);
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
            out.push(24);
            out.push(i32_key(*amount));
            out.push(condition_turn_key(*turn));
            push_zone_count_key(out, zone_count.as_ref());
            out.push(u64::from(*require_source_marker));
            out.push(u64::from(*per_source_marker));
            out.push(u64::from(*per_zone_count));
            out.push(u64::from(*exclude_source));
            push_sorted_card_id_set_key(out, target_ids);
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
            out.push(85);
            out.push(i32_key(*amount));
            out.push(condition_turn_key(*turn));
            push_zone_count_key(out, zone_count.as_ref());
            out.push(u64::from(*require_source_marker));
            out.push(u64::from(*per_source_marker));
            out.push(u64::from(*per_zone_count));
            out.push(u64::from(*exclude_source));
            push_sorted_card_id_set_key(out, target_ids);
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
            out.push(60);
            out.push(i32_key(*amount));
            out.push(condition_turn_key(*turn));
            push_zone_count_key(out, zone_count.as_ref());
            out.push(u64::from(*require_source_marker));
            out.push(u64::from(*per_source_marker));
            out.push(u64::from(*per_zone_count));
            out.push(u64::from(*exclude_source));
            push_sorted_card_id_set_key(out, target_ids);
        }
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
            out.push(41);
            out.push(i32_key(*amount));
            out.push(u64::from(*duration_turn));
            out.push(condition_turn_key(*turn));
            push_zone_count_key(out, zone_count.as_ref());
            out.push(u64::from(*require_source_marker));
            out.push(u64::from(*per_source_marker));
            out.push(u64::from(*per_zone_count));
            out.push(u64::from(*exclude_source));
            push_sorted_card_id_set_key(out, target_ids);
        }
        EffectTemplate::ConditionalCannotSideAttack {
            turn,
            zone_count,
            require_source_marker,
            exclude_source,
        } => {
            out.push(38);
            out.push(condition_turn_key(*turn));
            push_zone_count_key(out, zone_count.as_ref());
            out.push(u64::from(*require_source_marker));
            out.push(u64::from(*exclude_source));
        }
        EffectTemplate::CannotSideAttack { duration_turn } => {
            out.push(22);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::MoveToMarker { target_ids } => {
            out.push(25);
            push_sorted_card_id_set_key(out, target_ids);
        }
        EffectTemplate::MoveTopDeckToMarker => {
            out.push(47);
        }
        EffectTemplate::MoveToHand => {
            out.push(3);
        }
        EffectTemplate::MoveToWaitingRoom => {
            out.push(4);
        }
        EffectTemplate::MoveToStock => {
            out.push(5);
        }
        EffectTemplate::MoveToClock => {
            out.push(6);
        }
        EffectTemplate::MoveToMemory => {
            out.push(46);
        }
        EffectTemplate::MoveToDeckBottom => {
            out.push(27);
        }
        EffectTemplate::MoveWaitingRoomCardToSourceSlot { target_ids } => {
            out.push(72);
            push_sorted_card_id_set_key(out, target_ids);
        }
        EffectTemplate::RecycleWaitingRoomToDeckShuffle => {
            out.push(61);
        }
        EffectTemplate::ResetStockFromDeckTop { target } => {
            out.push(69);
            out.push(target_side_key(*target));
        }
        EffectTemplate::Heal => {
            out.push(7);
        }
        EffectTemplate::HealIfSourcePlayedFromHandThisTurn => {
            out.push(70);
        }
        EffectTemplate::RestTarget => {
            out.push(8);
        }
        EffectTemplate::StandTarget => {
            out.push(9);
        }
        EffectTemplate::StockCharge { count } => {
            out.push(10);
            out.push(*count as u64);
        }
        EffectTemplate::MillTop { target, count } => {
            out.push(11);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::MoveStageSlot { slot } => {
            out.push(12);
            out.push(*slot as u64);
        }
        EffectTemplate::MoveThisToOpenCenter { require_facing } => {
            out.push(32);
            out.push(u64::from(*require_facing));
        }
        EffectTemplate::MoveThisToOpenBack => {
            out.push(37);
        }
        EffectTemplate::SwapStageSlots => {
            out.push(13);
        }
        EffectTemplate::RandomDiscardFromHand { target, count } => {
            out.push(14);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::RandomMill { target, count } => {
            out.push(15);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::RevealZoneTop {
            target,
            zone,
            count,
            audience,
        } => {
            out.push(16);
            out.push(target_side_key(*target));
            out.push(target_zone_key(*zone));
            out.push(*count as u64);
            out.push(reveal_audience_key(*audience));
        }
        EffectTemplate::RevealTopIfLevelAtLeastMoveThisToHand { min_level } => {
            out.push(43);
            out.push(*min_level as u64);
        }
        EffectTemplate::RevealTopIfLevelAtLeastRestThis { min_level } => {
            out.push(44);
            out.push(*min_level as u64);
        }
        EffectTemplate::RevealTopIfLevelAtLeastMoveTopToStock { min_level } => {
            out.push(45);
            out.push(*min_level as u64);
        }
        EffectTemplate::LookTopDeckReorder { count } => {
            out.push(78);
            out.push(*count as u64);
        }
        EffectTemplate::LookTopCardTopOrWaitingRoom => {
            out.push(79);
        }
        EffectTemplate::LookTopCardTopOrBottom => {
            out.push(80);
        }
        EffectTemplate::SearchTopDeckToHandLevelAtLeastMillRest {
            look_count,
            choose_count,
            min_level,
        } => {
            out.push(65);
            out.push(*look_count as u64);
            out.push(*choose_count as u64);
            out.push(*min_level as u64);
        }
        EffectTemplate::RevealTopAndSalvageByRevealedLevel {
            count,
            climax_level,
        } => {
            out.push(66);
            out.push(*count as u64);
            out.push(*climax_level as u64);
        }
        EffectTemplate::ChangeController => {
            out.push(17);
        }
        EffectTemplate::CounterBackup { power } => {
            out.push(18);
            out.push(i32_key(*power));
        }
        EffectTemplate::CounterDamageReduce { amount } => {
            out.push(19);
            out.push(*amount as u64);
        }
        EffectTemplate::CounterDamageCancel => {
            out.push(20);
        }
        EffectTemplate::TriggerIcon { icon } => {
            out.push(28);
            out.push(match icon {
                TriggerIcon::Soul => 0,
                TriggerIcon::Shot => 1,
                TriggerIcon::Bounce => 2,
                TriggerIcon::Draw => 3,
                TriggerIcon::Choice => 4,
                TriggerIcon::Pool => 5,
                TriggerIcon::Treasure => 6,
                TriggerIcon::Gate => 7,
                TriggerIcon::Standby => 8,
            });
        }
        EffectTemplate::BattleOpponentReverseIf {
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(29);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentMoveToDeckBottomIf {
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(30);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(39);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(40);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentMoveToMemoryIf {
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(48);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentMoveToClockIf {
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(49);
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentMoveIf {
            destination,
            prelude,
            max_level,
            max_cost,
            level_gt_opponent_level,
        } => {
            out.push(81);
            out.push(battle_opponent_move_destination_key(*destination));
            out.push(battle_opponent_move_prelude_key(*prelude));
            push_opt_u8(out, *max_level);
            push_opt_u8(out, *max_cost);
            out.push(u64::from(*level_gt_opponent_level));
        }
        EffectTemplate::BattleOpponentTopDeckToStockIf { min_level } => {
            out.push(62);
            out.push(*min_level as u64);
        }
        EffectTemplate::Brainstorm {
            reveal_count,
            per_climax,
            mode,
        } => {
            out.push(23);
            out.push(*reveal_count as u64);
            out.push(*per_climax as u64);
            out.push(match mode {
                BrainstormMode::Draw => 0,
                BrainstormMode::SalvageCharacter => 1,
                BrainstormMode::LookTopToHand => 2,
                BrainstormMode::LookTopToHandThenDiscard => 3,
                BrainstormMode::SalvageCharacterThenDiscard => 4,
            });
        }
        EffectTemplate::SetTriggerCheckCount { count } => {
            out.push(33);
            out.push(*count as u64);
        }
        EffectTemplate::RestThisIfNoOtherRestCenter => {
            out.push(34);
        }
        EffectTemplate::CannotUseAutoEncoreForPlayer { target } => {
            out.push(71);
            out.push(target_side_key(*target));
        }
        EffectTemplate::SetTerminalOutcome { outcome } => {
            out.push(83);
            out.push(terminal_outcome_key(*outcome));
        }
        EffectTemplate::ApplyRuleOverride { kind } => {
            out.push(84);
            out.push(rule_override_key(*kind));
        }
    }
}

fn ability_def_key(def: &AbilityDef) -> Vec<u64> {
    let mut out = Vec::new();
    out.push(ability_kind_key(def.kind));
    out.push(ability_timing_key(def.timing));
    out.push(def.effects.len() as u64);
    for effect in &def.effects {
        effect_template_key(effect, &mut out);
    }
    out.push(def.effect_optional.len() as u64);
    for optional in &def.effect_optional {
        out.push(u64::from(*optional));
    }
    out.push(def.targets.len() as u64);
    for target in &def.targets {
        out.push(target_template_key(*target));
    }
    push_ability_cost_key(&mut out, &def.cost);
    out.push(card_type_key(def.target_card_type));
    push_opt_u16(&mut out, def.target_trait);
    push_opt_u8(&mut out, def.target_level_max);
    push_opt_u8(&mut out, def.target_cost_max);
    push_sorted_card_id_set_key(&mut out, &def.target_card_ids);
    push_opt_u8(&mut out, def.target_limit);
    push_ability_def_conditions_key(&mut out, &def.conditions);
    out
}

fn ability_template_key(template: &AbilityTemplate) -> Vec<u64> {
    let mut out = Vec::new();
    match template {
        AbilityTemplate::Vanilla => {
            out.push(0);
        }
        AbilityTemplate::ContinuousPower { amount } => {
            out.push(1);
            out.push(i32_key(*amount));
        }
        AbilityTemplate::ContinuousCannotAttack => {
            out.push(2);
        }
        AbilityTemplate::ContinuousAttackCost { cost } => {
            out.push(3);
            out.push(*cost as u64);
        }
        AbilityTemplate::AutoOnPlayDraw { count } => {
            out.push(4);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlaySalvage {
            count,
            optional,
            card_type,
        } => {
            out.push(5);
            out.push(*count as u64);
            out.push(u64::from(*optional));
            out.push(card_type_key(*card_type));
        }
        AbilityTemplate::AutoOnPlaySearchDeckTop {
            count,
            optional,
            card_type,
        } => {
            out.push(6);
            out.push(*count as u64);
            out.push(u64::from(*optional));
            out.push(card_type_key(*card_type));
        }
        AbilityTemplate::AutoOnPlayRevealDeckTop { count } => {
            out.push(7);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlayStockCharge { count } => {
            out.push(8);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlayMillTop { count } => {
            out.push(9);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlayHeal { count } => {
            out.push(10);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } => {
            out.push(11);
            out.push(*amount as u64);
            out.push(u64::from(*cancelable));
        }
        AbilityTemplate::AutoEndPhaseDraw { count } => {
            out.push(12);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnReverseDraw { count } => {
            out.push(13);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnReverseSalvage {
            count,
            optional,
            card_type,
        } => {
            out.push(14);
            out.push(*count as u64);
            out.push(u64::from(*optional));
            out.push(card_type_key(*card_type));
        }
        AbilityTemplate::EventDealDamage { amount, cancelable } => {
            out.push(15);
            out.push(*amount as u64);
            out.push(u64::from(*cancelable));
        }
        AbilityTemplate::ActivatedPlaceholder => {
            out.push(16);
        }
        AbilityTemplate::ActivatedTargetedPower {
            amount,
            count,
            target,
        } => {
            out.push(17);
            out.push(i32_key(*amount));
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedPaidTargetedPower {
            cost,
            amount,
            count,
            target,
        } => {
            out.push(18);
            out.push(*cost as u64);
            out.push(i32_key(*amount));
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedTargetedMoveToHand { count, target } => {
            out.push(19);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedPaidTargetedMoveToHand {
            cost,
            count,
            target,
        } => {
            out.push(20);
            out.push(*cost as u64);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedChangeController { count, target } => {
            out.push(21);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedPaidChangeController {
            cost,
            count,
            target,
        } => {
            out.push(22);
            out.push(*cost as u64);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::CounterBackup { power } => {
            out.push(23);
            out.push(i32_key(*power));
        }
        AbilityTemplate::CounterDamageReduce { amount } => {
            out.push(24);
            out.push(*amount as u64);
        }
        AbilityTemplate::CounterDamageCancel => {
            out.push(25);
        }
        AbilityTemplate::Bond {
            cost,
            count,
            target_ids,
        } => {
            out.push(29);
            push_ability_cost_key(&mut out, cost);
            out.push(*count as u64);
            push_sorted_card_id_set_key(&mut out, target_ids);
        }
        AbilityTemplate::EncoreVariant { cost } => {
            out.push(26);
            push_ability_cost_key(&mut out, cost);
        }
        AbilityTemplate::AbilityDef(def) => {
            out.push(27);
            out.extend(ability_def_key(def));
        }
        AbilityTemplate::Unsupported { id } => {
            out.push(28);
            out.push(*id as u64);
        }
    }
    out
}

pub(crate) fn ability_sort_key(spec: &AbilitySpec) -> (u8, Vec<u64>) {
    let tag = spec.template.tag() as u8;
    (tag, ability_template_key(&spec.template))
}
