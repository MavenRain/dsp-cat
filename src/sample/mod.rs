//! Fixed-point sample types for DSP processing.
//!
//! [`Sample`](element::Sample) is the fundamental data unit flowing
//! through the pipeline, analogous to
//! `GoldilocksElement`
//! in the NTT crate.  [`SampleFormat`](format::SampleFormat) tracks
//! the fixed-point interpretation (bit width and fractional bits)
//! at the pipeline level, not per-sample.

pub mod element;
pub mod format;
