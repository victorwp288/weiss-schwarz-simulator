use super::GameEnv;
use crate::legal::{Decision, DecisionKind};
use crate::state::{ChoiceOptionRef, ChoiceReason, ChoiceState, ChoiceZone};

/// Install a synthetic choice state with paging for tests.
pub fn install_choice_paging(env: &mut GameEnv, total: usize) {
    assert!(
        total <= u16::MAX as usize,
        "choice paging total {total} exceeds u16::MAX ({})",
        u16::MAX
    );
    let options = (0..total)
        .map(|idx| ChoiceOptionRef {
            card_id: (idx + 1) as u32,
            instance_id: (idx + 1) as u32,
            zone: ChoiceZone::WaitingRoom,
            index: Some(idx as u16),
            target_slot: None,
        })
        .collect::<Vec<_>>();
    env.state.turn.choice = Some(ChoiceState {
        id: 1,
        reason: ChoiceReason::TriggerTreasureSelect,
        player: 0,
        options,
        total_candidates: total as u16,
        page_start: 0,
        pending_trigger: None,
    });
    env.decision = Some(Decision {
        player: 0,
        kind: DecisionKind::Choice,
        focus_slot: None,
    });
    env.update_action_cache();
}

/// Force a deck refresh for a player (test harness only).
pub fn force_deck_refresh(env: &mut GameEnv, player: usize) {
    let deck = std::mem::take(&mut env.state.players[player].deck);
    env.state.players[player].waiting_room.extend(deck);
    env.update_action_cache();
}
