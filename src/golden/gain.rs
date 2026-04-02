//! Pure gain (scale) golden model.

use crate::interpret::signal::GainCoefficient;
use crate::sample::element::Sample;

/// Multiply each sample by a gain coefficient.
///
/// Uses widening multiply to `i64`, then truncates by right-shifting
/// by `shift` fractional bits.  A `shift` of 0 means the coefficient
/// is an integer multiplier.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::gain::apply_gain;
/// use dsp_cat::interpret::signal::GainCoefficient;
/// use dsp_cat::sample::element::Sample;
///
/// let input = vec![Sample::new(100), Sample::new(-50)];
/// let output = apply_gain(&input, GainCoefficient::new(2), 0);
/// assert_eq!(output[0].value(), 200);
/// assert_eq!(output[1].value(), -100);
/// ```
#[must_use]
pub fn apply_gain(input: &[Sample], coefficient: GainCoefficient, shift: u32) -> Vec<Sample> {
    let coeff = i64::from(coefficient.value());
    input
        .iter()
        .map(|s| {
            let product = i64::from(s.value()) * coeff;
            let shifted = product >> shift;
            Sample::new(i32::try_from(shifted.clamp(i64::from(i32::MIN), i64::from(i32::MAX)))
                .unwrap_or(i32::MAX))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gain_is_identity() {
        let input = vec![Sample::new(42), Sample::new(-7)];
        let output = apply_gain(&input, GainCoefficient::new(1), 0);
        assert_eq!(output, input);
    }

    #[test]
    fn zero_gain_produces_zeros() {
        let input = vec![Sample::new(100), Sample::new(-100)];
        let output = apply_gain(&input, GainCoefficient::new(0), 0);
        assert_eq!(output, vec![Sample::ZERO, Sample::ZERO]);
    }

    #[test]
    fn gain_with_shift() {
        let input = vec![Sample::new(1000)];
        // coefficient = 32768, shift = 15 => effective gain ~ 1.0
        let output = apply_gain(&input, GainCoefficient::new(32768), 15);
        assert_eq!(output[0].value(), 1000);
    }

    #[test]
    fn empty_input_produces_empty() {
        let output = apply_gain(&[], GainCoefficient::new(5), 0);
        assert!(output.is_empty());
    }
}
