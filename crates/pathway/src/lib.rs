//! First-party deterministic explicit-path routing engine for Jacquard.
//!
//! `engine` is the [`PathwayEngine`] state machine that implements the shared
//! [`RoutingEnginePlanner`] and [`RoutingEngine`] contracts. `contracts` holds
//! the pathway-specific read-only extension points that stay pathway-owned
//! rather than leaking into `jacquard-traits`. `topology` is the read-only
//! [`PathwayTopologyModel`] over shared `Configuration` objects and the
//! pathway-private estimate types. `committee` is the optional
//! [`CommitteeSelector`] used for local coordination when the profile asks for
//! repair plus partition tolerance. `choreography` is the internal
//! Telltale-backed protocol surface that will gradually absorb cooperative
//! pathway runtime behavior without changing the public engine-neutral routing
//! traits.
//!
//! [`PathwayEngine`]: engine::PathwayEngine
//! [`RoutingEnginePlanner`]: jacquard_traits::RoutingEnginePlanner
//! [`RoutingEngine`]: jacquard_traits::RoutingEngine
//! [`PathwayTopologyModel`]: crate::PathwayTopologyModel
//! [`CommitteeSelector`]: jacquard_traits::CommitteeSelector

#![forbid(unsafe_code)]

mod choreography;
mod committee;
mod contracts;
mod engine;
mod planner_model;
mod topology;
#[cfg(test)]
mod validation;

pub use committee::*;
pub use contracts::*;
pub use engine::*;
pub use planner_model::{PathwayPlannerModel, PathwayPlannerSeed};
pub use topology::*;
