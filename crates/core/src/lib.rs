//! Shared identifiers, data types, and constants for Contour routing.

#![forbid(unsafe_code)]

pub use contour_macros::{bounded_value, id_type, must_use_handle, public_model};

mod base;
mod connectivity;
mod content;
mod model;
mod routing;

pub use base::*;
pub use connectivity::*;
pub use content::*;
pub use model::*;
pub use routing::*;
