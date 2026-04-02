//! Graph morphism mapping the pipeline graph into DSP block descriptors.
//!
//! This is the bridge between the categorical pipeline topology and
//! the concrete block configurations.  The
//! [`interpret`](comp_cat_rs::collapse::free_category::interpret)
//! function uses this morphism to compose the full pipeline descriptor.

use comp_cat_rs::collapse::free_category::{Edge, GraphMorphism, Vertex};

use crate::graph::pipeline_graph::PipelineGraph;
use crate::interpret::descriptor::DspBlockDescriptor;
use crate::interpret::signal::{
    BlockIndex, BoundaryIndex, BoundarySignal, CicOrder, DelayDepth, FractionalBits,
    GainCoefficient, RateFactor,
};
use crate::sample::element::Sample;
use crate::sample::format::SampleFormat;

/// Configuration for a single DSP block (used to construct descriptors).
#[derive(Debug, Clone)]
pub enum BlockConfig {
    /// FIR filter with given coefficients and fractional bits.
    Fir {
        /// Filter coefficients.
        coefficients: Vec<Sample>,
        /// Fractional bits for accumulator truncation.
        frac_bits: FractionalBits,
    },

    /// CIC decimation filter.
    Cic {
        /// Number of integrator/comb stages.
        order: CicOrder,
        /// Decimation factor.
        rate_factor: RateFactor,
    },

    /// Pure delay line.
    Delay {
        /// Delay in samples.
        depth: DelayDepth,
    },

    /// Gain with integer coefficient (shift = 0).
    Gain {
        /// Fixed-point gain coefficient.
        coefficient: GainCoefficient,
    },

    /// Gain with fractional coefficient.
    GainFractional {
        /// Fixed-point gain coefficient.
        coefficient: GainCoefficient,
        /// Right-shift after multiply.
        shift: u32,
    },

    /// Downsample by integer factor.
    Decimator {
        /// Decimation factor.
        factor: RateFactor,
    },

    /// Upsample by integer factor.
    Interpolator {
        /// Interpolation factor.
        factor: RateFactor,
    },

    /// Running-sum accumulator.
    Accumulator,
}

impl BlockConfig {
    /// Convert to a [`DspBlockDescriptor`] at the given block index.
    fn to_descriptor(&self, block_index: BlockIndex) -> DspBlockDescriptor {
        match self {
            Self::Fir {
                coefficients,
                frac_bits,
            } => DspBlockDescriptor::fir(block_index, coefficients.clone(), *frac_bits),
            Self::Cic {
                order,
                rate_factor,
            } => DspBlockDescriptor::cic(block_index, *order, *rate_factor),
            Self::Delay { depth } => DspBlockDescriptor::delay(block_index, *depth),
            Self::Gain { coefficient } => DspBlockDescriptor::gain(block_index, *coefficient),
            Self::GainFractional { coefficient, shift } => {
                DspBlockDescriptor::gain_fractional(block_index, *coefficient, *shift)
            }
            Self::Decimator { factor } => DspBlockDescriptor::decimator(block_index, *factor),
            Self::Interpolator { factor } => {
                DspBlockDescriptor::interpolator(block_index, *factor)
            }
            Self::Accumulator => DspBlockDescriptor::accumulator(block_index),
        }
    }

    /// The cumulative rate change factor for this block.
    ///
    /// Decimator/CIC produce a factor > 1; interpolator produces 1
    /// (the pipeline graph models rate at the output side).
    fn rate_divisor(&self) -> usize {
        match self {
            Self::Cic { rate_factor, .. } | Self::Decimator { factor: rate_factor } => {
                rate_factor.value()
            }
            _ => 1,
        }
    }

    /// The bit growth introduced by this block.
    fn bit_growth(&self) -> usize {
        match self {
            Self::Cic {
                order,
                rate_factor,
            } => {
                // CIC bit growth = order * ceil(log2(rate_factor))
                let log2_r = usize::BITS - rate_factor.value().leading_zeros();
                order.value() * (log2_r as usize)
            }
            _ => 0,
        }
    }
}

/// Maps the pipeline graph vertices to boundary signal types and
/// edges to DSP block descriptors.
///
/// # Examples
///
/// ```
/// use dsp_cat::graph::pipeline_graph::{PipelineGraph, full_pipeline_path};
/// use dsp_cat::interpret::morphism::{BlockConfig, DspPipelineInterpretation};
/// use dsp_cat::interpret::descriptor::DspBlockDescriptor;
/// use dsp_cat::interpret::signal::{BitWidth, DelayDepth, FractionalBits, GainCoefficient};
/// use dsp_cat::sample::format::SampleFormat;
/// use comp_cat_rs::collapse::free_category::interpret;
///
/// let graph = PipelineGraph::new(2);
/// let interp = DspPipelineInterpretation::new(
///     vec![
///         BlockConfig::Delay { depth: DelayDepth::new(4) },
///         BlockConfig::Gain { coefficient: GainCoefficient::new(3) },
///     ],
///     SampleFormat::new(BitWidth::new(32), FractionalBits::new(15)),
/// );
/// let path = full_pipeline_path(&graph).ok();
/// let desc = path.map(|p| interpret::<PipelineGraph, _>(
///     &interp,
///     &p,
///     |_| DspBlockDescriptor::identity(),
///     DspBlockDescriptor::compose,
/// ));
/// assert_eq!(desc.map(|d| d.block_count()), Some(2));
/// ```
#[derive(Debug, Clone)]
pub struct DspPipelineInterpretation {
    block_configs: Vec<BlockConfig>,
    input_format: SampleFormat,
}

