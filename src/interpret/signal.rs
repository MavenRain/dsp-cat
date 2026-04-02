//! Newtypes for pipeline parameters and signal metadata.
//!
//! Every domain quantity gets its own newtype to prevent confusion
//! at function boundaries.

use crate::sample::format::SampleFormat;

/// Index of a DSP block in the pipeline (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct BlockIndex(usize);

impl BlockIndex {
    /// Create a new block index.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The underlying index.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for BlockIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "block[{}]", self.0)
    }
}

/// Index of an inter-block boundary (vertex) in the pipeline graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct BoundaryIndex(usize);

impl BoundaryIndex {
    /// Create a new boundary index.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The underlying index.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for BoundaryIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "boundary[{}]", self.0)
    }
}

/// Number of FIR filter taps (coefficients).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct TapCount(usize);

impl TapCount {
    /// Create a new tap count.
    pub fn new(count: usize) -> Self {
        Self(count)
    }

    /// The underlying count.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for TapCount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} taps", self.0)
    }
}

/// Decimation or interpolation factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct RateFactor(usize);

impl RateFactor {
    /// Create a new rate factor.
    pub fn new(factor: usize) -> Self {
        Self(factor)
    }

    /// The underlying factor.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for RateFactor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "x{}", self.0)
    }
}

/// Number of CIC integrator/comb stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct CicOrder(usize);

impl CicOrder {
    /// Create a new CIC order.
    pub fn new(order: usize) -> Self {
        Self(order)
    }

    /// The underlying order.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for CicOrder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "order {}", self.0)
    }
}

/// Delay depth in sample periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct DelayDepth(usize);

impl DelayDepth {
    /// Create a new delay depth.
    pub fn new(depth: usize) -> Self {
        Self(depth)
    }

    /// The underlying depth.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for DelayDepth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} samples", self.0)
    }
}

/// Pipeline latency in clock cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct PipelineLatency(usize);

impl PipelineLatency {
    /// Zero latency (passthrough).
    pub const ZERO: Self = Self(0);

    /// Create a new latency value.
    pub fn new(cycles: usize) -> Self {
        Self(cycles)
    }

    /// The underlying cycle count.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }

    /// Sum two latencies (for sequential composition).
    pub fn sum(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl core::fmt::Display for PipelineLatency {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} cycles", self.0)
    }
}

/// Bit width of a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct BitWidth(usize);

impl BitWidth {
    /// Create a new bit width.
    pub fn new(bits: usize) -> Self {
        Self(bits)
    }

    /// The underlying width.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for BitWidth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}-bit", self.0)
    }
}

/// Number of fractional bits in fixed-point representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct FractionalBits(usize);

impl FractionalBits {
    /// Create a new fractional-bits count.
    pub fn new(bits: usize) -> Self {
        Self(bits)
    }

    /// The underlying count.
    #[must_use]
    pub fn value(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for FractionalBits {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "frac {}", self.0)
    }
}

/// Gain coefficient as a raw fixed-point value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct GainCoefficient(i32);

impl GainCoefficient {
    /// Create a new gain coefficient.
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    /// The underlying value.
    #[must_use]
    pub fn value(self) -> i32 {
        self.0
    }
}

impl core::fmt::Display for GainCoefficient {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "gain({})", self.0)
    }
}

/// Signal type at an inter-block pipeline boundary.
///
/// This is the `Object` type produced by
/// [`GraphMorphism::map_vertex`](comp_cat_rs::collapse::free_category::GraphMorphism::map_vertex).
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct BoundarySignal {
    boundary: BoundaryIndex,
    format: SampleFormat,
    sample_rate_divisor: RateFactor,
}

impl BoundarySignal {
    /// Create a boundary signal descriptor.
    ///
    /// `sample_rate_divisor` of 1 means the original input rate;
    /// N means the rate has been decimated by a total factor of N.
    pub fn new(
        boundary: BoundaryIndex,
        format: SampleFormat,
        sample_rate_divisor: RateFactor,
    ) -> Self {
        Self {
            boundary,
            format,
            sample_rate_divisor,
        }
    }

    /// The boundary index in the pipeline graph.
    pub fn boundary(&self) -> BoundaryIndex {
        self.boundary
    }

    /// The fixed-point format at this boundary.
    pub fn format(&self) -> SampleFormat {
        self.format
    }

    /// The cumulative sample-rate divisor at this boundary.
    pub fn sample_rate_divisor(&self) -> RateFactor {
        self.sample_rate_divisor
    }
}

impl core::fmt::Display for BoundarySignal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}: {} @ /{}",
            self.boundary, self.format, self.sample_rate_divisor
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::signal::{BitWidth, FractionalBits};

    #[test]
    fn block_index_round_trips() {
        let idx = BlockIndex::new(3);
        assert_eq!(idx.value(), 3);
    }

    #[test]
    fn pipeline_latency_add() {
        let a = PipelineLatency::new(5);
        let b = PipelineLatency::new(7);
        assert_eq!(a.sum(b).value(), 12);
    }

    #[test]
    fn pipeline_latency_add_saturates() {
        let a = PipelineLatency::new(usize::MAX);
        let b = PipelineLatency::new(1);
        assert_eq!(a.sum(b).value(), usize::MAX);
    }

    #[test]
    fn boundary_signal_display() {
        let sig = BoundarySignal::new(
            BoundaryIndex::new(2),
            SampleFormat::new(BitWidth::new(32), FractionalBits::new(15)),
            RateFactor::new(4),
        );
        assert_eq!(sig.to_string(), "boundary[2]: Q16.15 @ /x4");
    }
}
