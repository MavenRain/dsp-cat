//! Pure golden reference models for all DSP blocks.
//!
//! Each module implements a block's algorithm as a pure function
//! operating on [`Sample`](crate::sample::element::Sample) slices.
//! These models serve as the ground truth for verifying the HDL
//! implementations.

pub mod accumulator;
pub mod cic;
pub mod decimator;
pub mod delay;
pub mod fir;
pub mod gain;
pub mod interpolator;
pub mod pipeline;
