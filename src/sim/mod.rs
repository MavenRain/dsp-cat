//! Simulation wrapped in [`Io`](comp_cat_rs::effect::io::Io).
//!
//! The golden-model simulation executes inside `Io::suspend`,
//! keeping the outer interface pure.  Call `run()` only at the
//! outermost boundary.  The [`hdl_cat`] `Testbench` path is
//! available via [`crate::hdl::pipeline::build_pipeline`].

pub mod runner;
