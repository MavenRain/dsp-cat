//! Unified error type for the `dsp-cat` crate.

use comp_cat_rs::collapse::free_category::FreeCategoryError;

/// All errors produced by this crate.
#[derive(Debug)]
pub enum Error {
    /// Sample value or format error.
    Sample(String),

    /// FIR filter configuration error (e.g., empty coefficients).
    Fir(String),

    /// CIC filter configuration error (e.g., order of zero).
    Cic(String),

    /// Pipeline graph construction or traversal error.
    Graph(FreeCategoryError),

    /// HDL simulation error.
    Simulation(String),

    /// Verification mismatch between golden model and simulation.
    VerificationMismatch {
        /// Which block in the pipeline.
        block_index: usize,
        /// Which sample in the output.
        sample_index: usize,
        /// Expected value from the golden model.
        expected: i32,
        /// Actual value from the simulation.
        actual: i32,
    },

    /// Invalid decimation or interpolation factor.
    InvalidRateFactor {
        /// The invalid factor.
        factor: usize,
    },

    /// Pipeline descriptor has no blocks.
    EmptyPipeline,

    /// IO error (e.g., VCD file write).
    Io(std::io::Error),

    /// hdl-cat hardware description or simulation error.
    Hdl(hdl_cat::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Sample(msg) => write!(f, "sample error: {msg}"),
            Self::Fir(msg) => write!(f, "FIR error: {msg}"),
            Self::Cic(msg) => write!(f, "CIC error: {msg}"),
            Self::Graph(e) => write!(f, "graph error: {e}"),
            Self::Simulation(msg) => write!(f, "simulation error: {msg}"),
            Self::VerificationMismatch {
                block_index,
                sample_index,
                expected,
                actual,
            } => write!(
                f,
                "verification mismatch at block {block_index}, \
                 sample {sample_index}: expected {expected}, got {actual}"
            ),
            Self::InvalidRateFactor { factor } => {
                write!(f, "invalid rate factor: {factor}")
            }
            Self::EmptyPipeline => write!(f, "pipeline has no blocks"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Hdl(e) => write!(f, "HDL error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Graph(e) => Some(e),
            Self::Hdl(e) => Some(e),
            Self::Sample(_)
            | Self::Fir(_)
            | Self::Cic(_)
            | Self::Simulation(_)
            | Self::VerificationMismatch { .. }
            | Self::InvalidRateFactor { .. }
            | Self::EmptyPipeline => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<FreeCategoryError> for Error {
    fn from(e: FreeCategoryError) -> Self {
        Self::Graph(e)
    }
}

impl From<hdl_cat::Error> for Error {
    fn from(e: hdl_cat::Error) -> Self {
        Self::Hdl(e)
    }
}
