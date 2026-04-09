//! Cross-cutting primitives and foundational shared types for jacquard-core.
//!
//! The `base` module is the lowest layer of `jacquard-core`. It provides the
//! building blocks that every other module in this crate depends on. Submodules
//! cover: constants (capacity and dimension bounds), errors (the shared error
//! enum hierarchy), identity (all node, route, and scope identifier newtypes),
//! qualifiers (belief, observation, estimate, and fact wrappers), and time
//! (the deterministic time model and integer-scaled metric types).
//!
//! The `bytes_newtype!` macro is defined here and re-exported to sibling
//! modules so they can declare fixed-size byte-array newtypes with the
//! standard `#[id_type]` derives in one line. No behavioral traits or
//! runtime-dependent code belongs in this module.

/// Fixed-size byte-array newtype with standard derives via `id_type`.
macro_rules! bytes_newtype {
    ($name:ident, $size:expr) => {
        #[id_type]
        pub struct $name(pub [u8; $size]);
    };
}

// Make available to sibling modules within `base` and to `content.rs`.
pub(crate) use bytes_newtype;

mod constants;
mod errors;
mod identity;
mod qualifiers;
mod time;

pub use constants::*;
pub use errors::*;
pub use identity::*;
pub use qualifiers::*;
pub use time::*;
