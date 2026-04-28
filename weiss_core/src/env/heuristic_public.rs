use crate::encode::{MAX_STAGE, PASS_ACTION_ID};
use crate::legal::{ActionDesc, DecisionKind};
use crate::state::{AttackType, ModifierKind};

use super::GameEnv;

const FRONT_ROW_SLOTS: [usize; 3] = [0, 1, 2];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HeuristicPublicProfile {
    Base,
    Aggressive,
    Control,
}

impl HeuristicPublicProfile {
    pub(crate) fn from_name(name: &str) -> anyhow::Result<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "" | "base" => Ok(Self::Base),
            "aggressive" => Ok(Self::Aggressive),
            "control" => Ok(Self::Control),
            other => anyhow::bail!(
                "unsupported heuristic public profile {other:?}; expected one of: base, aggressive, control"
            ),
        }
    }

    fn attack_priority(self) -> i64 {
        match self {
            Self::Base => 900,
            Self::Aggressive => 940,
            Self::Control => 870,
        }
    }

    fn play_priority(self) -> i64 {
        match self {
            Self::Control => 680,
            _ => 650,
        }
    }

    fn climax_priority(self) -> i64 {
        match self {
            Self::Base => 550,
            Self::Aggressive => 610,
            Self::Control => 505,
        }
    }

    fn move_priority(self) -> i64 {
        match self {
            Self::Base => 120,
            Self::Aggressive => 210,
            Self::Control => 195,
        }
    }

    fn pass_priority(self) -> i64 {
        match self {
            Self::Base => 160,
            Self::Aggressive => 115,
            Self::Control => 185,
        }
    }
}

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
    pub(crate) fn choose_heuristic_public_action_id_for_profile(
        &mut self,
        profile: HeuristicPublicProfile,
    ) -> u16 {
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
            let score = self.heuristic_public_score_action(action_id as usize, &board, profile);
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
        profile: HeuristicPublicProfile,
    ) -> (i64, i64, i64, i64) {
        let Some(action) = self.action_for_id(action_id) else {
            return (-1000, 0, 0, 0);
        };
        match action {
            ActionDesc::Attack { slot, attack_type } => (
                profile.attack_priority(),
                self.heuristic_public_score_attack(slot as usize, attack_type, board, profile),
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
                profile.play_priority(),
                self.heuristic_public_score_play_character(stage_slot as usize, board, profile),
                prefer_lower(hand_index),
                0,
            ),
            ActionDesc::ClimaxPlay { hand_index } => (
                profile.climax_priority(),
                self.heuristic_public_score_climax(board, profile),
                prefer_lower(hand_index),
                0,
            ),
            ActionDesc::Clock { hand_index } => (
                500,
                self.heuristic_public_score_clock(board, profile),
                prefer_lower(hand_index),
                0,
            ),
            ActionDesc::MainPlayEvent { hand_index } => (320, 10, prefer_lower(hand_index), 0),
            ActionDesc::ChoiceSelect { index } => (300, prefer_lower(index), 0, 0),
            ActionDesc::LevelUp { index } => (290, prefer_lower(index), 0, 0),
            ActionDesc::TriggerOrder { index } => (280, prefer_lower(index), 0, 0),
            ActionDesc::MulliganConfirm => (260, 0, 0, 0),
            ActionDesc::MainMove { from_slot, to_slot } => (
                profile.move_priority(),
                self.heuristic_public_score_move(
                    from_slot as usize,
                    to_slot as usize,
                    board,
                    profile,
                ),
                0,
                0,
            ),
            ActionDesc::ChoiceNextPage => {
                let remaining = (board.choice_total - (board.choice_page_start + 16)).max(0) as i64;
                (170, remaining, 0, 0)
            }
            ActionDesc::ChoicePrevPage => (170, board.choice_page_start.max(0) as i64, 0, 0),
            ActionDesc::Pass => (profile.pass_priority(), 0, 0, 0),
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
        profile: HeuristicPublicProfile,
    ) -> i64 {
        let attacker = board.self_stage.get(slot).copied().unwrap_or_default();
        let defender = board.opponent_stage.get(slot).copied().unwrap_or_default();
        if !attacker.occupied {
            return -1000;
        }
        let type_score = match attack_type {
            AttackType::Direct => {
                if defender.occupied {
                    match profile {
                        HeuristicPublicProfile::Base => 15,
                        HeuristicPublicProfile::Aggressive => 42,
                        HeuristicPublicProfile::Control => 0,
                    }
                } else {
                    match profile {
                        HeuristicPublicProfile::Base => 60,
                        HeuristicPublicProfile::Aggressive => 85,
                        HeuristicPublicProfile::Control => 38,
                    }
                }
            }
            AttackType::Frontal => {
                if attacker.power >= defender.power {
                    match profile {
                        HeuristicPublicProfile::Base => 45,
                        HeuristicPublicProfile::Aggressive => 40,
                        HeuristicPublicProfile::Control => 58,
                    }
                } else {
                    match profile {
                        HeuristicPublicProfile::Base => 25,
                        HeuristicPublicProfile::Aggressive => 12,
                        HeuristicPublicProfile::Control => 35,
                    }
                }
            }
            AttackType::Side => {
                if attacker.side_attack_allowed {
                    match profile {
                        HeuristicPublicProfile::Base => 40,
                        HeuristicPublicProfile::Aggressive => 18,
                        HeuristicPublicProfile::Control => 52,
                    }
                } else {
                    match profile {
                        HeuristicPublicProfile::Base => 5,
                        HeuristicPublicProfile::Aggressive => -10,
                        HeuristicPublicProfile::Control => 0,
                    }
                }
            }
        };
        let soul_scale = match profile {
            HeuristicPublicProfile::Base => 4,
            HeuristicPublicProfile::Aggressive => 7,
            HeuristicPublicProfile::Control => 2,
        };
        type_score
            + slot_preference(slot)
            + (attacker.effective_soul.max(0) as i64) * soul_scale
            + (attacker.power.max(0) as i64) / 1000
    }

    fn heuristic_public_score_play_character(
        &self,
        slot: usize,
        board: &PublicBoardScore,
        profile: HeuristicPublicProfile,
    ) -> i64 {
        let stage = board.self_stage.get(slot).copied().unwrap_or_default();
        if stage.occupied {
            return -1000;
        }
        let bonus = slot_preference(slot);
        if FRONT_ROW_SLOTS.contains(&slot) {
            let front_bonus = match profile {
                HeuristicPublicProfile::Base => 40,
                HeuristicPublicProfile::Aggressive => 60,
                HeuristicPublicProfile::Control => 22,
            };
            return front_bonus + bonus;
        }
        if slot < MAX_STAGE {
            let back_bonus = match profile {
                HeuristicPublicProfile::Base => 20,
                HeuristicPublicProfile::Aggressive => 6,
                HeuristicPublicProfile::Control => 38,
            };
            return back_bonus + bonus;
        }
        bonus
    }

    fn heuristic_public_score_move(
        &self,
        from_slot: usize,
        to_slot: usize,
        board: &PublicBoardScore,
        profile: HeuristicPublicProfile,
    ) -> i64 {
        let origin = board.self_stage.get(from_slot).copied().unwrap_or_default();
        let target = board.self_stage.get(to_slot).copied().unwrap_or_default();
        if !origin.occupied || target.occupied {
            return -1000;
        }
        let mut bonus = 0;
        if from_slot >= 3 && FRONT_ROW_SLOTS.contains(&to_slot) {
            bonus += match profile {
                HeuristicPublicProfile::Base => 30,
                HeuristicPublicProfile::Aggressive => 48,
                HeuristicPublicProfile::Control => 18,
            };
        }
        if to_slot == 1 && from_slot != 1 {
            bonus += match profile {
                HeuristicPublicProfile::Base => 15,
                HeuristicPublicProfile::Aggressive => 28,
                HeuristicPublicProfile::Control => 6,
            };
        }
        (slot_preference(to_slot) - slot_preference(from_slot)) + bonus
    }

    fn heuristic_public_score_climax(
        &self,
        board: &PublicBoardScore,
        profile: HeuristicPublicProfile,
    ) -> i64 {
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
        let attacker_scale = match profile {
            HeuristicPublicProfile::Base => 10,
            HeuristicPublicProfile::Aggressive => 16,
            HeuristicPublicProfile::Control => 6,
        };
        let defender_scale = match profile {
            HeuristicPublicProfile::Base => 4,
            HeuristicPublicProfile::Aggressive => 8,
            HeuristicPublicProfile::Control => 2,
        };
        let active_bonus = match profile {
            HeuristicPublicProfile::Base => 10,
            HeuristicPublicProfile::Aggressive => 18,
            HeuristicPublicProfile::Control => 6,
        };
        let inactive_bonus = match profile {
            HeuristicPublicProfile::Base => -20,
            HeuristicPublicProfile::Aggressive => -32,
            HeuristicPublicProfile::Control => -8,
        };
        attackers * attacker_scale
            + defenders * defender_scale
            + if attackers > 0 {
                active_bonus
            } else {
                inactive_bonus
            }
    }

    fn heuristic_public_score_clock(
        &self,
        board: &PublicBoardScore,
        profile: HeuristicPublicProfile,
    ) -> i64 {
        if board.self_level_count <= 0 && board.self_clock_count < 6 {
            let early_clock = match profile {
                HeuristicPublicProfile::Base => 40,
                HeuristicPublicProfile::Aggressive => 18,
                HeuristicPublicProfile::Control => 48,
            };
            return (early_clock - board.self_clock_count) as i64;
        }
        match profile {
            HeuristicPublicProfile::Base => 10,
            HeuristicPublicProfile::Aggressive => 4,
            HeuristicPublicProfile::Control => 14,
        }
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
