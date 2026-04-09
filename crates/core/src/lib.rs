//! Shared identifiers, data types, and constants for Jacquard routing.
//!
//! `jacquard-core` defines what exists in the shared routing world. It does
//! not own behavioral traits, runtime handlers, or engine-specific protocol
//! logic.
//!
//! ## Connectivity Surface
//!
//! Connectivity facts such as [`LinkEndpoint`], [`TransportIngressEvent`],
//! [`TransportObservation`], [`LinkState`], and [`ConnectivityPosture`] live
//! here as engine-neutral world data. Engines consume this surface; they do
//! not fork it.
//!
//! ## Service Surface
//!
//! Shared service facts such as [`ServiceDescriptor`], [`RouteServiceKind`],
//! [`NodeProfile`], and [`NodeState`] also live here. This surface describes
//! what nodes advertise and what the world currently observes, not what a
//! specific engine decides to do.
//!
//! ## Routing Engine Boundary
//!
//! Core owns the shared result and evidence shapes that cross engine
//! boundaries, including [`RouteCandidate`], [`RouteAdmission`],
//! [`RouteMaterializationProof`], and [`PublishedRouteRecord`]. Engines
//! and routers exchange these values, but the behavioral contracts for using
//! them live in `jacquard-traits`.
//!
//! ## Ownership
//!
//! `jacquard-core` is shared data only. It must not publish canonical route
//! truth, hide runtime mutation behind convenience helpers, or grow behavioral
//! traits that belong in `jacquard-traits`. Canonical route ownership remains
//! above this crate.

#![forbid(unsafe_code)]

pub use jacquard_macros::{bounded_value, id_type, must_use_handle, public_model};

mod authoring;
mod base;
mod connectivity;
mod content;
mod model;
mod routing;

pub use authoring::*;
pub use base::*;
pub use connectivity::*;
pub use content::*;
pub use model::*;
pub use routing::*;
