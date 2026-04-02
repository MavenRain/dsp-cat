//! DSP block descriptors with associative composition.
//!
//! [`DspBlockDescriptor`] is the `Morphism` type produced by
//! [`GraphMorphism::map_edge`](comp_cat_rs::collapse::free_category::GraphMorphism::map_edge)
//! and composed via
//! [`interpret`](comp_cat_rs::collapse::free_category::interpret).

use crate::interpret::signal::{
    BlockIndex, CicOrder, DelayDepth, FractionalBits, GainCoefficient, PipelineLatency, RateFactor,
    TapCount,
};
use crate::sample::element::Sample;

/// Descriptor for a single DSP block or a composed sub-pipeline.
///
/// Composition is associative with [`Identity`](Self::Identity) as the
/// neutral element, following the same pattern as
/// `SdfStageDescriptor` in goldilocks-ntt-hdl.
///
/// # Examples
///
/// ```
/// use dsp_cat::interpret::descriptor::DspBlockDescriptor;
/// use dsp_cat::interpret::signal::{BlockIndex, DelayDepth, GainCoefficient};
///
/// let a = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(4));
/// let b = DspBlockDescriptor::gain(BlockIndex::new(1), GainCoefficient::new(2));
/// let composed = a.compose(b);
/// assert_eq!(composed.block_count(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub enum DspBlockDescriptor {
    /// Identity (passthrough, no processing).
    Identity,

    /// FIR filter block.
    Fir {
        /// Block position in the pipeline.
        block_index: BlockIndex,
        /// Filter coefficients.
        coefficients: Vec<Sample>,
        /// Number of taps.
        tap_count: TapCount,
        /// Pipeline latency in clock cycles.
        latency: PipelineLatency,
        /// Fractional bits for accumulator truncation.
        frac_bits: FractionalBits,
    },

    /// CIC decimation filter block.
    Cic {
        /// Block position in the pipeline.
        block_index: BlockIndex,
        /// Number of integrator/comb stages.
        order: CicOrder,
        /// Decimation factor.
        rate_factor: RateFactor,
        /// Pipeline latency in clock cycles.
        latency: PipelineLatency,
    },

    /// Pure delay line.
    Delay {
        /// Block position in the pipeline.
        block_index: BlockIndex,
        /// Delay depth in samples.
        depth: DelayDepth,
    },

    /// Gain (multiply by constant).
    Gain {
        /// Block position in the pipeline.
        block_index: BlockIndex,
        /// Fixed-point gain coefficient.
        coefficient: GainCoefficient,
        /// Right-shift after multiply (fractional bits).
        shift: u32,
    },

    /// Downsample by integer factor.
    Decimator {
        /// Block position in the pipeline.
        block_index: BlockIndex,
        /// Decimation factor.
        factor: RateFactor,
    },

    /// Upsample by integer factor (zero insertion).
    Interpolator {
        /// Block position in the pipeline.
        block_index: BlockIndex,
        /// Interpolation factor.
        factor: RateFactor,
    },

    /// Running-sum accumulator.
    Accumulator {
        /// Block position in the pipeline.
        block_index: BlockIndex,
    },

    /// A composed sequence of block descriptors.
    Composed(Vec<DspBlockDescriptor>),
}

impl DspBlockDescriptor {
    /// The identity descriptor (passthrough).
    pub fn identity() -> Self {
        Self::Identity
    }

    /// Construct a FIR descriptor.
    pub fn fir(
        block_index: BlockIndex,
        coefficients: Vec<Sample>,
        frac_bits: FractionalBits,
    ) -> Self {
        let tap_count = TapCount::new(coefficients.len());
        let latency = PipelineLatency::new(coefficients.len());
        Self::Fir {
            block_index,
            coefficients,
            tap_count,
            latency,
            frac_bits,
        }
    }

    /// Construct a CIC descriptor.
    pub fn cic(block_index: BlockIndex, order: CicOrder, rate_factor: RateFactor) -> Self {
        let latency = PipelineLatency::new(2 * order.value() + 1);
        Self::Cic {
            block_index,
            order,
            rate_factor,
            latency,
        }
    }

    /// Construct a delay descriptor.
    pub fn delay(block_index: BlockIndex, depth: DelayDepth) -> Self {
        Self::Delay {
            block_index,
            depth,
        }
    }

    /// Construct a gain descriptor with integer coefficient (shift = 0).
    pub fn gain(block_index: BlockIndex, coefficient: GainCoefficient) -> Self {
        Self::Gain {
            block_index,
            coefficient,
            shift: 0,
        }
    }

    /// Construct a gain descriptor with fractional coefficient.
    pub fn gain_fractional(
        block_index: BlockIndex,
        coefficient: GainCoefficient,
        shift: u32,
    ) -> Self {
        Self::Gain {
            block_index,
            coefficient,
            shift,
        }
    }

    /// Construct a decimator descriptor.
    pub fn decimator(block_index: BlockIndex, factor: RateFactor) -> Self {
        Self::Decimator {
            block_index,
            factor,
        }
    }

    /// Construct an interpolator descriptor.
    pub fn interpolator(block_index: BlockIndex, factor: RateFactor) -> Self {
        Self::Interpolator {
            block_index,
            factor,
        }
    }

