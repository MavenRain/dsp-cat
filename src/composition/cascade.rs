//! Sequential cascade composition via descriptor composition.
//!
//! Composes a sequence of [`BlockConfig`]
//! values into a single [`DspBlockDescriptor`] by interpreting
//! the pipeline graph.

use comp_cat_rs::collapse::free_category::interpret;

use crate::graph::pipeline_graph::{full_pipeline_path, PipelineGraph};
use crate::interpret::descriptor::DspBlockDescriptor;
use crate::interpret::morphism::{BlockConfig, DspPipelineInterpretation};
use crate::interpret::signal::{BitWidth, FractionalBits};
use crate::sample::format::SampleFormat;

/// Compose a sequence of block configs into a single descriptor
/// via the free category interpretation.
///
/// # Errors
///
/// Returns an error if path construction fails.
///
/// # Examples
///
/// ```
/// use dsp_cat::composition::cascade::cascade;
/// use dsp_cat::interpret::morphism::BlockConfig;
/// use dsp_cat::interpret::signal::{DelayDepth, GainCoefficient};
///
/// let desc = cascade(&[
///     BlockConfig::Delay { depth: DelayDepth::new(4) },
///     BlockConfig::Gain { coefficient: GainCoefficient::new(2) },
/// ]).ok();
/// assert_eq!(desc.map(|d| d.block_count()), Some(2));
/// ```
pub fn cascade(configs: &[BlockConfig]) -> Result<DspBlockDescriptor, crate::error::Error> {
    let graph = PipelineGraph::new(configs.len());
    let interp = DspPipelineInterpretation::new(
        configs.to_vec(),
        SampleFormat::new(BitWidth::new(32), FractionalBits::new(15)),
    );
    let path = full_pipeline_path(&graph)?;
    Ok(interpret::<PipelineGraph, _>(
        &interp,
        &path,
        |_| DspBlockDescriptor::identity(),
        DspBlockDescriptor::compose,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::signal::{CicOrder, DelayDepth, GainCoefficient, RateFactor};

    #[test]
    fn empty_cascade_is_identity() -> Result<(), crate::error::Error> {
        let desc = cascade(&[])?;
        assert_eq!(desc.block_count(), 0);
        Ok(())
    }

    #[test]
    fn single_block_cascade() -> Result<(), crate::error::Error> {
        let desc = cascade(&[BlockConfig::Gain {
            coefficient: GainCoefficient::new(5),
        }])?;
        assert_eq!(desc.block_count(), 1);
        Ok(())
    }

    #[test]
    fn three_block_cascade() -> Result<(), crate::error::Error> {
        let desc = cascade(&[
            BlockConfig::Delay {
                depth: DelayDepth::new(4),
            },
            BlockConfig::Cic {
                order: CicOrder::new(2),
                rate_factor: RateFactor::new(8),
            },
            BlockConfig::Gain {
                coefficient: GainCoefficient::new(3),
            },
        ])?;
        assert_eq!(desc.block_count(), 3);
        Ok(())
    }
}
