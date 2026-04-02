//! Behavioral simulation wrapped in [`Io`](comp_cat_rs::effect::io::Io).
//!
//! The simulation executes the golden model inside `Io::suspend`,
//! keeping the outer interface pure.  Call `run()` only at the
//! outermost boundary.

pub mod runner;
