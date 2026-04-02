//! Composed pipeline golden model.
//!
//! Executes a chain of DSP blocks described by a
//! [`DspBlockDescriptor`]
//! by dispatching to the appropriate per-block golden model.

use crate::error::Error;
use crate::interpret::descriptor::DspBlockDescriptor;
use crate::sample::element::Sample;

/// Execute a composed pipeline golden model.
///
/// Walks the descriptor tree, feeding the output of each block
/// into the input of the next.
///
/// # Errors
///
/// Returns an error if any individual block fails.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::pipeline::pipeline_golden;
/// use dsp_cat::interpret::descriptor::DspBlockDescriptor;
/// use dsp_cat::interpret::signal::{BlockIndex, DelayDepth, GainCoefficient};
/// use dsp_cat::sample::element::Sample;
///
/// let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2))
///     .compose(DspBlockDescriptor::delay(BlockIndex::new(1), DelayDepth::new(1)));
/// let input = vec![Sample::new(5), Sample::new(10)];
/// let output = pipeline_golden(&input, &desc).ok();
/// let values: Option<Vec<i32>> = output.map(|v| v.iter().map(|s| s.value()).collect());
/// // Gain by 2: [10, 20], then delay by 1: [0, 10, 20]
/// assert_eq!(values, Some(vec![0, 10, 20]));
/// ```
pub fn pipeline_golden(
    input: &[Sample],
    descriptor: &DspBlockDescriptor,
) -> Result<Vec<Sample>, Error> {
    match descriptor {
        DspBlockDescriptor::Identity => Ok(input.to_vec()),

        DspBlockDescriptor::Fir {
            coefficients,
            frac_bits,
            ..
        } => crate::golden::fir::fir_convolve(input, coefficients, *frac_bits),

        DspBlockDescriptor::Cic {
            order, rate_factor, ..
        } => crate::golden::cic::cic_decimate(input, *order, *rate_factor),

        DspBlockDescriptor::Delay { depth, .. } => {
            Ok(crate::golden::delay::delay_line(input, *depth))
        }

        DspBlockDescriptor::Gain {
            coefficient, shift, ..
        } => Ok(crate::golden::gain::apply_gain(input, *coefficient, *shift)),

        DspBlockDescriptor::Decimator { factor, .. } => {
            crate::golden::decimator::decimate(input, *factor)
        }

        DspBlockDescriptor::Interpolator { factor, .. } => {
            crate::golden::interpolator::interpolate(input, *factor)
        }

        DspBlockDescriptor::Accumulator { .. } => {
            Ok(crate::golden::accumulator::accumulate(input))
        }

        DspBlockDescriptor::Composed(blocks) => blocks.iter().try_fold(
            input.to_vec(),
            |data, block| pipeline_golden(&data, block),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::signal::{BlockIndex, DelayDepth, GainCoefficient};

    #[test]
    fn identity_is_passthrough() -> Result<(), Error> {
        let input = vec![Sample::new(1), Sample::new(2)];
        let output = pipeline_golden(&input, &DspBlockDescriptor::Identity)?;
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn single_gain_block() -> Result<(), Error> {
        let input = vec![Sample::new(10)];
        let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(3));
        let output = pipeline_golden(&input, &desc)?;
        assert_eq!(output[0].value(), 30);
        Ok(())
    }

    #[test]
    fn composed_delay_then_gain() -> Result<(), Error> {
        let input = vec![Sample::new(5)];
        let desc = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(2))
            .compose(DspBlockDescriptor::gain(
                BlockIndex::new(1),
                GainCoefficient::new(4),
            ));
        let output = pipeline_golden(&input, &desc)?;
        // Delay by 2: [0, 0, 5], then gain by 4: [0, 0, 20]
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![0, 0, 20]);
        Ok(())
    }

    #[test]
    fn empty_composed_is_identity() -> Result<(), Error> {
        let input = vec![Sample::new(42)];
        let desc = DspBlockDescriptor::Composed(vec![]);
        let output = pipeline_golden(&input, &desc)?;
        assert_eq!(output, input);
        Ok(())
    }
}
