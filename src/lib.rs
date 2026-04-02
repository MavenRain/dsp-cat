//! Composable DSP signal processing pipeline.
//!
//! This crate provides a library of digital signal processing blocks
//! (FIR filter, CIC filter, delay line, gain, decimator, interpolator,
//! accumulator) that compose into pipelines via categorical abstractions
//! from [`comp_cat_rs`].
//!
//! # Architecture
//!
//! The crate is organized in two layers:
//!
//! - **Layer 1 (Pure)**: Domain types ([`sample`]), golden reference
//!   models ([`golden`]), free category graphs ([`graph`]), and
//!   pipeline interpretation ([`interpret`]).  No mutation.
//!
//! - **Layer 2 (HDL)**: `RustHDL` [`LogicBlock`](rust_hdl::prelude::LogicBlock)
//!   implementations ([`hdl`]) with `mut` quarantined to
//!   `Logic::update` methods, and behavioral simulation ([`sim`])
//!   wrapped in [`Io`](comp_cat_rs::effect::io::Io).

pub mod composition;
pub mod error;
pub mod golden;
pub mod graph;
pub mod hdl;
pub mod interpret;
pub mod sample;
pub mod sim;
