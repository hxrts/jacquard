//! Shared sealing module for `Send + Sync + 'static` blanket impls.
//!
//! Note: `effects.rs` uses its own inline `mod sealed` because the `Effect`
//! trait's sealing bound gates on `EffectDefinition`, which is a stronger
//! requirement than `Send + Sync + 'static`.

pub(crate) trait Sealed {}

impl<T> Sealed for T where T: ?Sized + Send + Sync + 'static {}
