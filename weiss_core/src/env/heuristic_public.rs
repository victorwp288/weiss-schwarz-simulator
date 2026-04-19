use crate::encode::{MAX_STAGE, PASS_ACTION_ID};
use crate::legal::{ActionDesc, DecisionKind};
use crate::state::{AttackType, ModifierKind};

use super::GameEnv;

const FRONT_ROW_SLOTS: [usize; 3] = [0, 1, 2];

#[derive(Clone, Copy, Debug, Default)]
struct StageSlotPublicScore {
    occupied: bool,
    has_attacked: bool,
    power: i32,
    effective_soul: i32,
    side_attack_allowed: bool,
}

#[derive(Clone, Copy, Debug)]
struct PublicBoardScore {
    self_level_count: i32,
    self_clock_count: i32,
    self_stage: [StageSlotPublicScore; MAX_STAGE],
    opponent_stage: [StageSlotPublicScore; MAX_STAGE],
    choice_page_start: i32,
    choice_total: i32,
}

fn slot_preference(slot: usize) -> i64 {
    match slot {
        0 => 20,
        1 => 30,
        2 => 15,
        3 => 8,
        4 => 6,
        _ => 0,
    }
}

fn prefer_lower(value: u8) -> i64 {
    -(value as i64)
}

impl GameEnv {
    pub(crate) fn choose_heuristic_public_action_id(&mut self) -> u16 {
        let Some(decision) = self.decision.as_ref() else {
            return PASS_ACTION_ID as u16;
        };
        let player = decision.player as usize;
        if self.action_ids_cache().is_empty() {
            return PASS_ACTION_ID as u16;
        }
        self.refresh_slot_power_cache();
        let board = self.heuristic_public_board(player);
        let mut best_action_id = PASS_ACTION_ID as u16;
        let mut best_score: Option<(i64, i64, i64, i64, i64)> = None;
        for &action_id in self.action_ids_cache() {
            let score = self.heuristic_public_score_action(action_id as usize, &board);
            let candidate = (score.0, score.1, score.2, score.3, -(action_id as i64));
            if best_score.is_none_or(|current| candidate > current) {
                best_score = Some(candidate);
                best_action_id = action_id;
            }
        }
        best_action_id
    }

    fn heuristic_public_board(&mut self, player: usize) -> PublicBoardScore {
        let opponent = 1usize.saturating_sub(player);
        let (choice_page_start, choice_total) =
            match (self.decision.as_ref(), self.state.turn.choice.as_ref()) {
                (Some(decision), Some(choice)) if decision.kind == DecisionKind::Choice => {
                    (choice.page_start as i32, choice.total_candidates as i32)
                }
                _ => (0, 0),
            };
        PublicBoardScore {
            self_level_count: self.state.players[player].level.len() as i32,
            self_clock_count: self.state.players[player].clock.len() as i32,
            self_stage: self.heuristic_public_stage(player),
            opponent_stage: self.heuristic_public_stage(opponent),
            choice_page_start,
            choice_total,
        }
    }

    fn heuristic_public_stage(&mut self, player: usize) -> [StageSlotPublicScore; MAX_STAGE] {
        let mut slot_card_ids = [0u32; MAX_STAGE];
        let mut slot_soul_mods = [0i32; MAX_STAGE];
        let mut slot_side_attack_allowed = [false; MAX_STAGE];
        for slot in 0..MAX_STAGE {
            let slot_state = &self.state.players[player].stage[slot];
            let card_id = slot_state.card.map(|card| card.id).unwrap_or(0);
            slot_card_ids[slot] = card_id;
            slot_side_attack_allowed[slot] = card_id != 0;
        }
        let use_derived_attack = self.state.turn.derived_attack.is_some();
        if !self.state.modifiers.is_empty() {
            for modifier in &self.state.modifiers {
                if modifier.target_player as usize != player {
                    continue;
                }
                let slot = modifier.target_slot as usize;
                if slot >= MAX_STAGE {
                    continue;
                }
                let card_id = slot_card_ids[slot];
                if card_id == 0 || modifier.target_card != card_id {
                    continue;
                }
                match modifier.kind {
                    ModifierKind::Soul => {
                        slot_soul_mods[slot] =
                            slot_soul_mods[slot].saturating_add(modifier.magnitude);
                    }
                    ModifierKind::CannotSideAttack
                        if !use_derived_attack && modifier.magnitude != 0 =>
                    {
                        slot_side_attack_allowed[slot] = false;
                    }
                    _ => {}
                }
            }
        }
        if let Some(derived) = self.state.turn.derived_attack.as_ref() {
            for (slot, card_id) in slot_card_ids.iter().enumerate() {
                if *card_id == 0 {
                    continue;
                }
                slot_side_attack_allowed[slot] =
                    !derived.per_player[player][slot].cannot_side_attack;
            }
        }

        let mut stage = [StageSlotPublicScore::default(); MAX_STAGE];
        for slot in 0..MAX_STAGE {
            let (card_inst, has_attacked) = {
                let slot_state = &self.state.players[player].stage[slot];
                (slot_state.card, slot_state.has_attacked)
            };
            let Some(card_inst) = card_inst else {
                continue;
            };
            let soul = self.db.soul_by_id(card_inst.id) as i32;
            stage[slot] = StageSlotPublicScore {
                occupied: true,
                has_attacked,
                power: self.compute_slot_power(player, slot),
                effective_soul: soul.saturating_add(slot_soul_mods[slot]).max(0),
                side_attack_allowed: slot_side_attack_allowed[slot],
            };
        }
        stage
    }

