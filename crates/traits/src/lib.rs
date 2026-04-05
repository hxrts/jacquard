//! Trait definitions for the abstract routing contract and mesh family.

#![forbid(unsafe_code)]

extern crate self as jacquard_traits;

mod effects;
mod handler;
mod hashing;
mod routing;

pub use effects::*;
pub use handler::*;
pub use hashing::*;
pub use jacquard_core;
pub use jacquard_macros::{
    bounded_value, effect_handler, effect_trait, id_type, must_use_handle, public_model,
};
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
