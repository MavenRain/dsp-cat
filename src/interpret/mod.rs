//! Pipeline interpretation: descriptors, newtypes, and graph morphisms.
//!
//! The [`GraphMorphism`](comp_cat_rs::collapse::free_category::GraphMorphism)
//! implementation maps the pipeline graph into composable
//! [`DspBlockDescriptor`](descriptor::DspBlockDescriptor) values,
//! bridging the categorical structure and the concrete HDL modules.

pub mod descriptor;
pub mod morphism;
pub mod signal;
