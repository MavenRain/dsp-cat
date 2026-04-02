//! Pure delay line golden model.

use crate::interpret::signal::DelayDepth;
use crate::sample::element::Sample;

/// Delay the input by `depth` samples, prepending zeros.
///
/// Output length equals `input.len() + depth`.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::delay::delay_line;
/// use dsp_cat::interpret::signal::DelayDepth;
/// use dsp_cat::sample::element::Sample;
///
/// let input = vec![Sample::new(1), Sample::new(2), Sample::new(3)];
/// let output = delay_line(&input, DelayDepth::new(2));
/// assert_eq!(output.len(), 5);
/// assert_eq!(output[0].value(), 0);
/// assert_eq!(output[1].value(), 0);
/// assert_eq!(output[2].value(), 1);
/// ```
#[must_use]
pub fn delay_line(input: &[Sample], depth: DelayDepth) -> Vec<Sample> {
    std::iter::repeat_n(Sample::ZERO, depth.value())
        .chain(input.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_delay_is_identity() {
        let input = vec![Sample::new(1), Sample::new(2)];
        let output = delay_line(&input, DelayDepth::new(0));
        assert_eq!(output, input);
    }

    #[test]
    fn delay_prepends_zeros() {
        let input = vec![Sample::new(10)];
        let output = delay_line(&input, DelayDepth::new(3));
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], Sample::ZERO);
        assert_eq!(output[1], Sample::ZERO);
        assert_eq!(output[2], Sample::ZERO);
        assert_eq!(output[3], Sample::new(10));
    }

    #[test]
    fn empty_input_produces_only_zeros() {
        let output = delay_line(&[], DelayDepth::new(2));
        assert_eq!(output, vec![Sample::ZERO, Sample::ZERO]);
    }
}
