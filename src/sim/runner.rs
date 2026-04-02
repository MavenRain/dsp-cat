//! Behavioral simulation wrapped in [`Io`].
//!
//! All mutable simulation state lives inside `Io::suspend`.
//! The outer interface is pure: `Io<Error, Vec<Sample>>`.

use comp_cat_rs::effect::io::Io;

use crate::error::Error;
use crate::golden::pipeline::pipeline_golden;
use crate::interpret::descriptor::DspBlockDescriptor;
use crate::sample::element::Sample;

/// Configuration for a behavioral pipeline simulation.
///
/// # Examples
///
/// ```
/// use dsp_cat::sim::runner::{SimConfig, simulate_pipeline};
/// use dsp_cat::interpret::descriptor::DspBlockDescriptor;
/// use dsp_cat::interpret::signal::{BlockIndex, GainCoefficient};
/// use dsp_cat::sample::element::Sample;
///
/// let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2));
/// let config = SimConfig::new(vec![Sample::new(10)], desc);
/// let result = simulate_pipeline(config).run().ok();
/// let values: Option<Vec<i32>> = result.map(|v| v.iter().map(|s| s.value()).collect());
/// assert_eq!(values, Some(vec![20]));
/// ```
#[derive(Debug, Clone)]
pub struct SimConfig {
    input: Vec<Sample>,
    descriptor: DspBlockDescriptor,
}

impl SimConfig {
    /// Create a simulation configuration.
    #[must_use]
    pub fn new(input: Vec<Sample>, descriptor: DspBlockDescriptor) -> Self {
        Self { input, descriptor }
    }

    /// The input samples.
    pub fn input(&self) -> &[Sample] {
        &self.input
    }

    /// The pipeline descriptor.
    pub fn descriptor(&self) -> &DspBlockDescriptor {
        &self.descriptor
    }
}

/// Build an [`Io`] that simulates the DSP pipeline behaviorally.
///
/// Nothing executes until [`Io::run`] is called at the boundary.
#[must_use]
pub fn simulate_pipeline(config: SimConfig) -> Io<Error, Vec<Sample>> {
    Io::suspend(move || pipeline_golden(config.input(), config.descriptor()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::signal::{BlockIndex, DelayDepth, GainCoefficient};

    #[test]
    fn simulate_identity() -> Result<(), Error> {
        let input = vec![Sample::new(1), Sample::new(2)];
        let config = SimConfig::new(input.clone(), DspBlockDescriptor::Identity);
        let result = simulate_pipeline(config).run()?;
        assert_eq!(result, input);
        Ok(())
    }

    #[test]
    fn simulate_gain() -> Result<(), Error> {
        let input = vec![Sample::new(5)];
        let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(3));
        let config = SimConfig::new(input, desc);
        let result = simulate_pipeline(config).run()?;
        assert_eq!(result[0].value(), 15);
        Ok(())
    }

    #[test]
    fn simulate_composed_pipeline() -> Result<(), Error> {
        let input = vec![Sample::new(10)];
        let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2))
            .compose(DspBlockDescriptor::delay(
                BlockIndex::new(1),
                DelayDepth::new(1),
            ));
        let config = SimConfig::new(input, desc);
        let result = simulate_pipeline(config).run()?;
        // Gain by 2: [20], then delay by 1: [0, 20]
        let values: Vec<i32> = result.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![0, 20]);
        Ok(())
    }
}
