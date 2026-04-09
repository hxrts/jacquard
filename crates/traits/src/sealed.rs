//! Shared sealing module for `Send + Sync + 'static` blanket impls.
//!
//! This crate-internal module provides a single `Sealed` trait that is
//! automatically satisfied by every type that is `Send + Sync + 'static`.
//! Traits in this crate use it as a supertrait to prevent external crates
//! from implementing them without satisfying all required bounds.
//!
//! This module is `pub(crate)` only; external crates cannot name or satisfy
//! `Sealed` directly. Traits that require a stricter sealing condition (such as
//! the `Effect` trait, which additionally requires `EffectDefinition`) define
//! their own inline `mod sealed` rather than reusing this one.

pub(crate) trait Sealed {}

impl<T> Sealed for T where T: ?Sized + Send + Sync + 'static {}
