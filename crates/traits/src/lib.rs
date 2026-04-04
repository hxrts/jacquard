//! Trait definitions for the abstract routing contract and mesh family.

#![forbid(unsafe_code)]

extern crate self as contour_traits;

mod effects;
mod handler;
mod hashing;
mod routing;

pub use contour_core;
pub use contour_macros::{effect_handler, effect_trait};
pub use effects::*;
pub use handler::*;
pub use hashing::*;
pub use routing::*;

// Backing traits for the effect_trait / effect_handler proc macros.
// These are never used directly. The macros emit impls against these
// marker traits so the compiler can enforce that a handler covers
// exactly the effect vocabulary it claims to handle.
#[doc(hidden)]
pub mod __private {
    use core::marker::PhantomData;

    pub trait EffectDefinition {}

    pub trait HandlerDefinition<E: ?Sized> {}

    pub struct HandlerToken<T: ?Sized, E: ?Sized>(pub PhantomData<fn() -> (*const T, *const E)>);
}
