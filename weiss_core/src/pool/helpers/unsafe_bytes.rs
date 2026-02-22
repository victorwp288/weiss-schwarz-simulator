//! Local `unsafe` utilities for hashing integer buffers.
//!
//! These helpers convert integer slices to their raw byte representation. This is used only for
//! hashing/fingerprinting debug outputs. Hashes remain **native-endian**, matching the prior
//! behavior that hashed raw memory bytes.

#[inline]
pub(super) fn i32_slice_as_bytes(slice: &[i32]) -> &[u8] {
    let len = slice.len().saturating_mul(std::mem::size_of::<i32>());
    // SAFETY:
    // - `slice.as_ptr()` is valid for `slice.len()` elements.
    // - `i32` has no padding bytes and is plain data.
    // - The returned byte slice is read-only and its lifetime is tied to `slice`.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, len) }
}

#[inline]
pub(super) fn u64_slice_as_bytes(slice: &[u64]) -> &[u8] {
    let len = slice.len().saturating_mul(std::mem::size_of::<u64>());
    // SAFETY: Same invariants as `i32_slice_as_bytes`.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, len) }
}

#[inline]
pub(super) fn u16_slice_as_bytes(slice: &[u16]) -> &[u8] {
    let len = slice.len().saturating_mul(std::mem::size_of::<u16>());
    // SAFETY: Same invariants as `i32_slice_as_bytes`.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, len) }
}
