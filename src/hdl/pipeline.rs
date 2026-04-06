//! Composed DSP pipeline built from a [`DspBlockDescriptor`].
//!
//! Recursively walks the descriptor tree, constructs a
//! [`RawDspBlock`] for each leaf, and folds them with
//! [`compose_raw`] into a single combined IR graph.

use crate::error::Error;
use crate::hdl::accumulator::build_accumulator;
use crate::hdl::cic::build_cic;
use crate::hdl::decimator::build_decimator;
use crate::hdl::delay::build_delay;
use crate::hdl::fir::build_fir;
use crate::hdl::gain::build_gain;
use crate::hdl::interpolator::build_interpolator;
use crate::hdl::raw::{compose_raw, identity_raw, RawDspBlock};
use crate::interpret::descriptor::DspBlockDescriptor;

/// Build a [`RawDspBlock`] from a [`DspBlockDescriptor`].
///
/// Recursively flattens composed descriptors and chains the
/// resulting blocks via [`compose_raw`].  An `Identity` descriptor
/// produces a passthrough block with no state.
///
/// # Errors
///
/// Propagates errors from individual block constructors or
/// from graph composition.
pub fn build_pipeline(desc: &DspBlockDescriptor) -> Result<RawDspBlock, Error> {
    match desc {
        DspBlockDescriptor::Identity => identity_raw(),

        DspBlockDescriptor::Delay { depth, .. } => build_delay(depth.value()),

        DspBlockDescriptor::Gain {
            coefficient, shift, ..
        } => build_gain(coefficient.value(), *shift),

        DspBlockDescriptor::Decimator { factor, .. } => {
            build_decimator(factor.value())
        }

        DspBlockDescriptor::Interpolator { factor, .. } => {
            build_interpolator(factor.value())
        }

        DspBlockDescriptor::Accumulator { .. } => build_accumulator(),

        DspBlockDescriptor::Fir {
            coefficients,
            frac_bits,
            ..
        } => {
            #[allow(clippy::cast_possible_truncation)]
            let shift = frac_bits.value() as u32;
            let coeff_values: Vec<i32> = coefficients.iter().map(|s| s.value()).collect();
            build_fir(&coeff_values, shift)
        }

        DspBlockDescriptor::Cic {
            order, rate_factor, ..
        } => build_cic(order.value(), rate_factor.value()),

        DspBlockDescriptor::Composed(blocks) => {
            blocks
                .iter()
                .try_fold(identity_raw()?, |acc, b| {
                    build_pipeline(b).and_then(|block| compose_raw(acc, block))
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_pipeline;
    use crate::interpret::descriptor::DspBlockDescriptor;
    use crate::interpret::signal::{
        BlockIndex, DelayDepth, GainCoefficient, RateFactor,
    };

    #[test]
    fn identity_pipeline_has_no_state() -> Result<(), crate::error::Error> {
        let block = build_pipeline(&DspBlockDescriptor::Identity)?;
        assert_eq!(block.state_wire_count(), 0);
        Ok(())
    }

    #[test]
    fn gain_then_delay_composes() -> Result<(), crate::error::Error> {
        let desc = DspBlockDescriptor::gain(
            BlockIndex::new(0),
            GainCoefficient::new(2),
        )
        .compose(DspBlockDescriptor::delay(
            BlockIndex::new(1),
            DelayDepth::new(1),
        ));
        let block = build_pipeline(&desc)?;
        // Gain: 2 state wires.  Delay(1): 2 state wires.
        assert_eq!(block.state_wire_count(), 4);
        Ok(())
    }

    #[test]
    fn decimator_pipeline() -> Result<(), crate::error::Error> {
        let desc = DspBlockDescriptor::decimator(
            BlockIndex::new(0),
            RateFactor::new(4),
        );
        let block = build_pipeline(&desc)?;
        assert_eq!(block.state_wire_count(), 1);
        Ok(())
    }
}
