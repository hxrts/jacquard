//! Proc macros that enforce trait-surface and effect-boundary contracts.
//!
//! This module contains three attribute macros:
//!
//! - `purity` — validates that every method receiver in a trait is consistent
//!   with the declared purity class (`pure`, `read_only`, or `effectful`).
//!   Traits annotated `pure` or `read_only` may not have `&mut self` methods.
//!   Traits annotated `effectful` must have at least one `&mut self` method.
//! - `effect_trait` — marks a trait as an effect surface, injects the required
//!   `Send + Sync + 'static` supertraits, and adds the sealed marker method
//!   used by the effect-handler linkage mechanism.
//! - `effect_handler` — marks a trait impl as a concrete effect handler and
//!   injects the `HandlerDefinition` impl that proves the relationship at
//!   compile time via a `PhantomData`-based marker method.

pub(crate) mod effect_handler;
pub(crate) mod effect_trait;
pub(crate) mod purity;
