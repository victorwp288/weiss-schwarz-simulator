use serde::{Deserialize, Serialize};

use crate::db::CardId;

/// Max number of reveal history entries tracked per player.
pub const REVEAL_HISTORY_LEN: usize = 8;

/// Ring buffer of recently revealed cards.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct RevealHistory {
    entries: [CardId; REVEAL_HISTORY_LEN],
    len: u8,
    head: u8,
}

impl RevealHistory {
    /// Create an empty reveal history.
    pub fn new() -> Self {
        Self {
            entries: [0; REVEAL_HISTORY_LEN],
            len: 0,
            head: 0,
        }
    }

    /// Push a newly revealed card into the history.
    pub fn push(&mut self, card: CardId) {
        let head = self.head as usize;
        self.entries[head] = card;
        if (self.len as usize) < REVEAL_HISTORY_LEN {
            self.len = self.len.saturating_add(1);
        }
        self.head = ((head + 1) % REVEAL_HISTORY_LEN) as u8;
    }

    /// Write entries in chronological order into `out`.
    pub fn write_chronological(&self, out: &mut [i32]) {
        out.fill(0);
        let len = self.len as usize;
        if len == 0 {
            return;
        }
        let start = if len < REVEAL_HISTORY_LEN {
            0
        } else {
            self.head as usize
        };
        for idx in 0..len.min(out.len()) {
            let entry_idx = if len < REVEAL_HISTORY_LEN {
                idx
            } else {
                (start + idx) % REVEAL_HISTORY_LEN
            };
            out[idx] = self.entries[entry_idx] as i32;
        }
    }
}

impl Default for RevealHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{RevealHistory, REVEAL_HISTORY_LEN};

    #[test]
    fn reveal_history_chronology_before_wrap() {
        let mut history = RevealHistory::new();
        history.push(10);
        history.push(20);
        history.push(30);
        let mut out = [0i32; REVEAL_HISTORY_LEN];
        history.write_chronological(&mut out);
        assert_eq!(&out[..3], &[10, 20, 30]);
        assert!(out[3..].iter().all(|entry| *entry == 0));
    }

    #[test]
    fn reveal_history_chronology_after_wrap_keeps_latest_entries() {
        let mut history = RevealHistory::new();
        for card in 1..=(REVEAL_HISTORY_LEN as u32 + 3) {
            history.push(card);
        }
        let mut out = [0i32; REVEAL_HISTORY_LEN];
        history.write_chronological(&mut out);
        let expected: Vec<i32> = (4..=(REVEAL_HISTORY_LEN as i32 + 3)).collect();
        assert_eq!(out.as_slice(), expected.as_slice());
    }
}
