//! Parallel (tensor product) composition for independent channels.
//!
//! Processes two independent signal streams through separate
//! pipeline descriptors, producing a pair of output streams.

use crate::error::Error;
use crate::golden::pipeline::pipeline_golden;
use crate::interpret::descriptor::DspBlockDescriptor;
use crate::sample::element::Sample;

/// Process two independent channels in parallel.
///
/// This is the monoidal tensor product: given two pipelines and
/// two inputs, produce two outputs with no interaction.
///
/// # Errors
///
/// Returns an error if either pipeline fails.
///
/// # Examples
///
/// ```
/// use dsp_cat::composition::parallel::parallel;
/// use dsp_cat::interpret::descriptor::DspBlockDescriptor;
/// use dsp_cat::interpret::signal::{BlockIndex, GainCoefficient};
/// use dsp_cat::sample::element::Sample;
///
/// let left_desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2));
/// let right_desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(3));
/// let input = vec![Sample::new(10)];
/// let (left, right) = parallel(&input, &left_desc, &input, &right_desc).ok().unzip();
/// let l_vals: Option<Vec<i32>> = left.map(|v| v.iter().map(|s| s.value()).collect());
/// let r_vals: Option<Vec<i32>> = right.map(|v| v.iter().map(|s| s.value()).collect());
/// assert_eq!(l_vals, Some(vec![20]));
/// assert_eq!(r_vals, Some(vec![30]));
/// ```
pub fn parallel(
    left_input: &[Sample],
    left_desc: &DspBlockDescriptor,
    right_input: &[Sample],
    right_desc: &DspBlockDescriptor,
) -> Result<(Vec<Sample>, Vec<Sample>), Error> {
    let left = pipeline_golden(left_input, left_desc)?;
    let right = pipeline_golden(right_input, right_desc)?;
    Ok((left, right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::signal::{BlockIndex, DelayDepth, GainCoefficient};

    #[test]
    fn parallel_independent_channels() -> Result<(), Error> {
        let input_a = vec![Sample::new(5)];
        let input_b = vec![Sample::new(10)];
        let desc_a = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2));
        let desc_b = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(1));
        let (out_a, out_b) = parallel(&input_a, &desc_a, &input_b, &desc_b)?;
        assert_eq!(out_a[0].value(), 10);
        assert_eq!(out_b.len(), 2); // delayed: [0, 10]
        Ok(())
    }

    #[test]
    fn parallel_identity_both() -> Result<(), Error> {
        let input = vec![Sample::new(42)];
        let (left, right) =
            parallel(&input, &DspBlockDescriptor::Identity, &input, &DspBlockDescriptor::Identity)?;
        assert_eq!(left, input);
        assert_eq!(right, input);
        Ok(())
    }
}
