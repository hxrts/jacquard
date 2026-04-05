//! Cross-cutting primitives and foundational shared types.

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
