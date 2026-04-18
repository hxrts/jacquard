//! Trait definitions for the abstract routing contract and engine-facing
//! middleware surfaces.
//!
//! `jacquard-traits` defines what components are allowed to do across crate
//! boundaries. It remains runtime-free and names only shared behavioral
//! contracts.
//!
//! ## Runtime-Free Boundary
//!
//! `jacquard-core` and `jacquard-traits` remain runtime-free. They define
//! shared data and behavior contracts, but they must not depend on concrete
//! engine runtimes or telltale runtime internals.
//!
//! ## Effect Capabilities
//!
//! Shared effect traits such as [`TransportSenderEffects`], [`StorageEffects`],
//! [`TimeEffects`], and [`RouteEventLogEffects`] live here. These are neutral
//! host/runtime capabilities, not engine-specific adapter traits.
//!
//! Host-owned supervision surfaces such as [`TransportDriver`] also live here,
//! but they are not effect capabilities and must not be treated as part of the
//! synchronous routing-effect vocabulary. Reusable adapter-support helpers such
//! as raw ingress mailboxes or peer/claim bookkeeping live in
//! `jacquard-adapter`, not in this contract crate.
//!
//! ## Engine And Router Contracts
//!
//! Shared behavioral boundaries such as [`RoutingEngine`],
//! [`RouterManagedEngine`], [`RoutingMiddleware`], and [`RouterEngineRegistry`]
//! live here. Engines implement these contracts; routers orchestrate across
//! them without depending on engine-private runtime details. Pathway-specific
//! read-only extension traits live in `jacquard-pathway`, not here.
//!
//! ## Ownership
//!
//! Router-facing contracts in this crate preserve the ownership split:
//! `jacquard-router` owns canonical route truth, while engines plan, admit,
//! materialize, and maintain route-private runtime state behind shared
//! boundaries. Observational crates may supply world facts and effect handlers,
//! but they must not publish canonical route truth.

#![forbid(unsafe_code)]

/// Expands to `#[must_use = "unread {name} result silently discards
/// {description}"]` applied to the following item.
/// Use on trait methods that return meaningful values whose results must not be
/// silently dropped.
macro_rules! must_use_evidence {
    ($name:literal, $desc:literal; $($item:tt)+) => {
        #[must_use = concat!("unread ", $name, " result silently discards ", $desc)]
        $($item)+
    };
}

extern crate self as jacquard_traits;

mod drivers;
mod effects;
mod handler;
mod hashing;
mod model;
mod routing;
mod simulator;
mod world;

pub use drivers::*;
pub use effects::*;
pub use handler::*;
pub use hashing::*;
pub use jacquard_core;
pub use jacquard_macros::{
    bounded_value, effect_handler, effect_trait, id_type, must_use_handle, public_model, purity,
};
pub use model::*;
pub use routing::*;
pub use simulator::*;
pub use world::*;

// Backing traits for the effect_trait / effect_handler proc macros.
// These are never used directly. The macros emit impls against these
// marker traits so the compiler can enforce that a handler covers
// exactly the effect vocabulary it claims to handle.
#[doc(hidden)]
pub mod __private {
    pub use rust_toolkit_effects::__private::{EffectDefinition, HandlerDefinition, HandlerToken};
}
