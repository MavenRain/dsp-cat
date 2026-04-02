//! `RustHDL` implementations of DSP blocks.
//!
//! Each block is a [`LogicBlock`](rust_hdl::prelude::LogicBlock) with
//! `mut` quarantined to `Logic::update`.  The composed
//! [`DspPipeline`](pipeline::DspPipeline) chains blocks by wiring
//! the output of block k to the input of block k+1.

pub mod accumulator;
pub mod cic;
pub mod common;
pub mod decimator;
pub mod delay;
pub mod fir;
pub mod gain;
pub mod interpolator;
pub mod pipeline;
