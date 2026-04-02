//! Pure accumulator (running sum) golden model.

use crate::sample::element::Sample;

/// Running sum: `output[n] = sum(input[0..=n])`.
///
/// This is a single integrator stage, the building block of CIC filters.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::accumulator::accumulate;
/// use dsp_cat::sample::element::Sample;
///
/// let input = vec![Sample::new(1), Sample::new(2), Sample::new(3)];
/// let output = accumulate(&input);
/// let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
/// assert_eq!(values, vec![1, 3, 6]);
/// ```
#[must_use]
pub fn accumulate(input: &[Sample]) -> Vec<Sample> {
    input
        .iter()
        .scan(Sample::ZERO, |acc, s| {
            *acc = *acc + *s;
            Some(*acc)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_produces_empty() {
        let output = accumulate(&[]);
        assert!(output.is_empty());
    }

    #[test]
    fn single_element_is_identity() {
        let output = accumulate(&[Sample::new(42)]);
        assert_eq!(output, vec![Sample::new(42)]);
    }

    #[test]
    fn accumulates_prefix_sums() {
        let input: Vec<Sample> = (1..=4).map(Sample::new).collect();
        let output = accumulate(&input);
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![1, 3, 6, 10]);
    }

    #[test]
    fn accumulate_saturates() {
        let input = vec![Sample::new(i32::MAX), Sample::new(1)];
        let output = accumulate(&input);
        assert_eq!(output[1].value(), i32::MAX);
    }
}
