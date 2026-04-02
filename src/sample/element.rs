//! Fixed-point sample newtype.
//!
//! [`Sample`] wraps `i32` with saturating arithmetic appropriate for
//! DSP pipelines.  The fractional-point position is tracked externally
//! by [`SampleFormat`](super::format::SampleFormat), so `Sample`
//! carries zero per-element overhead.

/// A fixed-point DSP sample stored as `i32`.
///
/// All arithmetic saturates rather than wrapping, which is the
/// standard DSP convention (clipping is preferable to wrap-around
/// distortion).
///
/// # Examples
///
/// ```
/// use dsp_cat::sample::element::Sample;
///
/// let a = Sample::new(100);
/// let b = Sample::new(200);
/// let sum = a + b;
/// assert_eq!(sum.value(), 300);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct Sample(i32);

impl Sample {
    /// The zero sample (additive identity).
    pub const ZERO: Self = Self(0);

    /// The unit sample (multiplicative identity in Q0 format).
    pub const ONE: Self = Self(1);

    /// Create a sample from a raw `i32` value.
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    /// The underlying `i32` value.
    #[must_use]
    pub fn value(self) -> i32 {
        self.0
    }

    /// Saturating addition.
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Widening multiply returning `i64`.
    ///
    /// Preserves full precision for accumulation in MAC units.
    /// The caller is responsible for truncating back to `i32`
    /// at the appropriate point.
    #[must_use]
    pub fn widening_mul(self, rhs: Self) -> i64 {
        i64::from(self.0) * i64::from(rhs.0)
    }
}

impl core::fmt::Display for Sample {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Sample({})", self.0)
    }
}

impl core::ops::Add for Sample {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl core::ops::Sub for Sample {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl core::ops::Neg for Sample {
    type Output = Self;

    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

impl From<i32> for Sample {
    fn from(value: i32) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_additive_identity() {
        let s = Sample::new(42);
        assert_eq!(s + Sample::ZERO, s);
        assert_eq!(Sample::ZERO + s, s);
    }

    #[test]
    fn add_is_commutative() {
        let a = Sample::new(10);
        let b = Sample::new(20);
        assert_eq!(a + b, b + a);
    }

    #[test]
    fn add_saturates_at_max() {
        let a = Sample::new(i32::MAX);
        let b = Sample::new(1);
        assert_eq!((a + b).value(), i32::MAX);
    }

    #[test]
    fn sub_saturates_at_min() {
        let a = Sample::new(i32::MIN);
        let b = Sample::new(1);
        assert_eq!((a - b).value(), i32::MIN);
    }

    #[test]
    fn negation_of_zero_is_zero() {
        assert_eq!(-Sample::ZERO, Sample::ZERO);
    }

    #[test]
    fn negation_round_trip() {
        let s = Sample::new(42);
        assert_eq!(-(-s), s);
    }

    #[test]
    fn widening_mul_preserves_precision() {
        let a = Sample::new(1_000_000);
        let b = Sample::new(1_000_000);
        assert_eq!(a.widening_mul(b), 1_000_000_000_000_i64);
    }

    #[test]
    fn from_i32_round_trips() {
        let v = 12345_i32;
        let s = Sample::from(v);
        assert_eq!(s.value(), v);
    }

    #[test]
    fn display_shows_value() {
        let s = Sample::new(-7);
        let text = s.to_string();
        assert_eq!(text, "Sample(-7)");
    }
}
