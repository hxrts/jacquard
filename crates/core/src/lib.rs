//! Shared identifiers, data types, and constants for Contour routing.

#![forbid(unsafe_code)]

pub use contour_macros::{bounded_value, id_type, must_use_handle, public_model};

mod admission;
mod capabilities;
mod connectivity;
mod constants;
mod content;
mod errors;
mod identity;
mod policy;
mod qualifiers;
mod runtime;
mod time;
mod topology;

pub use admission::*;
pub use capabilities::*;
pub use connectivity::*;
pub use constants::*;
pub use content::*;
pub use errors::*;
pub use identity::*;
pub use policy::*;
pub use qualifiers::*;
pub use runtime::*;
pub use time::*;
pub use topology::*;
