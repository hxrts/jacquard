//! Proc-macro crate entry point for `jacquard-macros`.
//!
//! This crate owns syntax-local code generation and annotation-site validation
//! for the Jacquard workspace. It exposes the following proc-macro attributes:
//!
//! - `#[id_type]` — wraps a single-field tuple struct as an opaque identifier
//!   with canonical derives and `new`/`get` constructors.
//! - `#[bounded_value]` — wraps a single-field tuple struct as a range-bounded
//!   numeric value with a `MAX` constant and a checked `new` constructor.
//! - `#[must_use_handle]` — applies a `#[must_use]` annotation to structs or
//!   enums that represent routing handles or leases.
//! - `#[public_model]` — applies canonical derives to shared model structs and
//!   enums, rejecting forbidden field types (`bool`, floats, `usize`,
//!   wall-clock time types) at compile time.
//! - `#[purity(..)]` — validates trait method receiver shapes against the
//!   declared purity class (`pure`, `read_only`, or `effectful`).
//! - `#[effect_trait]` — stamps a trait as an effect surface and injects the
//!   sealed marker plumbing required by the effect system.
//! - `#[effect_handler]` — stamps an impl block as a concrete effect handler
//!   and injects the `HandlerDefinition` impl that links it to its trait.

#![forbid(unsafe_code)]

mod model;
mod support;
mod traits;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn id_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::id_type::expand(attr, item)
}

#[proc_macro_attribute]
pub fn bounded_value(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::bounded_value::expand(attr, item)
}

#[proc_macro_attribute]
pub fn must_use_handle(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::must_use_handle::expand(attr, item)
}

#[proc_macro_attribute]
pub fn public_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::public_model::expand(attr, item)
}

#[proc_macro_attribute]
pub fn purity(attr: TokenStream, item: TokenStream) -> TokenStream {
    traits::purity::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    traits::effect_trait::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    traits::effect_handler::expand(attr, item)
}
