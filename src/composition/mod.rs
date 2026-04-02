//! Pipeline composition operators.
//!
//! [`cascade`] composes blocks sequentially via descriptor composition.
//! [`parallel`] composes independent channels via the monoidal
//! tensor product.

pub mod cascade;
pub mod parallel;
