use super::super::GameEnv;

impl GameEnv {
    pub(in crate::env) fn treasure_stock_available(&self, player: u8) -> bool {
        let p = player as usize;
        self.state
            .players
            .get(p)
            .is_some_and(|state| !state.deck.is_empty() || !state.waiting_room.is_empty())
    }
}