    /// Construct an accumulator descriptor.
    pub fn accumulator(block_index: BlockIndex) -> Self {
        Self::Accumulator { block_index }
    }

    /// Compose two descriptors sequentially.
    ///
    /// Associative with [`Identity`](Self::Identity) as neutral element.
    pub fn compose(self, other: Self) -> Self {
        match (self, other) {
            (Self::Identity, b) => b,
            (a, Self::Identity) => a,
            (Self::Composed(a), Self::Composed(b)) => {
                Self::Composed(a.into_iter().chain(b).collect())
            }
            (Self::Composed(a), b) => {
                Self::Composed(a.into_iter().chain(std::iter::once(b)).collect())
            }
            (a, Self::Composed(b)) => {
                Self::Composed(std::iter::once(a).chain(b).collect())
            }
            (a, b) => Self::Composed(vec![a, b]),
        }
    }

    /// The number of non-identity blocks in this descriptor.
    pub fn block_count(&self) -> usize {
        match self {
            Self::Identity => 0,
            Self::Composed(blocks) => blocks.iter().map(Self::block_count).sum(),
            _ => 1,
        }
    }

    /// Total pipeline latency across all composed blocks.
    pub fn total_latency(&self) -> PipelineLatency {
        match self {
            Self::Identity => PipelineLatency::ZERO,
            Self::Fir { latency, .. } | Self::Cic { latency, .. } => *latency,
            Self::Delay { depth, .. } => PipelineLatency::new(depth.value()),
            Self::Gain { .. }
            | Self::Decimator { .. }
            | Self::Interpolator { .. }
            | Self::Accumulator { .. } => PipelineLatency::new(1),
            Self::Composed(blocks) => blocks
                .iter()
                .fold(PipelineLatency::ZERO, |acc, b| acc.sum(b.total_latency())),
        }
    }

    /// Iterate over the leaf (non-Composed, non-Identity) blocks.
    pub fn singles(&self) -> Vec<&Self> {
        match self {
            Self::Identity => vec![],
            Self::Composed(blocks) => blocks.iter().flat_map(Self::singles).collect(),
            other => vec![other],
        }
    }
}

impl core::fmt::Display for DspBlockDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Identity => write!(f, "Identity"),
            Self::Fir { block_index, tap_count, .. } => {
                write!(f, "{block_index}: FIR({tap_count})")
            }
            Self::Cic { block_index, order, rate_factor, .. } => {
                write!(f, "{block_index}: CIC({order}, {rate_factor})")
            }
            Self::Delay { block_index, depth } => {
                write!(f, "{block_index}: Delay({depth})")
            }
            Self::Gain { block_index, coefficient, .. } => {
                write!(f, "{block_index}: Gain({coefficient})")
            }
            Self::Decimator { block_index, factor } => {
                write!(f, "{block_index}: Decimate({factor})")
            }
            Self::Interpolator { block_index, factor } => {
                write!(f, "{block_index}: Interpolate({factor})")
            }
            Self::Accumulator { block_index } => {
                write!(f, "{block_index}: Accumulator")
            }
            Self::Composed(blocks) => {
                write!(f, "Composed({} blocks)", blocks.len())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_neutral_left() {
        let d = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(5));
        let composed = DspBlockDescriptor::identity().compose(d.clone());
        assert_eq!(composed, d);
    }

    #[test]
    fn identity_is_neutral_right() {
        let d = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(5));
        let composed = d.clone().compose(DspBlockDescriptor::identity());
        assert_eq!(composed, d);
    }

    #[test]
    fn compose_two_singles() {
        let a = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(3));
        let b = DspBlockDescriptor::gain(BlockIndex::new(1), GainCoefficient::new(2));
        let composed = a.compose(b);
        assert_eq!(composed.block_count(), 2);
    }

    #[test]
    fn compose_is_associative_in_block_count() {
        let a = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(1));
        let b = DspBlockDescriptor::gain(BlockIndex::new(1), GainCoefficient::new(2));
        let c = DspBlockDescriptor::accumulator(BlockIndex::new(2));

        let left = a.clone().compose(b.clone()).compose(c.clone());
        let right = a.compose(b.compose(c));
        assert_eq!(left.block_count(), right.block_count());
        assert_eq!(left.block_count(), 3);
    }

    #[test]
    fn singles_flattens_composed() {
        let a = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(1));
        let b = DspBlockDescriptor::gain(BlockIndex::new(1), GainCoefficient::new(2));
        let c = DspBlockDescriptor::accumulator(BlockIndex::new(2));
        let composed = a.compose(b).compose(c);
        assert_eq!(composed.singles().len(), 3);
    }

    #[test]
    fn total_latency_sums_across_blocks() {
        let fir = DspBlockDescriptor::fir(
            BlockIndex::new(0),
            vec![Sample::new(1), Sample::new(2), Sample::new(3)],
            FractionalBits::new(0),
        );
        let delay = DspBlockDescriptor::delay(BlockIndex::new(1), DelayDepth::new(10));
        let composed = fir.compose(delay);
        assert_eq!(composed.total_latency().value(), 13);
    }

    #[test]
    fn identity_block_count_is_zero() {
        assert_eq!(DspBlockDescriptor::identity().block_count(), 0);
    }
}
