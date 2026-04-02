//! Free category graphs for DSP block FSMs and pipeline topology.
//!
//! The pipeline topology is a linear chain modeled as a
//! [`Graph`](comp_cat_rs::collapse::free_category::Graph).
//! Individual blocks (FIR, CIC) have their own internal FSM graphs.

pub mod cic_graph;
pub mod fir_graph;
pub mod pipeline_graph;