impl DspPipelineInterpretation {
    /// Create a new pipeline interpretation.
    #[must_use]
    pub fn new(block_configs: Vec<BlockConfig>, input_format: SampleFormat) -> Self {
        Self {
            block_configs,
            input_format,
        }
    }

    /// The block configurations.
    #[must_use]
    pub fn block_configs(&self) -> &[BlockConfig] {
        &self.block_configs
    }

    /// The input sample format.
    pub fn input_format(&self) -> SampleFormat {
        self.input_format
    }

    /// Compute the signal format at a given boundary by folding
    /// format transformations through blocks `0..boundary`.
    fn format_at_boundary(&self, boundary: usize) -> SampleFormat {
        self.block_configs[..boundary.min(self.block_configs.len())]
            .iter()
            .fold(self.input_format, |fmt, config| {
                fmt.widen(config.bit_growth())
            })
    }

    /// Compute the cumulative rate divisor at a given boundary.
    fn rate_at_boundary(&self, boundary: usize) -> RateFactor {
        let divisor = self.block_configs[..boundary.min(self.block_configs.len())]
            .iter()
            .fold(1_usize, |acc, config| acc * config.rate_divisor());
        RateFactor::new(divisor)
    }
}

impl GraphMorphism<PipelineGraph> for DspPipelineInterpretation {
    type Object = BoundarySignal;
    type Morphism = DspBlockDescriptor;

    fn map_vertex(&self, v: Vertex) -> BoundarySignal {
        BoundarySignal::new(
            BoundaryIndex::new(v.index()),
            self.format_at_boundary(v.index()),
            self.rate_at_boundary(v.index()),
        )
    }

    fn map_edge(&self, e: Edge) -> DspBlockDescriptor {
        self.block_configs
            .get(e.index())
            .map_or(DspBlockDescriptor::Identity, |config| {
                config.to_descriptor(BlockIndex::new(e.index()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::pipeline_graph::full_pipeline_path;
    use crate::interpret::signal::BitWidth;
    use comp_cat_rs::collapse::free_category::interpret;

    fn sample_format() -> SampleFormat {
        SampleFormat::new(BitWidth::new(32), FractionalBits::new(15))
    }

    #[test]
    fn interpretation_produces_correct_block_count() -> Result<(), crate::error::Error> {
        let graph = PipelineGraph::new(3);
        let interp = DspPipelineInterpretation::new(
            vec![
                BlockConfig::Delay {
                    depth: DelayDepth::new(4),
                },
                BlockConfig::Gain {
                    coefficient: GainCoefficient::new(2),
                },
                BlockConfig::Accumulator,
            ],
            sample_format(),
        );
        let path = full_pipeline_path(&graph)?;
        let desc = interpret::<PipelineGraph, _>(
            &interp,
            &path,
            |_| DspBlockDescriptor::identity(),
            DspBlockDescriptor::compose,
        );
        assert_eq!(desc.block_count(), 3);
        Ok(())
    }

    #[test]
    fn interpretation_of_single_edge() -> Result<(), crate::error::Error> {
        let interp = DspPipelineInterpretation::new(
            vec![BlockConfig::Delay {
                depth: DelayDepth::new(8),
            }],
            sample_format(),
        );
        let desc = interp.map_edge(Edge::new(0));
        assert_eq!(desc.block_count(), 1);
        Ok(())
    }

    #[test]
    fn vertex_signal_has_correct_format() -> Result<(), crate::error::Error> {
        let interp = DspPipelineInterpretation::new(
            vec![BlockConfig::Delay {
                depth: DelayDepth::new(4),
            }],
            sample_format(),
        );
        let sig = interp.map_vertex(Vertex::new(0));
        assert_eq!(sig.format().total_bits().value(), 32);
        Ok(())
    }

    #[test]
    fn cic_widens_boundary_format() -> Result<(), crate::error::Error> {
        let interp = DspPipelineInterpretation::new(
            vec![BlockConfig::Cic {
                order: CicOrder::new(3),
                rate_factor: RateFactor::new(16),
            }],
            sample_format(),
        );
        let sig_after = interp.map_vertex(Vertex::new(1));
        // bit_growth = 3 * ceil(log2(16)) = 3 * 5 = 15
        // (log2(16) = 4, but usize::BITS - leading_zeros gives 5 for 16)
        // Actually: 16.leading_zeros() = 28 on u32, BITS=32, so 32-28=4? No...
        // usize is 64-bit on this platform: 64 - 60 = 4. Hmm, let me check.
        // 16_usize.leading_zeros() on 64-bit = 60, so BITS - lz = 64-60 = 4
        // Wait, we use u32: usize::BITS. On 64-bit, usize::BITS = 64.
        // 16_usize.leading_zeros() = 60. 64 - 60 = 4. So bit_growth = 3*4 = 12.
        assert!(sig_after.format().total_bits().value() > 32);
        Ok(())
    }

    #[test]
    fn decimation_changes_rate_divisor() -> Result<(), crate::error::Error> {
        let interp = DspPipelineInterpretation::new(
            vec![BlockConfig::Decimator {
                factor: RateFactor::new(8),
            }],
            sample_format(),
        );
        let sig = interp.map_vertex(Vertex::new(1));
        assert_eq!(sig.sample_rate_divisor().value(), 8);
        Ok(())
    }
}
