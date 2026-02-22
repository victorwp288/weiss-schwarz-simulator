use std::collections::HashSet;

use crate::config::CurriculumConfig;
use crate::db::{CardColor, CardDb, CardStatic};
use crate::state::PlayerState;

/// Check whether a card is allowed under the current curriculum and optional explicit allow-list.
#[inline(always)]
pub(crate) fn card_set_allowed(
    card: &CardStatic,
    curriculum: &CurriculumConfig,
    allowed_card_sets: Option<&HashSet<String>>,
) -> bool {
    if let Some(set) = allowed_card_sets.or(curriculum.allowed_card_sets_cache.as_ref()) {
        return match &card.card_set {
            Some(set_id) => set.contains(set_id),
            None => false,
        };
    }
    if curriculum.allowed_card_sets.is_empty() {
        true
    } else {
        card.card_set
            .as_ref()
            .map(|s| curriculum.allowed_card_sets.iter().any(|a| a == s))
            .unwrap_or(false)
    }
}

/// Check the level requirement for playing a card from hand.
#[inline(always)]
pub(crate) fn meets_level_requirement(
    card: &CardStatic,
    level_count: usize,
    level_delta: i32,
) -> bool {
    let required_level = (card.level as i32).saturating_add(level_delta).max(0) as usize;
    required_level <= level_count
}

/// Check the stock cost requirement for playing a card from hand.
#[inline(always)]
pub(crate) fn meets_cost_requirement(
    card: &CardStatic,
    stock_count: usize,
    enforce_cost_requirement: bool,
) -> bool {
    !enforce_cost_requirement || stock_count >= card.cost as usize
}

/// Check the color requirement for playing a card from hand.
#[inline(always)]
pub(crate) fn meets_color_requirement(
    card: &CardStatic,
    player: &PlayerState,
    db: &CardDb,
    enforce_color_requirement: bool,
    ignore_color_requirement: bool,
) -> bool {
    if ignore_color_requirement || !enforce_color_requirement {
        return true;
    }
    if card.level == 0 || card.color == CardColor::Colorless {
        return true;
    }
    for card_id in player.level.iter().chain(player.clock.iter()) {
        if let Some(c) = db.get(card_id.id) {
            if c.color == card.color {
                return true;
            }
        }
    }
    false
}

/// Check all requirements for playing a card from hand under the current curriculum.
#[inline(always)]
pub(crate) fn meets_play_requirements(
    card: &CardStatic,
    player: &PlayerState,
    db: &CardDb,
    curriculum: &CurriculumConfig,
    level_delta: i32,
    ignore_color_requirement: bool,
) -> bool {
    meets_level_requirement(card, player.level.len(), level_delta)
        && meets_cost_requirement(
            card,
            player.stock.len(),
            curriculum.enforce_cost_requirement,
        )
        && meets_color_requirement(
            card,
            player,
            db,
            curriculum.enforce_color_requirement,
            ignore_color_requirement,
        )
}
