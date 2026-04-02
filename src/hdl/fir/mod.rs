//! FIR filter HDL modules.
//!
//! Implements a transposed (systolic) FIR filter with one
//! multiply-accumulate unit per tap.

pub mod fir_filter;
pub mod mac;
pub mod tap_chain;
