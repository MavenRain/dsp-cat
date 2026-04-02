//! Shared HDL constants and bit-conversion helpers.

use rust_hdl::prelude::*;

/// Sample width in bits.
pub const SAMPLE_WIDTH: usize = 32;

/// Convert a `Bits<32>` signal value to `i32`.
///
/// Reconstructs the value via `get_bit`, then interprets as signed.
#[must_use]
pub fn bits_to_i32(b: Bits<SAMPLE_WIDTH>) -> i32 {
    let raw = (0..SAMPLE_WIDTH).fold(0_u32, |acc, i| {
        acc | (u32::from(b.get_bit(i)) << i)
    });
    raw.cast_signed()
}

/// Clamp an `i64` to `i32` range and truncate.
///
/// The value is clamped to `[i32::MIN, i32::MAX]` before
/// narrowing, so the truncation is lossless.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn clamp_to_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Convert an `i32` to `Bits<32>`.
#[must_use]
pub fn i32_to_bits(v: i32) -> Bits<SAMPLE_WIDTH> {
    let raw = v.cast_unsigned();
    bits(u64::from(raw))
}
