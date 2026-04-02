//! Fixed-point format descriptor.
//!
//! [`SampleFormat`] tracks how many total bits and fractional bits
//! a sample occupies at a given pipeline boundary.  This is metadata,
//! not stored per-sample.

use crate::interpret::signal::{BitWidth, FractionalBits};

/// Describes the fixed-point interpretation of samples at a pipeline
/// boundary.
///
/// # Examples
///
/// ```
/// use dsp_cat::sample::format::SampleFormat;
/// use dsp_cat::interpret::signal::{BitWidth, FractionalBits};
///
/// let q15 = SampleFormat::new(BitWidth::new(32), FractionalBits::new(15));
/// assert_eq!(q15.total_bits().value(), 32);
/// assert_eq!(q15.fractional_bits().value(), 15);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SampleFormat {
    total_bits: BitWidth,
    fractional_bits: FractionalBits,
}

impl SampleFormat {
    /// Create a new sample format.
    pub fn new(total_bits: BitWidth, fractional_bits: FractionalBits) -> Self {
        Self {
            total_bits,
            fractional_bits,
        }
    }

    /// Total bit width.
    pub fn total_bits(self) -> BitWidth {
        self.total_bits
    }

    /// Number of fractional bits.
    pub fn fractional_bits(self) -> FractionalBits {
        self.fractional_bits
    }

    /// Integer bits (total minus fractional minus sign bit).
    #[must_use]
    pub fn integer_bits(self) -> usize {
        self.total_bits
            .value()
            .saturating_sub(self.fractional_bits.value())
            .saturating_sub(1)
    }

    /// Widen the format by additional bits (e.g., CIC bit growth).
    pub fn widen(self, extra_bits: usize) -> Self {
        Self {
            total_bits: BitWidth::new(self.total_bits.value() + extra_bits),
            fractional_bits: self.fractional_bits,
        }
    }
}

impl core::fmt::Display for SampleFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Q{}.{}",
            self.total_bits
                .value()
                .saturating_sub(self.fractional_bits.value())
                .saturating_sub(1),
            self.fractional_bits.value()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q15_format_has_correct_integer_bits() {
        let fmt = SampleFormat::new(BitWidth::new(32), FractionalBits::new(15));
        assert_eq!(fmt.integer_bits(), 16);
    }

    #[test]
    fn widen_adds_total_bits() {
        let fmt = SampleFormat::new(BitWidth::new(32), FractionalBits::new(15));
        let wider = fmt.widen(8);
        assert_eq!(wider.total_bits().value(), 40);
        assert_eq!(wider.fractional_bits().value(), 15);
    }

    #[test]
    fn display_shows_q_notation() {
        let fmt = SampleFormat::new(BitWidth::new(32), FractionalBits::new(15));
        assert_eq!(fmt.to_string(), "Q16.15");
    }
}
