//! First-party deterministic mesh routing engine for Jacquard.
//!
//! `engine` is the [`MeshEngine`] state machine that implements the shared
//! [`RoutingEnginePlanner`] and [`RoutingEngine`] contracts. `topology` is the
//! read-only [`MeshTopologyModel`] over shared `Configuration` objects and
//! the mesh-private estimate types. `committee` is the optional
//! [`CommitteeSelector`] used for local coordination when the profile asks
//! for repair plus partition tolerance.
//!
//! [`MeshEngine`]: engine::MeshEngine
//! [`RoutingEnginePlanner`]: jacquard_traits::RoutingEnginePlanner
//! [`RoutingEngine`]: jacquard_traits::RoutingEngine
//! [`MeshTopologyModel`]: jacquard_traits::MeshTopologyModel
//! [`CommitteeSelector`]: jacquard_traits::CommitteeSelector

#![forbid(unsafe_code)]

mod committee;
mod engine;
mod topology;

pub use committee::*;
pub use engine::*;
pub use topology::*;
