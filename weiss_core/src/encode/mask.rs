use crate::legal::ActionDesc;

use super::action_ids::action_id_for;
use super::constants::*;

/// Fill a dense action mask and lookup table from a legal action list.
pub fn fill_action_mask(
    actions: &[ActionDesc],
    mask: &mut [u8],
    lookup: &mut [Option<ActionDesc>],
) {
    mask.fill(0);
    for slot in lookup.iter_mut() {
        *slot = None;
    }
    for action in actions {
        if let Some(id) = action_id_for(action) {
            let max_len = mask.len().min(lookup.len());
            if id < max_len {
                mask[id] = 1;
                lookup[id] = Some(action.clone());
            }
        }
    }
}

/// Update sparse action mask buffers from a legal action list.
pub fn fill_action_mask_sparse(
    actions: &[ActionDesc],
    mask: &mut [u8],
    last_action_ids: &mut Vec<u16>,
    mask_bits: &mut [u64],
    write_mask: bool,
) {
    debug_assert!(
        mask_bits.len().saturating_mul(64) >= mask.len(),
        "mask_bits must cover all mask entries"
    );
    debug_assert!(
        mask.len() <= u16::MAX as usize,
        "sparse mask ids require mask.len() <= u16::MAX"
    );
    for &id_u16 in last_action_ids.iter() {
        let id = id_u16 as usize;
        if id < mask.len() {
            if write_mask {
                mask[id] = 0;
            }
            let word = id / 64;
            let bit = id % 64;
            if word < mask_bits.len() {
                mask_bits[word] &= !(1u64 << bit);
            }
        }
    }
    last_action_ids.clear();
    for action in actions.iter() {
        if let Some(id) = action_id_for(action) {
            if id < mask.len() {
                let id_u16 = u16::try_from(id).expect("action id out of u16 range");
                if write_mask {
                    mask[id] = 1;
                }
                last_action_ids.push(id_u16);
                let word = id / 64;
                let bit = id % 64;
                if word < mask_bits.len() {
                    mask_bits[word] |= 1u64 << bit;
                }
            }
        }
    }
}

/// Build a dense action mask and lookup table from a legal action list.
pub fn build_action_mask(actions: &[ActionDesc]) -> (Vec<u8>, Vec<Option<ActionDesc>>) {
    let mut mask = vec![0u8; ACTION_SPACE_SIZE];
    let mut lookup = vec![None; ACTION_SPACE_SIZE];
    fill_action_mask(actions, &mut mask, &mut lookup);
    (mask, lookup)
}
