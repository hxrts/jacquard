//! Trait definitions for the abstract routing contract and mesh family.

#![forbid(unsafe_code)]

mod effects;
mod hashing;
mod routing;

pub use contour_core;
pub use effects::*;
pub use hashing::*;
pub use routing::*;
