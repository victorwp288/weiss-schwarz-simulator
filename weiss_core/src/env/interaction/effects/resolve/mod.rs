mod battle;
mod brainstorm;
mod damage;
mod healing;
mod look_search;
mod modifiers;
mod movement;
mod prelude;
mod random;
mod reveal;
mod terminal;
mod trigger;

use prelude::*;

impl GameEnv {
    pub(in crate::env) fn resolve_effect_payload(
        &mut self,
        controller: u8,
        source_id: CardId,
        payload: &EffectPayload,
    ) {
        match &payload.spec.kind {
            EffectKind::Draw { count } => {
                self.draw_to_hand(controller, *count as usize);
            }
            EffectKind::Brainstorm {
                reveal_count,
                per_climax,
                mode,
            } => {
                brainstorm::brainstorm(
                    self,
                    controller,
                    source_id,
                    payload,
                    *reveal_count,
                    *per_climax,
                    *mode,
                );
            }
            EffectKind::BrainstormDrawChoice => {
                brainstorm::brainstorm_draw_choice(self, controller);
            }
            EffectKind::RandomDiscardFromHand { target, count } => {
                random::random_discard_from_hand(self, controller, *target, *count);
            }
            EffectKind::RandomMill { target, count } => {
                random::random_mill(self, controller, *target, *count);
            }
            EffectKind::RevealDeckTop { count, audience } => {
                reveal::reveal_deck_top(self, controller, *count, *audience);
            }
            EffectKind::RevealZoneTop {
                target,
                zone,
                count,
                audience,
            } => {
                reveal::reveal_zone_top(self, controller, *target, *zone, *count, *audience);
            }
            EffectKind::RevealTopIfLevelAtLeastMoveThisToHand { min_level } => {
                reveal::reveal_top_if_level_at_least_move_this_to_hand(
                    self, controller, payload, *min_level,
                );
            }
            EffectKind::RevealTopIfLevelAtLeastRestThis { min_level } => {
                reveal::reveal_top_if_level_at_least_rest_this(
                    self, controller, payload, *min_level,
                );
            }
            EffectKind::RevealTopIfLevelAtLeastMoveTopToStock { min_level } => {
                reveal::reveal_top_if_level_at_least_move_top_to_stock(
                    self, controller, *min_level,
                );
            }
            EffectKind::LookTopDeckReorder { count: _ } => {
                look_search::look_top_deck_reorder(self, controller, payload);
            }
            EffectKind::LookTopCardTopOrWaitingRoom => {
                look_search::look_top_card_top_or_waiting_room(self, payload);
            }
            EffectKind::LookTopCardTopOrBottom => {
                look_search::look_top_card_top_or_bottom(self, payload);
            }
            EffectKind::SearchTopDeckToHandLevelAtLeastMillRest {
                look_count,
                choose_count,
                min_level,
            } => {
                look_search::search_top_deck_to_hand_level_at_least_mill_rest(
                    self,
                    controller,
                    *look_count,
                    *choose_count,
                    *min_level,
                );
            }
            EffectKind::RevealTopAndSalvageByRevealedLevel {
                count,
                climax_level,
            } => {
                look_search::reveal_top_and_salvage_by_revealed_level(
                    self,
                    controller,
                    *count,
                    *climax_level,
                );
            }
            EffectKind::Damage {
                amount,
                cancelable,
                damage_type: _,
            } => {
                damage::damage(self, controller, source_id, payload, *amount, *cancelable);
            }
            EffectKind::AddModifier {
                kind,
                magnitude,
                duration,
            } => {
                modifiers::add_modifier(self, source_id, payload, *kind, *magnitude, *duration);
            }
            EffectKind::GrantAbilityDef { ability, duration } => {
                modifiers::grant_ability_def(self, controller, payload, ability, *duration);
            }
            EffectKind::AddPowerIfTargetLevelAtLeast {
                amount,
                min_level,
                duration,
            } => {
                modifiers::add_power_if_target_level_at_least(
                    self, source_id, payload, *amount, *min_level, *duration,
                );
            }
            EffectKind::AddPowerByTargetLevel {
                multiplier,
                duration,
            } => {
                modifiers::add_power_by_target_level(
                    self,
                    source_id,
                    payload,
                    *multiplier,
                    *duration,
                );
            }
            EffectKind::AddPowerIfBattleOpponentLevelAtLeast {
                amount,
                min_level,
                duration,
            } => {
                modifiers::add_power_if_battle_opponent_level_at_least(
                    self, source_id, payload, *amount, *min_level, *duration,
                );
            }
            EffectKind::AddSoulIfBattleOpponentLevelAtLeast {
                amount,
                min_level,
                duration,
            } => {
                modifiers::add_soul_if_battle_opponent_level_at_least(
                    self, source_id, payload, *amount, *min_level, *duration,
                );
            }
            EffectKind::AddPowerIfBattleOpponentLevelExact {
                amount,
                level,
                duration,
            } => {
                modifiers::add_power_if_battle_opponent_level_exact(
                    self, source_id, payload, *amount, *level, *duration,
                );
            }
            EffectKind::AddPowerIfOtherAttackerMatches {
                amount,
                duration,
                attacker_card_ids,
            } => {
                modifiers::add_power_if_other_attacker_matches(
                    self,
                    source_id,
                    payload,
                    *amount,
                    *duration,
                    attacker_card_ids,
                );
            }
            EffectKind::AddSoulIfMiddleCenter { amount } => {
                modifiers::add_soul_if_middle_center(self, source_id, payload, *amount);
            }
            EffectKind::FacingOpponentAddSoul { amount } => {
                modifiers::facing_opponent_add_soul(self, source_id, payload, *amount);
            }
            EffectKind::FacingOpponentAddModifier {
                kind,
                magnitude,
                duration,
            } => {
                modifiers::facing_opponent_add_modifier(
                    self, source_id, payload, *kind, *magnitude, *duration,
                );
            }
            EffectKind::SelfAddModifierIfFacingOpponent {
                kind,
                magnitude,
                duration,
                max_level,
                max_cost,
                level_gt_source_level,
            } => {
                modifiers::self_add_modifier_if_facing_opponent(
                    self,
                    source_id,
                    payload,
                    *kind,
                    *magnitude,
                    *duration,
                    *max_level,
                    *max_cost,
                    *level_gt_source_level,
                );
            }
            EffectKind::ConditionalAddModifier {
                kind,
                magnitude,
                duration,
                turn,
                zone_count,
                require_source_marker,
                per_source_marker,
                per_zone_count,
                exclude_source,
            } => {
                modifiers::conditional_add_modifier(
                    self,
                    controller,
                    source_id,
                    payload,
                    *kind,
                    *magnitude,
                    *duration,
                    *turn,
                    zone_count.as_ref(),
                    *require_source_marker,
                    *per_source_marker,
                    *per_zone_count,
                    *exclude_source,
                );
            }
            EffectKind::MoveToHand => {
                movement::move_to_hand(self, payload);
            }
            EffectKind::MoveToWaitingRoom => {
                movement::move_to_waiting_room(self, payload);
            }
            EffectKind::MoveToStock => {
                movement::move_to_stock(self, payload);
            }
            EffectKind::MoveToClock => {
                movement::move_to_clock(self, payload);
            }
            EffectKind::MoveToMemory => {
                movement::move_to_memory(self, payload);
            }
            EffectKind::MoveToDeckBottom => {
                movement::move_to_deck_bottom(self, payload);
            }
            EffectKind::MoveWaitingRoomCardToSourceSlot => {
                movement::move_waiting_room_card_to_source_slot(self, payload);
            }
            EffectKind::RecycleWaitingRoomToDeckShuffle => {
                movement::recycle_waiting_room_to_deck_shuffle(self, controller);
            }
            EffectKind::ResetStockFromDeckTop { target } => {
                movement::reset_stock_from_deck_top(self, controller, *target);
            }
            EffectKind::MoveToMarker => {
                movement::move_to_marker(self, payload);
            }
            EffectKind::MoveTopDeckToMarker => {
                movement::move_top_deck_to_marker(self, payload);
            }
            EffectKind::Heal => {
                healing::heal(self, payload);
            }
            EffectKind::HealIfSourcePlayedFromHandThisTurn => {
                healing::heal_if_source_played_from_hand_this_turn(self, payload);
            }
            EffectKind::MoveTriggerCardToHand => {
                trigger::move_trigger_card_to_hand(self, controller, source_id);
            }
            EffectKind::MoveTriggerCardToStock => {
                trigger::move_trigger_card_to_stock(self, controller);
            }
            EffectKind::MillTop { target, count } => {
                movement::mill_top(self, controller, *target, *count);
            }
            EffectKind::MoveStageSlot { slot } => {
                movement::move_stage_slot(self, payload, *slot);
            }
            EffectKind::MoveThisToOpenCenter { require_facing } => {
                movement::move_this_to_open_center(self, payload, *require_facing);
            }
            EffectKind::MoveThisToOpenBack => {
                movement::move_this_to_open_back(self, payload);
            }
            EffectKind::SwapStageSlots => {
                movement::swap_stage_slots(self, payload);
            }
            EffectKind::ChangeController { new_controller } => {
                movement::change_controller(self, controller, payload, *new_controller);
            }
            EffectKind::Standby { target_slot } => {
                trigger::standby(self, controller, payload, *target_slot);
            }
            EffectKind::TreasureStock { take_stock } => {
                trigger::treasure_stock(self, controller, *take_stock);
            }
            EffectKind::ModifyPendingAttackDamage { delta } => {
                damage::modify_pending_attack_damage(self, *delta);
            }
            EffectKind::EnableShotDamage { amount } => {
                damage::enable_shot_damage(self, *amount);
            }
            EffectKind::SetTriggerCheckCount { count } => {
                trigger::set_trigger_check_count(self, *count);
            }
            EffectKind::RestThisIfNoOtherRestCenter => {
                movement::rest_this_if_no_other_rest_center(self, payload);
            }
            EffectKind::RestTarget => {
                movement::rest_target(self, payload);
            }
            EffectKind::StandTarget => {
                movement::stand_target(self, payload);
            }
            EffectKind::StockCharge { count } => {
                movement::stock_charge(self, controller, *count);
            }
            EffectKind::CannotUseAutoEncoreForPlayer { target } => {
                modifiers::cannot_use_auto_encore_for_player(self, controller, *target);
            }
            EffectKind::TriggerIcon { icon } => {
                trigger::trigger_icon(self, controller, source_id, payload, *icon);
            }
            EffectKind::BattleOpponentReverseIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_reverse_if(
                    self,
                    payload,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentMoveToDeckBottomIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_move_to_deck_bottom_if(
                    self,
                    payload,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_move_to_stock_then_bottom_stock_to_waiting_room_if(
                    self,
                    payload,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_move_to_clock_after_clock_top_to_waiting_room_if(
                    self,
                    payload,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentMoveToMemoryIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_move_to_memory_if(
                    self,
                    payload,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentMoveToClockIf {
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_move_to_clock_if(
                    self,
                    payload,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentMove {
                destination,
                prelude,
                max_level,
                max_cost,
                level_gt_opponent_level,
            } => {
                battle::battle_opponent_move(
                    self,
                    payload,
                    *destination,
                    *prelude,
                    *max_level,
                    *max_cost,
                    *level_gt_opponent_level,
                );
            }
            EffectKind::BattleOpponentTopDeckToStockIf { min_level } => {
                battle::battle_opponent_top_deck_to_stock_if(self, controller, payload, *min_level);
            }
            EffectKind::CounterBackup { power } => {
                damage::counter_backup(self, controller, source_id, *power);
            }
            EffectKind::CounterDamageReduce { amount } => {
                damage::counter_damage_reduce(self, source_id, *amount);
            }
            EffectKind::CounterDamageCancel => {
                damage::counter_damage_cancel(self, source_id);
            }
            EffectKind::SetTerminalOutcome { outcome } => {
                terminal::set_terminal_outcome(self, controller, *outcome);
            }
            EffectKind::ApplyRuleOverride { kind } => {
                terminal::apply_rule_override(self, *kind);
            }
        }
    }
}
