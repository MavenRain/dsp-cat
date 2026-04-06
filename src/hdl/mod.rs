//! [`hdl_cat`] `Sync` Mealy machine implementations of DSP blocks.
//!
//! Each block is a function returning a [`RawDspBlock`](raw::RawDspBlock)
//! built from pure IR graph construction.  The composed pipeline
//! chains blocks by folding with [`compose_raw`](raw::compose_raw).

pub mod accumulator;
pub mod cic;
pub mod common;
pub mod decimator;
pub mod delay;
pub mod fir;
pub mod gain;
pub mod interpolator;
pub mod pipeline;
pub mod raw;