    fn heuristic_public_score_action(
        &self,
        action_id: usize,
        board: &PublicBoardScore,
    ) -> (i64, i64, i64, i64) {
        let Some(action) = self.action_for_id(action_id) else {
            return (-1000, 0, 0, 0);
        };
        match action {
            ActionDesc::Attack { slot, attack_type } => (
                900,
                self.heuristic_public_score_attack(slot as usize, attack_type, board),
                0,
                0,
            ),
            ActionDesc::EncorePay { slot } => (
                700,
                self.heuristic_public_score_slot(slot as usize, &board.self_stage),
                0,
                0,
            ),
            ActionDesc::MainPlayCharacter {
                hand_index,
                stage_slot,
            } => (
                650,
                self.heuristic_public_score_play_character(stage_slot as usize, board),
                prefer_lower(hand_index),
                0,
            ),
            ActionDesc::ClimaxPlay { hand_index } => (
                550,
                self.heuristic_public_score_climax(board),
                prefer_lower(hand_index),
                0,
            ),
            ActionDesc::Clock { hand_index } => (
                500,
                self.heuristic_public_score_clock(board),
                prefer_lower(hand_index),
                0,
            ),
            ActionDesc::MainPlayEvent { hand_index } => (320, 10, prefer_lower(hand_index), 0),
            ActionDesc::ChoiceSelect { index } => (300, prefer_lower(index), 0, 0),
            ActionDesc::LevelUp { index } => (290, prefer_lower(index), 0, 0),
            ActionDesc::TriggerOrder { index } => (280, prefer_lower(index), 0, 0),
            ActionDesc::MulliganConfirm => (260, 0, 0, 0),
            ActionDesc::MainMove { from_slot, to_slot } => (
                120,
                self.heuristic_public_score_move(from_slot as usize, to_slot as usize, board),
                0,
                0,
            ),
            ActionDesc::ChoiceNextPage => {
                let remaining = (board.choice_total - (board.choice_page_start + 16)).max(0) as i64;
                (170, remaining, 0, 0)
            }
            ActionDesc::ChoicePrevPage => (170, board.choice_page_start.max(0) as i64, 0, 0),
            ActionDesc::Pass => (160, 0, 0, 0),
            ActionDesc::MulliganSelect { hand_index } => (120, prefer_lower(hand_index), 0, 0),
            ActionDesc::EncoreDecline { slot } => (
                110,
                self.heuristic_public_score_slot(slot as usize, &board.self_stage),
                0,
                0,
            ),
            ActionDesc::Concede => (-1000, 0, 0, 0),
            _ => (-1000, 0, 0, 0),
        }
    }

    fn heuristic_public_score_attack(
        &self,
        slot: usize,
        attack_type: AttackType,
        board: &PublicBoardScore,
    ) -> i64 {
        let attacker = board.self_stage.get(slot).copied().unwrap_or_default();
        let defender = board.opponent_stage.get(slot).copied().unwrap_or_default();
        if !attacker.occupied {
            return -1000;
        }
        let type_score = match attack_type {
            AttackType::Direct => {
                if defender.occupied {
                    15
                } else {
                    60
                }
            }
            AttackType::Frontal => {
                if attacker.power >= defender.power {
                    45
                } else {
                    25
                }
            }
            AttackType::Side => {
                if attacker.side_attack_allowed {
                    40
                } else {
                    5
                }
            }
        };
        type_score
            + slot_preference(slot)
            + (attacker.effective_soul.max(0) as i64) * 4
            + (attacker.power.max(0) as i64) / 1000
    }

    fn heuristic_public_score_play_character(&self, slot: usize, board: &PublicBoardScore) -> i64 {
        let stage = board.self_stage.get(slot).copied().unwrap_or_default();
        if stage.occupied {
            return -1000;
        }
        let bonus = slot_preference(slot);
        if FRONT_ROW_SLOTS.contains(&slot) {
            return 40 + bonus;
        }
        if slot < MAX_STAGE {
            return 20 + bonus;
        }
        bonus
    }

    fn heuristic_public_score_move(
        &self,
        from_slot: usize,
        to_slot: usize,
        board: &PublicBoardScore,
    ) -> i64 {
        let origin = board.self_stage.get(from_slot).copied().unwrap_or_default();
        let target = board.self_stage.get(to_slot).copied().unwrap_or_default();
        if !origin.occupied || target.occupied {
            return -1000;
        }
        let mut bonus = 0;
        if from_slot >= 3 && FRONT_ROW_SLOTS.contains(&to_slot) {
            bonus += 30;
        }
        if to_slot == 1 && from_slot != 1 {
            bonus += 15;
        }
        (slot_preference(to_slot) - slot_preference(from_slot)) + bonus
    }

    fn heuristic_public_score_climax(&self, board: &PublicBoardScore) -> i64 {
        let attackers = FRONT_ROW_SLOTS
            .iter()
            .filter(|&&slot| {
                let stage = board.self_stage[slot];
                stage.occupied && !stage.has_attacked
            })
            .count() as i64;
        let defenders = FRONT_ROW_SLOTS
            .iter()
            .filter(|&&slot| board.opponent_stage[slot].occupied)
            .count() as i64;
        attackers * 10 + defenders * 4 + if attackers > 0 { 10 } else { -20 }
    }

    fn heuristic_public_score_clock(&self, board: &PublicBoardScore) -> i64 {
        if board.self_level_count <= 0 && board.self_clock_count < 6 {
            return (40 - board.self_clock_count) as i64;
        }
        10
    }

    fn heuristic_public_score_slot(
        &self,
        slot: usize,
        stage: &[StageSlotPublicScore; MAX_STAGE],
    ) -> i64 {
        let slot_state = stage.get(slot).copied().unwrap_or_default();
        slot_preference(slot) + (slot_state.power.max(0) as i64) / 1000
    }
}
