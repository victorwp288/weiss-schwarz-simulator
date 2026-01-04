use weiss_core::encode::*;
use weiss_core::state::REVEAL_HISTORY_LEN;

#[test]
fn observation_layout_constants_are_stable() {
    assert_eq!(OBS_ENCODING_VERSION, 1);
    assert_eq!(OBS_HEADER_LEN, 16);
    assert_eq!(OBS_REASON_LEN, 8);
    assert_eq!(OBS_REVEAL_LEN, REVEAL_HISTORY_LEN);
    assert_eq!(OBS_CONTEXT_LEN, 4);
    assert_eq!(PER_PLAYER_COUNTS, 9);
    assert_eq!(PER_STAGE_SLOT, 5);
    assert_eq!(MAX_STAGE, 5);
    assert_eq!(PER_PLAYER_STAGE, 25);
    assert_eq!(PER_PLAYER_CLIMAX_TOP, 1);
    assert_eq!(PER_PLAYER_LEVEL, 4);
    assert_eq!(PER_PLAYER_CLOCK_TOP, 7);
    assert_eq!(PER_PLAYER_WAITING_TOP, 5);
    assert_eq!(PER_PLAYER_RESOLUTION_TOP, 5);
    assert_eq!(PER_PLAYER_STOCK_TOP, 5);
    assert_eq!(PER_PLAYER_HAND, 50);
    assert_eq!(PER_PLAYER_DECK, 50);
    assert_eq!(
        PER_PLAYER_BLOCK_LEN,
        PER_PLAYER_COUNTS
            + PER_PLAYER_STAGE
            + PER_PLAYER_CLIMAX_TOP
            + PER_PLAYER_LEVEL
            + PER_PLAYER_CLOCK_TOP
            + PER_PLAYER_WAITING_TOP
            + PER_PLAYER_RESOLUTION_TOP
            + PER_PLAYER_STOCK_TOP
            + PER_PLAYER_HAND
            + PER_PLAYER_DECK
    );
    assert_eq!(OBS_REASON_BASE, OBS_HEADER_LEN + 2 * PER_PLAYER_BLOCK_LEN);
    assert_eq!(OBS_REVEAL_BASE, OBS_REASON_BASE + OBS_REASON_LEN);
    assert_eq!(OBS_CONTEXT_BASE, OBS_REVEAL_BASE + OBS_REVEAL_LEN);
    assert_eq!(OBS_LEN, OBS_CONTEXT_BASE + OBS_CONTEXT_LEN);
}
